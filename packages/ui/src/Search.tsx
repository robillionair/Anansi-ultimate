import { useState } from 'react';
import type { Client, HybridSearchResult, MemoryDetails, MemoryMetadata } from '@anansi/contracts';

export type SearchMode = 'title' | 'body' | 'hybrid';

export function Search({ client, workspaceId }: { client: Client; workspaceId: string }) {
  const [mode, setMode] = useState<SearchMode>('title');
  const [query, setQuery] = useState(''), [kind, setKind] = useState('');
  const [archived, setArchived] = useState(false), [expired, setExpired] = useState(false);
  const [results, setResults] = useState<MemoryMetadata[]>([]);
  const [hybridResults, setHybridResults] = useState<HybridSearchResult[]>([]);
  const [current, setCurrent] = useState<MemoryDetails | null>(null);
  const [busy, setBusy] = useState(false), [error, setError] = useState(''), [searched, setSearched] = useState(false);
  const [offset, setOffset] = useState(0), [next, setNext] = useState<number | null>(null);

  function clear() {
    setResults([]);
    setHybridResults([]);
    setCurrent(null);
    setError('');
    setSearched(false);
    setOffset(0);
    setNext(null);
  }

  async function search(at = 0) {
    setBusy(true);
    setError('');
    setCurrent(null);
    setResults([]);
    setHybridResults([]);
    setSearched(false);
    setNext(null);

    try {
      if (mode === 'hybrid') {
        const response = await client.hybridSearch(workspaceId, {
          query: query.trim() || undefined,
          kind: kind || undefined,
          includeArchived: archived,
          limit: 50,
        });
        setHybridResults(response.results);
        setSearched(true);
      } else {
        const isFullBody = mode === 'body';
        const result = await client.listMemories(workspaceId, {
          query,
          kind: kind || undefined,
          includeArchived: archived,
          includeExpired: expired,
          fullBody: isFullBody,
          offset: at,
        });
        setResults(result.memories);
        setOffset(at);
        setNext(result.nextOffset ?? null);
        setSearched(true);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Search unavailable. Retry.');
    } finally {
      setBusy(false);
    }
  }

  async function open(memoryId: string, revisionId?: string) {
    setBusy(true);
    setError('');
    setCurrent(null);
    try {
      setCurrent(await client.readMemory(workspaceId, memoryId, revisionId));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Record unavailable. Search again.');
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="card memory" aria-labelledby="search-title" aria-busy={busy}>
      <h2 id="search-title">
        {mode === 'title' ? 'Search saved titles' : mode === 'body' ? 'Search saved knowledge' : 'Search with hybrid semantic intelligence'}
      </h2>
      <p>
        {mode === 'title'
          ? 'Search record titles in this workspace. All words must match. Bodies and semantic search are not included. Nothing is sent to an AI provider.'
          : mode === 'body'
          ? 'Search titles and Markdown bodies in this workspace using keywords, exact phrases in quotes and prefix wildcards. Sealed records are excluded. Nothing is sent to an AI provider.'
          : 'Fuses lexical FTS5 BM25 keyword matching and pure Rust vector similarity using Reciprocal Rank Fusion (RRF). Sealed records are excluded fail-closed. Nothing is sent to an AI provider.'}
      </p>

      <div className="search-mode-selector" role="radiogroup" aria-label="Search mode">
        <label className="radio-label">
          <input
            type="radio"
            name="search-mode"
            value="title"
            checked={mode === 'title'}
            onChange={() => { clear(); setMode('title'); }}
          />
          Title only
        </label>
        <label className="radio-label">
          <input
            type="radio"
            name="search-mode"
            value="body"
            checked={mode === 'body'}
            onChange={() => { clear(); setMode('body'); }}
          />
          Full-body search (includes Markdown text)
        </label>
        <label className="radio-label">
          <input
            type="radio"
            name="search-mode"
            value="hybrid"
            checked={mode === 'hybrid'}
            onChange={() => { clear(); setMode('hybrid'); }}
          />
          Hybrid semantic
        </label>
      </div>

      <form onSubmit={(e) => { e.preventDefault(); void search(); }}>
        <fieldset disabled={busy}>
          <label htmlFor="search-query">{mode === 'title' ? 'Title keywords' : 'Search query'}</label>
          <input
            id="search-query"
            value={query}
            onChange={(e) => { clear(); setQuery(e.target.value); }}
            maxLength={mode === 'title' ? 256 : 512}
            required
            autoComplete="off"
          />
          <label htmlFor="search-kind">Record type</label>
          <select id="search-kind" value={kind} onChange={(e) => { clear(); setKind(e.target.value); }}>
            <option value="">All record types</option>
            {['memory', 'note', 'project', 'task', 'person', 'organization', 'decision', 'event', 'document'].map((k) => (
              <option key={k} value={k}>{k}</option>
            ))}
          </select>
          <label className="check-label">
            <input type="checkbox" checked={archived} onChange={(e) => { clear(); setArchived(e.target.checked); }} />
            Include archived search results
          </label>
          {mode !== 'hybrid' && (
            <label className="check-label">
              <input type="checkbox" checked={expired} onChange={(e) => { clear(); setExpired(e.target.checked); }} />
              Include expired search results
            </label>
          )}
          <button type="submit">
            {mode === 'title' ? 'Search titles' : mode === 'body' ? 'Search knowledge' : 'Run hybrid search'}
          </button>
        </fieldset>
      </form>

      <p className="help">
        {mode === 'title'
          ? 'Up to eight words, 256 UTF-8 bytes. Searches are limited to 10,000 eligible records and 32 MiB of metadata; narrow the record type if needed. Sealed records remain unavailable.'
          : mode === 'body'
          ? 'Searches full Markdown bodies and titles up to 16 terms or quoted phrases, bounded to 512 UTF-8 bytes and 10,000 eligible records. Sealed records remain unavailable.'
          : 'Reciprocal Rank Fusion fuses keyword rank and cosine vector rank. Bounded to top 100 eligible records. Sealed records remain unavailable.'}
      </p>

      {busy && <p role="status">Searching or opening a saved revision…</p>}
      {error && <p role="alert" className="error">{error}</p>}

      {searched && mode !== 'hybrid' && (
        <>
          <p role="status">
            {results.length
              ? `${results.length} ${mode === 'body' ? 'matches' : 'title matches'} on this page.`
              : mode === 'body'
              ? 'No matching records. Try different words or filters.'
              : 'No matching titles. Try different words or filters.'}
          </p>
          <ul aria-label={mode === 'body' ? 'Search results' : 'Title search results'}>
            {results.map((record) => (
              <li key={record.id}>
                <button className="secondary" disabled={busy} onClick={() => void open(record.id, record.revisionId)}>
                  {record.title}
                </button>
                <p>
                  {record.kind} · {record.status} · {record.privacy} · Revision {record.version}
                  {record.duplicateOf ? ` · (Duplicate of ${record.duplicateOf})` : ''}
                </p>
                {record.sectionHeading && (
                  <p className="evidence-heading">
                    Section: <strong>{record.sectionHeading}</strong>{record.lineNumber ? ` (line ${record.lineNumber})` : ''}
                  </p>
                )}
                {record.snippet && <blockquote className="search-snippet">{record.snippet}</blockquote>}
              </li>
            ))}
          </ul>
          <div className="actions">
            <button disabled={busy || offset === 0} onClick={() => void search(Math.max(0, offset - 50))}>
              Previous search results
            </button>
            <button disabled={busy || next === null} onClick={() => void search(next!)}>
              Next search results
            </button>
          </div>
        </>
      )}

      {searched && mode === 'hybrid' && (
        <>
          <p role="status">
            {hybridResults.length
              ? `${hybridResults.length} hybrid fused matches found.`
              : 'No matching records found. Try different terms or filters.'}
          </p>
          <ul aria-label="Hybrid search results" className="hybrid-search-results">
            {hybridResults.map((record) => (
              <li key={record.memoryId} className="hybrid-result-item">
                <button className="secondary" disabled={busy} onClick={() => void open(record.memoryId)}>
                  {record.title}
                </button>
                <p className="hybrid-meta">
                  {record.kind} · RRF Score: <strong>{record.rrfScore.toFixed(4)}</strong>
                  {record.lexicalRank !== null && record.lexicalRank !== undefined ? ` · Lexical #${record.lexicalRank}` : ''}
                  {record.semanticRank !== null && record.semanticRank !== undefined ? ` · Semantic #${record.semanticRank}` : ''}
                </p>
                {record.sectionHeading && (
                  <p className="evidence-heading">
                    Section: <strong>{record.sectionHeading}</strong>{record.lineNumber ? ` (line ${record.lineNumber})` : ''}
                  </p>
                )}
                {record.snippet && <blockquote className="search-snippet">{record.snippet}</blockquote>}
              </li>
            ))}
          </ul>
        </>
      )}

      {current && (
        <section aria-label="Search result revision">
          <h3>{current.memory.title}</h3>
          <p>
            Revision {current.memory.version} · {current.memory.provenance} · {current.memory.validationStatus}
          </p>
          <p>Sources: {current.memory.sourceReferences.join(', ') || 'none recorded'}</p>
          <p>
            Record: <code>{current.memory.id}</code> · Revision: <code>{current.memory.revisionId}</code>
          </p>
          <pre>{current.body}</pre>
        </section>
      )}
    </section>
  );
}
