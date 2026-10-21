import { EntityHistory } from './EntityHistory';
import { useEffect, useRef, useState } from 'react';
import type { ArtifactDetails, ArtifactRecord, Client, IdeaConcept, IdeaSet, MemoryDetails, MemoryDraft, MemoryMetadata, RelationshipRecord } from '@anansi/contracts';
import type { DisclosureRecord } from '@anansi/contracts';
import { DisclosurePanel } from './Providers';

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
  // M6 Slice 2: project-scoped idea generation (SS21-22).
  const [ideaForm, setIdeaForm] = useState({ topic: '', customer: '', budget: '', technology: '', objective: '' });
  const [ideaOptions, setIdeaOptions] = useState({ exploreUnrelatedDomains: false, preferSimple: false, preferHighRevenue: false, preferLocalFirst: false, preferSoftwareOnly: false });
  const [ideaLock, setIdeaLock] = useState(true);
  const [contextMemories, setContextMemories] = useState<MemoryMetadata[]>([]);
  const [selectedContext, setSelectedContext] = useState<string[]>([]);
  const [ideaRun, setIdeaRun] = useState<{ runId: string; status: string; partialChars: number; error: string | null } | null>(null);
  const [ideaSet, setIdeaSet] = useState<IdeaSet | null>(null);
  const [ideaDisclosure, setIdeaDisclosure] = useState<DisclosureRecord | null>(null);
  const [savedIdeaArtifact, setSavedIdeaArtifact] = useState('');
  const ideaRunRef = useRef('');

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

    // Load the memories the user may explicitly approve as
    // idea-generation context (nothing is selected by default).
    const memList = await client.listMemories(workspaceId, { limit: 100 });
    setContextMemories(memList.memories.filter(m => m.id !== id));
    setSelectedContext([]);
    setIdeaRun(null);
    setIdeaSet(null);
    setIdeaDisclosure(null);
    setSavedIdeaArtifact('');
    ideaRunRef.current = '';

    // Load this project's artifacts (latest versions)
    const artList = await client.listProjectArtifacts(workspaceId, id);
    setArtifacts(artList.artifacts);
    setArtifactVersions([]);
    setSelectedArtifact('');
    setViewedArtifact(null);
  }

  // Poll the active generation run; cancel it if the screen
  // unmounts (navigation), like the Providers screen.
  useEffect(() => {
    if (!ideaRun || (ideaRun.status !== 'running' && ideaRun.status !== 'cancelling')) return;
    const timer = setInterval(() => {
      void (async () => {
        try {
          const polled = await client.pollProjectIdeas(workspaceId, ideaRun.runId);
          setIdeaRun({ runId: polled.runId, status: polled.status, partialChars: polled.partialChars, error: polled.error ?? null });
          if (polled.disclosure) setIdeaDisclosure(polled.disclosure);
          if (polled.status === 'complete' && polled.ideaSet) setIdeaSet(polled.ideaSet);
          if (polled.status !== 'running' && polled.status !== 'cancelling') ideaRunRef.current = '';
        } catch {
          // Transient poll failures keep the run state; the
          // next interval retries.
        }
      })();
    }, 800);
    return () => clearInterval(timer);
  }, [ideaRun?.runId, ideaRun?.status, client, workspaceId]);

  useEffect(() => () => {
    if (ideaRunRef.current) {
      void client.cancelProjectIdeas(workspaceId, ideaRunRef.current).catch(() => undefined);
    }
  }, [client, workspaceId]);

  async function generateIdeas() {
    if (!current) return;
    setError('');
    setNotice('');
    setIdeaSet(null);
    setSavedIdeaArtifact('');
    const started = await client.startProjectIdeas(workspaceId, {
      projectId: current.memory.id,
      topic: ideaForm.topic,
      customer: ideaForm.customer,
      budget: ideaForm.budget,
      technology: ideaForm.technology,
      objective: ideaForm.objective,
      contextMemoryIds: selectedContext,
      options: ideaOptions,
      zeroCostLock: ideaLock,
    });
    ideaRunRef.current = started.runId;
    setIdeaRun({ runId: started.runId, status: 'running', partialChars: 0, error: null });
    if (started.disclosure) setIdeaDisclosure(started.disclosure);
  }

  async function cancelIdeas() {
    if (!ideaRun) return;
    const polled = await client.cancelProjectIdeas(workspaceId, ideaRun.runId);
    setIdeaRun({ runId: polled.runId, status: polled.status, partialChars: polled.partialChars, error: polled.error ?? null });
  }

  async function saveIdeas() {
    if (!ideaRun || !current) return;
    const saved = await client.saveProjectIdeas(workspaceId, ideaRun.runId);
    setSavedIdeaArtifact(saved.artifact.artifactId);
    const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
    setArtifacts(artList.artifacts);
    setNotice(saved.alreadySaved
      ? 'This idea set was already saved as an artifact; no duplicate was created.'
      : `Idea set saved as artifact ${saved.artifact.artifactId}.`);
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

          <section aria-labelledby="project-ideas-heading">
            <h3 id="project-ideas-heading">Idea generation</h3>
            <p className="help">
              Concepts are model-generated hypotheses, not researched facts. This milestone does not score
              ideas: the order below is the model's own order. Only the memories you explicitly select are
              sent, exactly as the disclosure after generation shows.
            </p>
            <form
              onSubmit={e => {
                e.preventDefault();
                void run(generateIdeas);
              }}
            >
              <label htmlFor="idea-topic">Industry or topic</label>
              <input id="idea-topic" value={ideaForm.topic} onChange={e => setIdeaForm(f => ({ ...f, topic: e.target.value }))} required autoComplete="off" />
              <label htmlFor="idea-customer">Intended customer</label>
              <input id="idea-customer" value={ideaForm.customer} onChange={e => setIdeaForm(f => ({ ...f, customer: e.target.value }))} required autoComplete="off" />
              <label htmlFor="idea-budget">Available budget</label>
              <input id="idea-budget" value={ideaForm.budget} onChange={e => setIdeaForm(f => ({ ...f, budget: e.target.value }))} required autoComplete="off" />
              <label htmlFor="idea-technology">Preferred technology</label>
              <input id="idea-technology" value={ideaForm.technology} onChange={e => setIdeaForm(f => ({ ...f, technology: e.target.value }))} required autoComplete="off" />
              <label htmlFor="idea-objective">Business or personal objective</label>
              <input id="idea-objective" value={ideaForm.objective} onChange={e => setIdeaForm(f => ({ ...f, objective: e.target.value }))} required autoComplete="off" />
              <fieldset>
                <legend>Approved memory context</legend>
                <p className="help">Nothing is scanned automatically. Selected memories are sent to the configured provider for this run only; sealed memories cannot be sent.</p>
                {contextMemories.length === 0 && <p className="help">No other memories in this workspace yet.</p>}
                {contextMemories.map(m => (
                  <label key={m.id}>
                    <input
                      type="checkbox"
                      disabled={m.privacy === 'sealed'}
                      checked={selectedContext.includes(m.id)}
                      onChange={e => setSelectedContext(prev => e.target.checked ? [...prev, m.id] : prev.filter(id => id !== m.id))}
                    />{' '}
                    {m.title} · {m.privacy}
                  </label>
                ))}
              </fieldset>
              <fieldset>
                <legend>Preferences</legend>
                {([
                  ['exploreUnrelatedDomains', 'Explore unrelated domains'],
                  ['preferSimple', 'Prefer simple products'],
                  ['preferHighRevenue', 'Prefer high-revenue opportunities'],
                  ['preferLocalFirst', 'Prefer local-first AI'],
                  ['preferSoftwareOnly', 'Prefer software-only products'],
                ] as const).map(([key, label]) => (
                  <label key={key}>
                    <input type="checkbox" checked={ideaOptions[key]} onChange={e => setIdeaOptions(o => ({ ...o, [key]: e.target.checked }))} /> {label}
                  </label>
                ))}
                <label>
                  <input type="checkbox" checked={ideaLock} onChange={e => setIdeaLock(e.target.checked)} /> Zero-Cost Lock (never spend)
                </label>
              </fieldset>
              <button type="submit" disabled={busy || ideaRun?.status === 'running' || ideaRun?.status === 'cancelling'}>
                Generate product ideas
              </button>
              {(ideaRun?.status === 'running' || ideaRun?.status === 'cancelling') && (
                <button type="button" className="secondary" disabled={ideaRun.status === 'cancelling'} onClick={() => void run(cancelIdeas)}>
                  Cancel generation
                </button>
              )}
            </form>
            {ideaRun && ideaRun.status === 'running' && <p role="status">Generating… {ideaRun.partialChars} characters received.</p>}
            {ideaRun && ideaRun.status === 'cancelling' && <p role="status">Cancelling… waiting for the provider stream to stop.</p>}
            {ideaRun && ideaRun.status === 'cancelled' && <p role="status">Generation cancelled. Nothing was saved.</p>}
            {ideaRun && (ideaRun.status === 'failed' || ideaRun.status === 'invalid') && (
              <p role="alert">Generation {ideaRun.status}: {ideaRun.error ?? 'the provider did not produce a usable set.'} Nothing was saved; you can generate again.</p>
            )}
            {ideaDisclosure && <DisclosurePanel disclosure={ideaDisclosure} />}
            {ideaSet && (
              <div>
                <IdeaSetView ideaSet={ideaSet} />
                {savedIdeaArtifact ? (
                  <p role="status">Saved as artifact {savedIdeaArtifact} in this project.</p>
                ) : (
                  <button type="button" disabled={busy} onClick={() => void run(saveIdeas)}>Save idea set as artifact</button>
                )}
              </div>
            )}
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
                    {(() => {
                      const parsedSet = parseIdeaSetBody(viewedArtifact.body);
                      return parsedSet
                        ? <IdeaSetView ideaSet={parsedSet} />
                        : <pre className="provider-output">{viewedArtifact.body}</pre>;
                    })()}
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


/** Parse a saved artifact body as an idea set, when it is one. */
function parseIdeaSetBody(body: string): IdeaSet | null {
  try {
    const parsed = JSON.parse(body) as Partial<IdeaSet>;
    if (parsed && Array.isArray(parsed.concepts) && typeof parsed.topic === 'string') {
      return parsed as IdeaSet;
    }
  } catch {
    // Not JSON: an ordinary text artifact.
  }
  return null;
}

function ConceptCard({ concept, index }: { concept: IdeaConcept; index: number }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <article className="card">
      <h4>{index + 1}. {concept.name}</h4>
      <p><strong>Problem:</strong> {concept.problem}</p>
      <p><strong>Target customer:</strong> {concept.targetCustomer}</p>
      <p><strong>Proposed solution:</strong> {concept.solution}</p>
      {expanded && (
        <div>
          <p><strong>Why it may be useful:</strong> {concept.whyUseful}</p>
          <p><strong>Differentiation hypothesis:</strong> {concept.differentiationHypothesis}</p>
          <p><strong>Potential revenue model:</strong> {concept.revenueModel}</p>
          <p><strong>Engineering complexity:</strong> {concept.engineeringComplexity}</p>
          <p><strong>Approximate implementation cost:</strong> {concept.implementationCost}</p>
          <p><strong>Key risks:</strong></p>
          <ul>{concept.keyRisks.map(r => <li key={r}>{r}</li>)}</ul>
          <p><strong>Required validation:</strong> {concept.requiredValidation}</p>
          <p><strong>Minimum viable version:</strong> {concept.minimumViableVersion}</p>
        </div>
      )}
      <div>
        <button type="button" className="secondary" onClick={() => setExpanded(v => !v)}>{expanded ? 'Close' : 'Explore'}</button>{' '}
        <button type="button" className="secondary" disabled title="Compare arrives with the evaluation milestone (M7).">Compare</button>{' '}
        <button type="button" className="secondary" disabled title="The Lexi-Collider arrives in M8.">Send to Collider</button>{' '}
        <button type="button" className="secondary" disabled title="Product projects arrive with the Build milestone (M9).">Start Building</button>
      </div>
    </article>
  );
}

/**
 * A generated idea set: model-generated hypotheses in the
 * model's order (no scoring in this milestone), the first
 * three concepts visible with expansion for the rest (SS22).
 */
export function IdeaSetView({ ideaSet }: { ideaSet: IdeaSet }) {
  const [showAll, setShowAll] = useState(false);
  const visible = showAll ? ideaSet.concepts : ideaSet.concepts.slice(0, 3);
  return (
    <div>
      <p className="help">
        Model-generated hypotheses for “{ideaSet.topic}” — not researched facts. Presented in the model's
        order; scoring arrives with the evaluation milestone.
      </p>
      {ideaSet.note && <p className="help">{ideaSet.note}</p>}
      {visible.map((concept, i) => <ConceptCard key={concept.name + i} concept={concept} index={i} />)}
      {!showAll && ideaSet.concepts.length > 3 && (
        <button type="button" className="secondary" onClick={() => setShowAll(true)}>
          Show all {ideaSet.concepts.length} concepts
        </button>
      )}
    </div>
  );
}
