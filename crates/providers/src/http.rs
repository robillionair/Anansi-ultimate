//! Bounded HTTP/1.1 transport over `std::net`, following the pattern
//! `anansi-core` already uses for loopback inference (`ask.rs`,
//! `vector.rs`): no HTTP-client dependency, no redirect following,
//! size-capped headers/bodies, chunked-transfer decoding, and
//! cancellation that shuts down the live socket (the same semantics as
//! `ask_stream.rs`: cancelling closes the backend connection).
//!
//! TLS: `std` provides none, so HTTPS is served by
//! [`RustlsTransport`] — rustls with full WebPKI certificate and
//! hostname validation (see `crate::tls`; the M5-D3 follow-up that
//! supersedes the original fail-closed note in the handover). The
//! plaintext [`StdHttpTransport`] remains for loopback-only use and
//! still fails closed with `TlsUnavailable` on HTTPS endpoints.
//! Both transports share the bounded request/response machinery in
//! this module; redirects are never followed by either.
//!
//! **Overall deadline:** `TimeoutConfig::total_ms` is one deadline
//! for a whole execution — DNS resolution, connect, TLS handshake,
//! request write and response read all share the clock that starts
//! in `execute` (see `Deadline`). Per-phase timeouts still apply
//! within it; the deadline is what stops a peer that dribbles bytes
//! slowly enough to never trip a per-read timeout. DNS resolution
//! and the TLS handshake run on worker threads so the caller can
//! enforce that deadline (and cancellation) by closing the socket:
//! `std`'s resolver offers no timeout or cancellation hook at all,
//! so a resolution that outlives its wait is abandoned — the worker
//! holds only the hostname and exits when the OS resolver returns.

use crate::config::{TimeoutConfig, ValidatedEndpoint};
use crate::error::{ProviderError, ProviderErrorCode};
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct CancelInner {
    flag: AtomicBool,
    sockets: Mutex<Vec<TcpStream>>,
}

/// Cooperative cancellation: setting the token shuts down every
/// registered socket, so a blocked read unblocks immediately instead of
/// waiting out its timeout.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    inner: Arc<CancelInner>,
}

impl Default for CancelInner {
    fn default() -> Self {
        Self {
            flag: AtomicBool::new(false),
            sockets: Mutex::new(Vec::new()),
        }
    }
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn is_cancelled(&self) -> bool {
        self.inner.flag.load(Ordering::SeqCst)
    }
    pub fn cancel(&self) {
        self.inner.flag.store(true, Ordering::SeqCst);
        if let Ok(sockets) = self.inner.sockets.lock() {
            for s in sockets.iter() {
                let _ = s.shutdown(std::net::Shutdown::Both);
            }
        }
    }
    fn register(&self, stream: &TcpStream) {
        if let Ok(clone) = stream.try_clone()
            && let Ok(mut sockets) = self.inner.sockets.lock()
        {
            sockets.push(clone);
        }
        if self.is_cancelled() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    }
    fn check(&self) -> Result<(), ProviderError> {
        if self.is_cancelled() {
            Err(ProviderError::new(
                ProviderErrorCode::Cancelled,
                "request cancelled by caller",
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub endpoint: ValidatedEndpoint,
    /// Path including the endpoint base path; may carry an adapter-built
    /// query string (e.g. Azure `api-version`). Never user-typed secrets.
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    body: BodyReader,
}

impl HttpResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
    /// Read the whole body with a hard cap (non-streaming responses).
    pub fn read_body_capped(&mut self, cap: usize) -> Result<Vec<u8>, ProviderError> {
        let mut out = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            let n = self.body.read(&mut chunk).map_err(map_io_error)?;
            if n == 0 {
                break;
            }
            if out.len() + n > cap {
                return Err(ProviderError::new(
                    ProviderErrorCode::MalformedResponse,
                    format!("response body exceeds the {cap}-byte bound"),
                ));
            }
            out.extend_from_slice(&chunk[..n]);
        }
        Ok(out)
    }
    /// Take the streaming body reader (consumes the response).
    pub fn into_body(self) -> BodyReader {
        self.body
    }
}

/// One overall deadline for a single transport execution, shared by
/// every phase (resolution, connect, handshake, write, read). The
/// per-phase timeouts in `TimeoutConfig` bound inactivity within a
/// phase; this bounds the whole, so slow-drip peers cannot stretch
/// an execution indefinitely.
#[derive(Debug, Clone, Copy)]
struct Deadline {
    start: Instant,
    total: Duration,
}

impl Deadline {
    fn new(timeouts: &TimeoutConfig) -> Self {
        Self {
            start: Instant::now(),
            total: timeouts.total(),
        }
    }
    fn start(&self) -> Instant {
        self.start
    }
    fn remaining(&self) -> Duration {
        self.total.saturating_sub(self.start.elapsed())
    }
    fn expired(&self) -> bool {
        self.start.elapsed() >= self.total
    }
}

pub trait Transport {
    fn execute(
        &self,
        request: &HttpRequest,
        timeouts: &TimeoutConfig,
        cancel: &CancellationToken,
    ) -> Result<HttpResponse, ProviderError>;
}

/// A live connection: plaintext TCP, or a rustls TLS session over
/// TCP. Read/write timeouts live on the raw socket in both cases,
/// and the cancellation token is registered on the raw socket, so
/// cancelling closes TLS sessions exactly like plaintext ones.
enum Conn {
    Plain(TcpStream),
    Tls(Box<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>),
}

impl Conn {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        match self {
            Conn::Plain(s) => s.set_read_timeout(timeout),
            Conn::Tls(s) => s.sock.set_read_timeout(timeout),
        }
    }
}

impl Read for Conn {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        match self {
            Conn::Plain(s) => s.read(out),
            Conn::Tls(s) => s.read(out),
        }
    }
}

impl Write for Conn {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Conn::Plain(s) => s.write(buf),
            Conn::Tls(s) => s.write(buf),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Conn::Plain(s) => s.flush(),
            Conn::Tls(s) => s.flush(),
        }
    }
}

/// Resolve once and connect to the resolved address (the Host header
/// and the TLS server name still carry the configured host). Every
/// failure here is pre-send: no request bytes can have reached the
/// provider, and errors are marked accordingly for retry safety.
fn resolve_and_connect(
    ep: &ValidatedEndpoint,
    timeouts: &TimeoutConfig,
    cancel: &CancellationToken,
    deadline: &Deadline,
) -> Result<TcpStream, ProviderError> {
    let addr = resolve_with_deadline(ep, timeouts, cancel, deadline)?;
    let connect_wait = timeouts.connect().min(deadline.remaining());
    if connect_wait.is_zero() {
        return Err(ProviderError::new(
            ProviderErrorCode::Timeout,
            format!(
                "overall request deadline expired before connecting to {}",
                ep.authority()
            ),
        )
        .pre_send());
    }
    let stream = TcpStream::connect_timeout(&addr, connect_wait).map_err(|e| {
        if e.kind() == io::ErrorKind::TimedOut {
            ProviderError::new(
                ProviderErrorCode::Timeout,
                format!("connect to {} timed out", ep.authority()),
            )
            .pre_send()
        } else {
            ProviderError::new(
                ProviderErrorCode::ConnectionLost,
                format!("connect to {} failed: {e}", ep.authority()),
            )
            .pre_send()
        }
    })?;
    cancel.register(&stream);
    let _ = stream.set_nodelay(true);
    let _ = stream.set_write_timeout(Some(timeouts.idle().min(deadline.remaining())));
    let _ = stream.set_read_timeout(Some(timeouts.first_byte().min(deadline.remaining())));
    Ok(stream)
}

/// Resolve a host under the overall deadline and the cancellation
/// token. `std::net::ToSocketAddrs` is a blocking OS call with no
/// timeout and no cancellation hook — the documented std limitation
/// — so it runs on a worker thread and only the *wait* is bounded:
/// on expiry or cancellation the worker is abandoned (it owns just
/// the hostname string; no request or credential material) and exits
/// whenever the OS resolver returns. The dispatch itself is never
/// held hostage by a stuck resolver.
fn resolve_with_deadline(
    ep: &ValidatedEndpoint,
    timeouts: &TimeoutConfig,
    cancel: &CancellationToken,
    deadline: &Deadline,
) -> Result<std::net::SocketAddr, ProviderError> {
    let (tx, rx) = std::sync::mpsc::channel();
    let target = format!("{}:{}", ep.host, ep.port);
    std::thread::spawn(move || {
        let _ = tx.send(target.to_socket_addrs().map(|mut it| it.next()));
    });
    let budget = timeouts.connect().min(deadline.remaining());
    let started = Instant::now();
    loop {
        let left = budget.saturating_sub(started.elapsed());
        if left.is_zero() {
            return Err(ProviderError::new(
                ProviderErrorCode::Timeout,
                format!(
                    "DNS resolution of {} exceeded the request deadline",
                    ep.host
                ),
            )
            .pre_send());
        }
        match rx.recv_timeout(left.min(Duration::from_millis(25))) {
            Ok(Ok(Some(addr))) => return Ok(addr),
            Ok(Ok(None)) => {
                return Err(ProviderError::new(
                    ProviderErrorCode::ConnectionLost,
                    format!("cannot resolve {}", ep.host),
                )
                .pre_send());
            }
            Ok(Err(e)) => {
                return Err(ProviderError::new(
                    ProviderErrorCode::ConnectionLost,
                    format!("cannot resolve {}: {e}", ep.host),
                )
                .pre_send());
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                cancel.check()?;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err(ProviderError::new(
                    ProviderErrorCode::ConnectionLost,
                    format!("DNS resolution of {} failed", ep.host),
                )
                .pre_send());
            }
        }
    }
}

/// The plaintext-only transport: loopback and plaintext-opt-in
/// custom endpoints. HTTPS fails closed with `TlsUnavailable`;
/// use [`RustlsTransport`] for production dispatch.
#[derive(Debug, Default)]
pub struct StdHttpTransport;

impl Transport for StdHttpTransport {
    fn execute(
        &self,
        request: &HttpRequest,
        timeouts: &TimeoutConfig,
        cancel: &CancellationToken,
    ) -> Result<HttpResponse, ProviderError> {
        cancel.check()?;
        let ep = &request.endpoint;
        if ep.scheme == "https" {
            return Err(ProviderError::new(
                ProviderErrorCode::TlsUnavailable,
                format!(
                    "endpoint {} is https but this transport has no TLS; use RustlsTransport (see M5-D3)",
                    ep.origin()
                ),
            )
            .pre_send());
        }
        let deadline = Deadline::new(timeouts);
        let stream = resolve_and_connect(ep, timeouts, cancel, &deadline)?;
        run_over(Conn::Plain(stream), request, timeouts, cancel, &deadline)
    }
}

/// The production transport: plaintext where endpoint policy allows
/// it, HTTPS via rustls with certificate-chain, expiry and hostname
/// validation always on (see `crate::tls` — there is no insecure
/// mode). Credentials are placed in headers bound to this one
/// request by the adapters; this transport never follows redirects,
/// so a credential can never be forwarded to a different origin.
#[derive(Debug, Clone)]
pub struct RustlsTransport {
    config: std::sync::Arc<rustls::ClientConfig>,
}

impl Default for RustlsTransport {
    fn default() -> Self {
        Self {
            config: crate::tls::TlsConfig::new().client_config(),
        }
    }
}

impl RustlsTransport {
    /// Mozilla root set, no additions.
    pub fn new() -> Self {
        Self {
            config: crate::tls::TlsConfig::new().client_config(),
        }
    }
    /// Custom trust configuration (e.g. an added private-CA root).
    /// Validation itself is never optional.
    pub fn with_tls_config(config: crate::tls::TlsConfig) -> Self {
        Self {
            config: config.client_config(),
        }
    }
}

impl Transport for RustlsTransport {
    fn execute(
        &self,
        request: &HttpRequest,
        timeouts: &TimeoutConfig,
        cancel: &CancellationToken,
    ) -> Result<HttpResponse, ProviderError> {
        cancel.check()?;
        let ep = &request.endpoint;
        let deadline = Deadline::new(timeouts);
        let sock = resolve_and_connect(ep, timeouts, cancel, &deadline)?;
        if ep.scheme != "https" {
            return run_over(Conn::Plain(sock), request, timeouts, cancel, &deadline);
        }
        let stream = tls_handshake(&self.config, ep, sock, timeouts, &deadline, cancel)?;
        run_over(
            Conn::Tls(Box::new(stream)),
            request,
            timeouts,
            cancel,
            &deadline,
        )
    }
}

/// Drive the TLS handshake on a worker thread so the overall
/// deadline (and cancellation) bind it absolutely: per-read socket
/// timeouts cannot stop a peer that dribbles handshake bytes one at
/// a time — every read returns promptly, yet the flight never
/// completes. The waiter below enforces the deadline by closing the
/// socket, which fails the worker's I/O and ends it; validation
/// failures the worker reports are mapped by `map_handshake_error`.
/// Either way, no request byte is written before validation passes.
fn tls_handshake(
    config: &std::sync::Arc<rustls::ClientConfig>,
    ep: &ValidatedEndpoint,
    mut sock: TcpStream,
    timeouts: &TimeoutConfig,
    deadline: &Deadline,
    cancel: &CancellationToken,
) -> Result<rustls::StreamOwned<rustls::ClientConnection, TcpStream>, ProviderError> {
    let server_name = crate::tls::server_name(&ep.host)?;
    let conn = rustls::ClientConnection::new(config.clone(), server_name).map_err(|e| {
        ProviderError::new(
            ProviderErrorCode::TlsError,
            format!("TLS setup for {} failed: {e}", ep.authority()),
        )
        .pre_send()
    })?;
    // Per-read inactivity bound inside the worker. The total bound is
    // the wait loop's job.
    let read_wait = timeouts
        .first_byte()
        .min(deadline.remaining())
        .max(Duration::from_millis(1));
    let _ = sock.set_read_timeout(Some(read_wait));
    let force_close = sock.try_clone().map_err(|e| {
        ProviderError::new(
            ProviderErrorCode::ConnectionLost,
            format!("cannot prepare TLS socket for {}: {e}", ep.authority()),
        )
        .pre_send()
    })?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut conn = conn;
        let outcome = loop {
            if !conn.is_handshaking() {
                break Ok(rustls::StreamOwned::new(conn, sock));
            }
            match conn.complete_io(&mut sock) {
                Ok(_) => continue,
                Err(e) => break Err(e),
            }
        };
        let _ = tx.send(outcome);
    });
    loop {
        if cancel.is_cancelled() {
            let _ = force_close.shutdown(std::net::Shutdown::Both);
            return Err(ProviderError::new(
                ProviderErrorCode::Cancelled,
                "request cancelled during TLS handshake",
            ));
        }
        let remaining = deadline.remaining();
        if remaining.is_zero() {
            let _ = force_close.shutdown(std::net::Shutdown::Both);
            return Err(ProviderError::new(
                ProviderErrorCode::Timeout,
                format!(
                    "TLS handshake with {} exceeded the overall request deadline",
                    ep.authority()
                ),
            )
            .pre_send());
        }
        match rx.recv_timeout(remaining.min(Duration::from_millis(25))) {
            Ok(Ok(stream)) => return Ok(stream),
            Ok(Err(e)) => return Err(map_handshake_error(e, ep, cancel)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err(ProviderError::new(
                    ProviderErrorCode::TlsError,
                    format!("TLS handshake with {} failed unexpectedly", ep.authority()),
                )
                .pre_send());
            }
        }
    }
}

fn map_handshake_error(
    e: io::Error,
    ep: &ValidatedEndpoint,
    cancel: &CancellationToken,
) -> ProviderError {
    if cancel.is_cancelled() {
        return ProviderError::new(
            ProviderErrorCode::Cancelled,
            "request cancelled during TLS handshake",
        );
    }
    match e.kind() {
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => ProviderError::new(
            ProviderErrorCode::Timeout,
            format!("TLS handshake with {} timed out", ep.authority()),
        )
        .pre_send(),
        _ => {
            // rustls validation failures arrive as io errors carrying
            // the rustls error as their source; its Display names the
            // certificate problem and contains no credential material.
            let detail = e
                .get_ref()
                .map(|src| src.to_string())
                .unwrap_or_else(|| e.to_string());
            let detail: String = detail.chars().take(240).collect();
            ProviderError::new(
                ProviderErrorCode::TlsError,
                format!("TLS handshake with {} failed: {detail}", ep.authority()),
            )
            .pre_send()
        }
    }
}

const MAX_HEADER_BYTES: usize = 64 * 1024;

/// Write the request and read the response over an established
/// connection (either kind). Shared by both transports.
fn run_over(
    conn: Conn,
    request: &HttpRequest,
    timeouts: &TimeoutConfig,
    cancel: &CancellationToken,
    deadline: &Deadline,
) -> Result<HttpResponse, ProviderError> {
    let ep = &request.endpoint;
    let mut stream = conn;
    let _ = stream.set_read_timeout(Some(
        timeouts
            .first_byte()
            .min(deadline.remaining())
            .max(Duration::from_millis(1)),
    ));
    {
        let mut wire = Vec::with_capacity(request.body.len() + 1024);
        wire.extend_from_slice(
            format!("{} {} HTTP/1.1\r\n", request.method, request.path).as_bytes(),
        );
        wire.extend_from_slice(format!("Host: {}\r\n", ep.authority()).as_bytes());
        wire.extend_from_slice(b"User-Agent: anansi-providers/0.1\r\n");
        wire.extend_from_slice(b"Accept: application/json, text/event-stream\r\n");
        wire.extend_from_slice(b"Connection: close\r\n");
        for (name, value) in &request.headers {
            if name.contains(['\r', '\n', ':']) || value.contains(['\r', '\n']) {
                return Err(ProviderError::new(
                    ProviderErrorCode::MalformedResponse,
                    "header contains forbidden characters",
                ));
            }
            wire.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
        }
        wire.extend_from_slice(
            format!("Content-Length: {}\r\n\r\n", request.body.len()).as_bytes(),
        );
        wire.extend_from_slice(&request.body);
        stream.write_all(&wire).map_err(map_io_error)?;
        cancel.check()?;

        // Read status line + headers (capped), keeping any over-read
        // bytes as the body prefix.
        let start = Instant::now();
        let mut raw: Vec<u8> = Vec::new();
        let mut buf = [0u8; 8192];
        let header_end;
        loop {
            if start.elapsed() > timeouts.first_byte() || deadline.expired() {
                return Err(ProviderError::new(
                    ProviderErrorCode::Timeout,
                    "timed out waiting for the provider response headers",
                ));
            }
            cancel.check()?;
            let n = stream.read(&mut buf).map_err(map_io_error)?;
            if n == 0 {
                return Err(ProviderError::new(
                    ProviderErrorCode::ConnectionLost,
                    "connection closed before response headers",
                ));
            }
            raw.extend_from_slice(&buf[..n]);
            if let Some(pos) = find_subslice(&raw, b"\r\n\r\n") {
                header_end = pos + 4;
                break;
            }
            if raw.len() > MAX_HEADER_BYTES {
                return Err(ProviderError::new(
                    ProviderErrorCode::MalformedResponse,
                    "response headers exceed the 64 KiB bound",
                ));
            }
        }
        let header_text = String::from_utf8_lossy(&raw[..header_end]).to_string();
        let mut lines = header_text.lines();
        let status_line = lines.next().unwrap_or("");
        let status: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| {
                ProviderError::new(
                    ProviderErrorCode::MalformedResponse,
                    format!("unparseable status line: {}", excerpt_line(status_line)),
                )
            })?;
        let mut headers = Vec::new();
        for line in lines {
            if let Some((k, v)) = line.split_once(':') {
                headers.push((k.trim().to_string(), v.trim().to_string()));
            }
        }
        let prefix = raw[header_end..].to_vec();
        let socket = SocketBody {
            stream,
            prefix,
            // The body phase shares the execution's overall clock:
            // `total_ms` spans connect, handshake, write and read.
            start: deadline.start(),
            timeouts: timeouts.clamped(),
            cancel: cancel.clone(),
            first_read_done: true,
        };
        let body = make_body_reader(&headers, socket)?;
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

fn excerpt_line(s: &str) -> String {
    s.chars().take(120).collect()
}

pub fn map_io_error(e: io::Error) -> ProviderError {
    match e.kind() {
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => ProviderError::new(
            ProviderErrorCode::Timeout,
            format!("provider request timed out: {e}"),
        ),
        io::ErrorKind::Interrupted => ProviderError::new(
            ProviderErrorCode::Cancelled,
            "provider request interrupted (cancelled)",
        ),
        _ => ProviderError::new(
            ProviderErrorCode::ConnectionLost,
            format!("connection error: {e}"),
        ),
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Socket body with total-deadline and cancellation enforcement on
/// every read.
pub struct SocketBody {
    stream: Conn,
    prefix: Vec<u8>,
    start: Instant,
    timeouts: TimeoutConfig,
    cancel: CancellationToken,
    first_read_done: bool,
}

impl Read for SocketBody {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if !self.prefix.is_empty() {
            let n = self.prefix.len().min(out.len());
            out[..n].copy_from_slice(&self.prefix[..n]);
            self.prefix.drain(..n);
            return Ok(n);
        }
        if self.cancel.is_cancelled() {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
        }
        let elapsed = self.start.elapsed();
        let total = self.timeouts.total();
        if elapsed >= total {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "total request deadline exceeded",
            ));
        }
        let remaining = total - elapsed;
        let wait = self.timeouts.idle().min(remaining);
        let _ = self.stream.set_read_timeout(Some(wait));
        self.first_read_done = true;
        match self.stream.read(out) {
            Ok(n) => Ok(n),
            Err(e) => Err(e),
        }
    }
}

enum BodyKind {
    Length(u64),
    Chunked,
    UntilClose,
}

/// Streaming body reader: transparently decodes chunked transfer
/// encoding; enforces the socket body's deadlines via the inner reader.
pub struct BodyReader {
    inner: SocketBody,
    kind: BodyKind,
    // chunked state
    chunk_remaining: u64,
    chunk_started: bool,
    finished: bool,
    buffer: Vec<u8>,
    buffer_pos: usize,
}

fn make_body_reader(
    headers: &[(String, String)],
    inner: SocketBody,
) -> Result<BodyReader, ProviderError> {
    let mut kind = BodyKind::UntilClose;
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("transfer-encoding") && v.to_ascii_lowercase().contains("chunked")
        {
            kind = BodyKind::Chunked;
        }
        if k.eq_ignore_ascii_case("content-length")
            && let Ok(n) = v.trim().parse::<u64>()
        {
            kind = BodyKind::Length(n);
        }
    }
    Ok(BodyReader {
        inner,
        kind,
        chunk_remaining: 0,
        chunk_started: false,
        finished: false,
        buffer: Vec::new(),
        buffer_pos: 0,
    })
}

impl BodyReader {
    fn fill_buffer(&mut self) -> io::Result<bool> {
        if self.buffer_pos < self.buffer.len() {
            return Ok(true);
        }
        self.buffer.clear();
        self.buffer_pos = 0;
        let mut tmp = [0u8; 8192];
        let n = self.inner.read(&mut tmp)?;
        if n == 0 {
            return Ok(false);
        }
        self.buffer.extend_from_slice(&tmp[..n]);
        Ok(true)
    }

    fn read_line(&mut self) -> io::Result<Vec<u8>> {
        let mut line = Vec::new();
        loop {
            if !self.fill_buffer()? {
                break;
            }
            while self.buffer_pos < self.buffer.len() {
                let b = self.buffer[self.buffer_pos];
                self.buffer_pos += 1;
                line.push(b);
                if b == b'\n' {
                    return Ok(line);
                }
                if line.len() > 8192 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "chunk header line too long",
                    ));
                }
            }
        }
        Ok(line)
    }
}

impl Read for BodyReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        match self.kind {
            BodyKind::Length(ref mut remaining) => {
                if *remaining == 0 {
                    return Ok(0);
                }
                let want = (*remaining).min(out.len() as u64) as usize;
                let n = self.inner.read(&mut out[..want])?;
                *remaining -= n as u64;
                Ok(n)
            }
            BodyKind::UntilClose => self.inner.read(out),
            BodyKind::Chunked => {
                if self.finished {
                    return Ok(0);
                }
                loop {
                    if self.chunk_remaining > 0 {
                        if !self.fill_buffer()? {
                            self.finished = true;
                            return Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "chunked body truncated",
                            ));
                        }
                        let avail = self.buffer.len() - self.buffer_pos;
                        let take = (self.chunk_remaining as usize).min(avail).min(out.len());
                        out[..take]
                            .copy_from_slice(&self.buffer[self.buffer_pos..self.buffer_pos + take]);
                        self.buffer_pos += take;
                        self.chunk_remaining -= take as u64;
                        if self.chunk_remaining == 0 {
                            // Consume trailing CRLF.
                            let _ = self.read_line()?;
                        }
                        return Ok(take);
                    }
                    // Need the next chunk header.
                    let line = self.read_line()?;
                    if line.is_empty() {
                        self.finished = true;
                        return Ok(0);
                    }
                    let text = String::from_utf8_lossy(&line);
                    let size_str = text.trim().split(';').next().unwrap_or("");
                    let size = u64::from_str_radix(size_str, 16).map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, "invalid chunk size")
                    })?;
                    if size == 0 {
                        // Trailer section: consume until an empty line.
                        loop {
                            let trailer = self.read_line()?;
                            if trailer.is_empty() || trailer == b"\r\n" || trailer == b"\n" {
                                break;
                            }
                        }
                        self.finished = true;
                        return Ok(0);
                    }
                    self.chunk_remaining = size;
                    self.chunk_started = true;
                }
            }
        }
    }
}
