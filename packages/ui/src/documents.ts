/**
 * Document import helpers (M13). The browser file pick is the
 * only picker: no filesystem path is ever read, sent or stored, only
 * the picked file's basename. TXT/Markdown bytes are decoded locally
 * with the shared strict decoder (`documentInput.ts`) before any
 * network traffic, so refusals (format, size, encoding, empty)
 * happen before an upload begins. A PDF is never decoded in the
 * browser — it is binary, and the server extracts its text layer
 * with the bounded extractor — so for `.pdf` only the raw bytes and
 * SHA-256 are computed here and the text is extracted server-side.
 * Uploads go through the bounded chunked path (`jobs/ingest-begin`
 * + `jobs/ingest-part`) with the client-computed SHA-256 declared up
 * front; the server re-verifies it at finish.
 */
import { decodeDocumentBytes } from './documentInput.ts';
import type { Client, Job, Privacy } from '@anansi/contracts';

/** Largest accepted PDF file, in bytes (mirrors the backend). */
export const MAX_PDF_BYTES = 1_048_576;

export type PendingDocument = {
  fileName: string;
  bytes: Uint8Array;
  text: string;
  kind: 'txt' | 'md' | 'pdf';
  bom: boolean;
  textChars: number;
  rawBytes: number;
  rawHash: string;
  textHash: string;
};

function sha256Hex(data: Uint8Array): Promise<string> {
  const cryptoObj = globalThis.crypto;
  if (!cryptoObj?.subtle) {
    return Promise.reject(Error('Secure hashing is unavailable in this environment.'));
  }
  return cryptoObj.subtle.digest('SHA-256', data as BufferSource).then(buffer =>
    [...new Uint8Array(buffer)]
      .map(byte => byte.toString(16).padStart(2, '0'))
      .join(''),
  );
}

/** Decode a picked file locally. Throws with the refusal reason. */
export async function prepareDocument(file: File): Promise<PendingDocument> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  // A PDF is never decoded in the browser: its text layer is
  // extracted server-side by the bounded extractor. Only the raw
  // bytes and SHA-256 are computed here; the text count is unknown
  // until the import runs.
  if (file.name.toLowerCase().endsWith('.pdf')) {
    if (bytes.length === 0 || bytes.length > MAX_PDF_BYTES) {
      throw Error(`The file must be 1 to ${MAX_PDF_BYTES} bytes.`);
    }
    return {
      fileName: file.name,
      bytes,
      text: '',
      kind: 'pdf',
      bom: false,
      textChars: 0,
      rawBytes: bytes.length,
      rawHash: await sha256Hex(bytes),
      textHash: '',
    };
  }
  const decoded = await decodeDocumentBytes(bytes, file.name);
  return {
    fileName: file.name,
    bytes,
    text: decoded.text,
    kind: decoded.source.kind as 'txt' | 'md',
    bom: decoded.source.bom,
    textChars: Number(decoded.source.textChars),
    rawBytes: Number(decoded.source.rawBytes),
    rawHash: decoded.source.rawHash,
    textHash: decoded.source.textHash,
  };
}

/** Received bytes for an in-flight ingestion job, or null. */
export function ingestionReceived(job: Job): number | null {
  const match = /^uploading:(\d+)$/.exec(job.checkpoint);
  return match ? Number(match[1]) : null;
}

export function isIngestionJob(job: Job): boolean {
  return job.selection.kind === 'ingestion';
}

export function ingestionJobLabel(job: Job): string {
  if (job.selection.kind === 'ingestion') return `Document import: ${job.selection.fileName}`;
  if (job.selection.kind === 'backup') {
    return job.selection.application ? 'Application data backup' : 'Workspace backup';
  }
  if (job.selection.kind === 'project') return 'Project export';
  if (job.selection.kind === 'records') return 'Selected records export';
  return 'Workspace export';
}

type Progress = (received: number, total: number) => void;

/**
 * Upload a prepared document in sequential 64 KiB parts. Returns the
 * job once the upload finishes (the job is then Queued for import).
 * The caller polls `listJobs` for the import outcome.
 */
export async function uploadDocument(
  client: Client,
  workspaceId: string,
  pending: PendingDocument,
  privacy: Privacy,
  projectId: string | null,
  onProgress: Progress,
): Promise<Job> {
  const sha = await sha256Hex(pending.bytes);
  if (sha !== pending.rawHash) {
    throw Error('The file changed after selection. Pick the file again.');
  }
  const opened = await client.ingestBegin(
    workspaceId,
    pending.fileName,
    pending.bytes.length,
    sha,
    privacy,
    projectId,
  );
  let result = opened;
  for (let offset = 0; offset < pending.bytes.length; offset += 65536) {
    const end = Math.min(offset + 65536, pending.bytes.length);
    result = await client.ingestPart(
      workspaceId,
      opened.id,
      offset,
      pending.bytes.subarray(offset, end),
      end === pending.bytes.length,
    );
    onProgress(result.received, pending.bytes.length);
  }
  if (!result.job) throw Error('Document import did not return a job.');
  return result.job;
}

/**
 * Resume an interrupted upload after a client restart. The user
 * re-picks the same file; the declared hash must match the job's
 * declared hash, otherwise this is a different file and the resume
 * is refused. Completed parts are not re-sent.
 */
export async function resumeUpload(
  client: Client,
  workspaceId: string,
  job: Job,
  pending: PendingDocument,
  onProgress: Progress,
): Promise<Job> {
  if (!isIngestionJob(job)) throw Error('This job is not a document import.');
  const received = ingestionReceived(job);
  if (received === null) throw Error('This import has no upload to resume.');
  if (job.selection.kind !== 'ingestion') throw Error('This job is not a document import.');
  if (pending.rawHash !== job.selection.expectedHash) {
    throw Error('This is a different file. Pick the file this import started with.');
  }
  if (pending.bytes.length !== job.selection.expectedBytes) {
    throw Error('The re-picked file has a different size. Pick the file this import started with.');
  }
  let resultReceived = received;
  for (let offset = received; offset < pending.bytes.length; offset += 65536) {
    const end = Math.min(offset + 65536, pending.bytes.length);
    const result = await client.ingestPart(
      workspaceId,
      job.id,
      offset,
      pending.bytes.subarray(offset, end),
      end === pending.bytes.length,
    );
    resultReceived = result.received;
    onProgress(resultReceived, pending.bytes.length);
  }
  const list = await client.listJobs(workspaceId);
  const updated = list.jobs.find(candidate => candidate.id === job.id);
  if (!updated) throw Error('The import job disappeared. Refresh jobs and try again.');
  return updated;
}
