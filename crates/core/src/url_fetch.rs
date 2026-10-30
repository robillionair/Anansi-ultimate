//! Backend-owned bounded URL text fetching for LEXI-COLLIDER.
//!
//! The browser never fetches a URL itself. The UI calls the
//! `collider-fetch-url` operation, which runs here: the URL is
//! validated, its host is resolved and every resolved address is
//! classified, and the connection goes to a validated address —
//! never to a re-resolved name — so a DNS answer that changes
//! between validation and connection (DNS rebinding) cannot
//! redirect the request to a private destination. Redirect
//! targets are re-validated by the same rules before connecting.
//!
//! Transport is synchronous `std::net::TcpStream` plus rustls
//! (ring + Mozilla roots, exactly the providers crate's TLS
//! posture). Certificate and hostname validation are rustls
//! defaults; there is no danger API, no custom verifier, and
//! no flag that weakens TLS. No cookies, authorization headers,
//! referer, or browser-session forwarding are ever sent, and
//! no JavaScript runs: HTML extraction reads the single
//! response body only and never requests external resources.
//!
//! Bounds (see the M8 URL input design and the URL input
//! repair design): at most 5 redirects, 5s connect, one
//! 20s total deadline across DNS, connect, TLS handshake,
//! writes, headers, bodies, trailers, and redirects —
//! every socket wait uses min(remaining, 10s), so elapsed
//! time past the deadline is bounded by scheduling
//! granularity only. The deadline is enforced beneath
//! rustls too: the TLS transport checks it on every
//! underlying socket read/write, so assembling one
//! encrypted record cannot outlive the deadline — 32 KiB
//! of headers, 1 MiB of body,
//! extracted text 1–8,000 chars. Anything outside the
//! bounds refuses with a generic error that never includes
//! resolved addresses or response bodies.

use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Largest downloaded response body, in bytes.
pub const MAX_FETCH_BYTES: u64 = 1_048_576;
/// Largest accepted header block, in bytes.
const MAX_HEADER_BYTES: usize = 32_768;
/// Largest accepted submitted URL, in characters.
const MAX_URL_CHARS: usize = 2_048;
/// Largest extracted text, in characters (the beam bound).
const MAX_TEXT_CHARS: usize = 8_000;
/// Maximum followed redirects.
const MAX_REDIRECTS: usize = 5;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const READ_TIMEOUT: Duration = Duration::from_secs(10);
const TOTAL_BUDGET: Duration = Duration::from_secs(20);
const DNS_TIMEOUT: Duration = Duration::from_secs(5);

/// Destination policy for one fetch.
///
/// The production policy is `strict()`: loopback refused,
/// non-default ports refused. The only relaxations are the
/// two flags below, and they are test-only: `allow_loopback`
/// lets a synthetic loopback server answer (the browser
/// end-to-end harness), and `allow_non_default_ports` lets
/// that server listen on an ephemeral port. Private,
/// link-local, reserved, documentation, multicast, and
/// unspecified addresses are never allowed by any policy.
#[derive(Debug, Clone, Copy)]
pub struct UrlFetchPolicy {
    pub allow_loopback: bool,
    pub allow_non_default_ports: bool,
    /// Total deadline budget for one fetch. Production
    /// uses 20s; tests may shorten it to prove deadline
    /// enforcement quickly (never lengthened).
    pub total_timeout: Duration,
}

impl UrlFetchPolicy {
    pub fn strict() -> Self {
        Self {
            allow_loopback: false,
            allow_non_default_ports: false,
            total_timeout: TOTAL_BUDGET,
        }
    }

    /// Test-only policy for synthetic loopback servers.
    pub fn test_loopback() -> Self {
        Self {
            allow_loopback: true,
            allow_non_default_ports: true,
            total_timeout: TOTAL_BUDGET,
        }
    }

    /// Shorten (never lengthen) the total budget. Test
    /// use only: it proves the deadline mechanism without
    /// waiting out the production budget.
    pub fn with_total_timeout(mut self, total: Duration) -> Self {
        self.total_timeout = total.min(TOTAL_BUDGET);
        self
    }
}

/// Select the fetch policy for one operation. The
/// loopback test policy is available only when the
/// caller build allows it (test or debug builds); in
/// any other build the policy is unconditionally
/// strict, so an ambient environment variable cannot
/// relax a production build.
pub fn policy_for_fetch(env_flag: bool, allow_test_policy: bool) -> UrlFetchPolicy {
    if env_flag && allow_test_policy {
        UrlFetchPolicy::test_loopback()
    } else {
        UrlFetchPolicy::strict()
    }
}

/// One total deadline spanning every phase of a fetch:
/// DNS, connect, TLS handshake, request write, header
/// lines, chunk headers and bodies, trailers,
/// close-delimited bodies, and every redirect hop.
#[derive(Debug, Clone, Copy)]
struct Deadline {
    end: Instant,
}

impl Deadline {
    fn new(total: Duration) -> Self {
        Self {
            end: Instant::now() + total,
        }
    }

    fn remaining(&self) -> Duration {
        self.end.saturating_duration_since(Instant::now())
    }

    fn check(&self) -> Result<(), String> {
        if Instant::now() >= self.end {
            Err(err("Fetching the URL timed out."))
        } else {
            Ok(())
        }
    }

    /// Socket timeout for the next wait: the per-call
    /// cap, shrunk to the remaining budget, so a wait
    /// that starts late cannot run past the deadline.
    fn socket_timeout(&self) -> Duration {
        self.remaining().min(READ_TIMEOUT)
    }

    fn past(&self) -> bool {
        Instant::now() >= self.end
    }
}

/// The outcome of one bounded fetch.
#[derive(Debug, Clone)]
pub struct UrlFetchResult {
    /// User-entered URL, trimmed, verbatim.
    pub submitted_url: String,
    /// Serialized URL of the final hop, fragment removed.
    pub final_url: String,
    /// `text` for text/plain, `html` for text/html.
    pub content_kind: String,
    /// `plain` for text/plain, `html-conservative` for text/html.
    pub extraction: String,
    /// Raw response body bytes of the final hop.
    pub fetched_bytes: u64,
    /// SHA-256 of the raw response body bytes (fetch-time
    /// evidence only; not re-verifiable without refetching).
    pub fetched_hash: String,
    /// SHA-256 of the extracted text bytes (the beam anchor).
    pub text_hash: String,
    /// Extracted text char count.
    pub text_chars: u64,
    /// Whether one leading BOM was stripped.
    pub bom: bool,
    /// The extracted text, ready for user review.
    pub text: String,
}

fn err(message: impl Into<String>) -> String {
    message.into()
}

/// Whether an address is a public destination under the policy.
///
/// IPv4 uses an explicit deny-list audited against the
/// IANA IPv4 Special-Purpose Address Registry. IPv6 uses
/// an allowlist: only ordinary global unicast inside
/// 2000::/3 (IANA IPv6 Address Space Registry) minus the
/// special-purpose exclusions inside that range (IANA
/// IPv6 Special-Purpose Address Registry) is public, so
/// reserved space needs no enumeration to be refused.
/// Transition forms are classified by what they name:
/// IPv4-mapped by the embedded IPv4 address, 6to4 by its
/// embedded IPv4 address. The prefix table below is the
/// auditable record; it does not depend on
/// standard-library stability.
pub fn is_public_address(ip: IpAddr, policy: UrlFetchPolicy) -> bool {
    if policy.allow_loopback && ip.is_loopback() {
        return true;
    }
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => is_public_v6(v6),
    }
}

/// Whether every address in a DNS answer is public. A
/// mixed answer is refused: one non-public address is
/// enough to reject the whole name.
fn all_public(addrs: &[SocketAddr], policy: UrlFetchPolicy) -> bool {
    addrs.iter().all(|a| is_public_address(a.ip(), policy))
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    // Unspecified, loopback, private, link-local, CGNAT,
    // documentation, benchmarking, multicast, reserved.
    if o[0] == 0 || o[0] == 127 || o[0] >= 224 {
        return false;
    }
    if o[0] == 10 {
        return false;
    }
    if o[0] == 100 && (64..128).contains(&o[1]) {
        return false;
    }
    if o[0] == 169 && o[1] == 254 {
        return false;
    }
    if o[0] == 172 && (16..32).contains(&o[1]) {
        return false;
    }
    if o[0] == 192 && o[1] == 0 && (o[2] == 0 || o[2] == 2) {
        return false;
    }
    if o[0] == 192
        && ((o[1] == 31 && o[2] == 196)
            || (o[1] == 52 && o[2] == 193)
            || (o[1] == 88 && o[2] == 99)
            || (o[1] == 175 && o[2] == 48))
    {
        return false;
    }
    if o[0] == 192 && o[1] == 168 {
        return false;
    }
    if o[0] == 198 && (o[1] == 18 || o[1] == 19) {
        return false;
    }
    if o[0] == 198 && o[1] == 51 && o[2] == 100 {
        return false;
    }
    if o[0] == 203 && o[1] == 0 && o[2] == 113 {
        return false;
    }
    true
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    // IPv4-mapped addresses embed an IPv4 address: classify
    // the embedded address, so ::ffff:127.0.0.1 cannot
    // bypass the IPv4 rules.
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_public_v4(v4);
    }
    let s = ip.segments();
    // 6to4 2002::/16 embeds an IPv4 address in bits
    // 16..48: classify it by the IPv4 rules, so a 6to4
    // address naming a private IPv4 address is refused.
    if s[0] == 0x2002 {
        let embedded = Ipv4Addr::new(
            (s[1] >> 8) as u8,
            (s[1] & 0xff) as u8,
            (s[2] >> 8) as u8,
            (s[2] & 0xff) as u8,
        );
        return is_public_v4(embedded);
    }
    // Global-unicast allowlist: only 2000::/3 can be
    // public. Everything outside it — loopback,
    // unspecified, link-local, ULA, deprecated site-local
    // fec0::/10 (RFC 3879), multicast, discard 100::/64,
    // NAT64 prefixes, and all other reserved space — is
    // refused by this rule alone, with no enumeration.
    if (s[0] & 0xe000) != 0x2000 {
        return false;
    }
    // Special-purpose exclusions inside 2000::/3.
    // The whole IANA-special 2001::/23 block is refused
    // at once, which covers Teredo 2001::/32, PCP/TURN
    // anycast 2001:1::/32, benchmarking 2001:2::/48,
    // ORCHID 2001:10::/28, ORCHIDv2 2001:20::/28, and
    // AS112 2001:4:112::/48, plus any future special
    // assignment inside that block.
    if s[0] == 0x2001 && (s[1] & 0xfe00) == 0x0000 {
        return false;
    }
    if s[0] == 0x2001 && s[1] == 0x0db8 {
        return false; // Documentation 2001:db8::/32
    }
    if s[0] == 0x3fff && (s[1] & 0xf000) == 0x0000 {
        return false; // Documentation 3fff::/20
    }
    if s[0] == 0x2620 && s[1] == 0x004d && s[2] == 0x4000 {
        return false; // AS112 2620:4d:4000::/48
    }
    true
}

/// One validated hop target: the URL to request and the
/// exact socket address to connect to.
struct HopTarget {
    url: url::Url,
    host: String,
    port: u16,
    addr: SocketAddr,
}

/// Validate one URL as a hop target: scheme, credentials,
/// port, host class, and — for DNS names — resolution with
/// every resolved address classified. Returns the address
/// to connect to, so the caller never re-resolves.
fn validate_target(
    url: &url::Url,
    policy: UrlFetchPolicy,
    deadline: &Deadline,
) -> Result<HopTarget, String> {
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(err("URL scheme must be http or https."));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(err("URLs with credentials are refused."));
    }
    let host = match url.host_str() {
        Some(host) if !host.is_empty() => host.to_string(),
        _ => return Err(err("URL must have a host.")),
    };
    let lower = host.to_ascii_lowercase();
    if lower == "localhost"
        || lower.ends_with(".localhost")
        || lower.ends_with(".local")
        || lower.ends_with(".internal")
        || lower.ends_with(".home.arpa")
    {
        return Err(err("URL host is not a public destination."));
    }
    let port = url.port_or_known_default().unwrap_or(443);
    let default_port = if scheme == "http" { 80 } else { 443 };
    if port != default_port && !policy.allow_non_default_ports {
        return Err(err("URL port must be the scheme default port."));
    }
    let addr = match url.host() {
        Some(url::Host::Ipv4(v4)) => {
            let ip = IpAddr::V4(v4);
            if !is_public_address(ip, policy) {
                return Err(err("URL host is not a public destination."));
            }
            SocketAddr::new(ip, port)
        }
        Some(url::Host::Ipv6(v6)) => {
            let ip = IpAddr::V6(v6);
            if !is_public_address(ip, policy) {
                return Err(err("URL host is not a public destination."));
            }
            SocketAddr::new(ip, port)
        }
        _ => {
            let addrs = resolve(&host, port, deadline)?;
            if addrs.is_empty() {
                return Err(err("URL host did not resolve."));
            }
            if !all_public(&addrs, policy) {
                return Err(err("URL host resolves to a non-public address."));
            }
            // Every resolved address passed, so connecting
            // to the first cannot reach an unvalidated one.
            addrs[0]
        }
    };
    Ok(HopTarget {
        url: url.clone(),
        host,
        port,
        addr,
    })
}

/// Resolve a host with a bounded wait. The system resolver
/// has no timeout, so resolution runs on a thread the
/// caller abandons on timeout; the thread itself is
/// harmless and exits when the resolver returns.
fn resolve(host: &str, port: u16, deadline: &Deadline) -> Result<Vec<SocketAddr>, String> {
    deadline.check()?;
    let host = host.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = (host.as_str(), port).to_socket_addrs().map(|i| i.collect());
        let _ = tx.send(result);
    });
    // The resolver thread cannot be killed, so on timeout
    // it is abandoned: it carries no socket and exits
    // when the resolver returns, while this worker still
    // returns by the deadline.
    match rx.recv_timeout(deadline.remaining().min(DNS_TIMEOUT)) {
        Ok(Ok(addrs)) => Ok(addrs),
        Ok(Err(_)) => Err(err("URL host did not resolve.")),
        Err(_) => {
            if deadline.past() {
                Err(err("Fetching the URL timed out."))
            } else {
                Err(err("URL host resolution timed out."))
            }
        }
    }
}

/// TCP transport that enforces the fetch deadline on
/// every underlying socket read and write, beneath
/// rustls. A timeout set once above StreamOwned cannot
/// bound record assembly: one StreamOwned read can issue
/// many transport reads while assembling a single
/// encrypted record, and each fragment that arrives
/// resets socket inactivity. This wrapper re-checks the
/// deadline and shrinks the socket timeout to the
/// remaining budget before every single socket call, so
/// record assembly, alerts, post-handshake traffic, and
/// fragmented writes are all bounded by the deadline.
struct DeadlineTcpStream {
    inner: TcpStream,
    deadline: Deadline,
}

impl DeadlineTcpStream {
    fn new(inner: TcpStream, deadline: Deadline) -> Self {
        Self { inner, deadline }
    }

    fn set_timeouts(&self, timeout: Duration) -> Result<(), String> {
        self.inner
            .set_read_timeout(Some(timeout))
            .map_err(|_| err("Could not configure the connection."))?;
        self.inner
            .set_write_timeout(Some(timeout))
            .map_err(|_| err("Could not configure the connection."))?;
        Ok(())
    }

    fn timeout_error(&self) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::TimedOut, "Fetching the URL timed out.")
    }
}

impl Read for DeadlineTcpStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.deadline.check().is_err() {
            return Err(self.timeout_error());
        }
        if self
            .inner
            .set_read_timeout(Some(self.deadline.socket_timeout()))
            .is_err()
        {
            return Err(std::io::Error::other("Could not configure the connection."));
        }
        match self.inner.read(buf) {
            Ok(n) => Ok(n),
            Err(_) if self.deadline.past() => Err(self.timeout_error()),
            Err(e) => Err(e),
        }
    }
}

impl Write for DeadlineTcpStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.deadline.check().is_err() {
            return Err(self.timeout_error());
        }
        if self
            .inner
            .set_write_timeout(Some(self.deadline.socket_timeout()))
            .is_err()
        {
            return Err(std::io::Error::other("Could not configure the connection."));
        }
        match self.inner.write(buf) {
            Ok(n) => Ok(n),
            Err(_) if self.deadline.past() => Err(self.timeout_error()),
            Err(e) => Err(e),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.deadline.check().is_err() {
            return Err(self.timeout_error());
        }
        match self.inner.flush() {
            Ok(()) => Ok(()),
            Err(_) if self.deadline.past() => Err(self.timeout_error()),
            Err(e) => Err(e),
        }
    }
}

/// A readable byte stream: plain TCP or rustls over TCP.
/// Both variants run over the deadline-aware transport,
/// so the deadline holds beneath rustls as well as
/// above it.
enum FetchStream {
    Plain(DeadlineTcpStream),
    // Boxed: StreamOwned is large, and the plain variant
    // would otherwise trip the large-enum-variant lint
    // (the same boxing the save-claim enum uses).
    Tls(Box<StreamOwned<ClientConnection, DeadlineTcpStream>>),
}

impl Read for FetchStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            FetchStream::Plain(s) => s.read(buf),
            FetchStream::Tls(s) => s.read(buf),
        }
    }
}

impl FetchStream {
    /// Set both socket timeouts on the underlying TCP
    /// stream (through the TLS layer when present), so
    /// the next wait honors the remaining budget. The
    /// transport re-sets them on every call anyway; this
    /// is defense in depth for the next outer read.
    fn set_timeouts(&self, timeout: Duration) -> Result<(), String> {
        match self {
            FetchStream::Plain(s) => s.set_timeouts(timeout),
            FetchStream::Tls(s) => s.get_ref().set_timeouts(timeout),
        }
    }
}

impl Write for FetchStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            FetchStream::Plain(s) => s.write(buf),
            FetchStream::Tls(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            FetchStream::Plain(s) => s.flush(),
            FetchStream::Tls(s) => s.flush(),
        }
    }
}

/// Small buffered reader with caps, over a fetch stream.
/// Every fill re-checks the total deadline and shrinks
/// the socket timeout to the remaining budget first, so
/// a dribbled stream cannot outlive the deadline: each
/// read returns data or waits at most the remaining time,
/// and the next fill re-checks before waiting again.
struct CappedReader<'a> {
    inner: &'a mut FetchStream,
    deadline: Deadline,
    buf: Vec<u8>,
    pos: usize,
}

impl<'a> CappedReader<'a> {
    fn new(inner: &'a mut FetchStream, deadline: Deadline) -> Self {
        Self {
            inner,
            deadline,
            buf: Vec::new(),
            pos: 0,
        }
    }

    fn fill(&mut self) -> Result<bool, String> {
        if self.pos < self.buf.len() {
            return Ok(true);
        }
        self.deadline.check()?;
        self.inner.set_timeouts(self.deadline.socket_timeout())?;
        self.buf.clear();
        self.pos = 0;
        let mut chunk = [0u8; 8192];
        match self.inner.read(&mut chunk) {
            Ok(0) => Ok(false),
            Ok(n) => {
                self.buf.extend_from_slice(&chunk[..n]);
                Ok(true)
            }
            Err(e) => {
                if self.deadline.past() {
                    Err(err("Fetching the URL timed out."))
                } else {
                    Err(err(format!("Failed reading the response: {e}")))
                }
            }
        }
    }

    fn read_line(&mut self, max: usize) -> Result<String, String> {
        let mut line: Vec<u8> = Vec::new();
        loop {
            if !self.fill()? {
                break;
            }
            let available = &self.buf[self.pos..];
            if let Some(nl) = available.iter().position(|b| *b == b'\n') {
                line.extend_from_slice(&available[..=nl]);
                self.pos += nl + 1;
                break;
            }
            line.extend_from_slice(available);
            self.pos = self.buf.len();
            if line.len() > max {
                return Err(err("Response header is too large."));
            }
        }
        if line.len() > max {
            return Err(err("Response header is too large."));
        }
        Ok(String::from_utf8_lossy(&line).into_owned())
    }

    fn read_exact_capped(&mut self, mut n: u64, cap: &mut u64) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        while n > 0 {
            if !self.fill()? {
                return Err(err("Response ended before its declared length."));
            }
            let available = &self.buf[self.pos..];
            let take = (available.len() as u64).min(n) as usize;
            if *cap < take as u64 {
                return Err(err("Downloaded content exceeds the size limit."));
            }
            out.extend_from_slice(&available[..take]);
            self.pos += take;
            *cap -= take as u64;
            n -= take as u64;
        }
        Ok(out)
    }

    fn read_to_end_capped(&mut self, cap: &mut u64) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        while self.fill()? {
            let available = &self.buf[self.pos..];
            let take = available.len();
            if *cap < take as u64 {
                return Err(err("Downloaded content exceeds the size limit."));
            }
            out.extend_from_slice(available);
            self.pos += take;
            *cap -= take as u64;
        }
        Ok(out)
    }
}

struct ResponseHead {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl ResponseHead {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Read one HTTP/1.x response, retaining headers.
fn read_response_inner(
    stream: &mut FetchStream,
    deadline: Deadline,
) -> Result<ResponseHead, String> {
    let mut reader = CappedReader::new(stream, deadline);
    let mut header_bytes = 0usize;
    let mut lines: Vec<String> = Vec::new();
    loop {
        deadline.check()?;
        let line = reader.read_line(MAX_HEADER_BYTES)?;
        header_bytes += line.len();
        if header_bytes > MAX_HEADER_BYTES {
            return Err(err("Response header is too large."));
        }
        if line.trim().is_empty() {
            break;
        }
        lines.push(line);
    }
    if lines.is_empty() {
        return Err(err("The server returned an empty response."));
    }
    let status_line = lines[0].trim_end().to_string();
    let mut parts = status_line.splitn(3, ' ');
    let _version = parts.next().unwrap_or("");
    let status: u16 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| err("The server returned a malformed response."))?;
    let mut headers: Vec<(String, String)> = Vec::new();
    for line in lines.iter().skip(1) {
        let trimmed = line.trim_end();
        let Some((name, value)) = trimmed.split_once(':') else {
            return Err(err("The server returned a malformed header."));
        };
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
    }
    let head = ResponseHead {
        status,
        headers,
        body: Vec::new(),
    };
    // Body rules: chunked wins over Content-Length; a
    // declared encoding other than identity refuses; a
    // declared length above the cap refuses before reading.
    let encoding = head.header("content-encoding").unwrap_or("identity");
    if encoding != "identity" {
        return Err(err("Compressed responses are not supported."));
    }
    let chunked = head
        .header("transfer-encoding")
        .map(|v| v.to_ascii_lowercase().contains("chunked"))
        .unwrap_or(false);
    let mut cap = MAX_FETCH_BYTES;
    let body = if chunked {
        let mut out = Vec::new();
        loop {
            deadline.check()?;
            let size_line = reader.read_line(256)?;
            let size_text = size_line.trim().split(';').next().unwrap_or("").trim();
            let size = usize::from_str_radix(size_text, 16)
                .map_err(|_| err("The server returned a malformed chunked body."))?;
            if size == 0 {
                // Trailers until a blank line.
                loop {
                    let trailer = reader.read_line(MAX_HEADER_BYTES)?;
                    if trailer.trim().is_empty() {
                        break;
                    }
                }
                break;
            }
            let chunk = reader.read_exact_capped(size as u64, &mut cap)?;
            out.extend_from_slice(&chunk);
            // The CRLF after the chunk.
            let crlf = reader.read_exact_capped(2, &mut cap)?;
            if crlf != b"\r\n" {
                return Err(err("The server returned a malformed chunked body."));
            }
        }
        out
    } else if let Some(length) = head.header("content-length") {
        let length: u64 = length
            .parse()
            .map_err(|_| err("The server returned a malformed content length."))?;
        if length > MAX_FETCH_BYTES {
            return Err(err("Downloaded content exceeds the size limit."));
        }
        reader.read_exact_capped(length, &mut cap)?
    } else {
        reader.read_to_end_capped(&mut cap)?
    };
    Ok(ResponseHead {
        status: head.status,
        headers: head.headers,
        body,
    })
}

fn build_tls_config(extra_roots: &[CertificateDer<'static>]) -> Arc<ClientConfig> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    for extra in extra_roots {
        let _ = roots.add(extra.clone());
    }
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("ring provider supports the default protocol versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
    Arc::new(config)
}

/// Connect to one validated hop target and perform a GET.
/// Returns the response head and body.
fn request_once(
    target: &HopTarget,
    tls_config: &Arc<ClientConfig>,
    deadline: Deadline,
) -> Result<ResponseHead, String> {
    deadline.check()?;
    let tcp = TcpStream::connect_timeout(&target.addr, deadline.remaining().min(CONNECT_TIMEOUT))
        .map_err(|e| {
        if deadline.past() {
            err("Fetching the URL timed out.")
        } else {
            err(format!("Could not connect to the URL host: {e}"))
        }
    })?;
    let mut transport = DeadlineTcpStream::new(tcp, deadline);
    let mut stream = if target.url.scheme() == "https" {
        let server_name = if let Ok(ip) = target.host.parse::<IpAddr>() {
            ServerName::IpAddress(ip.into())
        } else {
            ServerName::try_from(target.host.clone())
                .map_err(|_| err("URL host is not a valid TLS server name."))?
        };
        let mut conn = ClientConnection::new(tls_config.clone(), server_name)
            .map_err(|_| err("Could not start TLS."))?;
        // Drive the handshake explicitly over the
        // deadline-aware transport, so every handshake
        // read/write honors the remaining budget: a
        // stalled or dribbled handshake cannot outlive
        // the deadline. Certificate and hostname
        // validation are rustls defaults inside
        // process_new_packets.
        loop {
            deadline.check()?;
            while conn.wants_write() {
                conn.write_tls(&mut transport).map_err(|_| {
                    if deadline.past() {
                        err("Fetching the URL timed out.")
                    } else {
                        err("Could not complete TLS.")
                    }
                })?;
            }
            if !conn.is_handshaking() {
                break;
            }
            conn.read_tls(&mut transport).map_err(|_| {
                if deadline.past() {
                    err("Fetching the URL timed out.")
                } else {
                    err("Could not complete TLS.")
                }
            })?;
            conn.process_new_packets().map_err(|_| {
                if deadline.past() {
                    err("Fetching the URL timed out.")
                } else {
                    err("Could not complete TLS.")
                }
            })?;
        }
        FetchStream::Tls(Box::new(StreamOwned::new(conn, transport)))
    } else {
        FetchStream::Plain(transport)
    };
    let host_header = if target.port
        == if target.url.scheme() == "http" {
            80
        } else {
            443
        } {
        if target.host.contains(':') {
            format!("[{}]", target.host)
        } else {
            target.host.clone()
        }
    } else if target.host.contains(':') {
        format!("[{}]:{}", target.host, target.port)
    } else {
        format!("{}:{}", target.host, target.port)
    };
    let path = if target.url.path().is_empty() {
        "/"
    } else {
        target.url.path()
    };
    let target_text = match target.url.query() {
        Some(q) => format!("{path}?{q}"),
        None => path.to_string(),
    };
    let request = format!(
        "GET {target_text} HTTP/1.1\r\nHost: {host_header}\r\nUser-Agent: ANANSI-url-fetch/1\r\nAccept: text/plain, text/html\r\nAccept-Encoding: identity\r\nConnection: close\r\n\r\n"
    );
    deadline.check()?;
    stream.set_timeouts(deadline.socket_timeout())?;
    stream.write_all(request.as_bytes()).map_err(|_| {
        if deadline.past() {
            err("Fetching the URL timed out.")
        } else {
            err("Failed sending the request.")
        }
    })?;
    stream.flush().map_err(|_| {
        if deadline.past() {
            err("Fetching the URL timed out.")
        } else {
            err("Failed sending the request.")
        }
    })?;
    deadline.check()?;
    read_response_inner(&mut stream, deadline)
}

/// Strict UTF-8 decode with one leading BOM stripped.
fn decode_plain(body: &[u8]) -> Result<(String, bool), String> {
    let text =
        String::from_utf8(body.to_vec()).map_err(|_| err("Fetched content is not valid UTF-8."))?;
    let (text, bom) = match text.strip_prefix('\u{feff}') {
        Some(rest) => (rest.to_string(), true),
        None => (text, false),
    };
    Ok((text, bom))
}

/// Conservative HTML-to-text extraction (see the design).
fn extract_html(body: &[u8]) -> Result<(String, bool), String> {
    let (decoded, bom) = decode_plain(body)?;
    let mut current = decoded;
    for tag in ["head", "script", "style"] {
        let mut out = String::with_capacity(current.len());
        let mut remaining = current.as_str();
        loop {
            let lower = remaining.to_ascii_lowercase();
            let Some(open) = lower.find(&format!("<{tag}")) else {
                out.push_str(remaining);
                break;
            };
            // The open tag must end at a boundary (space,
            // '>', or '/') so `<header>` does not match head.
            // Byte indices in the lowercased copy match the
            // original: lowercasing is byte-length preserving
            // and the pattern is ASCII, so it cannot land
            // inside a multi-byte character.
            let after = &lower[open + tag.len() + 1..];
            let boundary = after
                .chars()
                .next()
                .map(|c| c == '>' || c == '/' || c.is_whitespace())
                .unwrap_or(false);
            if !boundary {
                out.push_str(&remaining[..open + tag.len() + 1]);
                remaining = &remaining[open + tag.len() + 1..];
                continue;
            }
            out.push_str(&remaining[..open]);
            let close = format!("</{tag}>");
            let Some(close_at) = lower[open..].find(&close) else {
                // Unclosed section: drop the rest.
                remaining = "";
                break;
            };
            remaining = &remaining[open + close_at + close.len()..];
        }
        current = out;
    }
    let rest = current.as_str();
    // Strip tags; block-level tags become newlines.
    const BLOCK: [&str; 17] = [
        "p",
        "div",
        "section",
        "article",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "li",
        "ul",
        "ol",
        "blockquote",
        "pre",
        "tr",
        "br",
    ];
    let mut text = String::with_capacity(rest.len());
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let Some(end) = rest[i..].find('>') else {
                break;
            };
            let tag_text = &rest[i + 1..i + end];
            let name: String = tag_text
                .trim_start_matches('/')
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase();
            if BLOCK.contains(&name.as_str()) {
                text.push('\n');
            }
            i += end + 1;
        } else {
            // Copy one UTF-8 char.
            let ch_len = utf8_len(bytes[i]);
            let end = (i + ch_len).min(bytes.len());
            text.push_str(&rest[i..end]);
            i = end;
        }
    }
    let decoded_entities = decode_entities(&text);
    // Collapse runs of more than two newlines to two, and
    // trim leading/trailing blank lines.
    let mut collapsed = String::with_capacity(decoded_entities.len());
    let mut newlines = 0;
    for ch in decoded_entities.chars() {
        if ch == '\n' {
            newlines += 1;
            if newlines <= 2 {
                collapsed.push(ch);
            }
        } else {
            newlines = 0;
            collapsed.push(ch);
        }
    }
    Ok((collapsed.trim().to_string(), bom))
}

fn utf8_len(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first >> 5 == 0b110 {
        2
    } else if first >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        let Some(semi) = tail.find(';') else {
            out.push('&');
            rest = &tail[1..];
            continue;
        };
        let entity = &tail[1..semi];
        let replacement: Option<String> = match entity {
            "amp" => Some("&".to_string()),
            "lt" => Some("<".to_string()),
            "gt" => Some(">".to_string()),
            "quot" => Some("\"".to_string()),
            "apos" => Some("'".to_string()),
            "nbsp" => Some(" ".to_string()),
            _ => {
                if let Some(hex) = entity
                    .strip_prefix("#x")
                    .or_else(|| entity.strip_prefix("#X"))
                {
                    u32::from_str_radix(hex, 16)
                        .ok()
                        .and_then(char::from_u32)
                        .map(|c| c.to_string())
                } else if let Some(dec) = entity.strip_prefix('#') {
                    dec.parse::<u32>()
                        .ok()
                        .and_then(char::from_u32)
                        .map(|c| c.to_string())
                } else {
                    None
                }
            }
        };
        match replacement {
            Some(rep) => out.push_str(&rep),
            None => out.push_str(&tail[..=semi]),
        }
        rest = &tail[semi + 1..];
    }
    out.push_str(rest);
    out
}

/// Fetch URL text under the policy. `extra_roots` extends
/// the Mozilla trust set and is test-only (synthetic CAs);
/// production callers pass an empty slice.
pub fn fetch_url_text(
    submitted: &str,
    policy: UrlFetchPolicy,
    extra_roots: &[CertificateDer<'static>],
) -> Result<UrlFetchResult, String> {
    let submitted_trimmed = submitted.trim();
    if submitted_trimmed.is_empty() || submitted_trimmed.chars().count() > MAX_URL_CHARS {
        return Err(err("URL must be 1 to 2048 characters."));
    }
    let mut current =
        url::Url::parse(submitted_trimmed).map_err(|_| err("URL could not be parsed."))?;
    current.set_fragment(None);
    let tls_config = build_tls_config(extra_roots);
    let deadline = Deadline::new(policy.total_timeout);
    let mut final_url = String::new();
    let mut fetched: Option<ResponseHead> = None;
    for hop in 0..=MAX_REDIRECTS {
        deadline.check()?;
        let target = validate_target(&current, policy, &deadline)?;
        let response = request_once(&target, &tls_config, deadline)?;
        match response.status {
            200 => {
                final_url = current.to_string();
                fetched = Some(response);
                break;
            }
            301 | 302 | 303 | 307 | 308 => {
                if hop == MAX_REDIRECTS {
                    return Err(err("URL redirects too many times."));
                }
                let location = response
                    .header("location")
                    .ok_or_else(|| err("Redirect response has no location."))?;
                current = current
                    .join(location)
                    .map_err(|_| err("Redirect target is not a valid URL."))?;
                current.set_fragment(None);
            }
            status => {
                return Err(err(format!("The server returned status {status}.")));
            }
        }
    }
    let response = fetched.ok_or_else(|| err("Fetching the URL failed."))?;
    let content_type = response.header("content-type").unwrap_or("").to_string();
    let mut parts = content_type.split(';');
    let media = parts.next().unwrap_or("").trim().to_ascii_lowercase();
    let charset = parts
        .find_map(|p| {
            let p = p.trim();
            p.strip_prefix("charset=")
                .or_else(|| p.strip_prefix("CHARSET="))
                .map(|c| c.trim().trim_matches('"').to_ascii_lowercase())
        })
        .unwrap_or_default();
    if !charset.is_empty() && charset != "utf-8" && charset != "utf8" {
        return Err(err("Fetched content must be UTF-8."));
    }
    let (content_kind, extraction, text, bom) = match media.as_str() {
        "text/plain" => {
            let (text, bom) = decode_plain(&response.body)?;
            ("text", "plain", text, bom)
        }
        "text/html" => {
            let (text, bom) = extract_html(&response.body)?;
            ("html", "html-conservative", text, bom)
        }
        _ => {
            return Err(err("Only plain text and HTML pages can be fetched."));
        }
    };
    let text_chars = text.chars().count() as u64;
    if text.trim().is_empty() || text_chars > MAX_TEXT_CHARS as u64 {
        return Err(err(format!(
            "Fetched text must be 1 to {MAX_TEXT_CHARS} characters after extraction."
        )));
    }
    let fetched_bytes = response.body.len() as u64;
    Ok(UrlFetchResult {
        submitted_url: submitted_trimmed.to_string(),
        final_url,
        content_kind: content_kind.to_string(),
        extraction: extraction.to_string(),
        fetched_bytes,
        fetched_hash: crate::content::hash(&response.body),
        text_hash: crate::content::hash(text.as_bytes()),
        text_chars,
        bom,
        text,
    })
}

#[cfg(test)]
mod url_fetch_tests {
    use super::*;

    fn strict() -> UrlFetchPolicy {
        UrlFetchPolicy::strict()
    }

    #[test]
    fn address_classification_covers_special_ranges() {
        let policy = strict();
        assert!(is_public_address("8.8.8.8".parse().unwrap(), policy));
        assert!(is_public_address("1.1.1.1".parse().unwrap(), policy));
        for blocked in [
            "0.0.0.0",
            "10.0.0.1",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.169.254",
            "172.16.0.9",
            "192.0.2.1",
            "192.168.1.1",
            "198.18.0.1",
            "198.51.100.7",
            "203.0.113.9",
            "224.0.0.1",
            "240.0.0.1",
        ] {
            assert!(
                !is_public_address(blocked.parse().unwrap(), policy),
                "{blocked} must be blocked"
            );
        }
        for blocked in [
            "::",
            "::1",
            "::2",
            "fe80::1",
            "fec0::1",
            "fc00::1",
            "fd00::1",
            "ff02::1",
            "100::1",
            "64:ff9b::1",
            "64:ff9b:1::1",
            "2001::1",
            "2001:1::1",
            "2001:2::1",
            "2001:10::1",
            "2001:1f::1",
            "2001:20::1",
            "2001:2f::1",
            "2001:db8::1",
            "2001:4:112::1",
            "2620:4d:4000::1",
            "3fff::1",
            "5f00::1",
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.1",
            // 6to4 naming a private or loopback IPv4 address.
            "2002:0a00:0001::",
            "2002:7f00:0001::",
        ] {
            assert!(
                !is_public_address(blocked.parse().unwrap(), policy),
                "{blocked} must be blocked"
            );
        }
        for public in [
            "2001:4860:4860::8888",
            "2606:4700:4700::1111",
            // Mapped and 6to4 forms of public IPv4 addresses.
            "::ffff:8.8.8.8",
            "2002:0808:0808::",
            // Just outside the private/CGNAT/benchmarking edges.
            "172.32.0.1",
            "100.128.0.1",
        ] {
            assert!(
                is_public_address(public.parse().unwrap(), policy),
                "{public} must be public"
            );
        }
        for blocked in [
            "0.1.2.3",
            "172.31.255.255",
            "100.127.255.255",
            "198.19.255.255",
            "192.0.0.170",
            "192.0.0.171",
            "255.255.255.255",
        ] {
            assert!(
                !is_public_address(blocked.parse().unwrap(), policy),
                "{blocked} must be blocked"
            );
        }
        // Loopback passes only under the test policy.
        assert!(is_public_address(
            "127.0.0.1".parse().unwrap(),
            UrlFetchPolicy::test_loopback()
        ));
        assert!(!is_public_address(
            "10.0.0.1".parse().unwrap(),
            UrlFetchPolicy::test_loopback()
        ));
    }

    #[test]
    fn policy_selection_is_strict_without_a_test_build() {
        // Both branches of the pure selector: the test
        // policy needs the flag AND a test/debug build.
        assert!(policy_for_fetch(true, true).allow_loopback);
        assert!(!policy_for_fetch(true, false).allow_loopback);
        assert!(!policy_for_fetch(false, true).allow_loopback);
        assert!(!policy_for_fetch(false, false).allow_loopback);
    }

    #[test]
    fn mixed_dns_answers_are_refused() {
        let policy = strict();
        let public: SocketAddr = "8.8.8.8:80".parse().unwrap();
        let private: SocketAddr = "10.0.0.1:80".parse().unwrap();
        assert!(all_public(&[public], policy));
        assert!(!all_public(&[public, private], policy));
        assert!(!all_public(&[private, public], policy));
    }

    #[test]
    fn numeric_ipv4_literal_encodings_are_classified() {
        // The WHATWG URL parser normalizes hex, decimal,
        // and octal IPv4 forms to dotted-decimal before
        // classification, so none of these reaches DNS or
        // a socket as anything but 127.0.0.1.
        let policy = strict();
        let deadline = Deadline::new(TOTAL_BUDGET);
        for literal in ["0x7f000001", "2130706433", "0177.0.0.1"] {
            let url = url::Url::parse(&format!("http://{literal}/")).unwrap();
            assert!(
                matches!(url.host(), Some(url::Host::Ipv4(v4)) if v4.octets() == [127, 0, 0, 1]),
                "{literal} must parse as IPv4 loopback"
            );
            assert!(
                validate_target(&url, policy, &deadline).is_err(),
                "{literal} must be refused"
            );
        }
    }

    #[test]
    fn target_validation_refuses_shapes_without_connecting() {
        let policy = strict();
        let deadline = Deadline::new(TOTAL_BUDGET);
        for bad in [
            "ftp://example.com/file.txt",
            "https://user:secret@example.com/",
            "https://example.com:8443/",
            "http://localhost:8080/",
            "http://foo.local/",
            "http://foo.internal/",
            "http://127.0.0.1/",
            "http://10.1.2.3/",
            "http://[::1]/",
            "http://169.254.169.254/latest/meta-data/",
        ] {
            // A parse failure is itself a refusal.
            let Ok(url) = url::Url::parse(bad) else {
                continue;
            };
            assert!(
                validate_target(&url, policy, &deadline).is_err(),
                "{bad} must be refused"
            );
        }
        // A public literal target validates (no connection made).
        let url = url::Url::parse("https://8.8.8.8/").unwrap();
        let target = validate_target(&url, policy, &deadline).unwrap();
        assert_eq!(target.port, 443);
        // Test policy allows loopback with any port.
        let test = UrlFetchPolicy::test_loopback();
        let url = url::Url::parse("http://127.0.0.1:45111/page").unwrap();
        let target = validate_target(&url, test, &deadline).unwrap();
        assert_eq!(target.port, 45111);
    }

    #[test]
    fn plain_decode_strips_one_bom_and_rejects_invalid_utf8() {
        let (text, bom) = decode_plain(&[0xef, 0xbb, 0xbf, b'h', b'i']).unwrap();
        assert_eq!(text, "hi");
        assert!(bom);
        assert!(decode_plain(&[0xff, 0xfe]).is_err());
        assert!(!decode_plain(b"plain").unwrap().1);
    }

    #[test]
    fn html_extraction_is_conservative() {
        let html = br#"<!doctype html><html><head><title>Hidden title</title><style>p{color:red}</style><script>var x=1;</script></head><body><h1>Title &amp; more</h1><p>First &#65; paragraph.</p><script>alert(1)</script><div>Second <b>bold</b> line<br>continued</div></body></html>"#;
        let (text, _) = extract_html(html).unwrap();
        assert!(!text.contains("Hidden title"));
        assert!(!text.contains("alert"));
        assert!(!text.contains("color:red"));
        assert!(text.contains("Title & more"));
        assert!(text.contains("First A paragraph."));
        assert!(text.contains("Second bold line"));
        assert!(text.contains("continued"));
        assert!(!text.contains('<'));
    }

    #[test]
    fn html_extraction_collapses_blank_lines() {
        let html = b"<p>One</p><p></p><p></p><p></p><p>Two</p>";
        let (text, _) = extract_html(html).unwrap();
        assert_eq!(text, "One\n\nTwo");
    }
}
