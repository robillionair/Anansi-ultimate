//! "Continue with ChatGPT" sign-in prototype — the open-source
//! Sign in with ChatGPT (SIWC) flow, host-side only.
//!
//! The documented flow (see
//! `docs/handoffs/connect-ai-chatgpt-signin-design.md`): dynamic
//! client registration (`client_id=dynamic_agent_client`), a
//! system-browser handoff, a loopback callback on
//! `http://127.0.0.1:<port>/auth/callback`, PKCE S256, a form-encoded
//! code exchange at the issuer token endpoint, and ID-token
//! validation against the issuer JWKS. ChatGPT plan usage is gated
//! on the granted scope `chatgpt.tokens.use.direct` — a valid ID
//! token alone never authorizes inference.
//!
//! This module is pure sign-in logic: it builds the authorization
//! URL, validates tokens, and exchanges codes through the existing
//! bounded provider transport. It never stores tokens (the caller
//! puts the credential record in the OS keyring), never logs token
//! or callback-code material, and never contacts OpenAI except
//! through the issuer endpoints resolved here. In tests the issuer
//! is a synthetic local server (see the debug-only,
//! loopback-locked `ANANSI_CHATGPT_TEST_ISSUER` hook — the same
//! pattern as the provider credential test hooks).

use anansi_providers::config::{ProviderConfig, TimeoutConfig, validate_endpoint};
use anansi_providers::error::ProviderError;
use anansi_providers::http::{
    CancellationToken, HttpRequest, RustlsTransport, StdHttpTransport, Transport,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::Digest;

/// Production issuer base (authorization, token, discovery).
pub const PRODUCTION_ISSUER_BASE: &str = "https://auth.openai.com";
/// The ID-token issuer claim in production.
pub const PRODUCTION_ISSUER: &str = "https://auth.openai.com";
/// Authorization endpoint path, relative to the issuer base.
pub const AUTHORIZE_PATH: &str = "/api/accounts/authorize";
/// Token endpoint path, relative to the issuer base.
pub const TOKEN_PATH: &str = "/api/accounts/oauth/token";
/// OpenID discovery path, relative to the issuer base.
pub const OPENID_CONFIGURATION_PATH: &str = "/.well-known/openid-configuration";
/// OS keyring service for ChatGPT tokens — distinct from the
/// provider API-key service, so the two stores cannot collide.
pub const CHATGPT_KEYRING_SERVICE: &str = "com.robillionair.anansi.chatgpt";
/// The scope that authorizes ChatGPT plan usage.
pub const PLAN_SCOPE: &str = "chatgpt.tokens.use.direct";
/// Identity scopes (sign-in without plan usage).
pub const IDENTITY_SCOPES: [&str; 3] = ["openid", "profile", "email"];
/// Plan-usage scopes, in addition to the identity scopes.
pub const PLAN_SCOPES: [&str; 3] = [
    "offline_access",
    "resource.invoke",
    "chatgpt.tokens.use.direct",
];
/// The resource the access token is bound to.
pub const RESOURCE: &str = "https://api.openai.com/v1";
/// `agent_name_hint` — the app's actual name, used consistently.
pub const AGENT_NAME_HINT: &str = "ANANSI";
/// First-time registration entrypoint — never the ID to save.
pub const DYNAMIC_CLIENT_ID: &str = "dynamic_agent_client";
/// Loopback callback path — fixed by the docs; only the port varies.
pub const CALLBACK_PATH: &str = "/auth/callback";
/// How long a sign-in attempt stays valid (listener + state).
pub const ATTEMPT_TTL_SECS: u64 = 600;
/// Refresh proactively when the access token has less than this left.
pub const REFRESH_AHEAD_SECS: u64 = 300;

/// Non-secret failure reasons. Messages carry categories and
/// bounded issuer error codes only — never token, code, or
/// credential material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatGptError {
    InvalidRequest(String),
    /// The callback `state` does not match the pending attempt.
    InvalidState,
    /// The attempt was already consumed (replayed callback).
    Replay,
    /// The user denied consent (`error=access_denied`).
    AccessDenied,
    /// A new-registration callback carried no issued client ID.
    MissingClientId,
    /// The callback carried a different client ID than the attempt.
    ClientIdMismatch,
    /// The code or refresh token was rejected (`invalid_grant`).
    InvalidGrant,
    /// ID-token validation failed; the string is a static category.
    InvalidToken(String),
    /// The granted scopes lack ChatGPT plan usage.
    MissingPlanScope,
    /// Transport / issuer failure (bounded message).
    Network(String),
}

impl ChatGptError {
    pub fn message(&self) -> String {
        match self {
            ChatGptError::InvalidRequest(m) => m.clone(),
            ChatGptError::InvalidState => {
                "The sign-in reply does not match this attempt. Start a new sign-in.".to_string()
            }
            ChatGptError::Replay => {
                "This sign-in reply was already used. Start a new sign-in.".to_string()
            }
            ChatGptError::AccessDenied => {
                "ChatGPT sign-in was declined. Nothing was connected.".to_string()
            }
            ChatGptError::MissingClientId => {
                "Registration did not return an issued client ID.".to_string()
            }
            ChatGptError::ClientIdMismatch => {
                "The sign-in reply belongs to a different registration.".to_string()
            }
            ChatGptError::InvalidGrant => {
                "The sign-in code was rejected. Start a new sign-in.".to_string()
            }
            ChatGptError::InvalidToken(reason) => {
                format!("The ChatGPT identity token failed validation ({reason}).")
            }
            ChatGptError::MissingPlanScope => {
                "ChatGPT plan usage was not granted. Sign-in is identity-only.".to_string()
            }
            ChatGptError::Network(m) => m.clone(),
        }
    }
}

impl From<ProviderError> for ChatGptError {
    fn from(e: ProviderError) -> Self {
        ChatGptError::Network(bounded(&e.to_string()))
    }
}

fn bounded(text: &str) -> String {
    text.chars().take(200).collect()
}

fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Endpoints

/// Issuer endpoints for one sign-in flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatGptEndpoints {
    /// Base URL of the issuer, no trailing slash, no path.
    pub issuer_base: String,
}

impl ChatGptEndpoints {
    pub fn production() -> Self {
        Self {
            issuer_base: PRODUCTION_ISSUER_BASE.to_string(),
        }
    }

    /// The expected ID-token `iss` for this issuer. Production:
    /// `https://auth.openai.com`. A synthetic test issuer uses its
    /// own base as the issuer.
    pub fn expected_issuer(&self) -> &str {
        &self.issuer_base
    }

    pub fn authorize_url(&self) -> String {
        format!("{}{}", self.issuer_base, AUTHORIZE_PATH)
    }

    pub fn token_url(&self) -> String {
        format!("{}{}", self.issuer_base, TOKEN_PATH)
    }

    pub fn openid_configuration_url(&self) -> String {
        format!("{}{}", self.issuer_base, OPENID_CONFIGURATION_PATH)
    }

    fn validated_base(&self) -> Result<anansi_providers::config::ValidatedEndpoint, ChatGptError> {
        let config = ProviderConfig::new("chatgpt", self.issuer_base.clone());
        validate_endpoint(&config).map_err(ChatGptError::from)
    }
}

/// Resolve the issuer: production, unless the debug-only test hook
/// names a loopback synthetic issuer. The hook is compiled out of
/// release builds entirely, and any non-loopback value is refused.
pub fn resolve_endpoints() -> Result<ChatGptEndpoints, ChatGptError> {
    #[cfg(debug_assertions)]
    if let Some(value) = std::env::var_os("ANANSI_CHATGPT_TEST_ISSUER") {
        let base = value.to_string_lossy().into_owned();
        if is_loopback_http_base(&base) {
            return Ok(ChatGptEndpoints { issuer_base: base });
        }
        return Err(ChatGptError::InvalidRequest(
            "ANANSI_CHATGPT_TEST_ISSUER must be an http://127.0.0.1 URL.".to_string(),
        ));
    }
    Ok(ChatGptEndpoints::production())
}

fn is_loopback_http_base(base: &str) -> bool {
    let Ok(url) = url::Url::parse(base) else {
        return false;
    };
    url.scheme() == "http"
        && url.host_str() == Some("127.0.0.1")
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
}

// ---------------------------------------------------------------------------
// Random material + PKCE

fn random_bytes(n: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; n];
    getrandom::fill(&mut bytes).expect("OS randomness for sign-in material");
    bytes
}

pub fn base64url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn base64url_decode(text: &str) -> Result<Vec<u8>, ChatGptError> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|_| ChatGptError::InvalidToken("undecodable".to_string()))
}

/// A fresh PKCE verifier: 32 random bytes, base64url, 43 chars.
pub fn new_pkce_verifier() -> String {
    base64url(&random_bytes(32))
}

/// S256 challenge: base64url SHA-256 of the verifier, no padding.
pub fn pkce_challenge(verifier: &str) -> String {
    let digest = sha2::Sha256::digest(verifier.as_bytes());
    base64url(&digest)
}

/// A fresh random `state` / `nonce` value (24 random bytes).
pub fn new_random_value() -> String {
    base64url(&random_bytes(24))
}

/// A fresh `ext_agent_host_id`: `urn:uuid:<32 lowercase hex>`.
pub fn new_host_id() -> String {
    let mut hex = String::with_capacity(32);
    for b in random_bytes(16) {
        hex.push_str(&format!("{b:02x}"));
    }
    format!("urn:uuid:{hex}")
}

// ---------------------------------------------------------------------------
// Authorization URL

pub struct AuthorizationRequest {
    /// `dynamic_agent_client` first time; the issued ID thereafter.
    pub client_id: String,
    /// Included only on initial dynamic registration.
    pub agent_name_hint: Option<String>,
    pub ext_agent_host_id: String,
    /// Retained ID token from a previous sign-in (may be expired).
    pub id_token_hint: Option<String>,
    pub login_hint: Option<String>,
    pub redirect_uri: String,
    /// Full scope string (identity + plan scopes).
    pub scope: String,
    pub state: String,
    pub nonce: String,
    pub code_challenge: String,
}

/// The full scope string for a plan-usage sign-in.
pub fn full_scope() -> String {
    let mut scopes: Vec<&str> = IDENTITY_SCOPES.to_vec();
    scopes.extend(PLAN_SCOPES);
    scopes.join(" ")
}

pub fn build_authorization_url(endpoints: &ChatGptEndpoints, req: &AuthorizationRequest) -> String {
    let mut q = url::form_urlencoded::Serializer::new(String::new());
    q.append_pair("client_id", &req.client_id);
    if let Some(hint) = &req.agent_name_hint {
        q.append_pair("agent_name_hint", hint);
    }
    q.append_pair("ext_agent_host_id", &req.ext_agent_host_id);
    if let Some(hint) = &req.id_token_hint {
        q.append_pair("id_token_hint", hint);
    }
    if let Some(hint) = &req.login_hint {
        q.append_pair("login_hint", hint);
    }
    q.append_pair("response_type", "code");
    q.append_pair("redirect_uri", &req.redirect_uri);
    q.append_pair("scope", &req.scope);
    q.append_pair("resource", RESOURCE);
    q.append_pair("state", &req.state);
    q.append_pair("nonce", &req.nonce);
    q.append_pair("code_challenge_method", "S256");
    q.append_pair("code_challenge", &req.code_challenge);
    format!("{}?{}", endpoints.authorize_url(), q.finish())
}

/// The loopback callback URI for a bound port.
pub fn redirect_uri_for_port(port: u16) -> String {
    format!("http://127.0.0.1:{port}{CALLBACK_PATH}")
}

// ---------------------------------------------------------------------------
// Token responses

/// A parsed token-endpoint response. Field names follow the OAuth
/// response; `scope` is the returned space-separated list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub scopes: Vec<String>,
}

impl TokenResponse {
    pub fn plan_usage(&self) -> bool {
        self.scopes.iter().any(|s| s == PLAN_SCOPE)
    }
}

fn parse_token_response(body: &[u8]) -> Result<TokenResponse, ChatGptError> {
    let value: serde_json::Value = serde_json::from_slice(body).map_err(|_| {
        ChatGptError::Network("The token endpoint returned an unreadable response.".to_string())
    })?;
    let access_token = value
        .get("access_token")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ChatGptError::Network("The token endpoint returned no access token.".to_string())
        })?
        .to_string();
    let token_type = value
        .get("token_type")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if !token_type.eq_ignore_ascii_case("bearer") {
        return Err(ChatGptError::Network(
            "The token endpoint returned an unsupported token type.".to_string(),
        ));
    }
    let expires_in = value
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .filter(|e| (60..=86_400).contains(e))
        .ok_or_else(|| {
            ChatGptError::Network("The token endpoint returned no usable expiry.".to_string())
        })?;
    let refresh_token = value
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let id_token = value
        .get("id_token")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let scopes = value
        .get("scope")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .split_whitespace()
        .map(str::to_string)
        .collect();
    Ok(TokenResponse {
        access_token,
        token_type,
        expires_in,
        refresh_token,
        id_token,
        scopes,
    })
}

/// Parse an OAuth error body (`{error, error_description}`) into
/// the mapped failure. `invalid_grant` is the only code with a
/// dedicated meaning here; anything else is a bounded network
/// error carrying the code, never the description verbatim (it
/// could echo request material).
fn token_error(body: &[u8]) -> ChatGptError {
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(body)
        && let Some(code) = value.get("error").and_then(|v| v.as_str())
    {
        if code == "invalid_grant" {
            return ChatGptError::InvalidGrant;
        }
        return ChatGptError::Network(format!(
            "The token endpoint refused the request ({})",
            bounded(code)
        ));
    }
    ChatGptError::Network("The token endpoint refused the request.".to_string())
}

// ---------------------------------------------------------------------------
// Bounded HTTP calls to the issuer

fn timeouts() -> TimeoutConfig {
    TimeoutConfig {
        connect_ms: 5_000,
        first_byte_ms: 10_000,
        idle_ms: 10_000,
        total_ms: 30_000,
    }
}

fn transport_for(endpoints: &ChatGptEndpoints) -> Box<dyn Transport> {
    if endpoints.issuer_base.starts_with("http://") {
        Box::new(StdHttpTransport)
    } else {
        Box::new(RustlsTransport::new())
    }
}

fn post_form(
    endpoints: &ChatGptEndpoints,
    path: &str,
    params: &[(&str, &str)],
) -> Result<(u16, Vec<u8>), ChatGptError> {
    let base = endpoints.validated_base()?;
    let mut q = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in params {
        q.append_pair(k, v);
    }
    let body = q.finish().into_bytes();
    let request = HttpRequest {
        method: "POST".to_string(),
        endpoint: base.clone(),
        path: format!("{}{}", base.base_path, path),
        headers: vec![(
            "Content-Type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        )],
        body,
    };
    let transport = transport_for(endpoints);
    let mut response = transport
        .execute(&request, &timeouts(), &CancellationToken::new())
        .map_err(ChatGptError::from)?;
    let status = response.status;
    let bytes = response
        .read_body_capped(64 * 1024)
        .map_err(ChatGptError::from)?;
    Ok((status, bytes))
}

fn get_json(endpoints: &ChatGptEndpoints, url: &str) -> Result<serde_json::Value, ChatGptError> {
    let base = endpoints.validated_base()?;
    // The URL is always an issuer URL (discovery / JWKS); split it
    // into the validated base + path so the transport only ever
    // talks to the validated endpoint.
    let parsed = url::Url::parse(url)
        .map_err(|_| ChatGptError::Network("The issuer returned an unusable URL.".to_string()))?;
    if parsed.origin().ascii_serialization() != base.origin() {
        return Err(ChatGptError::Network(
            "The issuer pointed at a different origin.".to_string(),
        ));
    }
    let path = if parsed.path().is_empty() {
        "/".to_string()
    } else {
        parsed.path().to_string()
    };
    let request = HttpRequest {
        method: "GET".to_string(),
        endpoint: base.clone(),
        path: format!("{}{}", base.base_path, path),
        headers: vec![],
        body: vec![],
    };
    let transport = transport_for(endpoints);
    let mut response = transport
        .execute(&request, &timeouts(), &CancellationToken::new())
        .map_err(ChatGptError::from)?;
    if response.status != 200 {
        return Err(ChatGptError::Network(
            "The issuer discovery request failed.".to_string(),
        ));
    }
    let bytes = response
        .read_body_capped(64 * 1024)
        .map_err(ChatGptError::from)?;
    serde_json::from_slice(&bytes).map_err(|_| {
        ChatGptError::Network("The issuer returned an unreadable response.".to_string())
    })
}

/// Exchange an authorization code for tokens.
pub fn exchange_authorization_code(
    endpoints: &ChatGptEndpoints,
    client_id: &str,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, ChatGptError> {
    let (status, body) = post_form(
        endpoints,
        TOKEN_PATH,
        &[
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("code", code),
            ("code_verifier", code_verifier),
            ("redirect_uri", redirect_uri),
            ("resource", RESOURCE),
        ],
    )?;
    if status == 200 {
        parse_token_response(&body)
    } else {
        Err(token_error(&body))
    }
}

/// Refresh an access token with a rotating refresh token.
pub fn refresh_access_token(
    endpoints: &ChatGptEndpoints,
    client_id: &str,
    refresh_token: &str,
) -> Result<TokenResponse, ChatGptError> {
    let (status, body) = post_form(
        endpoints,
        TOKEN_PATH,
        &[
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("refresh_token", refresh_token),
            ("resource", RESOURCE),
        ],
    )?;
    if status == 200 {
        parse_token_response(&body)
    } else {
        Err(token_error(&body))
    }
}

// ---------------------------------------------------------------------------
// OpenID discovery + JWKS + ID-token validation

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenIdConfiguration {
    pub issuer: String,
    pub jwks_uri: String,
    pub revocation_endpoint: Option<String>,
}

pub fn fetch_openid_configuration(
    endpoints: &ChatGptEndpoints,
) -> Result<OpenIdConfiguration, ChatGptError> {
    let value = get_json(endpoints, &endpoints.openid_configuration_url())?;
    let issuer = value
        .get("issuer")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ChatGptError::Network("The issuer discovery document has no issuer.".to_string())
        })?
        .to_string();
    let jwks_uri = value
        .get("jwks_uri")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ChatGptError::Network("The issuer discovery document has no JWKS address.".to_string())
        })?
        .to_string();
    let revocation_endpoint = value
        .get("revocation_endpoint")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Ok(OpenIdConfiguration {
        issuer,
        jwks_uri,
        revocation_endpoint,
    })
}

#[derive(Debug, Clone)]
struct Jwk {
    kid: String,
    n: Vec<u8>,
    e: Vec<u8>,
}

/// A parsed JWKS (RSA keys only — the issuer publishes RS256).
#[derive(Debug, Clone, Default)]
pub struct Jwks {
    keys: Vec<Jwk>,
}

pub fn fetch_jwks(endpoints: &ChatGptEndpoints) -> Result<Jwks, ChatGptError> {
    let discovery = fetch_openid_configuration(endpoints)?;
    let value = get_json(endpoints, &discovery.jwks_uri)?;
    parse_jwks(&value)
}

pub fn parse_jwks(value: &serde_json::Value) -> Result<Jwks, ChatGptError> {
    let keys = value
        .get("keys")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ChatGptError::Network("The JWKS has no keys.".to_string()))?;
    let mut out = Vec::new();
    for key in keys {
        let kty = key.get("kty").and_then(|v| v.as_str()).unwrap_or("");
        let alg = key.get("alg").and_then(|v| v.as_str()).unwrap_or("");
        if kty != "RSA" || (!alg.is_empty() && alg != "RS256") {
            continue;
        }
        let kid = key
            .get("kid")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let n = key
            .get("n")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChatGptError::Network("The JWKS key has no modulus.".to_string()))?;
        let e = key
            .get("e")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChatGptError::Network("The JWKS key has no exponent.".to_string()))?;
        out.push(Jwk {
            kid,
            n: base64url_decode(n)
                .map_err(|_| ChatGptError::Network("The JWKS key is unreadable.".to_string()))?,
            e: base64url_decode(e)
                .map_err(|_| ChatGptError::Network("The JWKS key is unreadable.".to_string()))?,
        });
    }
    if out.is_empty() {
        return Err(ChatGptError::Network(
            "The JWKS has no usable RSA key.".to_string(),
        ));
    }
    Ok(Jwks { keys: out })
}

/// The validated identity carried by an ID token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedIdentity {
    pub subject: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

/// Validate an ID token: RS256 signature against the JWKS, then
/// issuer, audience, expiry, nonce (when expected), and subject
/// (when expected — a returning account must match). The `aud`
/// claim may be a string or an array containing the client ID.
pub fn validate_id_token(
    token: &str,
    expected_issuer: &str,
    expected_audience: &str,
    expected_nonce: Option<&str>,
    expected_subject: Option<&str>,
    jwks: &Jwks,
) -> Result<ValidatedIdentity, ChatGptError> {
    let mut parts = token.split('.');
    let (Some(header_b64), Some(payload_b64), Some(signature_b64)) =
        (parts.next(), parts.next(), parts.next())
    else {
        return Err(ChatGptError::InvalidToken("malformed".to_string()));
    };
    if parts.next().is_some() {
        return Err(ChatGptError::InvalidToken("malformed".to_string()));
    }
    let header_bytes = base64url_decode(header_b64)
        .map_err(|_| ChatGptError::InvalidToken("header".to_string()))?;
    let header: serde_json::Value = serde_json::from_slice(&header_bytes)
        .map_err(|_| ChatGptError::InvalidToken("header".to_string()))?;
    if header.get("alg").and_then(|v| v.as_str()) != Some("RS256") {
        return Err(ChatGptError::InvalidToken("algorithm".to_string()));
    }
    let kid = header
        .get("kid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ChatGptError::InvalidToken("key-id".to_string()))?;
    let key = jwks
        .keys
        .iter()
        .find(|k| k.kid == kid)
        .ok_or_else(|| ChatGptError::InvalidToken("unknown-key".to_string()))?;
    let signature = base64url_decode(signature_b64)
        .map_err(|_| ChatGptError::InvalidToken("signature".to_string()))?;
    let signing_input = format!("{header_b64}.{payload_b64}");
    let public_key = ring::signature::RsaPublicKeyComponents {
        n: &key.n,
        e: &key.e,
    };
    public_key
        .verify(
            &ring::signature::RSA_PKCS1_2048_8192_SHA256,
            signing_input.as_bytes(),
            &signature,
        )
        .map_err(|_| ChatGptError::InvalidToken("signature".to_string()))?;
    let payload_bytes = base64url_decode(payload_b64)
        .map_err(|_| ChatGptError::InvalidToken("payload".to_string()))?;
    let claims: serde_json::Value = serde_json::from_slice(&payload_bytes)
        .map_err(|_| ChatGptError::InvalidToken("payload".to_string()))?;
    let issuer = claims.get("iss").and_then(|v| v.as_str()).unwrap_or("");
    if issuer != expected_issuer {
        return Err(ChatGptError::InvalidToken("issuer".to_string()));
    }
    let audience_ok = match claims.get("aud") {
        Some(serde_json::Value::String(aud)) => aud == expected_audience,
        Some(serde_json::Value::Array(auds)) => {
            auds.iter().any(|a| a.as_str() == Some(expected_audience))
        }
        _ => false,
    };
    if !audience_ok {
        return Err(ChatGptError::InvalidToken("audience".to_string()));
    }
    let now = now_unix_secs();
    let exp = claims.get("exp").and_then(|v| v.as_u64()).unwrap_or(0);
    if exp == 0 || exp + 30 < now {
        return Err(ChatGptError::InvalidToken("expired".to_string()));
    }
    if let Some(nonce) = expected_nonce {
        let actual = claims.get("nonce").and_then(|v| v.as_str()).unwrap_or("");
        if actual != nonce {
            return Err(ChatGptError::InvalidToken("nonce".to_string()));
        }
    }
    let subject = claims
        .get("sub")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ChatGptError::InvalidToken("subject".to_string()))?
        .to_string();
    if let Some(expected) = expected_subject
        && expected != subject
    {
        return Err(ChatGptError::InvalidToken("subject".to_string()));
    }
    Ok(ValidatedIdentity {
        subject,
        email: claims
            .get("email")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        name: claims
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

/// Revoke a refresh token at the issuer's revocation endpoint.
/// Returns `true` when revocation was confirmed (empty HTTP 200),
/// `false` when it could not be confirmed (the caller still clears
/// the local tokens and reports it). No revocation endpoint in
/// discovery means "not confirmable" rather than an error.
pub fn revoke_refresh_token(
    endpoints: &ChatGptEndpoints,
    client_id: &str,
    refresh_token: &str,
) -> Result<bool, ChatGptError> {
    let discovery = fetch_openid_configuration(endpoints)?;
    let Some(endpoint) = discovery.revocation_endpoint else {
        return Ok(false);
    };
    let (status, _) = post_form(
        endpoints,
        &endpoint_path(&endpoint)?,
        &[
            ("token", refresh_token),
            ("token_type_hint", "refresh_token"),
            ("client_id", client_id),
        ],
    )?;
    Ok(status == 200)
}

fn endpoint_path(url: &str) -> Result<String, ChatGptError> {
    let parsed = url::Url::parse(url)
        .map_err(|_| ChatGptError::Network("The issuer returned an unusable URL.".to_string()))?;
    Ok(parsed.path().to_string())
}

// ---------------------------------------------------------------------------
// Credential record + registration stub (labels only)

/// One registration's credential record — the only place tokens
/// live, and only inside the OS keyring. Serialized as JSON into
/// the keyring entry; never written to a file, log, or response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatGptCredentialRecord {
    pub email: Option<String>,
    pub subject: String,
    pub issuer: String,
    pub client_id: String,
    pub ext_agent_host_id: String,
    pub id_token: Option<String>,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_in: u64,
    pub scopes: Vec<String>,
    pub saved_at_ms: u64,
}

impl ChatGptCredentialRecord {
    /// Keyring reference (a label, not a secret).
    pub fn reference(&self) -> String {
        format!("chatgpt-{}", self.client_id)
    }

    pub fn expires_at_ms(&self) -> u64 {
        self.saved_at_ms + self.expires_in.saturating_mul(1000)
    }

    pub fn access_expired(&self, now_ms: u64) -> bool {
        now_ms >= self.expires_at_ms()
    }

    pub fn needs_refresh(&self, now_ms: u64) -> bool {
        let ahead_ms = REFRESH_AHEAD_SECS * 1000;
        now_ms + ahead_ms >= self.expires_at_ms()
    }

    pub fn plan_usage(&self) -> bool {
        self.scopes.iter().any(|s| s == PLAN_SCOPE)
    }
}

/// One saved registration — labels only, no tokens. This is what a
/// later launch offers as "choose a saved ChatGPT account".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatGptRegistration {
    pub email: Option<String>,
    pub subject: String,
    pub plan_usage: bool,
    pub last_verified_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_is_s256_base64url() {
        let verifier = new_pkce_verifier();
        assert_eq!(verifier.len(), 43);
        assert!(
            verifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
        let challenge = pkce_challenge(&verifier);
        let digest = sha2::Sha256::digest(verifier.as_bytes());
        assert_eq!(challenge, base64url(&digest));
        assert!(!challenge.contains('='));
    }

    #[test]
    fn authorization_url_carries_the_documented_parameters() {
        let endpoints = ChatGptEndpoints::production();
        let url = build_authorization_url(
            &endpoints,
            &AuthorizationRequest {
                client_id: DYNAMIC_CLIENT_ID.to_string(),
                agent_name_hint: Some(AGENT_NAME_HINT.to_string()),
                ext_agent_host_id: "urn:uuid:0123456789abcdef0123456789abcdef".to_string(),
                id_token_hint: None,
                login_hint: None,
                redirect_uri: "http://127.0.0.1:1455/auth/callback".to_string(),
                scope: full_scope(),
                state: "state-value".to_string(),
                nonce: "nonce-value".to_string(),
                code_challenge: "challenge-value".to_string(),
            },
        );
        assert!(url.starts_with("https://auth.openai.com/api/accounts/authorize?"));
        assert!(url.contains("client_id=dynamic_agent_client"));
        assert!(url.contains("agent_name_hint=ANANSI"));
        assert!(url.contains("ext_agent_host_id=urn%3Auuid%3A0123456789abcdef0123456789abcdef"));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A1455%2Fauth%2Fcallback"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains(
            "scope=openid+profile+email+offline_access+resource.invoke+chatgpt.tokens.use.direct"
        ));
        assert!(url.contains("resource=https%3A%2F%2Fapi.openai.com%2Fv1"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(!url.contains("localhost"));
    }

    #[test]
    fn test_issuer_override_is_loopback_locked() {
        unsafe {
            std::env::set_var("ANANSI_CHATGPT_TEST_ISSUER", "https://auth.openai.com");
        }
        let err = resolve_endpoints().unwrap_err();
        assert!(matches!(err, ChatGptError::InvalidRequest(_)));
        unsafe {
            std::env::set_var("ANANSI_CHATGPT_TEST_ISSUER", "http://127.0.0.1:9");
        }
        let endpoints = resolve_endpoints().unwrap();
        assert_eq!(endpoints.issuer_base, "http://127.0.0.1:9");
        unsafe {
            std::env::remove_var("ANANSI_CHATGPT_TEST_ISSUER");
        }
    }

    #[test]
    fn record_expiry_and_plan_usage() {
        let record = ChatGptCredentialRecord {
            email: Some("user@example.com".to_string()),
            subject: "sub-1".to_string(),
            issuer: PRODUCTION_ISSUER.to_string(),
            client_id: "oaiapp_test".to_string(),
            ext_agent_host_id: "urn:uuid:00".to_string(),
            id_token: None,
            access_token: "at".to_string(),
            refresh_token: Some("rt".to_string()),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            scopes: vec!["openid".to_string(), PLAN_SCOPE.to_string()],
            saved_at_ms: 1_000_000,
        };
        assert_eq!(record.expires_at_ms(), 1_000_000 + 3_600_000);
        assert!(record.plan_usage());
        assert!(!record.needs_refresh(1_000_000));
        assert!(record.needs_refresh(1_000_000 + 3_300_000));
        assert_eq!(record.reference(), "chatgpt-oaiapp_test");
    }
}
