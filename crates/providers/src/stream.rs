//! Incremental stream parsers. Both parsers accept arbitrarily split
//! byte feeds (a chunk boundary may fall inside a line, an event, or a
//! multi-byte UTF-8 sequence) and only emit complete units. That is the
//! property the Ask substrate's parsers are tested for in `anansi-core`,
//! and it is tested byte-by-byte here.

use crate::error::{ProviderError, ProviderErrorCode};

/// One Server-Sent Events unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
}

const MAX_LINE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Default)]
pub struct SseParser {
    buffer: Vec<u8>,
    data_lines: Vec<String>,
    event_type: Option<String>,
}

impl SseParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Result<Vec<SseEvent>, ProviderError> {
        self.buffer.extend_from_slice(bytes);
        if self.buffer.len() > MAX_LINE_BYTES && !self.buffer.contains(&b'\n') {
            return Err(ProviderError::new(
                ProviderErrorCode::MalformedResponse,
                "stream line exceeds the 1 MiB bound",
            ));
        }
        let mut events = Vec::new();
        while let Some(nl) = self.buffer.iter().position(|b| *b == b'\n') {
            let mut line: Vec<u8> = self.buffer.drain(..=nl).collect();
            if line.last() == Some(&b'\n') {
                line.pop();
            }
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            self.process_line(&line, &mut events);
        }
        Ok(events)
    }

    fn process_line(&mut self, line: &[u8], events: &mut Vec<SseEvent>) {
        if line.is_empty() {
            if !self.data_lines.is_empty() {
                events.push(SseEvent {
                    event: self.event_type.take(),
                    data: self.data_lines.join("\n"),
                });
                self.data_lines.clear();
            } else {
                self.event_type = None;
            }
            return;
        }
        if line.starts_with(b":") {
            return; // SSE comment / keepalive
        }
        let text = String::from_utf8_lossy(line);
        if let Some(v) = text.strip_prefix("data:") {
            self.data_lines
                .push(v.strip_prefix(' ').unwrap_or(v).to_string());
        } else if let Some(v) = text.strip_prefix("event:") {
            self.event_type = Some(v.strip_prefix(' ').unwrap_or(v).to_string());
        }
        // `id:` / `retry:` and unknown fields are ignored per SSE rules.
    }

    /// Flush a trailing event at end of stream (some providers omit the
    /// final blank line).
    pub fn finish(&mut self) -> Vec<SseEvent> {
        let mut events = Vec::new();
        if !self.buffer.is_empty() {
            let line = std::mem::take(&mut self.buffer);
            self.process_line(&line, &mut events);
        }
        if !self.data_lines.is_empty() {
            events.push(SseEvent {
                event: self.event_type.take(),
                data: self.data_lines.join("\n"),
            });
            self.data_lines.clear();
        }
        events
    }
}

/// Newline-delimited JSON parser (Ollama streaming shape).
#[derive(Debug, Default)]
pub struct NdjsonParser {
    buffer: Vec<u8>,
}

impl NdjsonParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Result<Vec<String>, ProviderError> {
        self.buffer.extend_from_slice(bytes);
        if self.buffer.len() > MAX_LINE_BYTES && !self.buffer.contains(&b'\n') {
            return Err(ProviderError::new(
                ProviderErrorCode::MalformedResponse,
                "stream line exceeds the 1 MiB bound",
            ));
        }
        let mut lines = Vec::new();
        while let Some(nl) = self.buffer.iter().position(|b| *b == b'\n') {
            let mut line: Vec<u8> = self.buffer.drain(..=nl).collect();
            if line.last() == Some(&b'\n') {
                line.pop();
            }
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            if !line.is_empty() {
                lines.push(String::from_utf8_lossy(&line).to_string());
            }
        }
        Ok(lines)
    }

    pub fn finish(&mut self) -> Vec<String> {
        if self.buffer.is_empty() {
            return Vec::new();
        }
        let line = String::from_utf8_lossy(&self.buffer).trim().to_string();
        self.buffer.clear();
        if line.is_empty() {
            Vec::new()
        } else {
            vec![line]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SSE_SAMPLE: &str = ": keepalive\n\nevent: message_start\ndata: {\"type\":\"message_start\"}\n\ndata: {\"delta\":\"Hel\"}\ndata: {\"delta\":\"lo\"}\n\ndata: [DONE]\n\n";

    #[test]
    fn sse_whole_feed() {
        let mut p = SseParser::new();
        let events = p.feed(SSE_SAMPLE.as_bytes()).unwrap();
        // The two consecutive `data:` lines form ONE event (SSE joins
        // multi-line data with '\n'); the comment block emits nothing.
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event.as_deref(), Some("message_start"));
        assert_eq!(events[1].data, "{\"delta\":\"Hel\"}\n{\"delta\":\"lo\"}");
        assert_eq!(events[2].data, "[DONE]");
    }

    #[test]
    fn sse_byte_by_byte_matches_whole() {
        let mut whole = SseParser::new();
        let expected = whole.feed(SSE_SAMPLE.as_bytes()).unwrap();
        let mut drip = SseParser::new();
        let mut got = Vec::new();
        for b in SSE_SAMPLE.as_bytes() {
            got.extend(drip.feed(&[*b]).unwrap());
        }
        got.extend(drip.finish());
        assert_eq!(got, expected);
    }

    #[test]
    fn ndjson_split_mid_line() {
        let mut p = NdjsonParser::new();
        assert!(p.feed(b"{\"a\":1}\n{\"b\":").unwrap().len() == 1);
        let rest = p.feed(b"2}\n").unwrap();
        assert_eq!(rest, vec!["{\"b\":2}".to_string()]);
    }
}
