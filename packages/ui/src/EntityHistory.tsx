import { useState } from 'react';
import type { Client, MemoryDetails, MemoryMetadata, RelationshipRecord } from '@anansi/contracts';

export function EntityHistory({ client, workspaceId, current, refresh }: {
  client: Client; workspaceId: string; current: MemoryDetails; refresh: () => Promise<void>;
}) {
  const [records, setRecords] = useState<MemoryMetadata[]>([]);
  const [edges, setEdges] = useState<RelationshipRecord[]>([]);
  const [selected, setSelected] = useState<MemoryDetails | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  async function run(action: () => Promise<void>) {
    setBusy(true); setError('');
    try { await action(); } catch (e) { setError(e instanceof Error ? e.message : 'History unavailable.'); }
    finally { setBusy(false); }
  }
  async function load() {
    setRecords((await client.listMemories(workspaceId, { id: current.memory.id, limit: 100 })).memories);
    setEdges((await client.listRelationships(workspaceId, { recordId: current.memory.id, includeHistory: true })).relationships);
  }
  return <section aria-label="Record and relationship history">
    <h3>History</h3>
    <button disabled={busy} onClick={() => void run(load)}>Load history</button>
    <p>Record revisions preserve the links saved with that revision. Relationship history records separate link actions. Up to 100 record revisions and 200 link revisions are shown.</p>
    {error && <p role="alert">{error}</p>}
    <ul aria-label="Record revisions">{records.map(r => <li key={r.revisionId}>
      <button disabled={busy} onClick={() => void run(async () => {
        setSelected(await client.readMemory(workspaceId, r.id, r.revisionId));
      })}>View revision {r.version}</button>
      {' '}{r.status} · {r.provenance}
    </li>)}</ul>
    {selected && <section aria-label="Historical record">
      <h4>{selected.memory.title} · Revision {selected.memory.version}</h4>
      <pre>{selected.body}</pre>
      <p>Saved relationship IDs: {selected.memory.relationships.join(', ') || 'None'}</p>
    </section>}
    <ul aria-label="Relationship revisions">{edges.map(r => <li key={`${r.id}:${r.version}`}>
      {r.predicate} · Version {r.version} · {r.archived ? 'Archived' : 'Active'} · {r.provenance}
      {r.archived && !edges.some(e => e.id === r.id && e.version > r.version) &&
        <button disabled={busy} onClick={() => void run(async () => {
          await client.restoreRelationship(workspaceId, r.id, r.version); await refresh(); await load();
        })}>Restore relationship</button>}
    </li>)}</ul>
  </section>;
}
