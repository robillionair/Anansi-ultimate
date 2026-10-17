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
  projectId: null,
  dueDate: null,
  completed: false,
});

export function Tasks({ client, workspaceId }: { client: Client; workspaceId: string }) {
  const [tasks, setTasks] = useState<MemoryMetadata[]>([]);
  const [projects, setProjects] = useState<MemoryMetadata[]>([]);
  const [current, setCurrent] = useState<MemoryDetails | null>(null);
  const [relationships, setRelationships] = useState<RelationshipRecord[]>([]);
  const [draft, setDraft] = useState<MemoryDraft>(blank);
  const [filter, setFilter] = useState<'all' | 'active' | 'completed'>('all');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');

  // Relationship creation state
  const [relTargetId, setRelTargetId] = useState('');
  const [relPredicate, setRelPredicate] = useState('depends_on');

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
    const taskRes = await client.listMemories(workspaceId, { kind: 'task' });
    setTasks(taskRes.memories);
    const projRes = await client.listMemories(workspaceId, { kind: 'project' });
    setProjects(projRes.memories);
  }

  async function open(id: string) {
    const t = await client.readMemory(workspaceId, id);
    setCurrent(t);
    setDraft({
      title: t.memory.title,
      body: t.body,
      privacy: t.memory.privacy,
      eventDate: t.memory.eventDate,
      validFrom: t.memory.validFrom,
      validUntil: t.memory.validUntil,
      confidence: t.memory.confidence,
      sourceReferences: t.memory.sourceReferences,
      projectId: t.memory.projectId ?? null,
      dueDate: t.memory.dueDate ?? null,
      completed: t.memory.completed ?? false,
    });

    const relList = await client.listRelationships(workspaceId, { recordId: id });
    setRelationships(relList.relationships);
  }

  async function toggleComplete(task: MemoryMetadata) {
    const full = await client.readMemory(workspaceId, task.id);
    const updatedDraft: MemoryDraft = {
      title: full.memory.title,
      body: full.body,
      privacy: full.memory.privacy,
      eventDate: full.memory.eventDate,
      validFrom: full.memory.validFrom,
      validUntil: full.memory.validUntil,
      confidence: full.memory.confidence,
      sourceReferences: full.memory.sourceReferences,
      projectId: full.memory.projectId,
      dueDate: full.memory.dueDate,
      completed: !task.completed,
    };
    await client.reviseMemory(workspaceId, task.id, full.memory.revisionId, updatedDraft);
    await refresh();
    if (current?.memory.id === task.id) {
      await open(task.id);
    }
    setNotice(`Task marked ${!task.completed ? 'completed' : 'incomplete'}.`);
  }

  useEffect(() => {
    void run(refresh);
  }, [client, workspaceId]);

  const displayedTasks = tasks.filter(t => {
    if (filter === 'active') return !t.completed;
    if (filter === 'completed') return !!t.completed;
    return true;
  });

  return (
    <section className="card memory" aria-labelledby="tasks-title" aria-busy={busy}>
      <h2 id="tasks-title">Tasks</h2>
      <p>Manage actionable items, due dates, project assignments, and completion states.</p>

      {error && <p role="alert" className="error">{error}</p>}
      {notice && <p role="status" className="notice">{notice}</p>}

      <div className="actions">
        <button
          disabled={busy}
          onClick={() => {
            setCurrent(null);
            setDraft(blank());
            setRelationships([]);
            setNotice('New unsaved task.');
          }}
        >
          New task
        </button>
        <button disabled={busy} className="secondary" onClick={() => void run(refresh)}>
          Refresh tasks
        </button>
      </div>

      <div style={{ marginTop: '16px', display: 'flex', gap: '8px', alignItems: 'center' }}>
        <span>Filter: </span>
        <button
          className={filter === 'all' ? '' : 'secondary'}
          disabled={busy}
          onClick={() => setFilter('all')}
        >
          All ({tasks.length})
        </button>
        <button
          className={filter === 'active' ? '' : 'secondary'}
          disabled={busy}
          onClick={() => setFilter('active')}
        >
          Pending ({tasks.filter(t => !t.completed).length})
        </button>
        <button
          className={filter === 'completed' ? '' : 'secondary'}
          disabled={busy}
          onClick={() => setFilter('completed')}
        >
          Completed ({tasks.filter(t => t.completed).length})
        </button>
      </div>

      <ul aria-label="Tasks list" style={{ marginTop: '16px' }}>
        {displayedTasks.map(t => {
          const proj = projects.find(p => p.id === t.projectId);
          return (
            <li key={t.id} style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <input
                type="checkbox"
                checked={!!t.completed}
                aria-label={`Mark "${t.title}" as ${t.completed ? 'incomplete' : 'complete'}`}
                onChange={() => void run(() => toggleComplete(t))}
                disabled={busy}
                style={{ width: '18px', height: '18px', cursor: 'pointer' }}
              />
              <button
                disabled={busy}
                className="secondary"
                style={{ textDecoration: t.completed ? 'line-through' : 'none' }}
                onClick={() => void run(() => open(t.id))}
              >
                {t.title}
              </button>
              {proj && <span> · Project: <em>{proj.title}</em></span>}
              {t.dueDate && <span> · Due: {t.dueDate}</span>}
            </li>
          );
        })}
      </ul>
      {!displayedTasks.length && <p>No tasks found for this filter.</p>}

      <form
        onSubmit={e => {
          e.preventDefault();
          void run(async () => {
            const saved = current
              ? await client.reviseMemory(workspaceId, current.memory.id, current.memory.revisionId, draft)
              : await client.createMemory(workspaceId, draft, 'task');
            await refresh();
            await open(saved.memory.id);
            setNotice(`Task saved. Revision ${saved.memory.version}.`);
          });
        }}
      >
        <h3>{current ? `Edit task: ${current.memory.title}` : 'Create a task'}</h3>
        <fieldset disabled={busy}>
          <label htmlFor="task-title">Task title</label>
          <input
            id="task-title"
            value={draft.title}
            onChange={e => setDraft(d => ({ ...d, title: e.target.value }))}
            required
            autoComplete="off"
          />

          <label htmlFor="task-project">Associated project (optional)</label>
          <select
            id="task-project"
            value={draft.projectId ?? ''}
            onChange={e => setDraft(d => ({ ...d, projectId: e.target.value || null }))}
          >
            <option value="">No project</option>
            {projects.map(p => (
              <option key={p.id} value={p.id}>
                {p.title}
              </option>
            ))}
          </select>

          <label htmlFor="task-due-date">Due date (UTC ISO string)</label>
          <input
            id="task-due-date"
            placeholder="2026-12-31T23:59:59Z"
            value={draft.dueDate ?? ''}
            onChange={e => setDraft(d => ({ ...d, dueDate: e.target.value || null }))}
          />

          <label className="check-label">
            <input
              type="checkbox"
              checked={!!draft.completed}
              onChange={e => setDraft(d => ({ ...d, completed: e.target.checked }))}
            />
            Mark as completed
          </label>

          <label htmlFor="task-body">Details & notes</label>
          <textarea
            id="task-body"
            value={draft.body}
            onChange={e => setDraft(d => ({ ...d, body: e.target.value }))}
            rows={4}
          />

          <button type="submit">Save task</button>
        </fieldset>
      </form>

      {current && (
        <section aria-labelledby="task-relationships-heading">
          <h3 id="task-relationships-heading">Connected relationships</h3>
          <p className="help">Record ID: <code>{current.memory.id}</code></p>
          <ul aria-label="Relationships for this task">
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
          {!relationships.length && <p>No relationships connected to this task.</p>}

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
              <option value="depends_on">depends_on</option>
              <option value="belongs_to_project">belongs_to_project</option>
              <option value="assigned_to">assigned_to</option>
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
