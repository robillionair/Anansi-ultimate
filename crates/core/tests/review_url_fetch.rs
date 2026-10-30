use anansi_core::url_fetch::{UrlFetchPolicy, fetch_url_text, is_public_address};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

#[test]
fn review_nonpublic_ipv6_destinations_are_refused() {
    for address in ["fec0::1", "::2"] {
        assert!(
            !is_public_address(address.parse().unwrap(), UrlFetchPolicy::strict()),
            "non-public destination admitted: {address}"
        );
    }
}

#[test]
fn review_content_length_dribble_obeys_total_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf);
        if stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 120\r\nConnection: close\r\n\r\n").is_err(){return;}
        for _ in 0..120 {
            if stream.write_all(b"x").is_err() {
                break;
            }
            let _ = stream.flush();
            std::thread::sleep(Duration::from_millis(200));
        }
    });
    let start = Instant::now();
    let result = fetch_url_text(
        &format!("http://{addr}/"),
        UrlFetchPolicy::test_loopback(),
        &[],
    );
    let elapsed = start.elapsed();
    server.join().unwrap();
    assert!(
        result.is_err(),
        "24-second drip completed successfully despite 20-second total budget; elapsed={elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(23),
        "total deadline exceeded: {elapsed:?}"
    );
}
