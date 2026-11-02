//! GHOST-PROMPT Slice 1: consent-gated, in-memory editor revision capture.
//!
//! MASTER_SPEC sections 52-55 and 86.1-4, 86.8-9. The engine is deliberately
//! offline and storage-free: it records bounded edit events, deleted
//! fragments, and factual pause durations in memory only, and it can wipe
//! them observably. It never reads a clock (callers pass `now_ms`, the
//! milliseconds since session start), never touches the filesystem or a
//! network, and never sees a password field: session creation refuses a
//! password editor scope outright. No inference about emotion or intent is
//! made from pauses or cadence anywhere in this module.

use sha2::{Digest, Sha256};

/// Maximum buffered edit events per session (ring behavior, oldest first).
pub const MAX_EDIT_EVENTS: usize = 500;
/// Maximum buffered edit-event text per session, in bytes (50 KiB).
pub const MAX_EVENT_TEXT_BYTES: usize = 51200;
/// Default pause threshold in milliseconds (2 seconds).
pub const DEFAULT_PAUSE_THRESHOLD_MS: u64 = 2000;
/// Default maximum session length in milliseconds (4 hours).
pub const DEFAULT_MAX_SESSION_MS: u64 = 14_400_000;

/// FNV-1a 32-bit hash, the cross-language deterministic hash shared with
/// the TypeScript mirror (offset 2166136261, prime 16777619, wrapping).
pub fn fnv1a_hex(text: &str) -> String {
    let mut hash: u32 = 2166136261;
    for byte in text.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16777619);
    }
    format!("{hash:08x}")
}

/// SHA-256 hex of the full text (Rust reference content hash).
pub fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let out = hasher.finalize();
    out.iter().map(|b| format!("{b:02x}")).collect()
}

/// Which editor a session may attach to. A password scope is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorScope {
    pub id: String,
    pub is_password: bool,
}

/// Session lifecycle: OFF by default, recording only after explicit consent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Off,
    Recording,
    Paused,
    /// Reached only via the auto-stop time bound; buffers are already wiped.
    Stopped,
}

/// How an edit reached the editor. The UI supplies the hint; the engine
/// infers it from the diff when no hint is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    Insert,
    Delete,
    Replace,
    Paste,
    Undo,
    Redo,
}

/// One bounded edit event. `deleted` holds the removed span (empty for pure
/// insertions); `inserted` holds the added span (empty for pure deletions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhostEditEvent {
    pub seq: u64,
    pub kind: EditKind,
    /// Character offset (Unicode scalar values) of the change in the old text.
    pub offset: usize,
    pub deleted: String,
    pub inserted: String,
    /// Character length of the full text after this edit.
    pub text_len_after: usize,
    /// FNV-1a hash of `deleted`, empty when nothing was deleted.
    pub deleted_hash: String,
    /// Milliseconds since session start at which the edit arrived.
    pub elapsed_ms: u64,
}

/// A factual pause: only the duration is recorded, never an interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhostPauseEvent {
    /// Milliseconds between the previous recorded edit and the next one.
    pub duration_ms: u64,
    /// Milliseconds since session start at which the following edit arrived.
    pub elapsed_ms: u64,
}

/// A deleted fragment for review: the event plus its position in sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhostDeletedFragment {
    pub seq: u64,
    pub offset: usize,
    pub text: String,
    pub hash: String,
}

/// The diff of one editor change: common prefix + common suffix over chars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDiff {
    pub offset: usize,
    pub deleted: String,
    pub inserted: String,
}

pub fn diff_texts(old: &str, new: &str) -> TextDiff {
    let old_chars: Vec<char> = old.chars().collect();
    let new_chars: Vec<char> = new.chars().collect();
    let mut prefix = 0usize;
    while prefix < old_chars.len()
        && prefix < new_chars.len()
        && old_chars[prefix] == new_chars[prefix]
    {
        prefix += 1;
    }
    let mut suffix = 0usize;
    while suffix < old_chars.len() - prefix
        && suffix < new_chars.len() - prefix
        && old_chars[old_chars.len() - 1 - suffix] == new_chars[new_chars.len() - 1 - suffix]
    {
        suffix += 1;
    }
    TextDiff {
        offset: prefix,
        deleted: old_chars[prefix..old_chars.len() - suffix].iter().collect(),
        inserted: new_chars[prefix..new_chars.len() - suffix].iter().collect(),
    }
}

fn infer_kind(diff: &TextDiff) -> EditKind {
    match (!diff.deleted.is_empty(), !diff.inserted.is_empty()) {
        (true, true) => EditKind::Replace,
        (true, false) => EditKind::Delete,
        (false, true) => EditKind::Insert,
        (false, false) => EditKind::Insert,
    }
}

/// The in-memory session. All state lives here; nothing is persisted.
#[derive(Debug)]
pub struct GhostSession {
    pub session_id: String,
    pub editor_id: String,
    pub state: SessionState,
    pub pause_threshold_ms: u64,
    pub max_session_ms: u64,
    pub current_text: String,
    pub current_text_fnv: String,
    pub current_text_sha256: String,
    pub events: Vec<GhostEditEvent>,
    pub pauses: Vec<GhostPauseEvent>,
    pub event_text_bytes: usize,
    pub next_seq: u64,
    pub last_edit_ms: Option<u64>,
    pub consent_acknowledged: bool,
}

impl GhostSession {
    /// Creates a session. Refuses password editor scopes outright.
    pub fn new(
        session_id: &str,
        scope: &EditorScope,
        pause_threshold_ms: u64,
        max_session_ms: u64,
    ) -> Result<Self, String> {
        if scope.is_password {
            return Err("refusing to capture a password or credential field".to_string());
        }
        Ok(Self {
            session_id: session_id.to_string(),
            editor_id: scope.id.clone(),
            state: SessionState::Off,
            pause_threshold_ms,
            max_session_ms,
            current_text: String::new(),
            current_text_fnv: fnv1a_hex(""),
            current_text_sha256: sha256_hex(""),
            events: Vec::new(),
            pauses: Vec::new(),
            event_text_bytes: 0,
            next_seq: 1,
            last_edit_ms: None,
            consent_acknowledged: false,
        })
    }

    pub fn new_default(session_id: &str, scope: &EditorScope) -> Result<Self, String> {
        Self::new(
            session_id,
            scope,
            DEFAULT_PAUSE_THRESHOLD_MS,
            DEFAULT_MAX_SESSION_MS,
        )
    }

    /// Explicit opt-in. This is the only Off -> Recording transition.
    pub fn enable(&mut self) {
        if self.state == SessionState::Off || self.state == SessionState::Stopped {
            self.state = SessionState::Recording;
            self.consent_acknowledged = true;
        }
    }

    pub fn pause(&mut self) {
        if self.state == SessionState::Recording {
            self.state = SessionState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.state == SessionState::Paused {
            self.state = SessionState::Recording;
        }
    }

    /// Records one editor change. Returns false when the event was refused
    /// (session off, paused, auto-stopped, or a no-op change). A pause event
    /// is synthesized when the gap since the previous recorded edit exceeds
    /// the threshold.
    pub fn record(
        &mut self,
        old_text: &str,
        new_text: &str,
        kind_hint: Option<EditKind>,
        now_ms: u64,
    ) -> bool {
        if self.state != SessionState::Recording {
            return false;
        }
        if now_ms > self.max_session_ms {
            self.wipe();
            self.state = SessionState::Stopped;
            return false;
        }
        let diff = diff_texts(old_text, new_text);
        if diff.deleted.is_empty() && diff.inserted.is_empty() {
            return false;
        }
        if let Some(last) = self.last_edit_ms {
            if now_ms > last {
                let gap = now_ms - last;
                if gap > self.pause_threshold_ms {
                    self.pauses.push(GhostPauseEvent {
                        duration_ms: gap,
                        elapsed_ms: now_ms,
                    });
                }
            }
        }
        let kind = kind_hint.unwrap_or_else(|| infer_kind(&diff));
        let deleted_hash = if diff.deleted.is_empty() {
            String::new()
        } else {
            fnv1a_hex(&diff.deleted)
        };
        self.event_text_bytes += diff.deleted.len() + diff.inserted.len();
        self.events.push(GhostEditEvent {
            seq: self.next_seq,
            kind,
            offset: diff.offset,
            deleted: diff.deleted,
            inserted: diff.inserted,
            text_len_after: new_text.chars().count(),
            deleted_hash,
            elapsed_ms: now_ms,
        });
        self.next_seq += 1;
        self.current_text = new_text.to_string();
        self.current_text_fnv = fnv1a_hex(new_text);
        self.current_text_sha256 = sha256_hex(new_text);
        self.last_edit_ms = Some(now_ms);
        while self.events.len() > MAX_EDIT_EVENTS || self.event_text_bytes > MAX_EVENT_TEXT_BYTES
        {
            if let Some(oldest) = self.events.first() {
                self.event_text_bytes = self
                    .event_text_bytes
                    .saturating_sub(oldest.deleted.len() + oldest.inserted.len());
            }
            self.events.remove(0);
        }
        true
    }

    /// Deleted fragments in sequence order, for the review table.
    pub fn deleted_fragments(&self) -> Vec<GhostDeletedFragment> {
        self.events
            .iter()
            .filter(|e| !e.deleted.is_empty())
            .map(|e| GhostDeletedFragment {
                seq: e.seq,
                offset: e.offset,
                text: e.deleted.clone(),
                hash: e.deleted_hash.clone(),
            })
            .collect()
    }

    /// Derived cadence facts only: total inserted characters and edit count.
    pub fn cadence(&self) -> (usize, usize) {
        (
            self.events.iter().map(|e| e.inserted.chars().count()).sum(),
            self.events.len(),
        )
    }

    /// Overwrites buffered text with zero bytes, then drops it all.
    pub fn wipe(&mut self) {
        for event in self.events.iter_mut() {
            zero_string(&mut event.deleted);
            zero_string(&mut event.inserted);
            event.deleted_hash.clear();
        }
        self.events.clear();
        self.pauses.clear();
        zero_string(&mut self.current_text);
        self.current_text_fnv = fnv1a_hex("");
        self.current_text_sha256 = sha256_hex("");
        self.event_text_bytes = 0;
        self.next_seq = 1;
        self.last_edit_ms = None;
    }

    /// Clears buffers but keeps the session recording.
    pub fn clear(&mut self) {
        if self.state == SessionState::Recording || self.state == SessionState::Paused {
            self.wipe();
        }
    }

    /// Wipes buffers and returns to OFF. Re-enabling starts empty.
    pub fn stop(&mut self) {
        self.wipe();
        self.state = SessionState::Off;
        self.consent_acknowledged = false;
    }

    /// Observable verification: true only when nothing remains.
    pub fn verify_wiped(&self) -> bool {
        self.events.is_empty()
            && self.pauses.is_empty()
            && self.current_text.is_empty()
            && self.event_text_bytes == 0
    }
}

fn zero_string(text: &mut String) {
    // Overwrite the buffer with zero bytes, then drop its contents. The
    // string is never observed between the overwrite and the clear.
    unsafe {
        for byte in text.as_mut_vec().iter_mut() {
            *byte = 0;
        }
    }
    text.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> GhostSession {
        let scope = EditorScope {
            id: "ask-prompt".to_string(),
            is_password: false,
        };
        let mut s = GhostSession::new_default("ghost-test", &scope).unwrap();
        s.enable();
        s
    }

    #[test]
    fn lifecycle_off_pause_resume_stop() {
        let scope = EditorScope {
            id: "ask-prompt".to_string(),
            is_password: false,
        };
        let mut s = GhostSession::new_default("ghost-life", &scope).unwrap();
        assert_eq!(s.state, SessionState::Off);
        assert!(!s.record("", "hello", None, 0));
        s.enable();
        assert_eq!(s.state, SessionState::Recording);
        assert!(s.record("", "hello", None, 0));
        s.pause();
        assert_eq!(s.state, SessionState::Paused);
        assert!(!s.record("hello", "hello world", None, 100));
        s.resume();
        assert!(s.record("hello", "hello world", None, 100));
        s.stop();
        assert_eq!(s.state, SessionState::Off);
        assert!(s.verify_wiped());
    }

    #[test]
    fn consent_gating_records_nothing_before_enable() {
        let scope = EditorScope {
            id: "ask-prompt".to_string(),
            is_password: false,
        };
        let mut s = GhostSession::new_default("ghost-gate", &scope).unwrap();
        s.record("", "secret draft", None, 0);
        assert!(s.events.is_empty());
        assert!(s.current_text.is_empty());
        s.enable();
        assert!(s.consent_acknowledged);
    }

    #[test]
    fn diff_spans_and_offsets() {
        let mut s = session();
        // Insert.
        assert!(s.record("", "hello world", Some(EditKind::Insert), 0));
        let e = s.events.last().unwrap();
        assert_eq!(e.offset, 0);
        assert_eq!(e.inserted, "hello world");
        assert!(e.deleted.is_empty());
        // Replace a middle word: "world" -> "there".
        assert!(s.record(
            "hello world",
            "hello there",
            Some(EditKind::Replace),
            500
        ));
        let e = s.events.last().unwrap();
        assert_eq!(e.offset, 6);
        assert_eq!(e.deleted, "world");
        assert_eq!(e.inserted, "there");
        // Delete a phrase with Backspace semantics.
        assert!(s.record("hello there", "hello", None, 900));
        let e = s.events.last().unwrap();
        assert_eq!(e.kind, EditKind::Delete);
        assert_eq!(e.offset, 5);
        assert_eq!(e.deleted, " there");
        // Paste appends.
        assert!(s.record("hello", "hello pasted", Some(EditKind::Paste), 1200));
        let e = s.events.last().unwrap();
        assert_eq!(e.kind, EditKind::Paste);
        assert_eq!(e.inserted, " pasted");
        // No-op records nothing.
        let before = s.events.len();
        assert!(!s.record("hello pasted", "hello pasted", None, 1300));
        assert_eq!(s.events.len(), before);
        // Undo hint is preserved.
        assert!(s.record("hello pasted", "hello", Some(EditKind::Undo), 1500));
        assert_eq!(s.events.last().unwrap().kind, EditKind::Undo);
    }

    #[test]
    fn pause_detection_threshold() {
        let mut s = session();
        s.record("", "a", None, 0);
        s.record("a", "ab", None, 1999);
        assert!(s.pauses.is_empty());
        s.record("ab", "abc", None, 2000);
        assert!(s.pauses.is_empty(), "gap of 1ms is below threshold");
        s.record("abc", "abcd", None, 4500);
        assert_eq!(s.pauses.len(), 1);
        assert_eq!(s.pauses[0].duration_ms, 2500);
    }

    #[test]
    fn ring_buffer_drops_oldest_first() {
        let mut s = session();
        let mut text = String::new();
        for i in 0..520 {
            let next = format!("{text}x");
            assert!(s.record(&text, &next, None, (i * 10) as u64));
            text = next;
        }
        assert_eq!(s.events.len(), MAX_EDIT_EVENTS);
        assert_eq!(s.events.first().unwrap().seq, 21);
        assert_eq!(s.events.last().unwrap().seq, 520);
    }

    #[test]
    fn size_bound_drops_oldest_first() {
        let mut s = session();
        let big = "b".repeat(20000);
        let mut text = String::new();
        for i in 0..4 {
            let next = format!("{text}{big}");
            assert!(s.record(&text, &next, None, (i * 10) as u64));
            text = next;
        }
        assert!(s.event_text_bytes <= MAX_EVENT_TEXT_BYTES);
        // First two 20KB inserts (40KB) plus the third (60KB) exceed the cap,
        // so only the newest two survive.
        assert_eq!(s.events.len(), 2);
    }

    #[test]
    fn auto_stop_at_max_session() {
        let mut s = session();
        s.record("", "draft", None, 0);
        assert!(!s.record("draft", "draft!", None, DEFAULT_MAX_SESSION_MS + 1));
        assert_eq!(s.state, SessionState::Stopped);
        assert!(s.verify_wiped());
    }

    #[test]
    fn password_scope_is_refused() {
        let scope = EditorScope {
            id: "login-password".to_string(),
            is_password: true,
        };
        assert!(GhostSession::new_default("ghost-pw", &scope).is_err());
    }

    #[test]
    fn wipe_is_complete_and_observable() {
        let mut s = session();
        s.record("", "alpha requirement", None, 0);
        s.record("alpha requirement", "beta requirement", None, 100);
        s.record("beta requirement", "beta", None, 3000);
        assert_eq!(s.deleted_fragments().len(), 2);
        let serialized = format!("{s:?}");
        assert!(serialized.contains("alpha"));
        s.stop();
        assert!(s.verify_wiped());
        let after = format!("{s:?}");
        assert!(!after.contains("alpha"));
        assert!(!after.contains("beta"));
        assert!(!after.contains("requirement"));
    }

    #[test]
    fn hashes_are_deterministic() {
        assert_eq!(fnv1a_hex(""), "811c9dc5");
        assert_eq!(fnv1a_hex("hello"), fnv1a_hex("hello"));
        assert_eq!(
            sha256_hex("hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        let mut s = session();
        s.record("", "hello", None, 0);
        assert_eq!(s.current_text_fnv, fnv1a_hex("hello"));
        assert_eq!(s.current_text_sha256, sha256_hex("hello"));
    }
}
