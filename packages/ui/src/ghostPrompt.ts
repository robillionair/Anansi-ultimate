// GHOST-PROMPT Slice 1: consent-gated, in-memory editor revision capture.
//
// TypeScript mirror of crates/core/src/ghost_prompt.rs. Same semantics:
// OFF by default, explicit opt-in only, bounded ring buffer (500 edit
// events / 50KB event text), factual pause durations only, observable
// wipe. FNV-1a is the deterministic hash in both languages; SHA-256 is
// computed in the Rust reference engine only, because the browser has no
// synchronous SHA-256 API and this module never fakes one. Nothing here
// touches disk, SQLite, or any network, and nothing is sent anywhere.

export const GHOST_MAX_EDIT_EVENTS = 500;
export const GHOST_MAX_EVENT_TEXT_BYTES = 51200;
export const GHOST_DEFAULT_PAUSE_THRESHOLD_MS = 2000;
export const GHOST_DEFAULT_MAX_SESSION_MS = 14_400_000;

export type GhostSessionState = 'off' | 'recording' | 'paused' | 'stopped';
export type GhostEditKind = 'insert' | 'delete' | 'replace' | 'paste' | 'undo' | 'redo';

export interface GhostEditEvent {
  seq: number;
  kind: GhostEditKind;
  offset: number;
  deleted: string;
  inserted: string;
  textLenAfter: number;
  deletedHash: string;
  elapsedMs: number;
}

export interface GhostPauseEvent {
  durationMs: number;
  elapsedMs: number;
}

export interface GhostDeletedFragment {
  seq: number;
  offset: number;
  text: string;
  hash: string;
}

export interface GhostTextDiff {
  offset: number;
  deleted: string;
  inserted: string;
}

export interface GhostFinding {
  kind: GhostFindingKind;
  text: string;
  evidence: string[];
}

export type GhostFindingKind =
  | 'price-changed'
  | 'privacy-removed'
  | 'audience-shift'
  | 'technical-term-substituted';

const PRICE_QUALIFIERS = ['cheap', 'free', 'budget', 'expensive'];
const PRIVACY_KEYWORDS = ['privacy', 'confidential', 'local', 'untracked', 'sealed'];
const AUDIENCE_TERMS = [
  'freelancers',
  'enterprise',
  'lawyers',
  'startups',
  'agencies',
  'developers',
];
const TECHNICAL_TERMS = [
  'api',
  'embedding',
  'vector',
  'kubernetes',
  'oauth',
  'websocket',
  'sql',
  'llm',
  'neural',
  'algorithm',
];
const TRIM_CHARS = ',.;:!?()[]"\'';

/** Tokenize exactly like the Rust mirror: lowercase, split on whitespace, trim surrounding punctuation, drop empties. */
export function ghostTokens(text: string): string[] {
  return text
    .split(/\s+/)
    .map((token) => {
      let trimmed = token;
      while (trimmed.length > 0 && TRIM_CHARS.includes(trimmed[0])) {
        trimmed = trimmed.slice(1);
      }
      while (
        trimmed.length > 0 &&
        TRIM_CHARS.includes(trimmed[trimmed.length - 1])
      ) {
        trimmed = trimmed.slice(0, -1);
      }
      return trimmed.toLowerCase();
    })
    .filter((token) => token !== '');
}

/** A price signal: a `$`-prefixed number (`$50`) or a qualifier word. */
function priceSignal(token: string): string | null {
  if (token.startsWith('$')) {
    const cleaned = token.slice(1).replaceAll(',', '');
    if (cleaned !== '' && !Number.isNaN(Number(cleaned))) return `$${cleaned}`;
    return null;
  }
  if (PRICE_QUALIFIERS.includes(token)) return token;
  return null;
}

function termsPresent(tokens: string[], terms: string[]): string[] {
  const found: string[] = [];
  for (const token of tokens) {
    if (terms.includes(token) && !found.includes(token)) found.push(token);
  }
  return found;
}

/**
 * M11 GHOST-PROMPT Slice 3: deterministic draft analysis (§57), mirrored
 * in crates/core/src/ghost_prompt.rs. Findings are objective observations
 * about text changes only — never intent, emotion, or subconscious claims.
 * Fixed order: price, privacy, audience, technical. At most one finding
 * per kind; evidence collects up to 5 triggers.
 */
export function analyzeGhostDraft(
  fragments: GhostDeletedFragment[],
  events: GhostEditEvent[],
  finalText: string,
): GhostFinding[] {
  const findings: GhostFinding[] = [];
  const finalTokens = ghostTokens(finalText);
  const sortedEvents = [...events].sort((a, b) => a.seq - b.seq);

  const signals: string[] = [];
  for (const event of sortedEvents) {
    for (const side of [event.deleted, event.inserted]) {
      for (const token of ghostTokens(side)) {
        const signal = priceSignal(token);
        if (signal !== null && !signals.includes(signal)) signals.push(signal);
      }
    }
  }
  if (signals.length >= 2) {
    findings.push({
      kind: 'price-changed',
      text: 'You changed the requested price several times.',
      evidence: [signals.join(' → ')],
    });
  }

  let deletedText = '';
  for (const fragment of fragments) deletedText += `${fragment.text} `;
  for (const event of sortedEvents) deletedText += `${event.deleted} `;
  const deletedTokens = ghostTokens(deletedText);
  const removed: string[] = [];
  for (const keyword of PRIVACY_KEYWORDS) {
    if (
      deletedTokens.includes(keyword) &&
      !finalTokens.includes(keyword) &&
      !removed.includes(keyword)
    ) {
      removed.push(keyword);
    }
  }
  if (removed.length > 0) {
    findings.push({
      kind: 'privacy-removed',
      text: 'You removed a requirement about privacy.',
      evidence: removed,
    });
  }

  const audienceEvidence: string[] = [];
  for (const event of sortedEvents) {
    const deletedTerms = termsPresent(ghostTokens(event.deleted), AUDIENCE_TERMS);
    const insertedTerms = termsPresent(ghostTokens(event.inserted), AUDIENCE_TERMS);
    for (const from of deletedTerms) {
      for (const to of insertedTerms) {
        if (from !== to) {
          const item = `${from} → ${to}`;
          if (audienceEvidence.length < 5 && !audienceEvidence.includes(item)) {
            audienceEvidence.push(item);
          }
        }
      }
      if (event.inserted === '' && !finalTokens.includes(from)) {
        const item = `${from} → (removed)`;
        if (audienceEvidence.length < 5 && !audienceEvidence.includes(item)) {
          audienceEvidence.push(item);
        }
      }
    }
  }
  if (audienceEvidence.length > 0) {
    findings.push({
      kind: 'audience-shift',
      text: 'You revised the target audience.',
      evidence: audienceEvidence,
    });
  }

  const technicalEvidence: string[] = [];
  for (const event of sortedEvents) {
    if (event.deleted === '' || event.inserted === '') continue;
    const deletedTerms = termsPresent(ghostTokens(event.deleted), TECHNICAL_TERMS);
    const insertedTerms = termsPresent(ghostTokens(event.inserted), TECHNICAL_TERMS);
    if (deletedTerms.length > 0 && insertedTerms.length === 0) {
      const item = `${deletedTerms[0]} → ${ghostTokens(event.inserted).join(' ')}`;
      if (technicalEvidence.length < 5 && !technicalEvidence.includes(item)) {
        technicalEvidence.push(item);
      }
    }
  }
  if (technicalEvidence.length > 0) {
    findings.push({
      kind: 'technical-term-substituted',
      text: 'You replaced a technical term with a broader description.',
      evidence: technicalEvidence,
    });
  }

  return findings;
}

/**
 * Compile the Panel B prompt: the final text plus exactly the selected
 * fragments, in seq order, under an explicit header. An empty selection
 * returns the final text unchanged — no header, no hidden context.
 * Mirrored in crates/core/src/ghost_prompt.rs.
 */
export function compileApprovedContext(
  finalText: string,
  selected: GhostDeletedFragment[],
): string {
  if (selected.length === 0) return finalText;
  const sorted = [...selected].sort((a, b) => a.seq - b.seq);
  let out = `${finalText}\n\n--- Approved draft context (earlier alternatives you selected) ---\n`;
  for (const fragment of sorted) {
    out += `- ${fragment.text} (deleted at offset ${fragment.offset})\n`;
  }
  return out.replace(/\n+$/, '');
}

export interface GhostEditorScope {
  id: string;
  isPassword: boolean;
}

export function ghostFnv1aHex(text: string): string {
  let hash = 2166136261;
  const bytes = new TextEncoder().encode(text);
  for (const byte of bytes) {
    hash ^= byte;
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(16).padStart(8, '0');
}

export function ghostDiffTexts(oldText: string, newText: string): GhostTextDiff {
  const oldChars = Array.from(oldText);
  const newChars = Array.from(newText);
  let prefix = 0;
  while (
    prefix < oldChars.length &&
    prefix < newChars.length &&
    oldChars[prefix] === newChars[prefix]
  ) {
    prefix += 1;
  }
  let suffix = 0;
  while (
    suffix < oldChars.length - prefix &&
    suffix < newChars.length - prefix &&
    oldChars[oldChars.length - 1 - suffix] === newChars[newChars.length - 1 - suffix]
  ) {
    suffix += 1;
  }
  return {
    offset: prefix,
    deleted: oldChars.slice(prefix, oldChars.length - suffix).join(''),
    inserted: newChars.slice(prefix, newChars.length - suffix).join(''),
  };
}

function inferKind(diff: GhostTextDiff): GhostEditKind {
  if (diff.deleted !== '' && diff.inserted !== '') return 'replace';
  if (diff.deleted !== '') return 'delete';
  return 'insert';
}

function byteLength(text: string): number {
  return new TextEncoder().encode(text).length;
}

export class GhostSession {
  readonly sessionId: string;
  readonly editorId: string;
  state: GhostSessionState = 'off';
  readonly pauseThresholdMs: number;
  readonly maxSessionMs: number;
  currentText = '';
  currentTextHash = ghostFnv1aHex('');
  events: GhostEditEvent[] = [];
  pauses: GhostPauseEvent[] = [];
  eventTextBytes = 0;
  consentAcknowledged = false;
  private nextSeq = 1;
  private lastEditMs: number | null = null;

  private constructor(
    sessionId: string,
    scope: GhostEditorScope,
    pauseThresholdMs: number,
    maxSessionMs: number,
  ) {
    this.sessionId = sessionId;
    this.editorId = scope.id;
    this.pauseThresholdMs = pauseThresholdMs;
    this.maxSessionMs = maxSessionMs;
  }

  /** Returns null for password scopes: capture is refused outright. */
  static create(
    sessionId: string,
    scope: GhostEditorScope,
    pauseThresholdMs = GHOST_DEFAULT_PAUSE_THRESHOLD_MS,
    maxSessionMs = GHOST_DEFAULT_MAX_SESSION_MS,
  ): GhostSession | null {
    if (scope.isPassword) return null;
    return new GhostSession(sessionId, scope, pauseThresholdMs, maxSessionMs);
  }

  /** Explicit opt-in. This is the only off -> recording transition. */
  enable(): void {
    if (this.state === 'off' || this.state === 'stopped') {
      this.state = 'recording';
      this.consentAcknowledged = true;
    }
  }

  pause(): void {
    if (this.state === 'recording') this.state = 'paused';
  }

  resume(): void {
    if (this.state === 'paused') {
      this.state = 'recording';
      // The paused interval is not an editing interval: the next edit
      // must not synthesize a pause event for time spent paused.
      this.lastEditMs = null;
    }
  }

  /** Records one editor change; false when refused or a no-op change. */
  record(
    oldText: string,
    newText: string,
    kindHint: GhostEditKind | null,
    nowMs: number,
  ): boolean {
    if (this.state !== 'recording') return false;
    if (nowMs > this.maxSessionMs) {
      this.wipe();
      this.state = 'stopped';
      return false;
    }
    const diff = ghostDiffTexts(oldText, newText);
    if (diff.deleted === '' && diff.inserted === '') return false;
    if (this.lastEditMs !== null && nowMs > this.lastEditMs) {
      const gap = nowMs - this.lastEditMs;
      if (gap > this.pauseThresholdMs) {
        this.pauses.push({ durationMs: gap, elapsedMs: nowMs });
      }
    }
    const kind = kindHint ?? inferKind(diff);
    this.eventTextBytes += byteLength(diff.deleted) + byteLength(diff.inserted);
    this.events.push({
      seq: this.nextSeq,
      kind,
      offset: diff.offset,
      deleted: diff.deleted,
      inserted: diff.inserted,
      textLenAfter: Array.from(newText).length,
      deletedHash: diff.deleted === '' ? '' : ghostFnv1aHex(diff.deleted),
      elapsedMs: nowMs,
    });
    this.nextSeq += 1;
    this.currentText = newText;
    this.currentTextHash = ghostFnv1aHex(newText);
    this.lastEditMs = nowMs;
    while (
      this.events.length > GHOST_MAX_EDIT_EVENTS ||
      this.eventTextBytes > GHOST_MAX_EVENT_TEXT_BYTES
    ) {
      const oldest = this.events[0];
      if (oldest) {
        this.eventTextBytes -= byteLength(oldest.deleted) + byteLength(oldest.inserted);
      }
      this.events.shift();
    }
    return true;
  }

  deletedFragments(): GhostDeletedFragment[] {
    return this.events
      .filter((event) => event.deleted !== '')
      .map((event) => ({
        seq: event.seq,
        offset: event.offset,
        text: event.deleted,
        hash: event.deletedHash,
      }));
  }

  /** Derived cadence facts only: total inserted characters, edit count. */
  cadence(): { insertedChars: number; editCount: number } {
    return {
      insertedChars: this.events.reduce(
        (sum, event) => sum + Array.from(event.inserted).length,
        0,
      ),
      editCount: this.events.length,
    };
  }

  /**
   * Drops every buffered string and resets all counters. JavaScript
   * strings are immutable, so in-place zeroing is impossible here: wipe
   * removes every reachable reference to draft text instead (the Rust
   * engine zeroes bytes in place). verifyWiped plus the serialization
   * assertion in tests observe the result.
   */
  wipe(): void {
    this.events = [];
    this.pauses = [];
    this.currentText = '';
    this.currentTextHash = ghostFnv1aHex('');
    this.eventTextBytes = 0;
    this.nextSeq = 1;
    this.lastEditMs = null;
  }

  /** Clears buffers but keeps the session recording. */
  clear(): void {
    if (this.state === 'recording' || this.state === 'paused') this.wipe();
  }

  /** Wipes buffers and returns to OFF. Re-enabling starts empty. */
  stop(): void {
    this.wipe();
    this.state = 'off';
    this.consentAcknowledged = false;
  }

  /** Observable verification: true only when nothing remains. */
  verifyWiped(): boolean {
    return (
      this.events.length === 0 &&
      this.pauses.length === 0 &&
      this.currentText === '' &&
      this.eventTextBytes === 0
    );
  }
}
