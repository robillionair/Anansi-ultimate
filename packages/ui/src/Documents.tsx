import { useEffect, useRef, useState } from 'react';
import type { Client, Job, MemoryDetails, MemoryMetadata, Privacy } from '@anansi/contracts';
import { renderMarkdown } from './markdown.ts';
import {
  ingestionJobLabel,
  ingestionReceived,
  isIngestionJob,
  prepareDocument,
  resumeUpload,
  uploadDocument,
} from './documents.ts';
import type { PendingDocument } from './documents.ts';

const SUPPORTED_NOTE = 'Only .txt, .md and text-layer .pdf files can be imported in this slice. DOCX and other formats are tracked as follow-up work.';

function formatRefusal(fileName: string, message: string): string {
  if (!message.includes('Only .txt and .md') && !message.includes('Only .txt, .md')) return message;
  const dot = fileName.lastIndexOf('.');
  const suffix = dot > 0 ? fileName.slice(dot + 1).toUpperCase() : 'This';
  return `${suffix} files are not supported yet. ${SUPPORTED_NOTE}`;
}

/**
 * Plain-text reader for an imported PDF. The stored body is the
 * pages joined by a form feed, so the reader splits on it and shows
 * one page at a time. A PDF body is never passed through the
 * Markdown renderer: its text is extracted plain text, and Markdown
 * markup in it (a `#` line, `**bold**`) is content, not formatting.
 */
function PdfReader({
  body,
  page,
  onPage,
}: {
  body: string;
  page: number;
  onPage: (page: number) => void;
}) {
  const pages = body.split('\x0c');
  const current = Math.min(Math.max(page, 0), pages.length - 1);
  return (
    <div>
      <p>
        Page {current + 1} of {pages.length}
      </p>
      <div className="actions">
        <button disabled={current === 0} className="secondary" onClick={() => onPage(current - 1)}>
          Previous page
        </button>
        <button
          disabled={current >= pages.length - 1}
          className="secondary"
          onClick={() => onPage(current + 1)}
        >
          Next page
        </button>
      </div>
      <pre>{pages[current]}</pre>
    </div>
  );
}

export function Documents({
  client,
  workspaceId,
  onNavigate,
}: {
  client: Client;
  workspaceId: string;
  onNavigate: (view: 'ask' | 'jobs') => void;
}) {
  const [documents, setDocuments] = useState<MemoryMetadata[]>([]);
  const [offset, setOffset] = useState(0);
  const [next, setNext] = useState<number | null>(null);
  const [current, setCurrent] = useState<MemoryDetails | null>(null);
  const [showSource, setShowSource] = useState(false);
  const [page, setPage] = useState(0);
  const [pending, setPending] = useState<PendingDocument | null>(null);
  const [privacy, setPrivacy] = useState<Privacy>('private');
  const [projectId, setProjectId] = useState('');
  const [projects, setProjects] = useState<MemoryMetadata[]>([]);
  const [activeJob, setActiveJob] = useState<Job | null>(null);
  const [received, setReceived] = useState(0);
  const [resumable, setResumable] = useState<Job[]>([]);
  const [query, setQuery] = useState('');
  const [searchResults, setSearchResults] = useState<MemoryMetadata[] | null>(null);
  const [searchPages, setSearchPages] = useState<Record<string, number>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const fileRef = useRef<HTMLInputElement | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  // The interval below captures the first render's closure, so the
  // in-flight job id must live in a ref, not in state.
  const activeJobIdRef = useRef<string | null>(null);

  async function run(action: () => Promise<void>) {
    setBusy(true);
    setError('');
    setNotice('');
    try {
      await action();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Documents are unavailable. Try again.');
    } finally {
      setBusy(false);
    }
  }

  async function refresh(at = offset) {
    const result = await client.listMemories(workspaceId, { offset: at, kind: 'document' });
    setDocuments(result.memories);
    setOffset(at);
    setNext(result.nextOffset ?? null);
  }

  async function refreshJobs() {
    const list = await client.listJobs(workspaceId);
    const open = list.jobs.filter(
      job => isIngestionJob(job) && job.state === 'Paused' && ingestionReceived(job) !== null,
    );
    setResumable(open);
    const activeId = activeJobIdRef.current;
    if (activeId) {
      const updated = list.jobs.find(job => job.id === activeId);
      if (updated) {
        setActiveJob(updated);
        if (updated.state === 'Completed') {
          setNotice(`Document imported: ${ingestionJobLabel(updated)}.`);
          setPending(null);
          setActiveJob(null);
          activeJobIdRef.current = null;
          await refresh(0);
        } else if (updated.state === 'Failed') {
          setError(`Import failed: ${updated.log}`);
          setActiveJob(null);
          activeJobIdRef.current = null;
        } else if (updated.state === 'Cancelled') {
          setNotice('Import cancelled. No document was published.');
          setActiveJob(null);
          activeJobIdRef.current = null;
        }
      }
    }
  }

  async function open(id: string) {
    const result = await client.readMemory(workspaceId, id);
    setCurrent(result);
    setShowSource(false);
    setPage(0);
  }

  useEffect(() => {
    void run(async () => {
      await refresh(0);
      const found = await client.listMemories(workspaceId, { kind: 'project', limit: 100 });
      setProjects(found.memories);
      await refreshJobs();
    });
    pollRef.current = setInterval(() => {
      void refreshJobs();
    }, 4000);
    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [client, workspaceId]);

  async function pick(file: File | undefined) {
    if (!file) return;
    await run(async () => {
      try {
        const prepared = await prepareDocument(file);
        setPending(prepared);
        setReceived(0);
        setNotice(`Ready to import ${prepared.fileName} (${prepared.rawBytes} bytes, ${prepared.textChars} characters). Review it below, then import.`);
      } catch (e) {
        setPending(null);
        throw Error(formatRefusal(file.name, e instanceof Error ? e.message : 'The file could not be read.'));
      }
    });
    if (fileRef.current) fileRef.current.value = '';
  }

  async function startImport() {
    if (!pending) return;
    await run(async () => {
      const job = await uploadDocument(
        client,
        workspaceId,
        pending,
        privacy,
        projectId || null,
        (got, _total) => setReceived(got),
      );
      setActiveJob(job);
      activeJobIdRef.current = job.id;
      setNotice(`Upload complete. Import queued as job ${job.id}.`);
    });
  }

  async function resume(job: Job) {
    if (!pending) {
      setError('Pick the same file again first, then resume its upload.');
      return;
    }
    await run(async () => {
      const updated = await resumeUpload(client, workspaceId, job, pending, (got, _total) =>
        setReceived(got),
      );
      setActiveJob(updated);
      activeJobIdRef.current = updated.id;
      setResumable(previous => previous.filter(candidate => candidate.id !== job.id));
      setNotice(`Upload resumed and complete. Import queued as job ${updated.id}.`);
    });
  }

  async function cancel(job: Job) {
    await run(async () => {
      await client.controlJob(workspaceId, job.id, job.version, 'cancel');
      setActiveJob(null);
      activeJobIdRef.current = null;
      setNotice('Import cancelled. No document was published and the upload staging was removed.');
      await refreshJobs();
    });
  }

  async function search() {
    if (!query.trim()) {
      setSearchResults(null);
      setSearchPages({});
      return;
    }
    await run(async () => {
      const result = await client.listMemories(workspaceId, {
        kind: 'document',
        query: query.trim(),
        // A plain list query searches titles only; searching imported
        // content needs the full-body FTS path.
        fullBody: true,
      });
      setSearchResults(result.memories);
      // The page of the first match is computed client-side from the
      // body: a PDF body is pages joined by a form feed, so the page
      // is the count of form feeds before the first match plus one.
      // TXT/Markdown bodies have no form feed, so they report page 1.
      const pages: Record<string, number> = {};
      const needle = query.trim().toLowerCase();
      for (const record of result.memories) {
        try {
          const details = await client.readMemory(workspaceId, record.id);
          const at = details.body.toLowerCase().indexOf(needle);
          if (at >= 0) {
            pages[record.id] = details.body.slice(0, at).split('\x0c').length;
          }
        } catch {
          // A result whose body cannot be read keeps no page label.
        }
      }
      setSearchPages(pages);
    });
  }

  const doc = current?.memory.document;
  return (
    <section className="card documents" aria-labelledby="documents-title" aria-busy={busy}>
      <h2 id="documents-title">Your documents</h2>
      <p>
        Import a local text, Markdown or text-layer PDF file as a document record. Imports are
        asynchronous, resumable and cancellable, run as persistent jobs, and stay in this
        workspace. Private is the default; Sealed import is refused until an encrypted vault
        path exists. Only .txt, .md and .pdf files are supported in this slice; a PDF's text
        is extracted on the server (scanned or image-only PDFs need OCR, a separate slice).
      </p>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {notice && (
        <p role="status" className="notice">
          {notice}
        </p>
      )}
      <div className="actions">
        <label className="file-label" htmlFor="document-file">
          Choose a .txt, .md or .pdf file
        </label>
        <input
          id="document-file"
          ref={fileRef}
          type="file"
          accept=".txt,.md,.pdf,text/plain,text/markdown,application/pdf"
          disabled={busy}
          onChange={event => void pick(event.target.files?.[0])}
        />
      </div>
      {pending && (
        <section aria-label="Review import">
          <h3>Review import</h3>
          <p>
            {pending.fileName} · {pending.rawBytes} bytes ·{' '}
            {pending.kind === 'pdf'
              ? 'PDF — text extracted on the server during import'
              : `${pending.textChars} characters`}
            {pending.bom ? ' · UTF-8 BOM (stripped and recorded)' : ''} · {pending.kind}
          </p>
          <p className="help">
            SHA-256 of the file as picked: {pending.rawHash}. The server re-verifies this hash
            before the import runs; a changed file refuses.
          </p>
          <label htmlFor="document-privacy">Privacy classification</label>
          <select
            id="document-privacy"
            value={privacy}
            disabled={busy}
            onChange={e => setPrivacy(e.target.value as Privacy)}
          >
            <option value="private">Private</option>
            <option value="standard">Standard</option>
            <option value="public">Public (not published)</option>
            <option value="sealed" disabled>
              Sealed — vault unavailable
            </option>
          </select>
          <label htmlFor="document-project">Project (optional)</label>
          <select
            id="document-project"
            value={projectId}
            disabled={busy}
            onChange={e => setProjectId(e.target.value)}
          >
            <option value="">No project</option>
            {projects.map(project => (
              <option key={project.id} value={project.id}>
                {project.title}
              </option>
            ))}
          </select>
          <div className="actions">
            <button disabled={busy} onClick={() => void startImport()}>
              Import document
            </button>
            <button
              disabled={busy}
              className="secondary"
              onClick={() => {
                setPending(null);
                setNotice('Import discarded before upload. Nothing was sent.');
              }}
            >
              Discard
            </button>
          </div>
        </section>
      )}
      {activeJob && (
        <section aria-label="Import progress">
          <h3>Import progress</h3>
          <p>
            {ingestionJobLabel(activeJob)} · State: <strong>{activeJob.state}</strong> ·{' '}
            {activeJob.state === 'Paused'
              ? `Uploaded ${received} of ${activeJob.selection.kind === 'ingestion' ? activeJob.selection.expectedBytes : 0} bytes`
              : `Progress: ${activeJob.progress}%`}
          </p>
          <p>
            Checkpoint {activeJob.checkpointVersion}: {activeJob.checkpoint}. {activeJob.log}
          </p>
          <div className="actions">
            {['Queued', 'Running', 'Paused'].includes(activeJob.state) && (
              <button disabled={busy} onClick={() => void cancel(activeJob)}>
                Cancel import
              </button>
            )}
            <button disabled={busy} className="secondary" onClick={() => void refreshJobs()}>
              Refresh import
            </button>
          </div>
        </section>
      )}
      {resumable.length > 0 && (
        <section aria-label="Interrupted uploads">
          <h3>Interrupted uploads</h3>
          <p>
            These imports stopped mid-upload (for example, the browser was closed). Pick the
            same file again above, then resume — completed parts are not re-sent.
          </p>
          <ul>
            {resumable.map(job => (
              <li key={job.id}>
                {ingestionJobLabel(job)} · uploaded {ingestionReceived(job)} of{' '}
                {job.selection.kind === 'ingestion' ? job.selection.expectedBytes : 0} bytes ·{' '}
                <button disabled={busy || !pending} onClick={() => void resume(job)}>
                  Resume upload
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
      <section aria-label="Search documents">
        <h3>Search documents</h3>
        <div className="actions">
          <input
            id="document-search"
            value={query}
            disabled={busy}
            onChange={e => setQuery(e.target.value)}
            placeholder="Search imported text"
            autoComplete="off"
          />
          <button disabled={busy} onClick={() => void search()}>
            Search documents
          </button>
          <button
            disabled={busy}
            className="secondary"
            onClick={() => {
              setQuery('');
              setSearchResults(null);
              setSearchPages({});
            }}
          >
            Clear
          </button>
        </div>
        {searchResults !== null && (
          <ul aria-label="Document search results">
            {searchResults.map(result => (
              <li key={result.id}>
                <button disabled={busy} className="secondary" onClick={() => void open(result.id)}>
                  {result.title}
                </button>
                <span>
                  {' '}
                  · {result.privacy}
                  {searchPages[result.id] !== undefined ? ` · Page ${searchPages[result.id]}` : ''}
                </span>
              </li>
            ))}
            {!searchResults.length && <li>No imported document matches that search.</li>}
          </ul>
        )}
      </section>
      <section aria-label="Imported documents">
        <h3>Imported documents</h3>
        <ul aria-label="Imported documents">
          {documents.map(record => (
            <li key={record.id}>
              <button disabled={busy} className="secondary" onClick={() => void open(record.id)}>
                {record.title}
              </button>
              <span>
                {' '}
                · {record.privacy} · Revision {record.version}
              </span>
            </li>
          ))}
        </ul>
        {!documents.length && <p>No imported documents yet.</p>}
        <div className="actions">
          <button disabled={busy || offset === 0} className="secondary" onClick={() => void refresh(Math.max(0, offset - 50))}>
            Previous documents
          </button>
          <button disabled={busy || next === null} className="secondary" onClick={() => void refresh(next!)}>
            Next documents
          </button>
        </div>
      </section>
      {current && (
        <section aria-label="Document reader">
          <h3>{current.memory.title}</h3>
          <p>
            {current.memory.privacy} · Revision {current.memory.version} · Provenance{' '}
            {current.memory.provenance}
          </p>
          {doc && (
            <details>
              <summary>Source provenance</summary>
              <p>
                Original file: {doc.fileName} · Media type: {doc.mediaType}
              </p>
              <p>
                File SHA-256: {doc.rawHash}
              </p>
              <p>
                Text SHA-256: {doc.textHash}
              </p>
              <p>
                {doc.rawBytes} bytes · {doc.textChars} characters
                {doc.bom ? ' · UTF-8 BOM recorded' : ' · No BOM'}
                {doc.pages ? ` · ${doc.pages.length} pages` : ''}
              </p>
              {doc.pages && (
                <p>
                  Pages:{' '}
                  {doc.pages
                    .map(entry => `Page ${entry.page}: ${entry.chars} characters`)
                    .join(' · ')}
                </p>
              )}
            </details>
          )}
          <div className="actions">
            {doc?.mediaType !== 'application/pdf' && (
              <button disabled={busy} className="secondary" onClick={() => setShowSource(value => !value)}>
                {showSource ? 'Show rendered text' : 'View source'}
              </button>
            )}
            <button disabled={busy} className="secondary" onClick={() => onNavigate('ask')}>
              Ask about this document
            </button>
            <button disabled={busy} className="secondary" onClick={() => onNavigate('jobs')}>
              Open jobs
            </button>
          </div>
          {doc?.mediaType === 'application/pdf' ? (
            <PdfReader body={current.body} page={page} onPage={setPage} />
          ) : showSource ? (
            <pre>{current.body}</pre>
          ) : (
            <div>{renderMarkdown(current.body)}</div>
          )}
        </section>
      )}
    </section>
  );
}
