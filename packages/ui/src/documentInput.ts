import type { ColliderDocumentSource } from '@anansi/contracts';

/**
 * Document input for collider beams (M8 document slice).
 *
 * One ordinary browser file pick supplies bytes; this module
 * turns those bytes into a beam text plus backend-verifiable
 * provenance, and guards the async load against races. No
 * filesystem path is ever read, sent or stored: the label
 * is the picked file's basename only, and the backend
 * recomputes both hashes against the submitted beam text.
 */

/** Largest accepted document file, in bytes (mirrors the backend). */
export const MAX_DOCUMENT_BYTES = 65536;
/** Largest accepted beam, in characters (the existing beam bound). */
export const MAX_DOCUMENT_CHARS = 8000;
/** Largest accepted file-name label, in characters. */
export const MAX_LABEL_CHARS = 255;

export type DocumentLoad = {
  text: string;
  source: ColliderDocumentSource;
};

function sha256Hex(data: Uint8Array): Promise<string> {
  const cryptoObj = globalThis.crypto;
  if (!cryptoObj?.subtle) {
    return Promise.reject(
      Error('Secure hashing is unavailable in this environment.'),
    );
  }
  return cryptoObj.subtle.digest('SHA-256', data as BufferSource).then(buffer =>
    [...new Uint8Array(buffer)]
      .map(byte => byte.toString(16).padStart(2, '0'))
      .join(''),
  );
}

function labelKind(label: string): 'txt' | 'md' | null {
  const dot = label.lastIndexOf('.');
  if (dot <= 0) return null;
  const suffix = label.slice(dot + 1).toLowerCase();
  if (suffix === 'txt') return 'txt';
  if (suffix === 'md') return 'md';
  return null;
}

/**
 * Decode picked file bytes into a beam text plus provenance.
 * Refuses visibly (throws) on: an unsupported or missing
 * extension, a path-like or overlong label, an oversized
 * file, invalid UTF-8 (strict decode — no replacement
 * characters), or empty/whitespace-only or overlong text.
 * A single leading UTF-8 BOM is stripped and recorded;
 * nothing else is normalized: indentation, CRLF/LF endings
 * and terminal newlines survive exactly.
 */
export async function decodeDocumentBytes(
  bytes: Uint8Array,
  filename: string,
): Promise<DocumentLoad> {
  const label = filename;
  if (
    label.length === 0 ||
    [...label].length > MAX_LABEL_CHARS ||
    label.includes('/') ||
    label.includes('\\') ||
    label.includes('\0')
  ) {
    throw Error('The file name must be a plain file name, not a path.');
  }
  const kind = labelKind(label);
  if (kind === null) {
    throw Error('Only .txt and .md files are supported in this slice.');
  }
  if (bytes.length > MAX_DOCUMENT_BYTES) {
    throw Error(
      `The file must be at most ${MAX_DOCUMENT_BYTES} bytes.`,
    );
  }
  let bom = false;
  let body = bytes;
  if (
    bytes.length >= 3 &&
    bytes[0] === 0xef &&
    bytes[1] === 0xbb &&
    bytes[2] === 0xbf
  ) {
    bom = true;
    body = bytes.slice(3);
  }
  let text: string;
  try {
    text = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(
      body,
    );
  } catch {
    throw Error('The file is not valid UTF-8 text.');
  }
  const textChars = [...text].length;
  if (text.trim().length === 0) {
    throw Error('The file is empty or contains only whitespace.');
  }
  if (textChars > MAX_DOCUMENT_CHARS) {
    throw Error(
      `The file's text must be at most ${MAX_DOCUMENT_CHARS} characters.`,
    );
  }
  // The raw hash covers the bytes exactly as picked
  // (including a stripped BOM); the text hash covers
  // the submitted text bytes. The backend recomputes
  // both from the beam text plus the BOM flag.
  const [rawHash, textHash] = await Promise.all([
    sha256Hex(bytes),
    sha256Hex(new TextEncoder().encode(text)),
  ]);
  return {
    text,
    source: {
      label,
      kind,
      bom,
      rawBytes: bytes.length,
      rawHash,
      textHash,
      textChars,
    },
  };
}

/**
 * The outcome of one document load. It never rejects: a
 * failed read is an outcome like a successful one, so the
 * caller can gate failures by the same generation identity
 * as successes. An obsolete failure must change neither
 * the error nor the loading state of a newer load.
 */
export type DocumentLoadOutcome = {
  generation: number;
  result: DocumentLoad | null;
  error: Error | null;
};

/**
 * Per-beam load session. Every new selection, edit or clear
 * invalidates pending reads: `load` resolves to its result
 * only when its generation is still current, otherwise to
 * null, and to its error for a failed read, so a late file
 * read — successful or failed — can never overwrite newer
 * user intent. Callers own the loading indicator: a
 * superseded load resolves without effect, and the
 * invalidating action clears the indicator itself. Callers
 * must check the outcome's generation before applying
 * either the result or the error.
 */
export class DocumentLoadSession {
  private generation = 0;

  get current(): number {
    return this.generation;
  }

  invalidate(): void {
    this.generation += 1;
  }

  async load(
    read: () => Promise<DocumentLoad>,
  ): Promise<DocumentLoadOutcome> {
    const generation = ++this.generation;
    try {
      const result = await read();
      return {
        generation,
        result: generation === this.generation ? result : null,
        error: null,
      };
    } catch (e) {
      return {
        generation,
        result: null,
        error: e instanceof Error ? e : Error('Reading the file failed.'),
      };
    }
  }
}
