import { EntityHistory } from './EntityHistory';
import { useEffect, useState } from 'react';
import type { Client, MemoryDetails, MemoryDraft, MemoryMetadata, RelationshipRecord } from '@anansi/contracts';

const SUPPORTED_KINDS = [
  'memory',
  'note',
  'project',
  'task',
  'person',
  'organization',
  'decision',
  'event',
  'document',
] as const;

type SupportedKind = (typeof SUPPORTED_KINDS)[number];

const blank = (): MemoryDraft => ({
  title: '',
  body: '',
  privacy: 'private',
  confidence: null,
  eventDate: null,
  validFrom: null,
  validUntil: null,
  sourceReferences: [],
  projectId: null,
  dueDate: null,
  completed: false,
});

export function KnowledgeRecords({ client, workspaceId }: { client: Client; workspaceId: string }) {
  const [records, setRecords] = useState<MemoryMetadata[]>([]);
  const [current, setCurrent] = useState<MemoryDetails | null>(null);
  const [relationships, setRelationships] = useState<RelationshipRecord[]>([]);
  const [selectedKind, setSelectedKind] = useState<string>('all');
  const [draftKind, setDraftKind] = useState<SupportedKind>('memory');
  const [draft, setDraft] = useState<MemoryDraft>(blank);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');

  // Relationship creation state
  const [relTargetId, setRelTargetId] = useState('');
  const [relPredicate, setRelPredicate] = useState('relates_to');

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
    const options = selectedKind === 'all' ? {} : { kind: selectedKind };
    const res = await client.listMemories(workspaceId, options);
    setRecords(res.memories);
  }

  async function open(id: string) {
    const r = await client.readMemory(workspaceId, id);
    setCurrent(r);
    setDraftKind((r.memory.kind as SupportedKind) || 'memory');
    setDraft({
      title: r.memory.title,
      body: r.body,
      privacy: r.memory.privacy,
      eventDate: r.memory.eventDate,
      validFrom: r.memory.validFrom,
      validUntil: r.memory.validUntil,
      confidence: r.memory.confidence,
      sourceReferences: r.memory.sourceReferences,
      projectId: r.memory.projectId ?? null,
      dueDate: r.memory.dueDate ?? null,
      completed: r.memory.completed ?? false,
    });

    const relList = await client.listRelationships(workspaceId, { recordId: id });
    setRelationships(relList.relationships);
  }

  useEffect(() => {
    void run(refresh);
  }, [client, workspaceId, selectedKind]);

  return (
    <section className="card memory" aria-labelledby="records-title" aria-busy={busy}>
      <h2 id="records-title">Knowledge Records</h2>
      <p>Browse and link knowledge entities across all supported kinds: decisions, events, documents, organizations, and memories.</p>

      {error && <p role="alert" className="error">{error}</p>}
      {notice && <p role="status" className="notice">{notice}</p>}

      <div style={{ display: 'flex', gap: '12px', alignItems: 'center', marginTop: '16px' }}>
        <label htmlFor="kind-filter" style={{ margin: 0 }}>Filter by kind:</label>
        <select
          id="kind-filter"
          value={selectedKind}
          onChange={e => setSelectedKind(e.target.value)}
          style={{ width: 'auto', minWidth: '160px' }}
        >
          <option value="all">All kinds</option>
          {SUPPORTED_KINDS.map(k => (
            <option key={k} value={k}>
              {k}
            </option>
          ))}
        </select>

        <button
          disabled={busy}
          onClick={() => {
            setCurrent(null);
            setDraft(blank());
            setRelationships([]);
            setNotice('New unsaved record.');
          }}
        >
          New record
        </button>
        <button disabled={busy} className="secondary" onClick={() => void run(refresh)}>
          Refresh
        </button>
      </div>

      <ul aria-label="Knowledge records list" style={{ marginTop: '16px' }}>
        {records.map(r => (
          <li key={r.id}>
            <button disabled={busy} className="secondary" onClick={() => void run(() => open(r.id))}>
              {r.title}
            </button>
            <span style={{ textTransform: 'uppercase', fontSize: '12px', fontWeight: 600, marginLeft: '6px' }}>
              [{r.kind}]
            </span>
            <span> · Rev {r.version} · {r.status}</span>
            {r.eventDate && <span> · Event: {r.eventDate}</span>}
          </li>
        ))}
      </ul>
      {!records.length && <p>No records found for the selected filter.</p>}

      <form
        onSubmit={e => {
          e.preventDefault();
          void run(async () => {
            const saved = current
              ? await client.reviseMemory(workspaceId, current.memory.id, current.memory.revisionId, draft)
              : await client.createMemory(workspaceId, draft, draftKind);
            await refresh();
            await open(saved.memory.id);
            setNotice(`Record saved (${saved.memory.kind}). Revision ${saved.memory.version}.`);
          });
        }}
      >
        <h3>{current ? `Edit record: ${current.memory.title}` : 'Create a knowledge record'}</h3>
        <fieldset disabled={busy}>
          {!current && (
            <>
              <label htmlFor="create-kind">Record kind</label>
              <select
                id="create-kind"
                value={draftKind}
                onChange={e => setDraftKind(e.target.value as SupportedKind)}
              >
                {SUPPORTED_KINDS.map(k => (
                  <option key={k} value={k}>
                    {k}
                  </option>
                ))}
              </select>
            </>
          )}

          <label htmlFor="record-title">Title</label>
          <input
            id="record-title"
            value={draft.title}
            onChange={e => setDraft(d => ({ ...d, title: e.target.value }))}
            required
            autoComplete="off"
          />

          {(draftKind === 'event' || draftKind === 'decision') && (
            <>
              <label htmlFor="record-event-date">Event date (UTC ISO timestamp)</label>
              <input
                id="record-event-date"
                placeholder="2026-10-17T12:00:00Z"
                value={draft.eventDate ?? ''}
                onChange={e => setDraft(d => ({ ...d, eventDate: e.target.value || null }))}
              />
            </>
          )}

          <label htmlFor="record-body">Content & description</label>
          <textarea
            id="record-body"
            value={draft.body}
            onChange={e => setDraft(d => ({ ...d, body: e.target.value }))}
            rows={5}
          />

          <button type="submit">Save record</button>
        </fieldset>
      </form>

      {current && <EntityHistory key={current.memory.id} client={client} workspaceId={workspaceId} current={current} refresh={() => open(current.memory.id)} />}
      {current && (
        <section aria-labelledby="record-relationships-heading">
          <h3 id="record-relationships-heading">Connected relationships</h3>
          <p className="help">Record ID: <code>{current.memory.id}</code></p>
          <ul aria-label="Relationships for this record">
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
                      await client.deleteRelationship(workspaceId, r.id, r.version);
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
          {!relationships.length && <p>No relationships connected to this record.</p>}

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
              <option value="relates_to">relates_to</option>
              <option value="belongs_to_project">belongs_to_project</option>
              <option value="assigned_to">assigned_to</option>
              <option value="decided_by">decided_by</option>
              <option value="documents">documents</option>
              <option value="depends_on">depends_on</option>
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
