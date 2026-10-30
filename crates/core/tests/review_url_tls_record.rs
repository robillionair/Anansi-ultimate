use anansi_core::url_fetch::{UrlFetchPolicy, fetch_url_text};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::Arc,
    time::{Duration, Instant},
};

#[test]
fn review_fragmented_encrypted_record_cannot_bypass_total_deadline() {
    let mut params = rcgen::CertificateParams::new(vec!["test".into()]).unwrap();
    params.subject_alt_names = vec![rcgen::SanType::IpAddress("127.0.0.1".parse().unwrap())];
    let key = rcgen::KeyPair::generate().unwrap();
    let cert = params.self_signed(&key).unwrap();
    let root = cert.der().clone();
    let der = cert.der().to_vec();
    let key_der = key.serialize_der();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (sock, _) = listener.accept().unwrap();
        sock.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(
                vec![CertificateDer::from(der)],
                PrivateKeyDer::try_from(key_der).unwrap(),
            )
            .unwrap();
        let mut tls = StreamOwned::new(ServerConnection::new(Arc::new(config)).unwrap(), sock);
        let mut buf = [0u8; 4096];
        let mut req = Vec::new();
        loop {
            match tls.read(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(n) => {
                    req.extend_from_slice(&buf[..n]);
                    if req.windows(4).any(|b| b == b"\r\n\r\n") {
                        break;
                    }
                }
            }
        }
        // Build a genuine encrypted HTTP record, then fragment the record itself.
        tls.conn.writer().write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 4\r\nConnection: close\r\n\r\ntext").unwrap();
        let mut encrypted = Vec::new();
        while tls.conn.wants_write() {
            tls.conn.write_tls(&mut encrypted).unwrap();
        }
        for byte in encrypted {
            if tls.sock.write_all(&[byte]).is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    });
    let policy = UrlFetchPolicy::test_loopback().with_total_timeout(Duration::from_millis(500));
    let start = Instant::now();
    let result = fetch_url_text(
        &format!("https://127.0.0.1:{}/", addr.port()),
        policy,
        &[root],
    );
    let elapsed = start.elapsed();
    server.join().unwrap();
    assert!(result.is_err(), "expired fetch must refuse");
    assert!(
        elapsed < Duration::from_millis(1500),
        "TLS record fragments bypass total deadline: {elapsed:?}"
    );
}
