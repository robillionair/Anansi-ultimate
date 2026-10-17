import { useEffect, useState } from 'react';
import type { Client, MemoryDetails, MemoryDraft, MemoryMetadata, RelationshipRecord } from '@anansi/contracts';

const blank = (): MemoryDraft => ({
  title: '',
  body: '',
  privacy: 'private',
  confidence: null,
  eventDate: null,
  validFrom: null,
  validUntil: null,
  sourceReferences: [],
});

export function People({ client, workspaceId }: { client: Client; workspaceId: string }) {
  const [people, setPeople] = useState<MemoryMetadata[]>([]);
  const [current, setCurrent] = useState<MemoryDetails | null>(null);
  const [relationships, setRelationships] = useState<RelationshipRecord[]>([]);
  const [draft, setDraft] = useState<MemoryDraft>(blank);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');

  // Relationship creation state
  const [relTargetId, setRelTargetId] = useState('');
  const [relPredicate, setRelPredicate] = useState('assigned_to');

  async function run(action: () => Promise<void>) {
    setBusy(true);
    setError('');
    setNotice('');
    try {
      await action();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Operation failed. Your draft was kept.');
    } finally {
      setBusy(false);
    }
  }

  async function refresh() {
    const res = await client.listMemories(workspaceId, { kind: 'person' });
    setPeople(res.memories);
  }

  async function open(id: string) {
    const p = await client.readMemory(workspaceId, id);
    setCurrent(p);
    setDraft({
      title: p.memory.title,
      body: p.body,
      privacy: p.memory.privacy,
      eventDate: p.memory.eventDate,
      validFrom: p.memory.validFrom,
      validUntil: p.memory.validUntil,
      confidence: p.memory.confidence,
      sourceReferences: p.memory.sourceReferences,
    });

    const relList = await client.listRelationships(workspaceId, { recordId: id });
    setRelationships(relList.relationships);
  }

  useEffect(() => {
    void run(refresh);
  }, [client, workspaceId]);

  return (
    <section className="card memory" aria-labelledby="people-title" aria-busy={busy}>
      <h2 id="people-title">People</h2>
      <p>Maintain contact records, roles, responsibilities, and team associations.</p>

      {error && <p role="alert" className="error">{error}</p>}
      {notice && <p role="status" className="notice">{notice}</p>}

      <div className="actions">
        <button
          disabled={busy}
          onClick={() => {
            setCurrent(null);
            setDraft(blank());
            setRelationships([]);
            setNotice('New unsaved person.');
          }}
        >
          New person
        </button>
        <button disabled={busy} className="secondary" onClick={() => void run(refresh)}>
          Refresh people
        </button>
      </div>

      <ul aria-label="Saved people">
        {people.map(p => (
          <li key={p.id}>
            <button disabled={busy} className="secondary" onClick={() => void run(() => open(p.id))}>
              {p.title}
            </button>
            <span> · Revision {p.version} · {p.status}</span>
          </li>
        ))}
      </ul>
      {!people.length && <p>No saved people records yet.</p>}

      <form
        onSubmit={e => {
          e.preventDefault();
          void run(async () => {
            const saved = current
              ? await client.reviseMemory(workspaceId, current.memory.id, current.memory.revisionId, draft)
              : await client.createMemory(workspaceId, draft, 'person');
            await refresh();
            await open(saved.memory.id);
            setNotice(`Person saved. Revision ${saved.memory.version}.`);
          });
        }}
      >
        <h3>{current ? `Edit person: ${current.memory.title}` : 'Add a person'}</h3>
        <fieldset disabled={busy}>
          <label htmlFor="person-name">Full name</label>
          <input
            id="person-name"
            value={draft.title}
            onChange={e => setDraft(d => ({ ...d, title: e.target.value }))}
            required
            autoComplete="off"
          />

          <label htmlFor="person-notes">Role, notes & bio</label>
          <textarea
            id="person-notes"
            value={draft.body}
            onChange={e => setDraft(d => ({ ...d, body: e.target.value }))}
            rows={4}
          />

          <button type="submit">Save person</button>
        </fieldset>
      </form>

      {current && (
        <section aria-labelledby="person-relationships-heading">
          <h3 id="person-relationships-heading">Connected relationships</h3>
          <p className="help">Record ID: <code>{current.memory.id}</code></p>
          <ul aria-label="Relationships for this person">
            {relationships.map(r => (
              <li key={r.id}>
                <span>
                  {r.sourceId === current.memory.id ? 'Self' : r.sourceId} &rarr; <strong>{r.predicate}</strong> &rarr; {r.targetId === current.memory.id ? 'Self' : r.targetId}
                </span>
                <button
                  disabled={busy}
                  className="secondary"
                  style={{ marginLeft: '12px' }}
                  onClick={() =>
                    void run(async () => {
                      await client.deleteRelationship(workspaceId, r.id);
                      await open(current.memory.id);
                      setNotice('Relationship removed.');
                    })
                  }
                >
                  Delete
                </button>
              </li>
            ))}
          </ul>
          {!relationships.length && <p>No relationships connected to this person.</p>}

          <form
            onSubmit={e => {
              e.preventDefault();
              void run(async () => {
                await client.createRelationship(workspaceId, {
                  sourceId: current.memory.id,
                  targetId: relTargetId,
                  predicate: relPredicate,
                });
                setRelTargetId('');
                await open(current.memory.id);
                setNotice('Relationship created.');
              });
            }}
          >
            <h4>Link another record</h4>
            <label htmlFor="target-record-id">Target record ID</label>
            <input
              id="target-record-id"
              value={relTargetId}
              onChange={e => setRelTargetId(e.target.value)}
              placeholder="Target memory or entity ID"
              required
            />
            <label htmlFor="rel-predicate">Predicate</label>
            <select
              id="rel-predicate"
              value={relPredicate}
              onChange={e => setRelPredicate(e.target.value)}
            >
              <option value="assigned_to">assigned_to</option>
              <option value="decided_by">decided_by</option>
              <option value="relates_to">relates_to</option>
              <option value="references">references</option>
            </select>
            <div className="actions">
              <button disabled={busy || !relTargetId} type="submit">
                Add relationship
              </button>
            </div>
          </form>
        </section>
      )}
    </section>
  );
}
