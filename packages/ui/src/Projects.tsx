import { EntityHistory } from './EntityHistory';
import { useEffect, useRef, useState } from 'react';
import type { ArtifactDetails, ArtifactRecord, Client, IdeaConcept, IdeaSet, OpportunityConcept, OpportunitySet, RubricDoc, MemoryDetails, MemoryDraft, MemoryMetadata, RelationshipRecord } from '@anansi/contracts';
import type { DisclosureRecord } from '@anansi/contracts';
import { DisclosurePanel } from './Providers';
import { IdeaRunController } from './ideaRun';
import {
  ColliderExperimentView,
  ColliderPanel,
  parseColliderExperimentBody,
} from './Collider';
import {
  ColliderTransferPanel,
  TransferConceptView,
  parseTransferConceptBody,
} from './Transfer';
import { ColliderArtifactViewer } from './ColliderArtifactViewer';
import { ColliderBuildProduct } from './ColliderBuildProduct';
import { SwarmtankPanel } from './Swarmtank';
import { SomniaPanel } from './Somnia';
import { PipelineNavigator } from './PipelineNavigator.tsx';
import { LaunchStudio } from './LaunchStudio.tsx';
import { OpportunityRunController } from './opportunityRun';
import {
  DeliverablePanel,
  DeliverableView,
  ForgeBuildPanel,
  ForgeRevisePanel,
  ForgeSpecPanel,
  parseDeliverableBody,
  parseReviewBody,
  SourcePackageView,
  SourceReviewPanel,
  SourceReviewView,
  SpecificationView,
  parseProductSpecBody,
  parseSourcePackageBody,
} from './Forge';

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

export function Projects({ client, workspaceId, onNavigateToAsk }: { client: Client; workspaceId: string; onNavigateToAsk?: () => void }) {
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
  const [launchStudioOpen, setLaunchStudioOpen] = useState(false);
  const [artifactVersions, setArtifactVersions] = useState<ArtifactRecord[]>([]);
  const [selectedArtifact, setSelectedArtifact] = useState<string>('');
  const [viewedArtifact, setViewedArtifact] = useState<ArtifactDetails | null>(null);
  const artifactViewEpoch = useRef(0);
  const [viewedArtifactEpoch, setViewedArtifactEpoch] = useState(0);
  const [artifactForm, setArtifactForm] = useState({ type: 'note', title: '', body: '', privacy: 'private', sources: '' });
  const [reviseForm, setReviseForm] = useState({ title: '', body: '', sources: '' });
  // Slice 4: revision instructions prefilled from selected
  // review findings. The prefill is applied only when it
  // targets the currently viewed package version, so
  // switching packages never inherits stale instructions.
  const [revisePrefill, setRevisePrefill] = useState<{ text: string; packageArtifactId: string; packageVersion: number } | null>(null);

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
  const ideaControllerRef = useRef<{ key: string; controller: IdeaRunController } | null>(null);
  function ideaController(): IdeaRunController {
    if (!ideaControllerRef.current || ideaControllerRef.current.key !== workspaceId) {
      const old = ideaControllerRef.current?.controller;
      if (old) void old.reset();
      ideaControllerRef.current = { key: workspaceId, controller: new IdeaRunController(client, workspaceId) };
    }
    return ideaControllerRef.current.controller;
  }

  // M6 Slice 4: project-scoped opportunity discovery (§21).
  // Sources are only what the user supplies here: typed or
  // pasted texts and explicitly selected memories.
  const [oppSources, setOppSources] = useState<{ kind: string; label: string; text: string }[]>([
    { kind: 'problem', label: '', text: '' },
  ]);
  const [oppFocus, setOppFocus] = useState('');
  const [oppSelectedMemories, setOppSelectedMemories] = useState<string[]>([]);
  const [oppLock, setOppLock] = useState(true);
  const [oppRun, setOppRun] = useState<{ runId: string; status: string; partialChars: number; error: string | null } | null>(null);
  const [oppSet, setOppSet] = useState<OpportunitySet | null>(null);
  const [oppDisclosure, setOppDisclosure] = useState<DisclosureRecord | null>(null);
  const [savedOppArtifact, setSavedOppArtifact] = useState('');
  const oppControllerRef = useRef<{ key: string; controller: OpportunityRunController } | null>(null);
  function oppController(): OpportunityRunController {
    if (!oppControllerRef.current || oppControllerRef.current.key !== workspaceId) {
      const old = oppControllerRef.current?.controller;
      if (old) void old.reset();
      oppControllerRef.current = { key: workspaceId, controller: new OpportunityRunController(client, workspaceId) };
    }
    return oppControllerRef.current.controller;
  }

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
    artifactViewEpoch.current += 1;
    // Switching projects supersedes any generation in flight:
    // cancel it and invalidate its late responses (the run
    // controller's epoch does the invalidating).
    await ideaController().reset();
    await oppController().reset();
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
    setOppSelectedMemories([]);
    setOppRun(null);
    setOppSet(null);
    setOppDisclosure(null);
    setSavedOppArtifact('');

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
          // Serialized + staleness-checked by the controller:
          // a tick during an in-flight poll is skipped, and a
          // response for a superseded run is dropped.
          const polled = await ideaController().pollOnce();
          if (!polled) return;
          setIdeaRun({ runId: polled.runId, status: polled.status, partialChars: polled.partialChars, error: polled.error ?? null });
          if (polled.disclosure) setIdeaDisclosure(polled.disclosure);
          if (polled.status === 'complete' && polled.ideaSet) setIdeaSet(polled.ideaSet);
        } catch {
          // Transient poll failures keep the run state; the
          // next interval retries.
        }
      })();
    }, 800);
    return () => clearInterval(timer);
  }, [ideaRun?.runId, ideaRun?.status, client, workspaceId]);

  useEffect(() => () => {
    void ideaControllerRef.current?.controller.reset();
    void oppControllerRef.current?.controller.reset();
  }, []);

  async function generateIdeas() {
    if (!current) return;
    setError('');
    setNotice('');
    setIdeaSet(null);
    setSavedIdeaArtifact('');
    const started = await ideaController().start({
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
    if (!started) return; // superseded while the start was in flight
    setIdeaRun({ runId: started.runId, status: 'running', partialChars: 0, error: null });
    if (started.disclosure) setIdeaDisclosure(started.disclosure);
  }

  async function cancelIdeas() {
    if (!ideaRun) return;
    const polled = await ideaController().cancel();
    if (!polled) return;
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

  // Poll the active opportunity-discovery run (same cadence
  // and staleness discipline as idea generation).
  useEffect(() => {
    if (!oppRun || (oppRun.status !== 'running' && oppRun.status !== 'cancelling')) return;
    const timer = setInterval(() => {
      void (async () => {
        try {
          const polled = await oppController().pollOnce();
          if (!polled) return;
          setOppRun({ runId: polled.runId, status: polled.status, partialChars: polled.partialChars, error: polled.error ?? null });
          if (polled.disclosure) setOppDisclosure(polled.disclosure);
          if (polled.status === 'complete' && polled.opportunitySet) setOppSet(polled.opportunitySet);
        } catch {
          // Transient poll failures keep the run state; the
          // next interval retries.
        }
      })();
    }, 800);
    return () => clearInterval(timer);
  }, [oppRun?.runId, oppRun?.status, client, workspaceId]);

  async function generateOpportunities() {
    if (!current) return;
    setError('');
    setNotice('');
    setOppSet(null);
    setSavedOppArtifact('');
    const sources = oppSources
      .filter(s => s.text.trim() !== '')
      .map(s => ({ kind: s.kind, label: s.label.trim() === '' ? null : s.label.trim(), text: s.text }));
    const started = await oppController().start({
      projectId: current.memory.id,
      focus: oppFocus.trim() === '' ? null : oppFocus.trim(),
      sources,
      memoryIds: oppSelectedMemories,
      zeroCostLock: oppLock,
    });
    if (!started) return; // superseded while the start was in flight
    setOppRun({ runId: started.runId, status: 'running', partialChars: 0, error: null });
    if (started.disclosure) setOppDisclosure(started.disclosure);
  }

  async function cancelOpportunities() {
    if (!oppRun) return;
    const polled = await oppController().cancel();
    if (!polled) return;
    setOppRun({ runId: polled.runId, status: polled.status, partialChars: polled.partialChars, error: polled.error ?? null });
  }

  async function saveOpportunities() {
    if (!oppRun || !current) return;
    const saved = await client.saveProjectOpportunities(workspaceId, oppRun.runId);
    setSavedOppArtifact(saved.artifact.artifactId);
    const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
    setArtifacts(artList.artifacts);
    setNotice(saved.alreadySaved
      ? 'This opportunity set was already saved as an artifact; no duplicate was created.'
      : `Opportunity set saved as artifact ${saved.artifact.artifactId}.`);
  }

  async function openArtifact(artifactId: string, version?: number) {
    const epoch = ++artifactViewEpoch.current;
    const versions = await client.listProjectArtifactVersions(workspaceId, artifactId);
    if (epoch !== artifactViewEpoch.current) return;
    const target = version === undefined
      ? versions.versions[versions.versions.length - 1]
      : versions.versions.find(item => item.version === version);
    if (version !== undefined && !target) throw new Error('The requested artifact version is unavailable.');
    if (target) {
      const details = await client.readProjectArtifact(workspaceId, artifactId, target.version);
      if (epoch !== artifactViewEpoch.current) return;
      // Publish a coherent selection only after the exact body has loaded.
      // Review-to-package navigation must not leave review history/editor state
      // attached to a package viewer or redirect a generic save to the review.
      setSelectedArtifact(artifactId);
      setArtifactVersions(versions.versions);
      setViewedArtifact(details);
      setViewedArtifactEpoch(epoch);
      setReviseForm({ title: details.artifact.title, body: details.body, sources: details.artifact.sourceReferences.join('\n') });
    }
  }

  async function viewArtifactVersion(artifactId: string, version: number) {
    const epoch = ++artifactViewEpoch.current;
    const details = await client.readProjectArtifact(workspaceId, artifactId, version);
    if (epoch !== artifactViewEpoch.current) return;
    setViewedArtifact(details);
    setViewedArtifactEpoch(epoch);
  }

  // A late post-save refresh must not override a newer explicit selection.
  // Callbacks capture the generation of the rendered view, not a pending read.
  async function refreshSavedArtifact(projectId: string, artifactId: string, epoch: number) {
    if (epoch !== artifactViewEpoch.current) return;
    const artList = await client.listProjectArtifacts(workspaceId, projectId);
    if (epoch !== artifactViewEpoch.current) return;
    setArtifacts(artList.artifacts);
    await openArtifact(artifactId);
  }

  useEffect(() => {
    artifactViewEpoch.current += 1;
    void run(refresh);
    return () => { artifactViewEpoch.current += 1; };
  }, [client, workspaceId]);

  useEffect(() => {
    if (launchStudioOpen) {
      document.getElementById('project-launch-heading')?.scrollIntoView();
    }
  }, [launchStudioOpen]);

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
            void ideaController().reset();
            void oppController().reset();
            setIdeaRun(null);
            setIdeaSet(null);
            setIdeaDisclosure(null);
            setSavedIdeaArtifact('');
            setOppRun(null);
            setOppSet(null);
            setOppDisclosure(null);
            setSavedOppArtifact('');
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
        <PipelineNavigator
          artifactTypes={artifacts.map(a => a.type)}
          onNavigateToAsk={onNavigateToAsk}
          onSelectLaunch={() => setLaunchStudioOpen(true)}
        />
      )}
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

          <section aria-labelledby="project-collider-heading">
            <h3 id="project-collider-heading">LEXI-COLLIDER</h3>
            <p className="help">
              Collide two beams of text and explore combinations across
              domains. Results are model-generated hypotheses, not researched
              facts. Only the two texts below, plus any memories you
              explicitly select as context, are sent, exactly as the
              disclosure after collision shows.
            </p>
            {current && (
              <ColliderPanel
                key={`collider:${workspaceId}:${current.memory.id}`}
                client={client}
                workspaceId={workspaceId}
                projectId={current.memory.id}
                contextMemories={contextMemories}
                notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                onSaved={async () => {
                  const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
                  setArtifacts(artList.artifacts);
                }}
                busy={busy}
              />
            )}
          </section>

          <section aria-labelledby="project-swarmtank-heading">
            <h3 id="project-swarmtank-heading">SwarmTank</h3>
            <p className="help">
              Explore how simulated buyer profiles might respond to a
              product. Personas are synthetic, decisions are a transparent
              seeded model, and the report never claims real demand.
            </p>
            {current && (
              <SwarmtankPanel
                key={`swarmtank:${workspaceId}:${current.memory.id}`}
                client={client}
                workspaceId={workspaceId}
                projectId={current.memory.id}
                artifacts={artifacts}
                onSaved={async () => {
                  const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
                  setArtifacts(artList.artifacts);
                }}
                notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                busy={busy}
              />
            )}
          </section>

          <section aria-labelledby="project-somnia-heading">
            <h3 id="project-somnia-heading">SOMNIA</h3>
            <p className="help">
              Isolated, manual, data-only replay of synthetic or redacted
              scenarios. The reference evaluator is deterministic and
              model-free; results are findings, never proof of real
              improvement, and nothing is applied, consolidated, or
              scheduled from here.
            </p>
            {current && (
              <SomniaPanel
                key={`somnia:${workspaceId}:${current.memory.id}`}
                client={client}
                workspaceId={workspaceId}
                projectId={current.memory.id}
                artifacts={artifacts}
                onSaved={async () => {
                  const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
                  setArtifacts(artList.artifacts);
                }}
                notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                busy={busy}
              />
            )}
          </section>

          <section aria-labelledby="project-opportunities-heading">
            <h3 id="project-opportunities-heading">Opportunity discovery</h3>
            <p className="help">
              Opportunities are derived only from source material you supply below — nothing is scanned,
              fetched or inferred automatically. The model treats supplied text as data, not instructions.
              Results are hypotheses grounded in those sources, never researched facts or market evidence.
            </p>
            <form
              onSubmit={e => {
                e.preventDefault();
                void run(generateOpportunities);
              }}
            >
              <fieldset>
                <legend>Supplied sources</legend>
                {oppSources.map((source, i) => (
                  <div key={i}>
                    <label>
                      Source kind{' '}
                      <select
                        value={source.kind}
                        onChange={e => setOppSources(prev => prev.map((s, x) => (x === i ? { ...s, kind: e.target.value } : s)))}
                      >
                        <option value="problem">Problem statement</option>
                        <option value="complaint">Customer complaint</option>
                        <option value="observation">Observation</option>
                        <option value="excerpt">Document excerpt</option>
                      </select>
                    </label>{' '}
                    <label>
                      Label (optional){' '}
                      <input
                        value={source.label}
                        onChange={e => setOppSources(prev => prev.map((s, x) => (x === i ? { ...s, label: e.target.value } : s)))}
                        autoComplete="off"
                      />
                    </label>
                    <label htmlFor={`opp-source-text-${i}`}>Source text</label>
                    <textarea
                      id={`opp-source-text-${i}`}
                      rows={3}
                      value={source.text}
                      onChange={e => setOppSources(prev => prev.map((s, x) => (x === i ? { ...s, text: e.target.value } : s)))}
                    />
                    {oppSources.length > 1 && (
                      <button
                        type="button"
                        className="secondary"
                        onClick={() => setOppSources(prev => prev.filter((_, x) => x !== i))}
                      >
                        Remove source
                      </button>
                    )}
                  </div>
                ))}
                <button
                  type="button"
                  className="secondary"
                  disabled={oppSources.length >= 8}
                  onClick={() => setOppSources(prev => [...prev, { kind: 'observation', label: '', text: '' }])}
                >
                  Add another source
                </button>
              </fieldset>
              <fieldset>
                <legend>Approved memory sources</legend>
                <p className="help">Only the memories you tick are sent, each under its own privacy class; sealed memories cannot be sent.</p>
                {contextMemories.length === 0 && <p className="help">No other memories in this workspace yet.</p>}
                {contextMemories.map(m => (
                  <label key={m.id}>
                    <input
                      type="checkbox"
                      disabled={m.privacy === 'sealed'}
                      checked={oppSelectedMemories.includes(m.id)}
                      onChange={e => setOppSelectedMemories(prev => e.target.checked ? [...prev, m.id] : prev.filter(id => id !== m.id))}
                    />{' '}
                    {m.title} · {m.privacy}
                  </label>
                ))}
              </fieldset>
              <label htmlFor="opp-focus">Focus (optional)</label>
              <input id="opp-focus" value={oppFocus} onChange={e => setOppFocus(e.target.value)} autoComplete="off" />
              <div>
                <p className="help">Review — exactly what will be sent to the configured provider:</p>
                <ul aria-label="Sources that will be sent">
                  {oppSources.filter(s => s.text.trim() !== '').map((s, i) => (
                    <li key={i}>
                      {s.kind === 'problem' ? 'Problem statement' : s.kind === 'complaint' ? 'Customer complaint' : s.kind === 'observation' ? 'Observation' : 'Document excerpt'}
                      {s.label.trim() ? ` — ${s.label.trim()}` : ''}: “{s.text.trim().slice(0, 80)}{s.text.trim().length > 80 ? '…' : ''}”
                    </li>
                  ))}
                  {oppSelectedMemories.map(id => {
                    const memory = contextMemories.find(m => m.id === id);
                    return <li key={id}>Memory: {memory ? `${memory.title} · ${memory.privacy}` : id}</li>;
                  })}
                  {oppSources.every(s => s.text.trim() === '') && oppSelectedMemories.length === 0 && (
                    <li>Nothing yet — add a source above or tick a memory.</li>
                  )}
                </ul>
              </div>
              <label>
                <input type="checkbox" checked={oppLock} onChange={e => setOppLock(e.target.checked)} /> Zero-Cost Lock (never spend)
              </label>
              <button type="submit" disabled={busy || oppRun?.status === 'running' || oppRun?.status === 'cancelling'}>
                Discover opportunities
              </button>
              {(oppRun?.status === 'running' || oppRun?.status === 'cancelling') && (
                <button type="button" className="secondary" disabled={oppRun.status === 'cancelling'} onClick={() => void run(cancelOpportunities)}>
                  Cancel discovery
                </button>
              )}
            </form>
            {oppRun && oppRun.status === 'running' && <p role="status">Discovering… {oppRun.partialChars} characters received.</p>}
            {oppRun && oppRun.status === 'cancelling' && <p role="status">Cancelling… waiting for the provider stream to stop.</p>}
            {oppRun && oppRun.status === 'cancelled' && <p role="status">Discovery cancelled. Nothing was saved.</p>}
            {oppRun && (oppRun.status === 'failed' || oppRun.status === 'invalid') && (
              <p role="alert">Discovery {oppRun.status}: {oppRun.error ?? 'the provider did not produce a usable set.'} Nothing was saved; you can try again.</p>
            )}
            {oppDisclosure && <DisclosurePanel disclosure={oppDisclosure} />}
            {oppSet && (
              <div>
                <OpportunitySetView opportunitySet={oppSet} />
                {savedOppArtifact ? (
                  <p role="status">Saved as artifact {savedOppArtifact} in this project.</p>
                ) : (
                  <button type="button" disabled={busy} onClick={() => void run(saveOpportunities)}>Save opportunity set as artifact</button>
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
                {['research', 'source', 'idea', 'concept', 'experiment', 'prototype', 'generated-file', 'simulation', 'agent-test', 'decision', 'launch-material', 'feedback', 'note', 'rubric', 'evaluation'].map(t => (
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
                      const parsedEvaluation = viewedArtifact.artifact.type === 'evaluation'
                        ? parseEvaluationBody(viewedArtifact.body)
                        : null;
                      if (parsedEvaluation) return <EvaluationView evaluation={parsedEvaluation} />;
                      const parsedSpec = viewedArtifact.artifact.type === 'specification'
                        ? parseProductSpecBody(viewedArtifact.body)
                        : null;
                      if (parsedSpec) {
                        return (
                          <div>
                            <SpecificationView doc={parsedSpec} />
                            {current && (
                              <ForgeBuildPanel
                                client={client}
                                workspaceId={workspaceId}
                                projectId={current.memory.id}
                                spec={viewedArtifact.artifact}
                                notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                                onSaved={() => {
                                  void (async () => {
                                    const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
                                    setArtifacts(artList.artifacts);
                                  })();
                                }}
                              />
                            )}
                          </div>
                        );
                      }
                      const parsedPackage = viewedArtifact.artifact.type === 'source-package'
                        ? parseSourcePackageBody(viewedArtifact.body)
                        : null;
                      if (parsedPackage) {
                        return (
                          <div>
                            <SourcePackageView
                              doc={parsedPackage}
                              artifact={viewedArtifact.artifact}
                              client={client}
                              workspaceId={workspaceId}
                              notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                            />
                            {current && (
                              <ForgeRevisePanel
                                key={`${workspaceId}:${current.memory.id}:${viewedArtifact.artifact.artifactId}:${viewedArtifact.artifact.version}`}
                                client={client}
                                workspaceId={workspaceId}
                                projectId={current.memory.id}
                                pkg={viewedArtifact.artifact}
                                doc={parsedPackage}
                                notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                                onSaved={() => { void refreshSavedArtifact(current.memory.id, viewedArtifact.artifact.artifactId, viewedArtifactEpoch); }}
                                prefill={revisePrefill && revisePrefill.packageArtifactId === viewedArtifact.artifact.artifactId && revisePrefill.packageVersion === viewedArtifact.artifact.version ? revisePrefill : undefined}
                              />
                            )}
                            {current && (
                              <SourceReviewPanel
                                key={`review:${workspaceId}:${current.memory.id}:${viewedArtifact.artifact.artifactId}:${viewedArtifact.artifact.version}`}
                                client={client}
                                workspaceId={workspaceId}
                                projectId={current.memory.id}
                                pkg={viewedArtifact.artifact}
                                doc={parsedPackage}
                                notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                                onSaved={() => {
                                  void (async () => {
                                    const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
                                    setArtifacts(artList.artifacts);
                                  })();
                                }}
                                onPrepareRevision={(text) => setRevisePrefill({ text, packageArtifactId: viewedArtifact.artifact.artifactId, packageVersion: viewedArtifact.artifact.version })}
                              />
                            )}
                            {current && (
                              <DeliverablePanel
                                key={`deliverable:${workspaceId}:${current.memory.id}:${viewedArtifact.artifact.artifactId}:${viewedArtifact.artifact.version}`}
                                client={client}
                                workspaceId={workspaceId}
                                projectId={current.memory.id}
                                pkg={viewedArtifact.artifact}
                                doc={parsedPackage}
                                notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                                onSaved={() => {
                                  void (async () => {
                                    const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
                                    setArtifacts(artList.artifacts);
                                  })();
                                }}
                              />
                            )}
                          </div>
                        );
                      }
                      const parsedDeliverable = viewedArtifact.artifact.type === 'deliverable'
                        ? parseDeliverableBody(viewedArtifact.body)
                        : null;
                      if (parsedDeliverable) {
                        return (
                          <DeliverableView
                            key={`deliverable:${workspaceId}:${current?.memory.id ?? ''}:${viewedArtifact.artifact.artifactId}:${viewedArtifact.artifact.version}`}
                            doc={parsedDeliverable}
                            artifact={viewedArtifact.artifact}
                            client={client}
                            workspaceId={workspaceId}
                            notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                            onSaved={() => { void refreshSavedArtifact(current.memory.id, viewedArtifact.artifact.artifactId, viewedArtifactEpoch); }}
                          />
                        );
                      }
                      const parsedReview = viewedArtifact.artifact.type === 'source-review'
                        ? parseReviewBody(viewedArtifact.body)
                        : null;
                      if (parsedReview) {
                        return (
                          <SourceReviewView
                            key={`review:${workspaceId}:${current?.memory.id ?? ''}:${viewedArtifact.artifact.artifactId}:${viewedArtifact.artifact.version}`}
                            doc={parsedReview}
                            artifact={viewedArtifact.artifact}
                            client={client}
                            workspaceId={workspaceId}
                            notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                            onSaved={() => { void refreshSavedArtifact(current.memory.id, viewedArtifact.artifact.artifactId, viewedArtifactEpoch); }}
                            onPrepareRevision={(packageArtifactId, packageVersion, text) => {
                              setRevisePrefill({ text, packageArtifactId, packageVersion });
                              void run(() => openArtifact(packageArtifactId, packageVersion));
                            }}
                          />
                        );
                      }
                      const parsedOpportunities = parseOpportunitySetBody(viewedArtifact.body);
                      if (parsedOpportunities) {
                        return (
                          <div>
                            <OpportunitySetView opportunitySet={parsedOpportunities} />
                            {viewedArtifact.artifact.type === 'idea' && current && (
                              <ForgeSpecPanel
                                client={client}
                                workspaceId={workspaceId}
                                projectId={current.memory.id}
                                source={viewedArtifact.artifact}
                                concepts={parsedOpportunities.concepts}
                                notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                                onSaved={() => {
                                  void (async () => {
                                    const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
                                    setArtifacts(artList.artifacts);
                                  })();
                                }}
                              />
                            )}
                            {viewedArtifact.artifact.type === 'idea' && current && (
                              <EvaluationPanel
                                client={client}
                                workspaceId={workspaceId}
                                projectId={current.memory.id}
                                source={viewedArtifact.artifact}
                                ideaSet={{
                                  concepts: parsedOpportunities.concepts,
                                  topic: parsedOpportunities.topic,
                                  note: parsedOpportunities.note ?? undefined,
                                }}
                                rubricArtifacts={artifacts.filter(a => a.type === 'rubric')}
                                onSaved={() => {
                                  void (async () => {
                                    const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
                                    setArtifacts(artList.artifacts);
                                  })();
                                }}
                                notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                              />
                            )}
                          </div>
                        );
                      }
                      const parsedExperiment = viewedArtifact.artifact.type === 'experiment'
                        ? parseColliderExperimentBody(viewedArtifact.body)
                        : null;
                      if (parsedExperiment) {
                        return (
                          <div>
                            <ColliderExperimentView
                              key={`collider:${workspaceId}:${current?.memory.id ?? ''}:${viewedArtifact.artifact.artifactId}:${viewedArtifact.artifact.version}`}
                              doc={parsedExperiment}
                              instanceScope={`saved-${viewedArtifact.artifact.artifactId}-v${viewedArtifact.artifact.version}`}
                              artifactId={viewedArtifact.artifact.artifactId}
                              version={viewedArtifact.artifact.version}
                            />
                            <ColliderArtifactViewer
                              key={`artifacts:${workspaceId}:${current?.memory.id ?? ''}:${viewedArtifact.artifact.artifactId}:${viewedArtifact.artifact.version}`}
                              doc={parsedExperiment}
                              body={viewedArtifact.body}
                              artifactId={viewedArtifact.artifact.artifactId}
                              version={viewedArtifact.artifact.version}
                              timestamp={viewedArtifact.artifact.timestamp}
                              notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                            />
                            {current && (
                              <ColliderBuildProduct
                                key={`build:${workspaceId}:${current.memory.id}:${viewedArtifact.artifact.artifactId}:${viewedArtifact.artifact.version}`}
                                client={client}
                                workspaceId={workspaceId}
                                projectId={current.memory.id}
                                doc={parsedExperiment}
                                experimentArtifactId={viewedArtifact.artifact.artifactId}
                                experimentVersion={viewedArtifact.artifact.version}
                                notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                                onTransferred={conceptId => {
                                  void (async () => {
                                    const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
                                    setArtifacts(artList.artifacts);
                                    await openArtifact(conceptId);
                                  })();
                                }}
                                busy={busy}
                              />
                            )}
                            {current && (
                              <ColliderTransferPanel
                                key={`transfer:${workspaceId}:${current.memory.id}:${viewedArtifact.artifact.artifactId}:${viewedArtifact.artifact.version}`}
                                client={client}
                                workspaceId={workspaceId}
                                projectId={current.memory.id}
                                experiment={parsedExperiment}
                                experimentArtifact={viewedArtifact.artifact}
                                notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                                onTransferred={conceptId => {
                                  void (async () => {
                                    const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
                                    setArtifacts(artList.artifacts);
                                    await openArtifact(conceptId);
                                  })();
                                }}
                                busy={busy}
                              />
                            )}
                          </div>
                        );
                      }
                      const parsedTransfer = viewedArtifact.artifact.type === 'concept'
                        ? parseTransferConceptBody(viewedArtifact.body)
                        : null;
                      if (parsedTransfer) {
                        return (
                          <div>
                            <TransferConceptView
                              key={`transfer-concept:${workspaceId}:${current?.memory.id ?? ''}:${viewedArtifact.artifact.artifactId}:${viewedArtifact.artifact.version}`}
                              doc={parsedTransfer}
                            />
                            {current && (
                              <ForgeSpecPanel
                                key={`forge-spec:${workspaceId}:${current.memory.id}:${viewedArtifact.artifact.artifactId}:${viewedArtifact.artifact.version}`}
                                client={client}
                                workspaceId={workspaceId}
                                projectId={current.memory.id}
                                source={viewedArtifact.artifact}
                                concepts={[{ name: parsedTransfer.selectedCombination.name }]}
                                notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                                onSaved={() => {
                                  void (async () => {
                                    const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
                                    setArtifacts(artList.artifacts);
                                  })();
                                }}
                              />
                            )}
                          </div>
                        );
                      }
                      const parsedSet = parseIdeaSetBody(viewedArtifact.body);
                      return parsedSet
                        ? (
                          <div>
                            <IdeaSetView ideaSet={parsedSet} compareHint="Use Compare & evaluate below for this saved set." />
                            {viewedArtifact.artifact.type === 'idea' && current && (
                              <ForgeSpecPanel
                                client={client}
                                workspaceId={workspaceId}
                                projectId={current.memory.id}
                                source={viewedArtifact.artifact}
                                concepts={parsedSet.concepts}
                                notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                                onSaved={() => {
                                  void (async () => {
                                    const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
                                    setArtifacts(artList.artifacts);
                                  })();
                                }}
                              />
                            )}
                            {viewedArtifact.artifact.type === 'idea' && current && (
                              <EvaluationPanel
                                client={client}
                                workspaceId={workspaceId}
                                projectId={current.memory.id}
                                source={viewedArtifact.artifact}
                                ideaSet={parsedSet}
                                rubricArtifacts={artifacts.filter(a => a.type === 'rubric')}
                                onSaved={() => {
                                  void (async () => {
                                    const artList = await client.listProjectArtifacts(workspaceId, current.memory.id);
                                    setArtifacts(artList.artifacts);
                                  })();
                                }}
                                notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
                              />
                            )}
                          </div>
                        )
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
          {current && launchStudioOpen && (
            <section aria-labelledby="project-launch-heading">
              <h3 id="project-launch-heading">Launch Studio</h3>
              <LaunchStudio
                key={`${current.memory.id}:${artifacts.map(a => a.artifactId).join(',')}`}
                artifacts={artifacts}
                readArtifact={async (artifactId, version) => {
                  const details = await client.readProjectArtifact(workspaceId, artifactId, version);
                  return details.body;
                }}
                dispatch={async (prompt: string) => {
                  const opened = await client.startAsk(workspaceId, prompt);
                  for (;;) {
                    const run = await client.pollAsk(workspaceId, opened.id);
                    if (run.state === 'completed') {
                      if (run.error) throw Error(run.error.message);
                      const id = opened.id;
                      await client.forgetAsk(workspaceId, id).catch(() => {});
                      return run.response?.answer ?? run.partialAnswer ?? '';
                    }
                    if (run.state === 'cancelled') {
                      await client.forgetAsk(workspaceId, opened.id).catch(() => {});
                      return '';
                    }
                    await new Promise(resolve => setTimeout(resolve, 50));
                  }
                }}
                notify={(message, isError) => { if (isError) setError(message); else setNotice(message); }}
              />
            </section>
          )}
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

function ConceptCard({ concept, index, compareHint }: { concept: IdeaConcept; index: number; compareHint?: string }) {
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
        <button type="button" className="secondary" disabled title={compareHint}>Compare</button>{' '}
        <button type="button" className="secondary" disabled title="The Lexi-Collider arrives in M8.">Send to Collider</button>{' '}
        <button type="button" className="secondary" disabled title="Product projects arrive with the Build milestone (M9).">Start Building</button>
      </div>
    </article>
  );
}

/** Parse a saved artifact body as an opportunity set, when it is one. */
function parseOpportunitySetBody(body: string): OpportunitySet | null {
  try {
    const parsed = JSON.parse(body) as Partial<OpportunitySet>;
    if (
      parsed &&
      parsed.format === 'anansi-opportunity-set' &&
      Array.isArray(parsed.concepts) &&
      Array.isArray(parsed.sources)
    ) {
      return parsed as OpportunitySet;
    }
  } catch {
    // Not JSON: an ordinary text artifact.
  }
  return null;
}

const OPP_SOURCE_KIND_LABELS: Record<string, string> = {
  problem: 'Problem statement',
  complaint: 'Customer complaint',
  observation: 'Observation',
  excerpt: 'Document excerpt',
  memory: 'Memory',
};

function oppSourceLabel(source: OpportunitySet['sources'][number]): string {
  const kind = OPP_SOURCE_KIND_LABELS[source.kind] ?? source.kind;
  return source.label ? `${kind}: ${source.label}` : kind;
}

function OpportunityCard({ concept, index, sources }: { concept: OpportunityConcept; index: number; sources: OpportunitySet['sources'] }) {
  const [expanded, setExpanded] = useState(false);
  const cited = concept.sourceReferences
    .map(id => sources.find(s => s.sourceId === id))
    .filter((s): s is OpportunitySet['sources'][number] => Boolean(s));
  return (
    <article className="card">
      <h4>{index + 1}. {concept.name}</h4>
      <p><strong>Problem:</strong> {concept.problem}</p>
      <p><strong>Target customer:</strong> {concept.targetCustomer}</p>
      <p><strong>Proposed solution:</strong> {concept.solution}</p>
      <p>
        <strong>Derived from:</strong>{' '}
        {cited.length ? cited.map(s => oppSourceLabel(s)).join('; ') : concept.sourceReferences.join(', ')}
      </p>
      {expanded && (
        <div>
          <p><strong>Why it may be useful:</strong> {concept.whyUseful}</p>
          <p><strong>Differentiation hypothesis:</strong> {concept.differentiationHypothesis}</p>
          <p><strong>Potential revenue model:</strong> {concept.revenueModel}</p>
          <p><strong>Engineering complexity:</strong> {concept.engineeringComplexity}</p>
          <p><strong>Approximate implementation cost:</strong> {concept.implementationCost}</p>
          <p><strong>Key risks:</strong></p>
          <ul>{concept.keyRisks.map(r => <li key={r}>{r}</li>)}</ul>
          {concept.assumptions.length > 0 && (
            <>
              <p><strong>Assumptions (unverified):</strong></p>
              <ul>{concept.assumptions.map(a => <li key={a}>{a}</li>)}</ul>
            </>
          )}
          <p><strong>Required validation:</strong> {concept.requiredValidation}</p>
          <p><strong>Minimum viable version:</strong> {concept.minimumViableVersion}</p>
        </div>
      )}
      <div>
        <button type="button" className="secondary" onClick={() => setExpanded(v => !v)}>{expanded ? 'Close' : 'Explore'}</button>
      </div>
    </article>
  );
}

/**
 * A discovered opportunity set: the supplied sources it was
 * derived from, then the opportunities in the model's order
 * (no scoring here — assessment is the user's, via Compare
 * & evaluate). First three visible, expansion for the rest.
 */
export function OpportunitySetView({ opportunitySet }: { opportunitySet: OpportunitySet }) {
  const [showAll, setShowAll] = useState(false);
  const visible = showAll ? opportunitySet.concepts : opportunitySet.concepts.slice(0, 3);
  return (
    <div>
      <p className="help">
        Model-generated opportunities for “{opportunitySet.topic}” — hypotheses derived only from the
        sources below, not researched facts or market evidence.
      </p>
      {opportunitySet.note && <p className="help">{opportunitySet.note}</p>}
      <h4>Supplied sources</h4>
      <ul aria-label="Supplied sources for this set">
        {opportunitySet.sources.map(source => (
          <li key={source.sourceId}>
            <strong>{oppSourceLabel(source)}</strong>
            {source.kind === 'memory'
              ? <> · memory {source.memoryId}</>
              : <>: “{source.text && source.text.length > 300 ? `${source.text.slice(0, 300)}…` : source.text}”</>}
          </li>
        ))}
      </ul>
      {visible.map((concept, i) => (
        <OpportunityCard key={concept.name + i} concept={concept} index={i} sources={opportunitySet.sources} />
      ))}
      {!showAll && opportunitySet.concepts.length > 3 && (
        <button type="button" className="secondary" onClick={() => setShowAll(true)}>
          Show all {opportunitySet.concepts.length} opportunities
        </button>
      )}
    </div>
  );
}

/**
 * A generated idea set: model-generated hypotheses in the
 * model's order (no scoring in this milestone), the first
 * three concepts visible with expansion for the rest (SS22).
 */
export function IdeaSetView({ ideaSet, compareHint = 'Save the idea set as an artifact to compare and evaluate it.' }: { ideaSet: IdeaSet; compareHint?: string }) {
  const [showAll, setShowAll] = useState(false);
  const visible = showAll ? ideaSet.concepts : ideaSet.concepts.slice(0, 3);
  return (
    <div>
      <p className="help">
        Model-generated hypotheses for “{ideaSet.topic}” — not researched facts. Presented in the model's
        order; scoring arrives with the evaluation milestone.
      </p>
      {ideaSet.note && <p className="help">{ideaSet.note}</p>}
      {visible.map((concept, i) => <ConceptCard key={concept.name + i} concept={concept} index={i} compareHint={compareHint} />)}
      {!showAll && ideaSet.concepts.length > 3 && (
        <button type="button" className="secondary" onClick={() => setShowAll(true)}>
          Show all {ideaSet.concepts.length} concepts
        </button>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------
// M6 Slice 3: transparent evaluation & comparison.

interface SavedEvaluation {
  format: string;
  sourceArtifactId: string;
  sourceVersion: number;
  rubricArtifactId: string;
  rubricVersion: number;
  rubric: RubricDoc;
  concepts: IdeaConcept[];
  conceptIndices: number[];
  assessments: { conceptIndex: number; dimension: string; score: number | null; rationale: string }[];
  summary: { conceptIndex: number; weightedScore: number | null; assessedDimensions: number; totalDimensions: number }[];
  note: string;
}

function parseEvaluationBody(body: string): SavedEvaluation | null {
  try {
    const parsed = JSON.parse(body) as SavedEvaluation;
    if (parsed && parsed.format === 'anansi-evaluation' && Array.isArray(parsed.concepts)) {
      return parsed;
    }
  } catch {
    // Not an evaluation document.
  }
  return null;
}

/** The read-only rendering of a saved evaluation, entirely
 * from its own stored document: rubric snapshot as used,
 * provenance, scores or "not assessed", and the summary. */
export function EvaluationView({ evaluation }: { evaluation: SavedEvaluation }) {
  const labelFor = (key: string) =>
    evaluation.rubric.dimensions.find(d => d.key === key)?.label ?? key;
  return (
    <div>
      <p className="help">
        Evaluation of idea artifact {evaluation.sourceArtifactId} (version {evaluation.sourceVersion}) under
        rubric artifact {evaluation.rubricArtifactId} (version {evaluation.rubricVersion}).
      </p>
      <p className="help">{evaluation.note}</p>
      <h4>Rubric as used: {evaluation.rubric.name}</h4>
      <ul>
        {evaluation.rubric.dimensions.map(d => (
          <li key={d.key}>{d.label} — weight {d.weight}. {d.description}</li>
        ))}
      </ul>
      <p className="help">Later versions of that rubric do not change this evaluation.</p>
      {evaluation.concepts.map((concept, position) => {
        const conceptIndex = evaluation.conceptIndices[position];
        const summary = evaluation.summary.find(s => s.conceptIndex === conceptIndex);
        return (
          <article key={conceptIndex} className="card">
            <h4>{concept.name}</h4>
            <p>
              <strong>Weighted summary:</strong>{' '}
              {summary && summary.weightedScore !== null
                ? `${summary.weightedScore} (assessed ${summary.assessedDimensions} of ${summary.totalDimensions} dimensions)`
                : 'not assessed'}
            </p>
            <p><strong>Problem:</strong> {concept.problem}</p>
            <p><strong>Target customer:</strong> {concept.targetCustomer}</p>
            <p><strong>Proposed solution:</strong> {concept.solution}</p>
            <ul>
              {evaluation.rubric.dimensions.map(d => {
                const entry = evaluation.assessments.find(
                  a => a.conceptIndex === conceptIndex && a.dimension === d.key,
                );
                return (
                  <li key={d.key}>
                    {labelFor(d.key)}:{' '}
                    {entry && entry.score !== null ? `${entry.score} / 5` : 'not assessed'}
                    {entry?.rationale ? ` — ${entry.rationale}` : ''}
                  </li>
                );
              })}
            </ul>
          </article>
        );
      })}
    </div>
  );
}

interface EvalPanelProps {
  client: Client;
  workspaceId: string;
  projectId: string;
  source: ArtifactRecord;
  ideaSet: IdeaSet;
  rubricArtifacts: ArtifactRecord[];
  onSaved: () => void;
  notify: (message: string, isError: boolean) => void;
}

function EvaluationPanel(props: EvalPanelProps) {
  const { client, workspaceId, projectId, source, ideaSet } = props;
  const [standard, setStandard] = useState<RubricDoc | null>(null);
  const [choice, setChoice] = useState<string>('standard');
  const [doc, setDoc] = useState<RubricDoc | null>(null);
  const [docVersion, setDocVersion] = useState(0);
  // While a chosen rubric artifact is being read, the editor
  // still shows the previous document: an edit made in that
  // window would be silently overwritten when the read
  // resolves, and an evaluation saved then would cite the
  // new artifact with the old document/version. The form is
  // disabled until the read lands, and a superseded read
  // never applies.
  const [loadingRubric, setLoadingRubric] = useState(false);
  const rubricLoadSeq = useRef(0);
  const [selection, setSelection] = useState<number[]>([]);
  const [scores, setScores] = useState<Record<string, { score: string; rationale: string }>>({});
  const [panelError, setPanelError] = useState('');
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void (async () => {
      try {
        const response = await client.getStandardRubric(workspaceId, projectId);
        setStandard(response.rubric);
        setDoc(current => current ?? response.rubric);
      } catch (error) {
        setPanelError(error instanceof Error ? error.message : 'The standard rubric could not be loaded.');
      }
    })();
  }, [client, workspaceId, projectId]);

  async function chooseRubric(value: string) {
    const seq = ++rubricLoadSeq.current;
    setChoice(value);
    setPanelError('');
    if (value === 'standard') {
      setDoc(standard);
      setDocVersion(0);
      setLoadingRubric(false);
      return;
    }
    const record = props.rubricArtifacts.find(a => a.artifactId === value);
    if (!record) return;
    setLoadingRubric(true);
    try {
      const details = await client.readProjectArtifact(workspaceId, value, record.version);
      if (rubricLoadSeq.current !== seq) return;
      setDoc(JSON.parse(details.body) as RubricDoc);
      setDocVersion(record.version);
    } catch (error) {
      if (rubricLoadSeq.current === seq) {
        setPanelError(error instanceof Error ? error.message : 'That rubric could not be read.');
      }
    } finally {
      if (rubricLoadSeq.current === seq) setLoadingRubric(false);
    }
  }

  function patchDimension(index: number, patch: Partial<RubricDoc['dimensions'][number]>) {
    setDoc(current => current && {
      ...current,
      dimensions: current.dimensions.map((d, i) => (i === index ? { ...d, ...patch } : d)),
    });
  }

  async function saveRubric() {
    if (!doc) return;
    setBusy(true);
    setPanelError('');
    try {
      const body = JSON.stringify(doc);
      if (choice === 'standard') {
        const details = await client.registerProjectArtifact(workspaceId, projectId, {
          type: 'rubric', title: doc.name, body, sourceReferences: [], privacy: 'private',
        });
        setChoice(details.artifact.artifactId);
        setDocVersion(1);
        props.notify(`Rubric saved as project artifact ${details.artifact.artifactId} (version 1).`, false);
      } else {
        const details = await client.reviseProjectArtifact(workspaceId, choice, docVersion, {
          title: doc.name, body, sourceReferences: [],
        });
        setDocVersion(details.artifact.version);
        props.notify(`Rubric saved as version ${details.artifact.version}; earlier evaluations keep the rubric they used.`, false);
      }
      props.onSaved();
    } catch (error) {
      setPanelError(error instanceof Error ? error.message : 'The rubric could not be saved.');
    } finally {
      setBusy(false);
    }
  }

  function preview(conceptIndex: number): { score: number | null; assessed: number; total: number } {
    if (!doc) return { score: null, assessed: 0, total: 0 };
    let weighted = 0;
    let weights = 0;
    let assessed = 0;
    let total = 0;
    for (const dim of doc.dimensions) {
      if (dim.weight <= 0) continue;
      total += 1;
      const entry = scores[`${conceptIndex}:${dim.key}`];
      if (entry && entry.score !== '') {
        weighted += Number(entry.score) * dim.weight;
        weights += dim.weight;
        assessed += 1;
      }
    }
    return {
      score: weights > 0 ? Math.round((weighted / weights) * 100) / 100 : null,
      assessed,
      total,
    };
  }

  async function saveEvaluation() {
    if (!doc) return;
    if (selection.length === 0) {
      setPanelError('Select at least one concept to evaluate.');
      return;
    }
    setBusy(true);
    setPanelError('');
    try {
      // The evaluation must cite an exact rubric artifact
      // version: a rubric that only exists as the standard
      // definition is registered first, transparently.
      let rubricArtifactId = choice;
      let rubricVersion = docVersion;
      if (choice === 'standard') {
        const details = await client.registerProjectArtifact(workspaceId, projectId, {
          type: 'rubric', title: doc.name, body: JSON.stringify(doc), sourceReferences: [], privacy: 'private',
        });
        rubricArtifactId = details.artifact.artifactId;
        rubricVersion = 1;
        setChoice(rubricArtifactId);
        setDocVersion(1);
      }
      const assessments = [] as { conceptIndex: number; dimension: string; score: number | null; rationale: string }[];
      for (const conceptIndex of [...selection].sort((a, b) => a - b)) {
        for (const dim of doc.dimensions) {
          const entry = scores[`${conceptIndex}:${dim.key}`];
          if (entry && (entry.score !== '' || entry.rationale.trim() !== '')) {
            assessments.push({
              conceptIndex,
              dimension: dim.key,
              score: entry.score === '' ? null : Number(entry.score),
              rationale: entry.rationale,
            });
          }
        }
      }
      const saved = await client.saveProjectEvaluation(workspaceId, {
        projectId,
        sourceArtifactId: source.artifactId,
        sourceVersion: source.version,
        rubricArtifactId,
        rubricVersion,
        conceptIndices: [...selection].sort((a, b) => a - b),
        assessments,
      });
      props.notify(
        saved.alreadySaved
          ? `An identical evaluation is already saved as artifact ${saved.artifact.artifactId}; no duplicate was created.`
          : `Evaluation saved as artifact ${saved.artifact.artifactId}.`,
        false,
      );
      props.onSaved();
    } catch (error) {
      setPanelError(error instanceof Error ? error.message : 'The evaluation could not be saved.');
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-labelledby="evaluation-heading">
      <h4 id="evaluation-heading">Compare &amp; evaluate</h4>
      <p className="help">
        Your own assessments under a rubric you can inspect and change. Scores are structured judgement —
        not market evidence or a probability of success. Missing assessments stay “not assessed” and are
        never counted as zero.
      </p>
      {panelError && <p role="alert" className="error">{panelError}</p>}
      <fieldset>
        <legend>Concepts to compare (up to 3)</legend>
        {ideaSet.concepts.map((concept, i) => (
          <label key={concept.name + i}>
            <input
              type="checkbox"
              checked={selection.includes(i)}
              disabled={!selection.includes(i) && selection.length >= 3}
              onChange={e => setSelection(prev => e.target.checked ? [...prev, i] : prev.filter(x => x !== i))}
            />{' '}
            {concept.name}
          </label>
        ))}
      </fieldset>
      <label htmlFor="eval-rubric-choice">Rubric</label>
      <select id="eval-rubric-choice" value={choice} onChange={e => void chooseRubric(e.target.value)}>
        <option value="standard">Standard rubric (owner-approved starting point)</option>
        {props.rubricArtifacts.map(a => (
          <option key={a.artifactId} value={a.artifactId}>{a.title} (v{a.version})</option>
        ))}
      </select>
      {loadingRubric && <p className="help" role="status">Loading the selected rubric…</p>}
      {doc && (
        <div>
          <label htmlFor="eval-rubric-name">Rubric name</label>
          <input id="eval-rubric-name" disabled={loadingRubric} value={doc.name} onChange={e => setDoc({ ...doc, name: e.target.value })} />
          {doc.dimensions.map((dim, i) => (
            <div key={dim.key + i}>
              <label>
                Dimension label{' '}
                <input disabled={loadingRubric} value={dim.label} onChange={e => patchDimension(i, { label: e.target.value })} />
              </label>{' '}
              <label>
                Weight{' '}
                <input type="number" min="0" step="0.5" disabled={loadingRubric} value={dim.weight}
                  onChange={e => patchDimension(i, { weight: Number(e.target.value) })} />
              </label>{' '}
              <label>
                Description{' '}
                <input disabled={loadingRubric} value={dim.description} onChange={e => patchDimension(i, { description: e.target.value })} />
              </label>{' '}
              <button type="button" className="secondary" disabled={loadingRubric || doc.dimensions.length <= 1}
                onClick={() => setDoc({ ...doc, dimensions: doc.dimensions.filter((_, x) => x !== i) })}>
                Remove dimension
              </button>
              <span className="help"> key: {dim.key}</span>
            </div>
          ))}
          <button type="button" className="secondary" disabled={loadingRubric || doc.dimensions.length >= 8}
            onClick={() => {
              const n = doc.dimensions.length + 1;
              setDoc({ ...doc, dimensions: [...doc.dimensions, { key: `dimension-${n}`, label: `Dimension ${n}`, description: '', weight: 1 }] });
            }}>
            Add dimension
          </button>{' '}
          <button type="button" className="secondary" disabled={busy || loadingRubric} onClick={() => void saveRubric()}>
            {choice === 'standard' ? 'Save rubric as artifact' : 'Save rubric as new version'}
          </button>
        </div>
      )}
      {doc && selection.map(conceptIndex => {
        const concept = ideaSet.concepts[conceptIndex];
        const summary = preview(conceptIndex);
        return (
          <article key={conceptIndex} className="card">
            <h4>{concept.name}</h4>
            <p><strong>Problem:</strong> {concept.problem}</p>
            <p><strong>Target customer:</strong> {concept.targetCustomer}</p>
            <p><strong>Proposed solution:</strong> {concept.solution}</p>
            {doc.dimensions.map(dim => {
              const key = `${conceptIndex}:${dim.key}`;
              const entry = scores[key] ?? { score: '', rationale: '' };
              return (
                <div key={dim.key}>
                  <label>
                    {dim.label} score{' '}
                    <select value={entry.score}
                      onChange={e => setScores(prev => ({ ...prev, [key]: { ...entry, score: e.target.value } }))}>
                      <option value="">not assessed</option>
                      {[1, 2, 3, 4, 5].map(n => <option key={n} value={n}>{n}</option>)}
                    </select>
                  </label>{' '}
                  <label>
                    Rationale{' '}
                    <input value={entry.rationale}
                      onChange={e => setScores(prev => ({ ...prev, [key]: { ...entry, rationale: e.target.value } }))} />
                  </label>
                </div>
              );
            })}
            <p role="status">
              Preview summary:{' '}
              {summary.score !== null
                ? `${summary.score} (assessed ${summary.assessed} of ${summary.total} dimensions)`
                : 'not assessed'}
              {' '}— the saved value is recomputed by the core.
            </p>
          </article>
        );
      })}
      <button type="button" disabled={busy || loadingRubric || selection.length === 0} onClick={() => void saveEvaluation()}>
        Save evaluation
      </button>
    </section>
  );
}
