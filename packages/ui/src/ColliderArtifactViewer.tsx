// M8 §27 artifact viewer: tabbed reading of the six
// derived documents for a SAVED experiment, with
// search/filter, per-artifact copy, and the
// deterministic export bundle. The viewer renders
// only for parsed saved experiments; a live,
// unapproved, or unparsable collision has no viewer,
// no export, and no Build action — those fail closed
// by absence. Only the active tab renders, so a large
// document never pays for the other five. Search
// shows matching lines with their section heading as
// plain text (never re-parsed markdown), so a filter
// cannot corrupt a code fence.
import { useMemo, useState } from 'react';
import type { ColliderExperimentDoc } from '@anansi/contracts';
import { deriveAllArtifacts } from './colliderArtifacts';
import { SafeMarkdown } from './colliderMarkdown';
import { buildExperimentExport } from './colliderExport';

function sectionFor(lines: string[], index: number): string {
  for (let i = index; i >= 0; i -= 1) {
    if (lines[i].startsWith('#')) return lines[i].replace(/^#+\s*/, '');
  }
  return '';
}

export function ColliderArtifactViewer({
  doc,
  body,
  artifactId,
  version,
  timestamp,
  notify,
}: {
  doc: ColliderExperimentDoc;
  body: string;
  artifactId: string;
  version: number;
  timestamp: string;
  notify: (message: string, isError: boolean) => void;
}) {
  const artifacts = useMemo(() => deriveAllArtifacts(doc), [doc]);
  const [active, setActive] = useState(artifacts[0]?.file ?? 'CONCEPT.md');
  const [query, setQuery] = useState('');
  const [copied, setCopied] = useState('');
  const [exporting, setExporting] = useState(false);
  const current = artifacts.find(a => a.file === active) ?? artifacts[0];
  const searchLines = useMemo(() => {
    if (!current || !query.trim()) return null;
    const needle = query.trim().toLowerCase();
    const lines = current.text.split('\n');
    const hits: Array<{ line: string; section: string }> = [];
    lines.forEach((line, idx) => {
      if (line.toLowerCase().includes(needle)) {
        hits.push({ line, section: sectionFor(lines, idx) });
      }
    });
    return hits;
  }, [current, query]);
  async function copy(text: string, file: string) {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(file);
    } catch {
      notify('Copy failed: clipboard unavailable. Select the text manually.', true);
    }
  }
  async function exportBundle() {
    setExporting(true);
    try {
      const built = await buildExperimentExport({
        doc,
        body,
        artifactId,
        version,
        timestamp,
      });
      const url = URL.createObjectURL(new Blob([built.bytes as BlobPart], { type: 'application/zip' }));
      const link = document.createElement('a');
      link.href = url;
      link.download = built.filename;
      link.click();
      setTimeout(() => URL.revokeObjectURL(url), 60000);
      notify('Full experiment exported. The bundle manifest lists every file with its SHA-256 hash.', false);
    } catch (err) {
      notify(err instanceof Error ? err.message : 'Export unavailable. Retry.', true);
    } finally {
      setExporting(false);
    }
  }
  if (!current) return null;
  return (
    <section aria-label="Experiment artifacts">
      <h3>Experiment artifacts</h3>
      <p className="help">
        Six documents derived deterministically from this saved experiment version
        ({artifactId}@v{version}). They organize the record&apos;s own fields and
        add no new claims; anything the record does not state is marked as such.
        {doc.artifacts
          ? ' Their SHA-256 hashes are recorded in the saved experiment.'
          : ' This experiment predates recorded artifact hashes; its documents are derived live from the record.'}
      </p>
      <div role="tablist" aria-label="Artifact documents">
        {artifacts.map(a => (
          <button
            key={a.file}
            type="button"
            role="tab"
            aria-selected={a.file === active}
            onClick={() => {
              setActive(a.file);
              setQuery('');
            }}
          >
            {a.file}
          </button>
        ))}
      </div>
      <div>
        <label>
          Search this document{' '}
          <input
            type="search"
            value={query}
            onChange={e => setQuery(e.target.value)}
            aria-label={`Search ${current.file}`}
          />
        </label>{' '}
        <button type="button" onClick={() => void copy(current.text, current.file)}>
          {copied === current.file ? 'Copied' : `Copy ${current.file}`}
        </button>{' '}
        <button type="button" disabled={exporting} onClick={() => void exportBundle()}>
          {exporting ? 'Exporting…' : 'Export Full Experiment'}
        </button>
      </div>
      {searchLines ? (
        <div>
          <p className="help">
            {searchLines.length} matching line{searchLines.length === 1 ? '' : 's'} in {current.file}.
          </p>
          <ul>
            {searchLines.map((hit, i) => (
              <li key={i}>
                {hit.section ? <strong>{hit.section}: </strong> : null}
                {hit.line}
              </li>
            ))}
          </ul>
        </div>
      ) : (
        <SafeMarkdown text={current.text} />
      )}
    </section>
  );
}
