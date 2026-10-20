//! Credential references and secret resolution.
//!
//! Configuration stores only `CredentialRef` values — opaque identifiers
//! that the integrator binds to the OS credential vault (Windows
//! Credential Manager / macOS Keychain, MASTER_SPEC §70) through the
//! existing keyring boundary in `anansi-core`
//! (`crates/core/src/vault/credential_store.rs`). Secret *values* exist
//! only inside `SecretValue`, are resolved at dispatch time, are never
//! serialized, and are zeroed on drop.

use crate::error::{ProviderError, ProviderErrorCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A reference to a credential held by the native secret boundary.
/// Contains no secret material; safe to persist and to log.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialRef {
    /// Provider this credential belongs to, e.g. `openai`.
    pub provider: String,
    /// Opaque vault account/reference identifier chosen by the user at
    /// setup time (MASTER_SPEC §15: the user explicitly pastes the key
    /// into the OS vault; ANANSI stores only this reference).
    pub reference: String,
}

impl CredentialRef {
    pub fn new(provider: impl Into<String>, reference: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            reference: reference.into(),
        }
    }
}

/// How a provider authenticates, as verified against official provider
/// documentation (see `docs/handoffs/M5-provider-verification.md`).
/// User OAuth for inference is listed only where a provider officially
/// offers it; API keys are never relabelled as OAuth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthMethod {
    /// User-owned API key (the §15 fallback path where OAuth is absent).
    ApiKey,
    /// Official user OAuth (Authorization Code + PKCE) for inference.
    OAuthUser,
    /// Cloud IAM signing (e.g. SigV4, Google ADC) — a separate mechanism,
    /// never emulated with an API key.
    CloudIam,
    /// No authentication (loopback local runtimes).
    None,
    /// Documented as not offered for inference.
    Unsupported,
}

/// Secret material that must never reach logs, errors, or Debug output.
pub struct SecretValue {
    bytes: Vec<u8>,
}

impl SecretValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            bytes: value.into().into_bytes(),
        }
    }
    /// The only way to read the secret: at the point a header is built.
    pub fn expose(&self) -> &str {
        std::str::from_utf8(&self.bytes).unwrap_or("")
    }
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretValue([REDACTED], len={})", self.bytes.len())
    }
}

impl Drop for SecretValue {
    fn drop(&mut self) {
        self.bytes.fill(0);
    }
}

/// Material the dispatcher hands to an adapter's header builder.
pub enum AuthMaterial {
    ApiKey(SecretValue),
    BearerToken(SecretValue),
    None,
}

impl fmt::Debug for AuthMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthMaterial::ApiKey(_) => write!(f, "AuthMaterial::ApiKey([REDACTED])"),
            AuthMaterial::BearerToken(_) => write!(f, "AuthMaterial::BearerToken([REDACTED])"),
            AuthMaterial::None => write!(f, "AuthMaterial::None"),
        }
    }
}

/// Resolves a `CredentialRef` to secret material. The production
/// implementation belongs to the integrator (OS keyring via
/// `anansi-core`); this crate never persists secrets itself.
pub trait SecretResolver {
    fn resolve(&self, credential: &CredentialRef) -> Result<AuthMaterial, ProviderError>;
}

/// Fail-closed default: without a wired resolver, any credentialed
/// dispatch is refused before network access.
pub struct UnavailableResolver;

impl SecretResolver for UnavailableResolver {
    fn resolve(&self, credential: &CredentialRef) -> Result<AuthMaterial, ProviderError> {
        Err(ProviderError::new(
            ProviderErrorCode::PermissionRefused,
            format!(
                "no secret resolver is configured; cannot resolve credential reference '{}' for provider '{}'",
                credential.reference, credential.provider
            ),
        ))
    }
}

/// In-memory resolver for tests and integrator wiring exercises only.
/// Values are fake credentials in tests; never use for real secrets in
/// persisted configuration.
#[derive(Default)]
pub struct InMemoryResolver {
    keys: HashMap<String, String>,
}

impl InMemoryResolver {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert(&mut self, reference: impl Into<String>, fake_value: impl Into<String>) {
        self.keys.insert(reference.into(), fake_value.into());
    }
}

impl SecretResolver for InMemoryResolver {
    fn resolve(&self, credential: &CredentialRef) -> Result<AuthMaterial, ProviderError> {
        match self.keys.get(&credential.reference) {
            Some(v) => Ok(AuthMaterial::ApiKey(SecretValue::new(v.clone()))),
            None => Err(ProviderError::new(
                ProviderErrorCode::AuthFailed,
                format!(
                    "credential reference '{}' not found for provider '{}'",
                    credential.reference, credential.provider
                ),
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// OAuth (MASTER_SPEC §15): PKCE S256 contracts. Live token exchange needs
// the TLS transport and the system-browser hand-off (decision M5-D8);
// the cryptographic pieces and state validation are implemented and
// tested here because they are transport-independent.
// ---------------------------------------------------------------------------

pub struct PkcePair {
    pub verifier: String,
    pub challenge: String,
    pub method: &'static str, // always "S256"
}

/// Build a PKCE pair from a caller-supplied high-entropy verifier.
/// The verifier is validated against RFC 7636 §4.1 character/length
/// rules; the challenge is `BASE64URL(SHA256(verifier))`.
pub fn pkce_from_verifier(verifier: &str) -> Result<PkcePair, ProviderError> {
    let valid = (43..=128).contains(&verifier.len())
        && verifier
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~'));
    if !valid {
        return Err(ProviderError::new(
            ProviderErrorCode::PermissionRefused,
            "PKCE verifier must be 43–128 chars of [A-Za-z0-9-._~] (RFC 7636)",
        ));
    }
    let digest = sha256(verifier.as_bytes());
    Ok(PkcePair {
        verifier: verifier.to_string(),
        challenge: base64_url_no_pad(&digest),
        method: "S256",
    })
}

/// Validate the `state` returned to the redirect against the value sent
/// (§15: validate state). Constant-shape comparison; any mismatch or
/// absence is a refusal.
pub fn validate_oauth_state(expected: &str, returned: Option<&str>) -> Result<(), ProviderError> {
    match returned {
        Some(r) if !expected.is_empty() && r == expected => Ok(()),
        _ => Err(ProviderError::new(
            ProviderErrorCode::PermissionRefused,
            "OAuth state validation failed; authorization response discarded",
        )),
    }
}

/// Build an Authorization Code + PKCE authorization URL for the system
/// browser (§15). Query parameters are percent-encoded; the caller opens
/// the URL in the system browser and must never log it with codes.
pub fn build_authorization_url(
    authorization_endpoint: &str,
    client_id: &str,
    redirect_uri: &str,
    scopes: &[&str],
    state: &str,
    pkce: &PkcePair,
) -> String {
    let q = [
        ("response_type", "code".to_string()),
        ("client_id", client_id.to_string()),
        ("redirect_uri", redirect_uri.to_string()),
        ("scope", scopes.join(" ")),
        ("state", state.to_string()),
        ("code_challenge", pkce.challenge.clone()),
        ("code_challenge_method", pkce.method.to_string()),
    ];
    let query = q
        .iter()
        .map(|(k, v)| format!("{}={}", k, percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{authorization_endpoint}?{query}")
}

fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn base64_url_no_pad(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(TABLE[n as usize & 63] as char);
        }
    }
    out
}

/// Minimal SHA-256 (FIPS 180-4), included so PKCE needs no extra
/// dependency: the crate's dependency posture mirrors `anansi-core`
/// (decision M5-D2). Verified against published test vectors below.
pub fn sha256(input: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut msg = input.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for block in msg.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_fips_vectors() {
        let hex = |d: [u8; 32]| d.iter().map(|b| format!("{b:02x}")).collect::<String>();
        assert_eq!(
            hex(sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn pkce_matches_rfc7636_appendix_b() {
        let pair = pkce_from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk").unwrap();
        assert_eq!(
            pair.challenge,
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        assert_eq!(pair.method, "S256");
    }

    #[test]
    fn pkce_rejects_short_verifier() {
        assert!(pkce_from_verifier("short").is_err());
    }

    #[test]
    fn oauth_state_validation() {
        assert!(validate_oauth_state("abc", Some("abc")).is_ok());
        assert!(validate_oauth_state("abc", Some("xyz")).is_err());
        assert!(validate_oauth_state("abc", None).is_err());
    }

    #[test]
    fn secret_debug_is_redacted() {
        let s = SecretValue::new("sk-fake-1234567890");
        let dbg = format!("{s:?}");
        assert!(!dbg.contains("sk-fake"), "{dbg}");
        assert_eq!(s.expose(), "sk-fake-1234567890");
    }

    #[test]
    fn authorization_url_contains_pkce_and_state() {
        let pair = pkce_from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk").unwrap();
        let url = build_authorization_url(
            "https://provider.example/oauth/authorize",
            "client-123",
            "http://127.0.0.1:1455/callback",
            &["openid", "inference"],
            "state-xyz",
            &pair,
        );
        assert!(url.contains("code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=state-xyz"));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A1455%2Fcallback"));
    }
}
