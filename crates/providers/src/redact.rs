//! Secret scrubbing for errors and logs. The redactor knows the exact
//! credential values in play for one dispatch (registered by the
//! dispatcher from the resolver) plus generic credential-shaped
//! patterns, and removes them from any provider-supplied text before
//! that text can be stored in an error, a disclosure record, or a log.

pub const REDACTED: &str = "[REDACTED]";

#[derive(Debug, Default, Clone)]
pub struct Redactor {
    secrets: Vec<String>,
}

impl Redactor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one exact secret value. Empty or very short values are
    /// ignored to avoid scrubbing ordinary prose.
    pub fn register(&mut self, secret: &str) {
        if secret.len() >= 4 && !self.secrets.iter().any(|s| s == secret) {
            self.secrets.push(secret.to_string());
        }
    }

    pub fn scrub(&self, input: &str) -> String {
        let mut out = input.to_string();
        for secret in &self.secrets {
            if out.contains(secret.as_str()) {
                out = out.replace(secret.as_str(), REDACTED);
            }
        }
        out = scrub_bearer_tokens(&out);
        out = scrub_assignment(&out, "api-key");
        out = scrub_assignment(&out, "x-api-key");
        out = scrub_assignment(&out, "authorization");
        out
    }

    /// Cap an excerpt of provider-supplied text for inclusion in an
    /// error message: scrub first, then truncate on a char boundary.
    pub fn excerpt(&self, input: &str, max_chars: usize) -> String {
        let scrubbed = self.scrub(input);
        if scrubbed.chars().count() <= max_chars {
            scrubbed
        } else {
            let mut s: String = scrubbed.chars().take(max_chars).collect();
            s.push('…');
            s
        }
    }
}

/// Replace `Bearer <token>` values (any case) with `Bearer [REDACTED]`.
fn scrub_bearer_tokens(input: &str) -> String {
    let lower = input.to_lowercase();
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    let mut rest_lower = lower.as_str();
    while let Some(pos) = rest_lower.find("bearer ") {
        let after = pos + "bearer ".len();
        let token_end = rest[after..]
            .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == '}')
            .map(|i| after + i)
            .unwrap_or(rest.len());
        out.push_str(&rest[..after]);
        out.push_str(REDACTED);
        rest = &rest[token_end..];
        rest_lower = &rest_lower[token_end..];
    }
    out.push_str(rest);
    out
}

/// Replace the value after `name:` or `name=` (e.g. echoed headers).
fn scrub_assignment(input: &str, name: &str) -> String {
    let lower = input.to_lowercase();
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    let mut rest_lower = lower.as_str();
    while let Some(pos) = rest_lower.find(name) {
        let mut idx = pos + name.len();
        // Skip separators: spaces, ':' or '='.
        let bytes = rest.as_bytes();
        while idx < rest.len() && (bytes[idx] == b' ' || bytes[idx] == b':' || bytes[idx] == b'=') {
            idx += 1;
        }
        if idx >= rest.len() {
            break;
        }
        let val_end = rest[idx..]
            .find(|c: char| {
                c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == '}' || c == '&'
            })
            .map(|i| idx + i)
            .unwrap_or(rest.len());
        if val_end == idx {
            break;
        }
        out.push_str(&rest[..idx]);
        out.push_str(REDACTED);
        rest = &rest[val_end..];
        rest_lower = &rest_lower[val_end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrubs_exact_secret_and_bearer() {
        let mut r = Redactor::new();
        r.register("sk-fake-1234567890");
        let s = r.scrub(
            "upstream said: bad key sk-fake-1234567890 in Authorization: Bearer sk-fake-1234567890",
        );
        assert!(!s.contains("sk-fake-1234567890"), "{s}");
        assert!(s.contains(REDACTED));
    }

    #[test]
    fn scrubs_unregistered_bearer_and_api_key_headers() {
        let r = Redactor::new();
        let s = r.scrub("echo: x-api-key: abcdefgh12345678 end");
        assert!(!s.contains("abcdefgh12345678"), "{s}");
        let s2 = r.scrub("token was Bearer zzz999888777 ok");
        assert!(!s2.contains("zzz999888777"), "{s2}");
    }

    #[test]
    fn excerpt_caps_length() {
        let r = Redactor::new();
        let s = r.excerpt(&"a".repeat(500), 10);
        assert_eq!(s.chars().count(), 11);
    }
}
