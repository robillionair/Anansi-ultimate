// HOME Command Dashboard (Slice 1, MASTER_SPEC §8).
//
// Calm landing screen: a small number of meaningful actions, nothing
// more. All figures are derived client-side from workspace memory and
// artifact lists plus one local readiness read; zero inference calls,
// zero fake traffic. Container + presentational split: static markup
// cannot run effects, so unit tests render DashboardView with data
// props while click behavior is covered by e2e.

import { useEffect, useState } from 'react';
import type { Client, Job, MemoryMetadata, WorkspaceDetails } from '@anansi/contracts';
import { derivePipeline, type PipelineDerivation } from './lifecycle.ts';
import { toggleLarpMode, useLarpModeStore } from './larp-mode/store.ts';
import type { View } from './Navigation.tsx';

export function formatUptimeMs(ms: number): string {
  if (!Number.isFinite(ms) || ms <= 0) return '0s';
  const totalSeconds = Math.floor(ms / 1000);
  if (totalSeconds < 60) return `${totalSeconds}s`;
  const minutes = Math.floor(totalSeconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    const rest = minutes % 60;
    return rest === 0 ? `${hours}h` : `${hours}h ${rest}m`;
  }
  const days = Math.floor(hours / 24);
  const rest = hours % 24;
  return rest === 0 ? `${days}d` : `${days}d ${rest}h`;
}

export function projectExcerpt(snippet: string | null | undefined): string {
  const text = (snippet ?? '').trim();
  if (!text) return 'No description yet.';
  return text.length > 160 ? `${text.slice(0, 157)}…` : text;
}

export interface DashboardTelemetry {
  memoryNodes: number;
  activeNotes: number;
  relationships: number;
}

export function countTelemetry(
  memories: MemoryMetadata[],
  relationshipCount: number,
): DashboardTelemetry {
  return {
    memoryNodes: memories.filter((m) => m.kind === 'memory').length,
    activeNotes: memories.filter(
      (m) => m.kind === 'note' && m.status !== 'archived' && m.status !== 'superseded',
    ).length,
    relationships: relationshipCount,
  };
}

export function stageBadge(derivation: PipelineDerivation): string {
  if (!derivation.current) return 'Pipeline complete';
  const stage = derivation.stages.find((s) => s.id === derivation.current);
  return `Current stage: ${stage ? stage.label : derivation.current}`;
}

export function formatBackupInterval(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return '0s';
  if (seconds % 86400 === 0) return `${seconds / 86400}d`;
  if (seconds % 3600 === 0) return `${seconds / 3600}h`;
  if (seconds % 60 === 0) return `${seconds / 60}m`;
  return `${seconds}s`;
}

export function formatBackupStatus(schedule: {
  enabled: boolean;
  intervalSeconds: number;
}): string {
  return schedule.enabled
    ? `Auto-backup every ${formatBackupInterval(schedule.intervalSeconds)}`
    : 'Manual backups only';
}

// Job-type label reusing Jobs.tsx's exact selection mapping: Job
// carries no dedicated type field, so the label derives from its
// export selection.
export function jobTypeLabel(job: Job): string {
  const selection = job.selection;
  if (selection.kind === 'backup') {
    return selection.application ? 'Application data backup' : 'Workspace backup';
  }
  if (selection.kind === 'project') return 'Project export';
  if (selection.kind === 'records') return 'Selected records export';
  return 'Workspace export';
}

export interface QueueStatus {
  idle: boolean;
  activeCount: number;
  latestType: string | null;
}

// Contract correction: JobState values are capitalized ("Queued" |
// "Running" | ...). There is no lowercase 'running' / 'pending'
// status. Active work is Queued or Running — the same set Jobs.tsx
// treats as pausable; Paused is not active work.
export function deriveQueueStatus(jobs: Job[]): QueueStatus {
  const active = jobs.filter((j) => j.state === 'Queued' || j.state === 'Running');
  if (!active.length) return { idle: true, activeCount: 0, latestType: null };
  let latest = active[0];
  for (const job of active) {
    const time = Date.parse(job.createdAt);
    const latestTime = Date.parse(latest.createdAt);
    if (Number.isFinite(time) && (!Number.isFinite(latestTime) || time > latestTime)) {
      latest = job;
    }
  }
  return { idle: false, activeCount: active.length, latestType: jobTypeLabel(latest) };
}

export interface SortableProject {
  id: string;
  updatedAt: string;
  latestArtifactAt: string | null;
}

export function latestArtifactTimestamp(timestamps: string[]): string | null {
  let best: string | null = null;
  let bestTime = Number.NaN;
  for (const timestamp of timestamps) {
    const time = Date.parse(timestamp);
    if (Number.isFinite(time) && (!Number.isFinite(bestTime) || time > bestTime)) {
      best = timestamp;
      bestTime = time;
    }
  }
  return best;
}

function recencyKey(project: SortableProject): number {
  const times = [Date.parse(project.updatedAt)];
  if (project.latestArtifactAt) times.push(Date.parse(project.latestArtifactAt));
  const finite = times.filter(Number.isFinite);
  return finite.length ? Math.max(...finite) : 0;
}

// Most recent first; ties keep list order (Array sort is stable).
export function sortProjectsByRecency<T extends SortableProject>(projects: T[]): T[] {
  return [...projects].sort((a, b) => recencyKey(b) - recencyKey(a));
}

export interface DashboardProject {
  id: string;
  title: string;
  excerpt: string;
  stageBadge: string;
}

export interface DashboardData {
  telemetry: DashboardTelemetry;
  projects: DashboardProject[];
  totalProjects: number;
  backupStatus: string;
  queue: QueueStatus;
  uptime: string | null;
}

export function DashboardView({
  workspace,
  data,
  larpOn,
  onNavigate,
  onOpenProject,
  onToggleLarp,
  onManageWorkspace,
}: {
  workspace: WorkspaceDetails;
  data: DashboardData;
  larpOn: boolean;
  onNavigate: (view: View) => void;
  onOpenProject: (projectId: string, section: 'walk' | 'collider') => void;
  onToggleLarp: () => void;
  onManageWorkspace: () => void;
}) {
  const firstProject = data.projects[0];
  return (
    <>
      <section className="card" role="region" aria-labelledby="dashboard-overview-heading">
        <h2 id="dashboard-overview-heading">Workspace overview</h2>
        <p>
          <strong>{workspace.workspace.name}</strong>
          <span className="workspace-id"> · Workspace ID: <code>{workspace.workspace.id}</code></span>
        </p>
        <p>
          <span className="badge ready" role="status">
            {data.uptime ? `Ready · core up ${data.uptime}` : 'Ready'}
          </span>
        </p>
        <dl className="dashboard-stats">
          <div>
            <dt>Memory nodes</dt>
            <dd><strong>{data.telemetry.memoryNodes}</strong></dd>
          </div>
          <div>
            <dt>Active notes</dt>
            <dd><strong>{data.telemetry.activeNotes}</strong></dd>
          </div>
          <div>
            <dt>Recorded relationships</dt>
            <dd><strong>{data.telemetry.relationships}</strong></dd>
          </div>
        </dl>
        <div className="actions">
          <button className="secondary" onClick={onManageWorkspace}>
            Manage workspace
          </button>
        </div>
      </section>

      <section className="card" role="region" aria-labelledby="dashboard-health-heading">
        <h2 id="dashboard-health-heading">System &amp; storage health</h2>
        <p>Backup: {data.backupStatus}</p>
        <p>
          {data.queue.idle
            ? 'Background queue idle'
            : `${data.queue.activeCount} active job(s) · latest: ${data.queue.latestType}`}
        </p>
        <div className="actions">
          <button className="secondary" onClick={() => onNavigate('backups')}>
            Backups
          </button>
          <button className="secondary" onClick={() => onNavigate('jobs')}>
            Jobs
          </button>
        </div>
      </section>

      <section className="card" role="region" aria-labelledby="dashboard-projects-heading">
        <h2 id="dashboard-projects-heading">Recent &amp; active projects</h2>
        {data.projects.length === 0 ? (
          <p>No projects yet — create your first project.</p>
        ) : (
          <ul aria-label="Dashboard projects">
            {data.projects.map((project) => (
              <li key={project.id}>
                <strong>{project.title}</strong>
                <p>{project.excerpt}</p>
                <p>
                  <span className="badge">{project.stageBadge}</span>
                </p>
                <div className="actions">
                  <button onClick={() => onOpenProject(project.id, 'walk')}>
                    Continue Pipeline
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
        {data.totalProjects > 5 && (
          <div className="actions">
            <button className="secondary" onClick={() => onNavigate('projects')}>
              View all in Projects
            </button>
          </div>
        )}
      </section>

      <section className="card" role="region" aria-labelledby="dashboard-actions-heading">
        <h2 id="dashboard-actions-heading">Quick actions</h2>
        <p>Four meaningful actions. Everything else is one navigation click away.</p>
        <div className="actions">
          <button onClick={() => onNavigate('projects')}>+ New Project</button>
          <button onClick={() => onNavigate('ask')}>Ask ANANSI</button>
          <button
            onClick={() =>
              firstProject ? onOpenProject(firstProject.id, 'collider') : onNavigate('projects')
            }
            title={
              firstProject
                ? `Open ${firstProject.title} at its LEXI-COLLIDER section.`
                : 'Create a project first, then collide its ideas.'
            }
          >
            LEXI-COLLIDER
          </button>
          <button onClick={onToggleLarp}>
            LARP Telemetry Engine <span>{larpOn ? 'On' : 'Off'}</span>
          </button>
        </div>
      </section>
    </>
  );
}

export function Dashboard({
  client,
  workspace,
  onNavigate,
  onOpenProject,
  onManageWorkspace,
}: {
  client: Client;
  workspace: WorkspaceDetails;
  onNavigate: (view: View) => void;
  onOpenProject: (projectId: string, section: 'walk' | 'collider') => void;
  onManageWorkspace: () => void;
}) {
  const [data, setData] = useState<DashboardData | null>(null);
  const [error, setError] = useState('');
  const [epoch, setEpoch] = useState(0);
  const larpOn = useLarpModeStore();
  const workspaceId = workspace.workspace.id;

  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        const projectList = await client.listMemories(workspaceId, { kind: 'project' });
        const sortable: (DashboardProject & SortableProject)[] = [];
        for (const project of projectList.memories) {
          const artList = await client.listProjectArtifacts(workspaceId, project.id);
          sortable.push({
            id: project.id,
            title: project.title,
            excerpt: projectExcerpt(project.snippet),
            stageBadge: stageBadge(derivePipeline(artList.artifacts.map((a) => a.type))),
            updatedAt: project.updatedAt,
            latestArtifactAt: latestArtifactTimestamp(artList.artifacts.map((a) => a.timestamp)),
          });
        }
        const ordered = sortProjectsByRecency(sortable);
        const projects: DashboardProject[] = ordered
          .slice(0, 5)
          .map(({ id, title, excerpt, stageBadge: badge }) => ({
            id,
            title,
            excerpt,
            stageBadge: badge,
          }));
        const memories: MemoryMetadata[] = [];
        let offset: number | undefined;
        for (;;) {
          const page = await client.listMemories(workspaceId, { limit: 100, offset });
          memories.push(...page.memories);
          const next = page.nextOffset ?? undefined;
          if (next === undefined || next === offset) break;
          offset = next;
        }
        const relList = await client.listRelationships(workspaceId);
        const schedule = await client.readBackupSchedule(workspaceId);
        const jobs: Job[] = [];
        let jobOffset = 0;
        for (;;) {
          const page = await client.listJobs(workspaceId, jobOffset);
          jobs.push(...page.jobs);
          const next = page.nextOffset ?? undefined;
          if (next === undefined || next === jobOffset) break;
          jobOffset = next;
        }
        let uptime: string | null = null;
        try {
          const readiness = await client.readiness();
          uptime = formatUptimeMs(readiness.uptimeMs);
        } catch {
          uptime = null;
        }
        if (!cancelled) {
          setData({
            telemetry: countTelemetry(memories, relList.relationships.length),
            projects,
            totalProjects: ordered.length,
            backupStatus: formatBackupStatus(schedule),
            queue: deriveQueueStatus(jobs),
            uptime,
          });
          setError('');
        }
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : 'Dashboard unavailable.');
      }
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, [client, workspaceId, epoch]);

  if (error) {
    return (
      <section className="card" role="region" aria-labelledby="dashboard-error-heading">
        <h2 id="dashboard-error-heading">Dashboard unavailable</h2>
        <p className="error" role="alert">{error}</p>
        <div className="actions">
          <button onClick={() => setEpoch((v) => v + 1)}>Retry</button>
        </div>
      </section>
    );
  }
  if (!data) return <p>Loading your dashboard…</p>;
  return (
    <DashboardView
      workspace={workspace}
      data={data}
      larpOn={larpOn}
      onNavigate={onNavigate}
      onOpenProject={onOpenProject}
      onToggleLarp={() => toggleLarpMode()}
      onManageWorkspace={onManageWorkspace}
    />
  );
}
