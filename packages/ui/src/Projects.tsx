import { EntityHistory } from './EntityHistory';
import { useEffect, useState } from 'react';
import type { ArtifactDetails, ArtifactRecord, Client, MemoryDetails, MemoryDraft, MemoryMetadata, RelationshipRecord } from '@anansi/contracts';

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

  // Artifact registry state (M6 Slice 1)
  const [artifacts, setArtifacts] = useState<ArtifactRecord[]>([]);
  const [artifactVersions, setArtifactVersions] = useState<ArtifactRecord[]>([]);
  const [selectedArtifact, setSelectedArtifact] = useState<string>('');
  const [viewedArtifact, setViewedArtifact] = useState<ArtifactDetails | null>(null);
  const [artifactForm, setArtifactForm] = useState({ type: 'note', title: '', body: '', privacy: 'private', sources: '' });
  const [reviseForm, setReviseForm] = useState({ title: '', body: '', sources: '' });

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

    // Load this project's artifacts (latest versions)
    const artList = await client.listProjectArtifacts(workspaceId, id);
    setArtifacts(artList.artifacts);
    setArtifactVersions([]);
    setSelectedArtifact('');
    setViewedArtifact(null);
  }

  async function openArtifact(artifactId: string) {
    const versions = await client.listProjectArtifactVersions(workspaceId, artifactId);
    setSelectedArtifact(artifactId);
    setArtifactVersions(versions.versions);
    const latest = versions.versions[versions.versions.length - 1];
    if (latest) {
      const details = await client.readProjectArtifact(workspaceId, artifactId, latest.version);
      setViewedArtifact(details);
      setReviseForm({ title: details.artifact.title, body: details.body, sources: details.artifact.sourceReferences.join('\n') });
    }
  }

  async function viewArtifactVersion(artifactId: string, version: number) {
    const details = await client.readProjectArtifact(workspaceId, artifactId, version);
    setViewedArtifact(details);
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

          <section aria-labelledby="project-artifacts-heading">
            <h3 id="project-artifacts-heading">Artifacts</h3>
            <p className="help">Versioned registry (§9): every version is immutable — revising creates a new version; earlier versions stay readable exactly as written.</p>
            <ul aria-label="Artifacts in this project">
              {artifacts.map(a => (
                <li key={a.artifactId}>
                  <button disabled={busy} className="secondary" onClick={() => void run(() => openArtifact(a.artifactId))}>
                    {a.title}
                  </button>
                  <span> · {a.type} · Latest version {a.version} · {a.privacy} · {a.validationStatus}</span>
                </li>
              ))}
            </ul>
            {!artifacts.length && <p>No artifacts registered for this project yet.</p>}

            <form
              onSubmit={e => {
                e.preventDefault();
                void run(async () => {
                  const created = await client.registerProjectArtifact(workspaceId, current.memory.id, {
                    type: artifactForm.type,
                    title: artifactForm.title,
                    body: artifactForm.body,
                    privacy: artifactForm.privacy,
                    sourceReferences: artifactForm.sources ? artifactForm.sources.split('\n').filter(Boolean) : [],
                  });
                  const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
                  setArtifacts(artList.artifacts);
                  setArtifactForm({ type: 'note', title: '', body: '', privacy: 'private', sources: '' });
                  await openArtifact(created.artifact.artifactId);
                  setNotice(`Artifact registered. Version ${created.artifact.version}.`);
                });
              }}
            >
              <h4>Register an artifact</h4>
              <label htmlFor="artifact-type">Artifact type</label>
              <select id="artifact-type" value={artifactForm.type} onChange={e => setArtifactForm(f => ({ ...f, type: e.target.value }))}>
                {['research', 'source', 'idea', 'concept', 'experiment', 'prototype', 'generated-file', 'simulation', 'agent-test', 'decision', 'launch-material', 'feedback', 'note'].map(t => (
                  <option key={t} value={t}>{t}</option>
                ))}
              </select>
              <label htmlFor="artifact-title">Artifact title</label>
              <input id="artifact-title" value={artifactForm.title} onChange={e => setArtifactForm(f => ({ ...f, title: e.target.value }))} required autoComplete="off" />
              <label htmlFor="artifact-body">Artifact text</label>
              <textarea id="artifact-body" value={artifactForm.body} onChange={e => setArtifactForm(f => ({ ...f, body: e.target.value }))} rows={5} required />
              <label htmlFor="artifact-privacy">Privacy classification</label>
              <select id="artifact-privacy" value={artifactForm.privacy} onChange={e => setArtifactForm(f => ({ ...f, privacy: e.target.value }))}>
                <option value="private">Private</option>
                <option value="standard">Standard</option>
              </select>
              <label htmlFor="artifact-sources">Source references (one per line)</label>
              <textarea id="artifact-sources" rows={2} value={artifactForm.sources} onChange={e => setArtifactForm(f => ({ ...f, sources: e.target.value }))} />
              <button type="submit" disabled={busy}>Register artifact</button>
            </form>

            {selectedArtifact && (
              <div>
                <h4>Version history</h4>
                <ul aria-label="Artifact versions">
                  {artifactVersions.map(v => (
                    <li key={v.version}>
                      <button disabled={busy} className="secondary" onClick={() => void run(() => viewArtifactVersion(selectedArtifact, v.version))}>
                        View version {v.version}
                      </button>
                      <span> · {v.timestamp} · {v.validationStatus} · {v.privacy}</span>
                    </li>
                  ))}
                </ul>
                {viewedArtifact && (
                  <div>
                    <p>
                      Viewing version {viewedArtifact.artifact.version} of {viewedArtifact.artifact.title} —{' '}
                      {viewedArtifact.artifact.version === artifactVersions[artifactVersions.length - 1]?.version
                        ? 'the latest version.'
                        : 'an immutable earlier version (read-only).'}
                    </p>
                    <pre className="provider-output">{viewedArtifact.body}</pre>
                    <p className="help">
                      Artifact {viewedArtifact.artifact.artifactId} · Project {viewedArtifact.artifact.projectId} · Hash {viewedArtifact.artifact.contentHash.slice(0, 12)}… ·
                      Generator {viewedArtifact.artifact.generator ? JSON.stringify(viewedArtifact.artifact.generator) : 'none'} ·
                      Sources: {viewedArtifact.artifact.sourceReferences.join(', ') || 'none'}
                    </p>
                    <form
                      onSubmit={e => {
                        e.preventDefault();
                        void run(async () => {
                          const latest = artifactVersions[artifactVersions.length - 1];
                          const revised = await client.reviseProjectArtifact(workspaceId, selectedArtifact, latest.version, {
                            title: reviseForm.title,
                            body: reviseForm.body,
                            sourceReferences: reviseForm.sources ? reviseForm.sources.split('\n').filter(Boolean) : [],
                          });
                          const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
                          setArtifacts(artList.artifacts);
                          await openArtifact(selectedArtifact);
                          setNotice(`Artifact revised. Version ${revised.artifact.version} created; earlier versions are unchanged.`);
                        });
                      }}
                    >
                      <h4>Revise artifact (creates a new version)</h4>
                      <label htmlFor="artifact-revise-title">Title</label>
                      <input id="artifact-revise-title" value={reviseForm.title} onChange={e => setReviseForm(f => ({ ...f, title: e.target.value }))} required autoComplete="off" />
                      <label htmlFor="artifact-revise-body">Text</label>
                      <textarea id="artifact-revise-body" value={reviseForm.body} onChange={e => setReviseForm(f => ({ ...f, body: e.target.value }))} rows={5} required />
                      <label htmlFor="artifact-revise-sources">Source references (one per line)</label>
                      <textarea id="artifact-revise-sources" rows={2} value={reviseForm.sources} onChange={e => setReviseForm(f => ({ ...f, sources: e.target.value }))} />
                      <button type="submit" disabled={busy}>Save new version</button>
                    </form>
                  </div>
                )}
              </div>
            )}
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
