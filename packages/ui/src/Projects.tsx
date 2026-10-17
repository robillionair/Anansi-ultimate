import { EntityHistory } from './EntityHistory';
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

export function Projects({ client, workspaceId }: { client: Client; workspaceId: string }) {
  const [projects, setProjects] = useState<MemoryMetadata[]>([]);
  const [current, setCurrent] = useState<MemoryDetails | null>(null);
  const [tasks, setTasks] = useState<MemoryMetadata[]>([]);
  const [relationships, setRelationships] = useState<RelationshipRecord[]>([]);
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
    const res = await client.listMemories(workspaceId, { kind: 'project' });
    setProjects(res.memories);
  }

  async function open(id: string) {
    const proj = await client.readMemory(workspaceId, id);
    setCurrent(proj);
    setDraft({
      title: proj.memory.title,
      body: proj.body,
      privacy: proj.memory.privacy,
      eventDate: proj.memory.eventDate,
      validFrom: proj.memory.validFrom,
      validUntil: proj.memory.validUntil,
      confidence: proj.memory.confidence,
      sourceReferences: proj.memory.sourceReferences,
      projectId: proj.memory.projectId,
      dueDate: proj.memory.dueDate,
      completed: proj.memory.completed,
    });

    // Load tasks for this project
    const taskList = await client.listMemories(workspaceId, { kind: 'task' });
    setTasks(taskList.memories.filter(t => t.projectId === id));

    // Load relationships for this project
    const relList = await client.listRelationships(workspaceId, { recordId: id });
    setRelationships(relList.relationships);
  }

  useEffect(() => {
    void run(refresh);
  }, [client, workspaceId]);

  return (
    <section className="card memory" aria-labelledby="projects-title" aria-busy={busy}>
      <h2 id="projects-title">Projects</h2>
      <p>Organize work into structured initiatives. Associate tasks, team members, and reference documents.</p>

      {error && <p role="alert" className="error">{error}</p>}
      {notice && <p role="status" className="notice">{notice}</p>}

      <div className="actions">
        <button
          disabled={busy}
          onClick={() => {
            setCurrent(null);
            setDraft(blank());
            setTasks([]);
            setRelationships([]);
            setNotice('New unsaved project.');
          }}
        >
          New project
        </button>
        <button disabled={busy} className="secondary" onClick={() => void run(refresh)}>
          Refresh projects
        </button>
      </div>

      <ul aria-label="Saved projects">
        {projects.map(p => (
          <li key={p.id}>
            <button disabled={busy} className="secondary" onClick={() => void run(() => open(p.id))}>
              {p.title}
            </button>
            <span> · Revision {p.version} · {p.status}</span>
          </li>
        ))}
      </ul>
      {!projects.length && <p>No saved projects yet. Create one above.</p>}

      <form
        onSubmit={e => {
          e.preventDefault();
          void run(async () => {
            const saved = current
              ? await client.reviseMemory(workspaceId, current.memory.id, current.memory.revisionId, draft)
              : await client.createMemory(workspaceId, draft, 'project');
            await refresh();
            await open(saved.memory.id);
            setNotice(`Project saved. Revision ${saved.memory.version}.`);
          });
        }}
      >
        <h3>{current ? `Edit project: ${current.memory.title}` : 'Create a project'}</h3>
        <fieldset disabled={busy}>
          <label htmlFor="project-title">Project title</label>
          <input
            id="project-title"
            value={draft.title}
            onChange={e => setDraft(d => ({ ...d, title: e.target.value }))}
            required
            autoComplete="off"
          />

          <label htmlFor="project-body">Description & objectives</label>
          <textarea
            id="project-body"
            value={draft.body}
            onChange={e => setDraft(d => ({ ...d, body: e.target.value }))}
            rows={5}
          />

          <button type="submit">Save project</button>
        </fieldset>
      </form>

      {current && <EntityHistory key={current.memory.id} client={client} workspaceId={workspaceId} current={current} refresh={() => open(current.memory.id)} />}
      {current && (
        <>
          <section aria-labelledby="project-tasks-heading">
            <h3 id="project-tasks-heading">Project tasks</h3>
            <ul aria-label="Tasks in this project">
              {tasks.map(t => (
                <li key={t.id}>
                  <strong>{t.title}</strong>
                  <span>{t.completed ? ' (Completed)' : ' (Pending)'}</span>
                  {t.dueDate && <span> · Due: {t.dueDate}</span>}
                </li>
              ))}
            </ul>
            {!tasks.length && <p>No tasks associated with this project yet.</p>}
          </section>

          <section aria-labelledby="project-relationships-heading">
            <h3 id="project-relationships-heading">Connected relationships</h3>
            <p className="help">Record ID: <code>{current.memory.id}</code></p>
            <ul aria-label="Relationships for this project">
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
                    Archive relationship
                  </button>
                </li>
              ))}
            </ul>
            {!relationships.length && <p>No relationships connected to this project.</p>}

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
                <option value="depends_on">depends_on</option>
                <option value="documents">documents</option>
                <option value="references">references</option>
              </select>
              <div className="actions">
                <button disabled={busy || !relTargetId} type="submit">
                  Add relationship
                </button>
              </div>
            </form>
          </section>
        </>
      )}
    </section>
  );
}
