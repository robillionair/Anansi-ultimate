//! "Continue with ChatGPT" sign-in prototype tests, driven through
//! the actual application entry point (`Core::ai_command`) against
//! a synthetic local OAuth issuer. No test contacts OpenAI: the
//! issuer is a loopback server speaking the documented endpoints
//! (discovery, JWKS, token, revocation) and signing ID tokens with
//! a committed TEST RSA key (synthetic, never a real credential).
//!
//! The test simulates the browser by issuing the callback GET to
//! the loopback listener the start op bound — the same request a
//! system browser would deliver after the user consents.

use anansi_core::{CommandError, Core, ErrorCode};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Once};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Test RSA key (synthetic, committed for tests only).
//
// PKCS#8 DER, base64. The matching public JWK (kid "test-key-1")
// is served by the fake issuer at /jwks. This key signs ID tokens
// for the synthetic issuer only; it is not a real credential.

const TEST_KEY_DER_B64: &str = concat!(
    "MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCthrj4u4svO2XJhooFrIUp2z/nKaAT",
    "GcYOWHe66wyaet9G4eIfH3xqO2DMmD4/qdQZSrCsgn2nXsihAl8ERjguNSBVy8q5G1A2MtlVl2nKAwoA",
    "jPMKBohbEkuCjiwQLX+B9iomqfH4ITm3g64a1zLcfAxQDtJsuAlVqTa7D5Al1Lj2gCXkMKcjeQaRVEBm",
    "g6+3+sM7T0N5e9aBHliKTseEUDN6JEhcgFN44QILgObxaIuDwesQYYsy4TWN5GeCxe04v+A23T+uchhT",
    "G4Ro5dEgFjaVx3Eq5hgalJRg1nkroFjV55pazvXgnUpuahJ8adCMuYRV3puDUvwH6HlifV6jAgMBAAEC",
    "ggEAEaGlu8g6jTMI1aAw+ACeizE23gMRu78bq6t7hueJIU0o0Kf09NqbJqQf4sn5UTitT2CScQTVRaNQ",
    "2WNHSkTkQnWtcmpBj+thjBBAmvzGh/slEgdATp15T1+dgod1nLMyyH69q64K4v5lQetETn/dMB/+lEfd",
    "Q6Szdhu6bulW6TD4PEB4w3RUiu1F1gG+9sLugmmVG3sW+k1O0HX8kMWAI9sZgdGxAyoTTuevAS62XJze",
    "7vpzsVZNfjC0OQshrbqOOnoH6mfugk3B19daGYgNKNk+L+VTZF1QOzUtl2I1BcgU46c6B/1gNya40ceE",
    "kuEyrd7ySv3O7eQvBcROKq0Q2QKBgQDZVPAu71wvwam5QlFt1AAJXBzzI2MeZCLWUvPWY+v8Fjm787e8",
    "9lYFbzt2SUQBMFewOSyog2dguYqURnhxNj3BIlrwpiaxhB3j+pl5HZ+81tewj1JLJ15ZtEqzec+VC7RH",
    "PUajbDR0Ie6qA3kulqADKhmtrlvWGq/b+yw6fx403QKBgQDMZoX39XEC4t/D5fWFoD4rtWZFE6XGOflY",
    "13GNYoxu4ZCg7sEDT+cWEEw+VlX7Ci2D1ii+nkb75iqYck/U5XQ4qFGLemvci0ZluTni+HWIAcymgVgS",
    "14+3nQR0vL78HUP3AMUJkbTo/FlbxopiZvg4ri5/qFBzYV3tzhjcPbTpfwKBgQDYnLfnj9qqHrQzWc4f",
    "UTXYHl8cVyDroZK3W2Z9QIpsbxqMIpX3WO7T69GwS8XkCXLcyZ+hNZdL6KIetz5EWHOkk8FgKbpXI/9G",
    "wvdEqCoV5WyFJdQ3470+W1w5Fie3vjqo5oIJUEfXmBpwiZW2wZoqjUOawDX4aBDviKDEjP5mYQKBgFmU",
    "bQ44wSiUqaC/PZUsn1Kkk4eejr6LVd47vYZLNGS78pi1Sl3bPNmMlv2S9OeqlDv9Naokz9NCElTxdE9H",
    "qGHUUrigFHl5iIa4YMg6P7g1oa0vbpwDYPcFl5hZyBHR+aFc8v24n+KkBb2VC70Ih6y/mZ1U9L08ZN/9",
    "vSN437qdAoGAUOi6wIh/TXjkEgPMX0YZhvi3vc7XVag14pugOzbKEQ/GYe1SQs1RzBVGAqwfdvSvHVnb",
    "vqVKbnvM0G1ZvSuk3/rQueVSxhbqzISjRTc4FJ0Z1vt3WrSpxOHTGc/+F7YmtXKOOLv5QO3xwKb/9itB",
    "ij9J8UgF5lhUo1bDjvGlrzs=",
);
const TEST_KEY_KID: &str = "test-key-1";
const TEST_KEY_N: &str = "rYa4-LuLLztlyYaKBayFKds_5ymgExnGDlh3uusMmnrfRuHiHx98ajtgzJg-P6nUGUqwrIJ9p17IoQJfBEY4LjUgVcvKuRtQNjLZVZdpygMKAIzzCgaIWxJLgo4sEC1_gfYqJqnx-CE5t4OuGtcy3HwMUA7SbLgJVak2uw-QJdS49oAl5DCnI3kGkVRAZoOvt_rDO09DeXvWgR5Yik7HhFAzeiRIXIBTeOECC4Dm8WiLg8HrEGGLMuE1jeRngsXtOL_gNt0_rnIYUxuEaOXRIBY2lcdxKuYYGpSUYNZ5K6BY1eeaWs714J1KbmoSfGnQjLmEVd6bg1L8B-h5Yn1eow";
const TEST_KEY_E: &str = "AQAB";

fn b64u(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Mint a test ID token signed with the synthetic issuer's key.
fn mint_id_token(
    issuer_base: &str,
    client_id: &str,
    nonce: Option<&str>,
    exp_offset_secs: i64,
    tamper_signature: bool,
) -> String {
    use base64::Engine;
    let der = base64::engine::general_purpose::STANDARD
        .decode(TEST_KEY_DER_B64)
        .unwrap();
    let key = ring::signature::RsaKeyPair::from_pkcs8(&der).unwrap();
    let now = now_secs();
    let header = json!({"alg": "RS256", "typ": "JWT", "kid": TEST_KEY_KID});
    let mut claims = json!({
        "iss": issuer_base,
        "aud": client_id,
        "sub": "sub-test-user",
        "email": "user@example.com",
        "name": "Test User",
        "iat": now,
        "exp": now + exp_offset_secs,
    });
    if let Some(n) = nonce {
        claims["nonce"] = json!(n);
    }
    let header_b64 = b64u(&serde_json::to_vec(&header).unwrap());
    let payload_b64 = b64u(&serde_json::to_vec(&claims).unwrap());
    let signing_input = format!("{header_b64}.{payload_b64}");
    let mut signature = vec![0u8; 256];
    key.sign(
        &ring::signature::RSA_PKCS1_SHA256,
        &ring::rand::SystemRandom::new(),
        signing_input.as_bytes(),
        &mut signature,
    )
    .unwrap();
    if tamper_signature {
        signature[0] ^= 0xff;
    }
    format!("{signing_input}.{}", b64u(&signature))
}

// ---------------------------------------------------------------------------
// Synthetic OAuth issuer

#[derive(Debug, Clone)]
struct TokenRequest {
    grant_type: String,
    client_id: String,
    code: Option<String>,
    code_verifier: Option<String>,
    redirect_uri: Option<String>,
    resource: Option<String>,
    refresh_token: Option<String>,
}

#[derive(Debug, Clone)]
struct IssuerMode {
    /// Nonce the ID token must carry (from the authorization URL).
    expected_nonce: Option<String>,
    /// `exp` offset for issued ID tokens (default 3600; negative
    /// means already expired).
    id_token_exp_offset: i64,
    /// Corrupt the ID token signature.
    tamper_signature: bool,
    /// Scopes granted by the token endpoint.
    scopes: Vec<String>,
    /// Access token issued on a code grant.
    access_token: String,
    /// Refresh token issued on a code grant.
    refresh_token: String,
    /// Access token issued on a refresh grant.
    refresh_access_token: String,
    /// Refresh token issued on a refresh grant (rotation).
    refresh_refresh_token: String,
    /// `expires_in` for issued tokens.
    expires_in: u64,
    /// The refresh grant fails with `invalid_grant`.
    refresh_invalid_grant: bool,
    /// HTTP status for revocation (default 200).
    revoke_status: u16,
}

impl Default for IssuerMode {
    fn default() -> Self {
        Self {
            expected_nonce: None,
            id_token_exp_offset: 3600,
            tamper_signature: false,
            scopes: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
                "offline_access".to_string(),
                "resource.invoke".to_string(),
                "chatgpt.tokens.use.direct".to_string(),
            ],
            access_token: "at-test-access-0001".to_string(),
            refresh_token: "rt-test-refresh-0001".to_string(),
            refresh_access_token: "at-test-access-0002".to_string(),
            refresh_refresh_token: "rt-test-refresh-0002".to_string(),
            expires_in: 3600,
            refresh_invalid_grant: false,
            revoke_status: 200,
        }
    }
}

struct FakeIssuer {
    base_url: String,
    mode: Arc<Mutex<IssuerMode>>,
    token_hits: Arc<AtomicUsize>,
    revoke_hits: Arc<AtomicUsize>,
    last_token_request: Arc<Mutex<Option<TokenRequest>>>,
    last_revoke_token: Arc<Mutex<Option<String>>>,
}

fn read_http_request(stream: &mut std::net::TcpStream) -> Option<(String, String, Vec<u8>)> {
    let mut raw = Vec::new();
    let mut buf = [0u8; 4096];
    let header_end;
    loop {
        let n = stream.read(&mut buf).unwrap_or(0);
        if n == 0 {
            return None;
        }
        raw.extend_from_slice(&buf[..n]);
        if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }
        if raw.len() > 16_384 {
            return None;
        }
    }
    let head = String::from_utf8_lossy(&raw[..header_end]).to_string();
    let mut content_length = 0usize;
    for line in head.lines().skip(1) {
        if let Some((k, v)) = line.split_once(':')
            && k.trim().eq_ignore_ascii_case("content-length")
        {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    while raw.len() - header_end < content_length {
        let n = stream.read(&mut buf).unwrap_or(0);
        if n == 0 {
            break;
        }
        raw.extend_from_slice(&buf[..n]);
    }
    let first = head.lines().next().unwrap_or("");
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("").to_string();
    let body = raw[header_end..].to_vec();
    Some((method, target, body))
}

fn write_http(stream: &mut std::net::TcpStream, status: u16, body: &str) {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let _ = stream.write_all(
        format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .as_bytes(),
    );
    let _ = stream.flush();
}

fn start_fake_issuer() -> FakeIssuer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let mode = Arc::new(Mutex::new(IssuerMode::default()));
    let mode_thread = mode.clone();
    let token_hits = Arc::new(AtomicUsize::new(0));
    let token_hits_thread = token_hits.clone();
    let revoke_hits = Arc::new(AtomicUsize::new(0));
    let revoke_hits_thread = revoke_hits.clone();
    let last_token_request = Arc::new(Mutex::new(None));
    let last_token_thread = last_token_request.clone();
    let last_revoke_token = Arc::new(Mutex::new(None));
    let last_revoke_thread = last_revoke_token.clone();
    let base_thread = base_url.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mode = mode_thread.clone();
            let token_hits = token_hits_thread.clone();
            let revoke_hits = revoke_hits_thread.clone();
            let last_token_request = last_token_thread.clone();
            let last_revoke_token = last_revoke_thread.clone();
            let base_url = base_thread.clone();
            std::thread::spawn(move || {
                let Some((method, target, body)) = read_http_request(&mut stream) else {
                    return;
                };
                let path = target.split('?').next().unwrap_or("").to_string();
                match (method.as_str(), path.as_str()) {
                    ("GET", "/.well-known/openid-configuration") => {
                        write_http(
                            &mut stream,
                            200,
                            &json!({
                                "issuer": base_url,
                                "jwks_uri": format!("{base_url}/jwks"),
                                "revocation_endpoint": format!("{base_url}/api/accounts/oauth/revoke"),
                            })
                            .to_string(),
                        );
                    }
                    ("GET", "/jwks") => {
                        write_http(
                            &mut stream,
                            200,
                            &json!({
                                "keys": [{
                                    "kty": "RSA", "alg": "RS256", "use": "sig",
                                    "kid": TEST_KEY_KID, "n": TEST_KEY_N, "e": TEST_KEY_E,
                                }],
                            })
                            .to_string(),
                        );
                    }
                    ("POST", "/api/accounts/oauth/token") => {
                        token_hits.fetch_add(1, Ordering::SeqCst);
                        let form: HashMap<String, String> =
                            url::form_urlencoded::parse(&body).into_owned().collect();
                        let request = TokenRequest {
                            grant_type: form.get("grant_type").cloned().unwrap_or_default(),
                            client_id: form.get("client_id").cloned().unwrap_or_default(),
                            code: form.get("code").cloned(),
                            code_verifier: form.get("code_verifier").cloned(),
                            redirect_uri: form.get("redirect_uri").cloned(),
                            resource: form.get("resource").cloned(),
                            refresh_token: form.get("refresh_token").cloned(),
                        };
                        *last_token_request.lock().unwrap() = Some(request.clone());
                        let mode = mode.lock().unwrap().clone();
                        if request.grant_type == "refresh_token" && mode.refresh_invalid_grant {
                            write_http(
                                &mut stream,
                                400,
                                &json!({"error": "invalid_grant"}).to_string(),
                            );
                            return;
                        }
                        let is_refresh = request.grant_type == "refresh_token";
                        let nonce = if is_refresh {
                            None
                        } else {
                            mode.expected_nonce.clone()
                        };
                        let id_token = mint_id_token(
                            &base_url,
                            &request.client_id,
                            nonce.as_deref(),
                            mode.id_token_exp_offset,
                            mode.tamper_signature,
                        );
                        write_http(
                            &mut stream,
                            200,
                            &json!({
                                "access_token": if is_refresh { mode.refresh_access_token } else { mode.access_token },
                                "token_type": "Bearer",
                                "expires_in": mode.expires_in,
                                "refresh_token": if is_refresh { mode.refresh_refresh_token } else { mode.refresh_token },
                                "id_token": id_token,
                                "scope": mode.scopes.join(" "),
                            })
                            .to_string(),
                        );
                    }
                    ("POST", "/api/accounts/oauth/revoke") => {
                        revoke_hits.fetch_add(1, Ordering::SeqCst);
                        let form: HashMap<String, String> =
                            url::form_urlencoded::parse(&body).into_owned().collect();
                        *last_revoke_token.lock().unwrap() =
                            form.get("token").cloned();
                        let status = mode.lock().unwrap().revoke_status;
                        write_http(&mut stream, status, "");
                    }
                    _ => write_http(&mut stream, 404, &json!({"error": "not_found"}).to_string()),
                }
            });
        }
    });
    FakeIssuer {
        base_url,
        mode,
        token_hits,
        revoke_hits,
        last_token_request,
        last_revoke_token,
    }
}

impl FakeIssuer {
    fn set_mode(&self, f: impl FnOnce(&mut IssuerMode)) {
        f(&mut self.mode.lock().unwrap());
    }
}

// ---------------------------------------------------------------------------
// Harness

static ISSUER_LOCK: Mutex<()> = Mutex::new(());

/// Serialize every test that sets the issuer env var (process
/// global). The listener thread reads it during exchange, so the
/// guard must be held for the whole test.
fn issuer_lock() -> MutexGuard<'static, ()> {
    ISSUER_LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

/// Install the keyring mock once per test process (same pattern as
/// the provider-dispatch tests) and the credential test hook.
fn test_env_once() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        unsafe {
            std::env::set_var("ANANSI_PROVIDER_TEST_CREDENTIAL", "<redacted>");
        }
    });
}

struct IssuerGuard;

impl IssuerGuard {
    fn set(base_url: &str) -> Self {
        unsafe {
            std::env::set_var("ANANSI_CHATGPT_TEST_ISSUER", base_url);
        }
        IssuerGuard
    }
}

impl Drop for IssuerGuard {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var("ANANSI_CHATGPT_TEST_ISSUER");
        }
    }
}

fn create_workspace(core: &Core, name: &str) -> String {
    let details = core
        .workspace_command("create", json!({ "contractVersion": 1, "name": name }))
        .unwrap();
    details["workspace"]["id"].as_str().unwrap().to_string()
}

fn core() -> (tempfile::TempDir, Core, String) {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(dir.path().to_path_buf());
    let ws = create_workspace(&core, "main");
    (dir, core, ws)
}

fn stamped(session_id: &str, mut body: Value) -> Value {
    body["callerContext"] = json!({ "session": session_id });
    body
}

fn signin_op(core: &Core, op: &str, ws: &str, client_id: Option<&str>) -> Result<Value, CommandError> {
    let mut body = json!({ "workspaceId": ws });
    if let Some(id) = client_id {
        body["clientId"] = json!(id);
    }
    core.ai_command(op, body)
}

fn signin_op_browser(
    core: &Core,
    op: &str,
    ws: &str,
    session: &str,
) -> Result<Value, CommandError> {
    core.ai_command(op, stamped(session, json!({ "workspaceId": ws })))
}

fn status(core: &Core, ws: &str) -> Value {
    signin_op(core, "chatgpt-signin-status", ws, None).unwrap()
}

/// Poll status until `predicate` holds or the timeout elapses.
fn poll_status(core: &Core, ws: &str, timeout: Duration, predicate: impl Fn(&Value) -> bool) -> Value {
    let start = Instant::now();
    loop {
        let s = status(core, ws);
        if predicate(&s) {
            return s;
        }
        if start.elapsed() > timeout {
            panic!("status predicate not met in time; last status: {s}");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Parse a URL query string into a map.
fn parse_query(url: &str) -> HashMap<String, String> {
    let query = url.split_once('?').map(|(_, q)| q).unwrap_or("");
    url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect()
}

/// Issue the callback GET a system browser would deliver after
/// the user consents. Returns (status, body).
fn callback_get(url: &str) -> (u16, String) {
    let parsed = url::Url::parse(url).unwrap();
    let port = parsed.port().unwrap();
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    let target = format!(
        "{}?{}",
        parsed.path(),
        parsed.query().unwrap_or("")
    );
    stream
        .write_all(
            format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").as_bytes(),
        )
        .unwrap();
    let mut raw = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => raw.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }
    let text = String::from_utf8_lossy(&raw).to_string();
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = text.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body)
}

/// Start a sign-in and drive the browser half: extract state +
/// nonce from the authorization URL, arm the issuer with the
/// nonce, and deliver the callback for `issued_client_id`.
/// Returns the start response.
fn drive_signin(
    core: &Core,
    ws: &str,
    issuer: &FakeIssuer,
    issued_client_id: &str,
) -> Value {
    let start = signin_op(core, "chatgpt-signin-start", ws, None).unwrap();
    let url = start["authorizationUrl"].as_str().unwrap().to_string();
    let params = parse_query(&url);
    let state = params.get("state").cloned().unwrap();
    let nonce = params.get("nonce").cloned().unwrap();
    issuer.set_mode(|m| m.expected_nonce = Some(nonce));
    let redirect_uri = start["redirectUri"].as_str().unwrap();
    let callback = format!(
        "{redirect_uri}?code=code-test-0001&state={state}&client_id={issued_client_id}"
    );
    let (http_status, _body) = callback_get(&callback);
    assert_eq!(http_status, 200);
    start
}

static CLIENT_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A fresh issued client ID per test (the keyring mock is
/// process-global, so records must not collide across tests).
fn fresh_client_id() -> String {
    format!(
        "oaiapp_test_{}",
        CLIENT_COUNTER.fetch_add(1, Ordering::SeqCst)
    )
}

// ---------------------------------------------------------------------------
// Minimal provider fakes for the regression tests (AC-S9)

struct FakeProvider {
    base_url: String,
    hits: Arc<AtomicUsize>,
}

fn start_fake_provider() -> FakeProvider {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_thread = hits.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let hits = hits_thread.clone();
            std::thread::spawn(move || {
                let Some((_, target, _)) = read_http_request(&mut stream) else {
                    return;
                };
                hits.fetch_add(1, Ordering::SeqCst);
                let path = target.split('?').next().unwrap_or("");
                let body = if path == "/api/chat" {
                    "{\"model\":\"llama-x\",\"message\":{\"role\":\"assistant\",\"content\":\"core reply\"},\"done\":true,\"prompt_eval_count\":4,\"eval_count\":2}".to_string()
                } else {
                    "{\"id\":\"x\",\"choices\":[{\"message\":{\"role\":\"assistant\",\"content\":\"authed reply\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":2}}".to_string()
                };
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .as_bytes(),
                );
                let _ = stream.flush();
            });
        }
    });
    FakeProvider {
        base_url: format!("http://127.0.0.1:{port}"),
        hits,
    }
}

fn chat(text: &str) -> Value {
    json!({
        "model": "llama-x",
        "messages": [{
            "role": "user",
            "segments": [{ "text": text, "privacy": "standard" }]
        }]
    })
}

fn configure(core: &Core, ws: &str, config: Value) -> Result<Value, CommandError> {
    core.ai_command(
        "providers-configure",
        json!({ "workspaceId": ws, "config": config }),
    )
}

fn ollama_config(base_url: &str) -> Value {
    json!({
        "providers": [{
            "config": {
                "provider": "ollama",
                "baseUrl": base_url,
                "credential": null,
                "enabled": true
            },
            "locality": "local",
            "models": [{
                "id": "llama-x",
                "provider": "ollama",
                "capabilities": { "chat": true, "streaming": true, "vision": false,
                                  "coding": true, "deepReasoning": false, "embeddings": false },
                "contextWindow": 8192,
                "cost": "free",
                "speed": "balanced"
            }]
        }]
    })
}

fn dispatch(core: &Core, ws: &str, extra: Value, text: &str) -> Result<Value, CommandError> {
    let mut body = json!({ "workspaceId": ws, "request": chat(text), "mode": "auto" });
    if let (Some(dst), Some(src)) = (body.as_object_mut(), extra.as_object()) {
        for (k, v) in src {
            dst.insert(k.clone(), v.clone());
        }
    }
    core.ai_command("providers-dispatch", stamped("host-session", body))
}

// ---------------------------------------------------------------------------
// Tests: start + authorization URL (AC-S2)

#[test]
fn signin_start_returns_documented_authorization_url() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    let start = signin_op(&core, "chatgpt-signin-start", &ws, None).unwrap();
    let url = start["authorizationUrl"].as_str().unwrap();
    let redirect_uri = start["redirectUri"].as_str().unwrap();
    assert!(url.starts_with(&format!("{}/api/accounts/authorize?", issuer.base_url)));
    assert!(url.contains("client_id=dynamic_agent_client"));
    assert!(url.contains("agent_name_hint=ANANSI"));
    assert!(url.contains("ext_agent_host_id=urn%3Auuid%3A"));
    assert!(url.contains("response_type=code"));
    assert!(url.contains("scope=openid+profile+email+offline_access+resource.invoke+chatgpt.tokens.use.direct"));
    assert!(url.contains("resource=https%3A%2F%2Fapi.openai.com%2Fv1"));
    assert!(url.contains("code_challenge_method=S256"));
    assert!(!url.contains("localhost"));
    // The listener is already bound: the redirect port answers.
    assert!(redirect_uri.starts_with("http://127.0.0.1:"));
    assert!(redirect_uri.ends_with("/auth/callback"));
    let port: u16 = redirect_uri
        .trim_start_matches("http://127.0.0.1:")
        .trim_end_matches("/auth/callback")
        .parse()
        .unwrap();
    let probe = std::net::TcpStream::connect(("127.0.0.1", port));
    assert!(probe.is_ok(), "loopback listener is not bound before the URL is returned");
    // The host id is persisted (labels only) under provider-config.
    let host_path = _dir
        .path()
        .join("provider-config")
        .join("chatgpt-host.json");
    assert!(host_path.exists());
    let host_json: Value =
        serde_json::from_str(&std::fs::read_to_string(&host_path).unwrap()).unwrap();
    assert!(host_json["ext_agent_host_id"]
        .as_str()
        .unwrap()
        .starts_with("urn:uuid:"));
    // A second start reuses the same host id.
    let start2 = signin_op(&core, "chatgpt-signin-start", &ws, None).unwrap();
    assert_eq!(
        parse_query(start["authorizationUrl"].as_str().unwrap())["ext_agent_host_id"],
        parse_query(start2["authorizationUrl"].as_str().unwrap())["ext_agent_host_id"]
    );
}

// ---------------------------------------------------------------------------
// Tests: full flow + storage (AC-S2, AC-S4, AC-S5)

#[test]
fn signin_full_flow_stores_tokens_in_keyring_only() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    let client_id = fresh_client_id();
    drive_signin(&core, &ws, &issuer, &client_id);
    let s = poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "succeeded"
    });
    assert_eq!(s["signedIn"], true);
    assert_eq!(s["email"], "user@example.com");
    assert_eq!(s["planUsage"], true);
    assert!(s["expiresAtMs"].as_u64().unwrap() > 0);
    assert_eq!(s["accounts"].as_array().unwrap().len(), 1);
    assert_eq!(s["accounts"][0]["clientId"], client_id);
    // The exchange happened exactly once, with the documented
    // grant shape: issued client id, code verifier, same redirect
    // URI and resource — never dynamic_agent_client.
    assert_eq!(issuer.token_hits.load(Ordering::SeqCst), 1);
    let last = issuer.last_token_request.lock().unwrap().clone().unwrap();
    assert_eq!(last.grant_type, "authorization_code");
    assert_eq!(last.client_id, client_id);
    assert_eq!(last.code.as_deref(), Some("code-test-0001"));
    assert!(last.code_verifier.as_deref().map(|v| v.len() >= 43).unwrap_or(false));
    assert_eq!(last.resource.as_deref(), Some("https://api.openai.com/v1"));
    assert!(last.redirect_uri.as_deref().unwrap().starts_with("http://127.0.0.1:"));
    // No token material anywhere on disk: the provider-config
    // files hold labels only (host id, registration stub).
    let mut found_token = false;
    for entry in walkdir(&_dir.path().join("provider-config")) {
        let text = std::fs::read_to_string(&entry).unwrap_or_default();
        if text.contains("at-test-access-0001")
            || text.contains("rt-test-refresh-0001")
            || text.contains("eyJ")
        {
            found_token = true;
        }
    }
    assert!(!found_token, "token material leaked into a file");
    // ... and none in the status response either.
    let status_text = serde_json::to_string(&s).unwrap();
    assert!(!status_text.contains("at-test-access-0001"));
    assert!(!status_text.contains("rt-test-refresh-0001"));
}

fn walkdir(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(walkdir(&path));
            } else {
                out.push(path);
            }
        }
    }
    out
}

#[test]
fn signin_reauthorization_uses_saved_client_id_and_hint() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    let client_id = fresh_client_id();
    drive_signin(&core, &ws, &issuer, &client_id);
    poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "succeeded"
    });
    let start = signin_op(&core, "chatgpt-signin-start", &ws, Some(&client_id)).unwrap();
    let url = start["authorizationUrl"].as_str().unwrap();
    let params = parse_query(url);
    assert_eq!(params["client_id"], client_id);
    assert!(!params.contains_key("agent_name_hint"));
    assert!(params.contains_key("id_token_hint"));
    assert_eq!(
        params.get("login_hint").map(|s| s.as_str()),
        Some("user@example.com")
    );
    // An unknown client ID cannot be re-authorized.
    let err = signin_op(&core, "chatgpt-signin-start", &ws, Some("oaiapp_nope")).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
}

// ---------------------------------------------------------------------------
// Tests: callback failure paths (AC-S3, AC-S4)

#[test]
fn signin_wrong_state_is_refused_without_exchange() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    let start = signin_op(&core, "chatgpt-signin-start", &ws, None).unwrap();
    let redirect_uri = start["redirectUri"].as_str().unwrap();
    let callback = format!(
        "{redirect_uri}?code=code-test-0001&state=wrong-state&client_id={}",
        fresh_client_id()
    );
    let (http_status, body) = callback_get(&callback);
    assert_eq!(http_status, 200);
    assert!(body.contains("does not match"));
    let s = poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "failed"
    });
    assert_eq!(s["pendingAttempt"]["reason"], "state");
    assert_eq!(s["signedIn"], false);
    assert_eq!(issuer.token_hits.load(Ordering::SeqCst), 0);
}

#[test]
fn signin_replayed_callback_is_rejected() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    let client_id = fresh_client_id();
    let start = signin_op(&core, "chatgpt-signin-start", &ws, None).unwrap();
    let url = start["authorizationUrl"].as_str().unwrap();
    let params = parse_query(url);
    let state = params.get("state").cloned().unwrap();
    let nonce = params.get("nonce").cloned().unwrap();
    issuer.set_mode(|m| m.expected_nonce = Some(nonce));
    let redirect_uri = start["redirectUri"].as_str().unwrap();
    let callback = format!(
        "{redirect_uri}?code=code-test-0001&state={state}&client_id={client_id}"
    );
    let (http_status, _) = callback_get(&callback);
    assert_eq!(http_status, 200);
    poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "succeeded"
    });
    // The same callback delivered again: rejected, no second
    // exchange, the first record intact.
    let (http_status, body) = callback_get(&callback);
    assert_eq!(http_status, 200);
    assert!(body.contains("already used"));
    assert_eq!(issuer.token_hits.load(Ordering::SeqCst), 1);
    let s = status(&core, &ws);
    assert_eq!(s["signedIn"], true);
}

#[test]
fn signin_denied_consent_stops_before_exchange() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    let start = signin_op(&core, "chatgpt-signin-start", &ws, None).unwrap();
    let params = parse_query(start["authorizationUrl"].as_str().unwrap());
    let state = params.get("state").cloned().unwrap();
    let redirect_uri = start["redirectUri"].as_str().unwrap();
    let callback =
        format!("{redirect_uri}?error=access_denied&state={state}");
    let (http_status, body) = callback_get(&callback);
    assert_eq!(http_status, 200);
    assert!(body.contains("declined"));
    let s = poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "failed"
    });
    assert_eq!(s["pendingAttempt"]["reason"], "denied");
    assert_eq!(s["signedIn"], false);
    assert_eq!(issuer.token_hits.load(Ordering::SeqCst), 0);
}

#[test]
fn signin_new_registration_without_client_id_is_incomplete() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    let start = signin_op(&core, "chatgpt-signin-start", &ws, None).unwrap();
    let params = parse_query(start["authorizationUrl"].as_str().unwrap());
    let state = params.get("state").cloned().unwrap();
    let redirect_uri = start["redirectUri"].as_str().unwrap();
    let callback = format!("{redirect_uri}?code=code-test-0001&state={state}");
    callback_get(&callback);
    let s = poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "failed"
    });
    assert_eq!(s["pendingAttempt"]["reason"], "missing-client-id");
    assert_eq!(issuer.token_hits.load(Ordering::SeqCst), 0);
}

#[test]
fn signin_callback_client_id_mismatch_is_rejected() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    let client_id = fresh_client_id();
    drive_signin(&core, &ws, &issuer, &client_id);
    poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "succeeded"
    });
    // Re-authorize the saved account, but the callback names a
    // different registration.
    let start = signin_op(&core, "chatgpt-signin-start", &ws, Some(&client_id)).unwrap();
    let params = parse_query(start["authorizationUrl"].as_str().unwrap());
    let state = params.get("state").cloned().unwrap();
    let redirect_uri = start["redirectUri"].as_str().unwrap();
    let other = fresh_client_id();
    let callback = format!(
        "{redirect_uri}?code=code-test-0001&state={state}&client_id={other}"
    );
    callback_get(&callback);
    let s = poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "failed"
    });
    assert_eq!(s["pendingAttempt"]["reason"], "client-id");
    // The first record is untouched: still signed in.
    assert_eq!(s["signedIn"], true);
}

#[test]
fn signin_wrong_nonce_is_refused() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    let start = signin_op(&core, "chatgpt-signin-start", &ws, None).unwrap();
    let params = parse_query(start["authorizationUrl"].as_str().unwrap());
    let state = params.get("state").cloned().unwrap();
    // The issuer mints a token with a different nonce than the
    // attempt saved.
    issuer.set_mode(|m| m.expected_nonce = Some("someone-elses-nonce".to_string()));
    let redirect_uri = start["redirectUri"].as_str().unwrap();
    let callback = format!(
        "{redirect_uri}?code=code-test-0001&state={state}&client_id={}",
        fresh_client_id()
    );
    callback_get(&callback);
    let s = poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "failed"
    });
    assert_eq!(s["pendingAttempt"]["reason"], "token-nonce");
    assert_eq!(s["signedIn"], false);
}

#[test]
fn signin_expired_id_token_is_refused() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    issuer.set_mode(|m| m.id_token_exp_offset = -3600);
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    drive_signin(&core, &ws, &issuer, &fresh_client_id());
    let s = poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "failed"
    });
    assert_eq!(s["pendingAttempt"]["reason"], "token-expired");
    assert_eq!(s["signedIn"], false);
}

#[test]
fn signin_tampered_signature_is_refused() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    issuer.set_mode(|m| m.tamper_signature = true);
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    drive_signin(&core, &ws, &issuer, &fresh_client_id());
    let s = poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "failed"
    });
    assert_eq!(s["pendingAttempt"]["reason"], "token-signature");
    assert_eq!(s["signedIn"], false);
}

#[test]
fn signin_missing_plan_scope_is_identity_only() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    issuer.set_mode(|m| {
        m.scopes = vec![
            "openid".to_string(),
            "profile".to_string(),
            "email".to_string(),
            "offline_access".to_string(),
            "resource.invoke".to_string(),
        ]
    });
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    drive_signin(&core, &ws, &issuer, &fresh_client_id());
    let s = poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "succeeded"
    });
    // Identity is retained, plan usage is disabled, and nothing
    // proceeds to inference on this path.
    assert_eq!(s["signedIn"], true);
    assert_eq!(s["email"], "user@example.com");
    assert_eq!(s["planUsage"], false);
    assert_eq!(s["planUsageReason"], "scope_missing");
}

// ---------------------------------------------------------------------------
// Tests: refresh + disconnect (AC-S6, AC-S7)

#[test]
fn signin_refresh_rotates_tokens() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    issuer.set_mode(|m| m.expires_in = 60);
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    let client_id = fresh_client_id();
    drive_signin(&core, &ws, &issuer, &client_id);
    let before = poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "succeeded"
    });
    let expires_before = before["expiresAtMs"].as_u64().unwrap();
    // expires_in 60s is inside the 300s refresh window, so a
    // refresh is due immediately.
    let refreshed = signin_op(&core, "chatgpt-signin-refresh", &ws, Some(&client_id)).unwrap();
    assert_eq!(refreshed["refreshed"], true);
    assert!(refreshed["expiresAtMs"].as_u64().unwrap() > expires_before);
    // The refresh grant used the issued client ID (never the
    // dynamic entrypoint) and the saved refresh token, and the
    // rotating tokens were replaced together.
    let last = issuer.last_token_request.lock().unwrap().clone().unwrap();
    assert_eq!(last.grant_type, "refresh_token");
    assert_eq!(last.client_id, client_id);
    assert_eq!(last.refresh_token.as_deref(), Some("rt-test-refresh-0001"));
    let after = status(&core, &ws);
    assert_eq!(after["signedIn"], true);
    assert_eq!(after["planUsage"], true);
    // A second refresh right away is still due (expires_in is
    // 60s) and chains off the ROTATED refresh token, proving the
    // replacement was stored.
    let again = signin_op(&core, "chatgpt-signin-refresh", &ws, Some(&client_id)).unwrap();
    assert_eq!(again["refreshed"], true);
    let last = issuer.last_token_request.lock().unwrap().clone().unwrap();
    assert_eq!(last.grant_type, "refresh_token");
    assert_eq!(last.refresh_token.as_deref(), Some("rt-test-refresh-0002"));
}

#[test]
fn signin_refresh_nothing_to_do_when_token_is_fresh() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    let client_id = fresh_client_id();
    drive_signin(&core, &ws, &issuer, &client_id);
    poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "succeeded"
    });
    // expires_in 3600 is outside the 300s refresh window: no
    // network call, no new tokens.
    let out = signin_op(&core, "chatgpt-signin-refresh", &ws, Some(&client_id)).unwrap();
    assert_eq!(out["refreshed"], false);
    assert_eq!(issuer.token_hits.load(Ordering::SeqCst), 1);
}

#[test]
fn signin_refresh_invalid_grant_clears_record() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    issuer.set_mode(|m| m.expires_in = 60);
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    let client_id = fresh_client_id();
    drive_signin(&core, &ws, &issuer, &client_id);
    poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "succeeded"
    });
    issuer.set_mode(|m| m.refresh_invalid_grant = true);
    let err = signin_op(&core, "chatgpt-signin-refresh", &ws, Some(&client_id)).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert!(err.message.contains("Sign in again"));
    let s = status(&core, &ws);
    assert_eq!(s["signedIn"], false);
    // The registration stub survives for re-sign-in with the
    // issued client ID.
    assert_eq!(s["accounts"].as_array().unwrap().len(), 1);
    assert_eq!(s["accounts"][0]["clientId"], client_id);
}

#[test]
fn signin_refresh_is_serialized() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    issuer.set_mode(|m| m.expires_in = 60);
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    let client_id = fresh_client_id();
    drive_signin(&core, &ws, &issuer, &client_id);
    poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "succeeded"
    });
    // Two concurrent refreshes: exactly one token exchange, one
    // success, one refusal — the rotating token is never raced.
    let core = Arc::new(core);
    let ws_arc = Arc::new(ws);
    let client_arc = Arc::new(client_id);
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let core = core.clone();
            let ws = ws_arc.clone();
            let client_id = client_arc.clone();
            std::thread::spawn(move || {
                signin_op(&core, "chatgpt-signin-refresh", &ws, Some(&client_id))
            })
        })
        .collect();
    let mut successes = 0;
    let mut refusals = 0;
    for handle in handles {
        match handle.join().unwrap() {
            Ok(v) => {
                assert_eq!(v["refreshed"], true);
                successes += 1;
            }
            Err(e) => {
                assert_eq!(e.code, ErrorCode::InvalidRequest);
                assert!(e.message.contains("already running"));
                refusals += 1;
            }
        }
    }
    assert_eq!(successes, 1);
    assert_eq!(refusals, 1);
    let refresh_grants = issuer
        .token_hits
        .load(Ordering::SeqCst)
        .saturating_sub(1); // minus the code exchange
    assert_eq!(refresh_grants, 1);
}

#[test]
fn signin_disconnect_revokes_and_clears() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    let client_id = fresh_client_id();
    drive_signin(&core, &ws, &issuer, &client_id);
    poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "succeeded"
    });
    let out = signin_op(&core, "chatgpt-signin-disconnect", &ws, Some(&client_id)).unwrap();
    assert_eq!(out["disconnected"], true);
    assert_eq!(out["remoteRevocationConfirmed"], true);
    assert_eq!(issuer.revoke_hits.load(Ordering::SeqCst), 1);
    assert_eq!(
        issuer.last_revoke_token.lock().unwrap().as_deref(),
        Some("rt-test-refresh-0001")
    );
    let s = status(&core, &ws);
    assert_eq!(s["signedIn"], false);
    // The registration stub survives for re-sign-in.
    assert_eq!(s["accounts"].as_array().unwrap().len(), 1);
    assert_eq!(s["accounts"][0]["clientId"], client_id);
    assert_eq!(s["accounts"][0]["planUsage"], false);
}

#[test]
fn signin_disconnect_unconfirmed_revocation_is_reported() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    issuer.set_mode(|m| m.revoke_status = 500);
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    let client_id = fresh_client_id();
    drive_signin(&core, &ws, &issuer, &client_id);
    poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "succeeded"
    });
    let out = signin_op(&core, "chatgpt-signin-disconnect", &ws, Some(&client_id)).unwrap();
    assert_eq!(out["disconnected"], true);
    assert_eq!(out["remoteRevocationConfirmed"], false);
    // Local tokens are cleared even when revocation cannot be
    // confirmed.
    let s = status(&core, &ws);
    assert_eq!(s["signedIn"], false);
}

// ---------------------------------------------------------------------------
// Tests: browser boundaries (AC-S8)

#[test]
fn browser_cannot_start_refresh_or_disconnect() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    for op in [
        "chatgpt-signin-start",
        "chatgpt-signin-refresh",
        "chatgpt-signin-disconnect",
    ] {
        let err = signin_op_browser(&core, op, &ws, "browser-1").unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden, "op {op}");
    }
    // No attempt was created by the refused starts.
    let s = status(&core, &ws);
    assert!(s["pendingAttempt"].is_null());
    // The refusals are recorded for the host's approval queue.
    let access = core
        .ai_command("providers-session-access", json!({ "workspaceId": ws }))
        .unwrap();
    let pending = access["pending"].as_array().unwrap();
    assert!(
        pending.iter().any(|p| p["session"] == "browser-1" && p["attempts"].as_u64().unwrap() >= 3),
        "browser refusals not recorded: {pending:?}"
    );
}

#[test]
fn browser_status_shows_labels_only() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    let client_id = fresh_client_id();
    drive_signin(&core, &ws, &issuer, &client_id);
    poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "succeeded"
    });
    let s = signin_op_browser(&core, "chatgpt-signin-status", &ws, "browser-1").unwrap();
    assert_eq!(s["signedIn"], true);
    assert_eq!(s["email"], "user@example.com");
    let text = serde_json::to_string(&s).unwrap();
    assert!(!text.contains("at-test-access-0001"));
    assert!(!text.contains("rt-test-refresh-0001"));
    assert!(!text.contains("eyJ"));
}

// ---------------------------------------------------------------------------
// Tests: regression after a failed sign-in (AC-S9)

#[test]
fn failed_signin_leaves_local_and_apikey_flows_working() {
    let _lock = issuer_lock();
    test_env_once();
    let issuer = start_fake_issuer();
    let _guard = IssuerGuard::set(&issuer.base_url);
    let (_dir, core, ws) = core();
    // A failed sign-in (wrong state) first.
    let start = signin_op(&core, "chatgpt-signin-start", &ws, None).unwrap();
    let redirect_uri = start["redirectUri"].as_str().unwrap();
    callback_get(&format!(
        "{redirect_uri}?code=code-test-0001&state=wrong-state&client_id={}",
        fresh_client_id()
    ));
    poll_status(&core, &ws, Duration::from_secs(10), |s| {
        s["pendingAttempt"]["status"] == "failed"
    });
    // Local Ollama dispatch still works.
    let fake = start_fake_provider();
    configure(&core, &ws, ollama_config(&fake.base_url)).unwrap();
    let permit = core
        .ai_command(
            "providers-authorize",
            json!({
                "workspaceId": ws,
                "provider": "ollama", "model": "llama-x",
                "maxEstimatedTotalTokens": 100000
            }),
        )
        .unwrap();
    let permit_id = permit["permitId"].as_str().unwrap().to_string();
    let out = dispatch(
        &core,
        &ws,
        json!({ "permitId": permit_id }),
        "hello",
    )
    .unwrap();
    assert_eq!(out["response"]["text"], "core reply");
    assert_eq!(fake.hits.load(Ordering::SeqCst), 1);
    // API-key setup still works: reference, prompt store, add,
    // dispatch to a keyed OpenAI-shaped provider.
    let reference = format!("openai-e2e-{}", std::process::id());
    let set = core
        .ai_command(
            "providers-credential-set",
            json!({ "workspaceId": ws, "provider": "openai", "reference": reference }),
        )
        .unwrap();
    assert_eq!(set["stored"], true);
    let added = core
        .ai_command(
            "providers-configure-add",
            json!({
                "workspaceId": ws,
                "provider": {
                    "config": {
                        "provider": "openai", "baseUrl": fake.base_url,
                        "credential": { "provider": "openai", "reference": reference },
                        "enabled": true, "customEndpoint": true
                    },
                    "locality": "local",
                    "models": [{
                        "id": "gpt-x", "provider": "openai",
                        "capabilities": { "chat": true, "streaming": true, "vision": false, "coding": true, "deepReasoning": false, "embeddings": false },
                        "contextWindow": 8192,
                        "cost": "unknown",
                        "speed": "balanced"
                    }]
                }
            }),
        )
        .unwrap();
    assert_eq!(added["added"], "openai");
    let permit = core
        .ai_command(
            "providers-authorize",
            json!({
                "workspaceId": ws,
                "provider": "openai", "model": "gpt-x",
                "maxEstimatedTotalTokens": 100000,
                "allowUnknownCost": true
            }),
        )
        .unwrap();
    let permit_id = permit["permitId"].as_str().unwrap().to_string();
    let out = core
        .ai_command(
            "providers-dispatch",
            stamped(
                "host-session",
                json!({
                    "workspaceId": ws,
                    "request": {
                        "model": "gpt-x",
                        "messages": [{
                            "role": "user",
                            "segments": [{ "text": "hello", "privacy": "standard" }]
                        }]
                    },
                    // Auto mode scores all models and picks the free
                    // Ollama one; the keyed provider is reached by
                    // explicit selection, as the router requires.
                    "mode": "chooseModel",
                    "chosen": ["openai", "gpt-x"],
                    "permitId": permit_id
                }),
            ),
        )
        .unwrap();
    assert_eq!(out["response"]["text"], "authed reply");
}
