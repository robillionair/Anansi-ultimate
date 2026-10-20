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
) -> Result<TcpStream, ProviderError> {
    let addr = format!("{}:{}", ep.host, ep.port)
        .to_socket_addrs()
        .map_err(|e| {
            ProviderError::new(
                ProviderErrorCode::ConnectionLost,
                format!("cannot resolve {}: {e}", ep.host),
            )
            .pre_send()
        })?
        .next()
        .ok_or_else(|| {
            ProviderError::new(
                ProviderErrorCode::ConnectionLost,
                format!("cannot resolve {}", ep.host),
            )
            .pre_send()
        })?;
    let stream = TcpStream::connect_timeout(&addr, timeouts.connect()).map_err(|e| {
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
    let _ = stream.set_write_timeout(Some(timeouts.idle()));
    let _ = stream.set_read_timeout(Some(timeouts.first_byte()));
    Ok(stream)
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
        let stream = resolve_and_connect(ep, timeouts, cancel)?;
        run_over(Conn::Plain(stream), request, timeouts, cancel)
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
        let mut sock = resolve_and_connect(ep, timeouts, cancel)?;
        if ep.scheme != "https" {
            return run_over(Conn::Plain(sock), request, timeouts, cancel);
        }
        let server_name = crate::tls::server_name(&ep.host)?;
        let mut tls_conn = rustls::ClientConnection::new(self.config.clone(), server_name)
            .map_err(|e| {
                ProviderError::new(
                    ProviderErrorCode::TlsError,
                    format!("TLS setup for {} failed: {e}", ep.authority()),
                )
                .pre_send()
            })?;
        // Drive the handshake explicitly, before any request bytes
        // exist: certificate and hostname validation failures surface
        // here as TlsError (pre-send), never as a half-sent request.
        while tls_conn.is_handshaking() {
            if cancel.is_cancelled() {
                return Err(ProviderError::new(
                    ProviderErrorCode::Cancelled,
                    "request cancelled during TLS handshake",
                ));
            }
            tls_conn
                .complete_io(&mut sock)
                .map_err(|e| map_handshake_error(e, ep, cancel))?;
        }
        let stream = rustls::StreamOwned::new(tls_conn, sock);
        run_over(Conn::Tls(Box::new(stream)), request, timeouts, cancel)
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
) -> Result<HttpResponse, ProviderError> {
    let ep = &request.endpoint;
    let mut stream = conn;
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
            if start.elapsed() > timeouts.first_byte() {
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
            start,
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
