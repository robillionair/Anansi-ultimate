//! M8 URL input integration: the backend-owned fetch
//! service against synthetic loopback servers only. No real
//! external site is ever contacted: the strict policy
//! refuses every non-public destination, and the loopback
//! policy exists so these tests (and the browser harness)
//! can run a local server. Hit counts prove refusals
//! happen before connecting.

use anansi_core::content;
use anansi_core::url_fetch::{UrlFetchPolicy, fetch_url_text};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// A one-shot-per-connection HTTP server on loopback.
/// The handler maps a request path to a raw response.
struct RouteServer {
    addr: SocketAddr,
    hits: Arc<Mutex<HashMap<String, usize>>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl RouteServer {
    fn start(handler: impl Fn(&str) -> Vec<u8> + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let hits: Arc<Mutex<HashMap<String, usize>>> = Arc::new(Mutex::new(HashMap::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let hits_ref = hits.clone();
        let stop_ref = stop.clone();
        let handler = Arc::new(handler);
        let handle = std::thread::spawn(move || {
            while !stop_ref.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let handler = handler.clone();
                        let hits = hits_ref.clone();
                        std::thread::spawn(move || {
                            let mut buf = [0u8; 8192];
                            let mut request = Vec::new();
                            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).ok();
                            loop {
                                match stream.read(&mut buf) {
                                    Ok(0) => break,
                                    Ok(n) => {
                                        request.extend_from_slice(&buf[..n]);
                                        if request.windows(4).any(|w| w == b"\r\n\r\n") {
                                            break;
                                        }
                                    }
                                    Err(_) => break,
                                }
                            }
                            let text = String::from_utf8_lossy(&request);
                            let path = text
                                .lines()
                                .next()
                                .unwrap_or("")
                                .split_whitespace()
                                .nth(1)
                                .unwrap_or("/")
                                .to_string();
                            *hits.lock().unwrap().entry(path.clone()).or_insert(0) += 1;
                            let response = handler(&path);
                            let _ = stream.write_all(&response);
                            let _ = stream.flush();
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            addr,
            hits,
            stop,
            handle: Some(handle),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.addr.port(), path)
    }

    fn hit_count(&self, path: &str) -> usize {
        *self.hits.lock().unwrap().get(path).unwrap_or(&0)
    }
}

impl Drop for RouteServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn response(status: u16, reason: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut out = format!("HTTP/1.1 {status} {reason}\r\n").into_bytes();
    for (name, value) in headers {
        out.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    out.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    out.extend_from_slice(b"Connection: close\r\n\r\n");
    out.extend_from_slice(body);
    out
}

fn policy() -> UrlFetchPolicy {
    UrlFetchPolicy::test_loopback()
}

#[test]
fn plain_text_fetch_returns_exact_text_and_hashes() {
    let body = "Fetched page text.\nSecond line.\n";
    let server = RouteServer::start(move |_| {
        response(
            200,
            "OK",
            &[("Content-Type", "text/plain; charset=utf-8")],
            body.as_bytes(),
        )
    });
    let url = server.url("/page.txt");
    let fetched = fetch_url_text(&url, policy(), &[]).unwrap();
    assert_eq!(fetched.submitted_url, url);
    assert_eq!(fetched.final_url, url);
    assert_eq!(fetched.content_kind, "text");
    assert_eq!(fetched.extraction, "plain");
    assert_eq!(fetched.fetched_bytes, body.len() as u64);
    assert_eq!(fetched.fetched_hash, content::hash(body.as_bytes()));
    assert_eq!(fetched.text_hash, content::hash(body.as_bytes()));
    assert_eq!(fetched.text_chars, body.chars().count() as u64);
    assert!(!fetched.bom);
    assert_eq!(fetched.text, body);
    assert_eq!(server.hit_count("/page.txt"), 1);
}

#[test]
fn html_fetch_extracts_conservatively() {
    let html = "<html><head><title>Hidden</title><script>alert(1)</script></head>\
        <body><h1>Head &amp; line</h1><p>Body &#66; text.</p></body></html>";
    let server = RouteServer::start(move |_| {
        response(200, "OK", &[("Content-Type", "text/html")], html.as_bytes())
    });
    let fetched = fetch_url_text(&server.url("/page.html"), policy(), &[]).unwrap();
    assert_eq!(fetched.content_kind, "html");
    assert_eq!(fetched.extraction, "html-conservative");
    assert!(!fetched.text.contains("Hidden"));
    assert!(!fetched.text.contains("alert"));
    assert!(fetched.text.contains("Head & line"));
    assert!(fetched.text.contains("Body B text."));
    assert_eq!(fetched.fetched_hash, content::hash(html.as_bytes()));
    assert_eq!(fetched.text_hash, content::hash(fetched.text.as_bytes()));
}

#[test]
fn redirect_chain_updates_final_url() {
    let server = RouteServer::start(move |path| match path {
        "/old" => response(
            302,
            "Found",
            &[("Location", "/new")],
            b"",
        ),
        "/new" => response(
            200,
            "OK",
            &[("Content-Type", "text/plain")],
            b"New page.",
        ),
        _ => response(404, "Not Found", &[], b""),
    });
    let fetched = fetch_url_text(&server.url("/old"), policy(), &[]).unwrap();
    assert_eq!(fetched.final_url, server.url("/new"));
    assert_eq!(fetched.text, "New page.");
    assert_eq!(server.hit_count("/old"), 1);
    assert_eq!(server.hit_count("/new"), 1);
}

#[test]
fn redirect_to_private_refuses_without_connecting() {
    let server = RouteServer::start(move |path| match path {
        "/to-private" => response(
            302,
            "Found",
            &[("Location", "http://10.0.0.1/")],
            b"",
        ),
        _ => response(404, "Not Found", &[], b""),
    });
    let error = fetch_url_text(&server.url("/to-private"), policy(), &[]).unwrap_err();
    assert!(
        error.contains("not a public destination"),
        "refusal must come from validation, not a connect attempt: {error}"
    );
    assert_eq!(server.hit_count("/to-private"), 1);
}

#[test]
fn redirect_loop_refuses_after_the_bound() {
    let server = RouteServer::start(move |path| match path {
        "/a" => response(302, "Found", &[("Location", "/b")], b""),
        "/b" => response(302, "Found", &[("Location", "/a")], b""),
        _ => response(404, "Not Found", &[], b""),
    });
    let error = fetch_url_text(&server.url("/a"), policy(), &[]).unwrap_err();
    assert!(error.contains("too many times"), "{error}");
    // Initial request plus five followed redirects.
    assert_eq!(server.hit_count("/a") + server.hit_count("/b"), 6);
}

#[test]
fn adversarial_targets_refuse_with_zero_server_hits() {
    let server = RouteServer::start(move |_| {
        response(200, "OK", &[("Content-Type", "text/plain")], b"should not arrive")
    });
    let port = server.addr.port();
    let strict = UrlFetchPolicy::strict();
    // Credentials, private, link-local, and (under the
    // strict policy) loopback and non-default ports: all
    // refused before any connection, so the server that
    // would have answered sees zero hits.
    for bad in [
        format!("http://user:secret@127.0.0.1:{port}/page"),
        format!("http://10.0.0.1:{port}/"),
        format!("http://169.254.169.254:{port}/latest/meta-data/"),
        format!("http://127.0.0.1:{port}/page"),
        "ftp://127.0.0.1/file".to_string(),
        "http://[::1]/".to_string(),
    ] {
        let policy = if bad.contains("user:secret") || bad.contains("10.0.0.1") || bad.contains("169.254") {
            policy()
        } else {
            strict
        };
        assert!(
            fetch_url_text(&bad, policy, &[]).is_err(),
            "{bad} must be refused"
        );
    }
    assert_eq!(server.hit_count("/page"), 0);
}

#[test]
fn body_and_type_bounds_refuse() {
    let big = "x".repeat(1_048_577);
    let server = RouteServer::start(move |path| match path {
        "/big" => response(
            200,
            "OK",
            &[("Content-Type", "text/plain")],
            big.as_bytes(),
        ),
        "/pdf" => response(
            200,
            "OK",
            &[("Content-Type", "application/pdf")],
            b"%PDF-1.4",
        ),
        "/latin" => response(
            200,
            "OK",
            &[("Content-Type", "text/plain; charset=iso-8859-1")],
            &[0xe9],
        ),
        "/bad-utf8" => response(
            200,
            "OK",
            &[("Content-Type", "text/plain")],
            &[0xff, 0xfe],
        ),
        "/missing" => response(404, "Not Found", &[], b"nope"),
        "/chunked" => {
            let mut out = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".to_vec();
            out.extend_from_slice(b"5\r\nHello\r\n6\r\n world\r\n0\r\n\r\n");
            out
        }
        _ => response(404, "Not Found", &[], b""),
    });
    for bad in ["/big", "/pdf", "/latin", "/bad-utf8", "/missing"] {
        let error = fetch_url_text(&server.url(bad), policy(), &[]).unwrap_err();
        assert!(!error.is_empty(), "{bad} must refuse");
    }
    let chunked = fetch_url_text(&server.url("/chunked"), policy(), &[]).unwrap();
    assert_eq!(chunked.text, "Hello world");
}

#[test]
fn https_fetch_validates_certificates() {
    // Self-signed leaf for the loopback IP SAN, used as
    // its own test root: the client trusts exactly this
    // certificate, and a wrong-name certificate refuses.
    use rcgen::{CertificateParams, SanType};
    use rustls::pki_types::CertificateDer;
    use rustls::{ServerConfig, ServerConnection, StreamOwned};
    use std::net::TcpListener as StdListener;

    fn make_cert(sans: Vec<SanType>) -> (Vec<u8>, Vec<u8>, CertificateDer<'static>) {
        let mut params = CertificateParams::new(vec!["test".to_string()]).unwrap();
        params.subject_alt_names = sans;
        let key = rcgen::KeyPair::generate().unwrap();
        let cert = params.self_signed(&key).unwrap();
        let der = cert.der().to_vec();
        (der.clone(), key.serialize_der(), CertificateDer::from(der))
    }

    fn serve_https_once(
        cert_der: Vec<u8>,
        key_der: Vec<u8>,
    ) -> (SocketAddr, JoinHandle<()>) {
        let listener = StdListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let cert = rustls::pki_types::CertificateDer::from(cert_der);
            let key = rustls::pki_types::PrivateKeyDer::try_from(key_der).unwrap();
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            let config = ServerConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(vec![cert], key)
                .unwrap();
            let conn = ServerConnection::new(Arc::new(config)).unwrap();
            let mut tls = StreamOwned::new(conn, stream);
            let mut buf = [0u8; 4096];
            let mut request = Vec::new();
            loop {
                match tls.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        request.extend_from_slice(&buf[..n]);
                        if request.windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            let body = b"Secure page.";
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = tls.write_all(resp.as_bytes());
            let _ = tls.write_all(body);
            let _ = tls.flush();
        });
        (addr, handle)
    }

    let (good_der, good_key, good_root) = make_cert(vec![SanType::IpAddress(
        "127.0.0.1".parse().unwrap(),
    )]);
    let (addr, handle) = serve_https_once(good_der, good_key);
    let url = format!("https://127.0.0.1:{}/secure", addr.port());
    let fetched = fetch_url_text(&url, policy(), &[good_root]).unwrap();
    assert_eq!(fetched.text, "Secure page.");
    assert_eq!(fetched.content_kind, "text");
    handle.join().unwrap();

    // A certificate for a different name must refuse.
    let (bad_der, bad_key, bad_root) = make_cert(vec![SanType::DnsName(
        "example.com".try_into().unwrap(),
    )]);
    let (addr, handle) = serve_https_once(bad_der, bad_key);
    let url = format!("https://127.0.0.1:{}/secure", addr.port());
    // The chain trusts this root, so the only possible
    // failure is the hostname check itself.
    let error = fetch_url_text(&url, policy(), &[bad_root]).unwrap_err();
    assert!(!error.is_empty(), "wrong-name certificate must refuse");
    handle.join().unwrap();
}
