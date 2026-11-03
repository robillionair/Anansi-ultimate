// HOME Command Dashboard Slice 1 tests. Static markup cannot run
// effects, so the container's loading is covered by e2e; here we cover
// the pure helpers and the presentational DashboardView structure.

import test from 'node:test';
import assert from 'node:assert/strict';
import { renderToStaticMarkup } from 'react-dom/server';
import React from 'react';
import {
  DashboardView,
  countTelemetry,
  deriveQueueStatus,
  formatBackupInterval,
  formatBackupStatus,
  formatUptimeMs,
  latestArtifactTimestamp,
  projectExcerpt,
  sortProjectsByRecency,
  stageBadge,
  type DashboardData,
} from '../src/Dashboard.tsx';
import { derivePipeline } from '../src/lifecycle.ts';

test('formatUptimeMs renders seconds, minutes, hours, days', () => {
  assert.equal(formatUptimeMs(0), '0s');
  assert.equal(formatUptimeMs(45000), '45s');
  assert.equal(formatUptimeMs(60000), '1m');
  assert.equal(formatUptimeMs(3600000), '1h');
  assert.equal(formatUptimeMs(7380000), '2h 3m');
  assert.equal(formatUptimeMs(90000000), '1d 1h');
  assert.equal(formatUptimeMs(-5), '0s');
});

test('projectExcerpt falls back and truncates', () => {
  assert.equal(projectExcerpt(null), 'No description yet.');
  assert.equal(projectExcerpt('   '), 'No description yet.');
  assert.equal(projectExcerpt('  Hello world  '), 'Hello world');
  const long = 'x'.repeat(200);
  assert.equal(projectExcerpt(long).length, 158);
  assert.ok(projectExcerpt(long).endsWith('…'));
});

test('countTelemetry separates memory nodes, active notes, relationships', () => {
  const memories = [
    { kind: 'memory', status: 'durable' },
    { kind: 'memory', status: 'archived' },
    { kind: 'note', status: 'durable' },
    { kind: 'note', status: 'archived' },
    { kind: 'note', status: 'superseded' },
    { kind: 'project', status: 'durable' },
    { kind: 'task', status: 'durable' },
  ] as never[];
  assert.deepEqual(countTelemetry(memories, 7), {
    memoryNodes: 2,
    activeNotes: 1,
    relationships: 7,
  });
});

test('stageBadge names the current stage or pipeline completion', () => {
  assert.equal(stageBadge(derivePipeline([])), 'Current stage: Discover');
  assert.equal(stageBadge(derivePipeline(['concept'])), 'Current stage: Build');
  assert.equal(
    stageBadge(derivePipeline(['concept', 'specification', 'simulation', 'somnia-evaluation', 'launch-material'])),
    'Pipeline complete',
  );
});

const workspace = {
  contractVersion: 1,
  schemaVersion: 11,
  settings: { intent: 'explore', onboardingComplete: true, reducedMotion: false },
  workspace: { id: 'ws-1', name: 'Aurora Workspace' },
} as never;

function render(data: DashboardData): string {
  return renderToStaticMarkup(
    React.createElement(DashboardView, {
      workspace,
      data,
      larpOn: false,
      onNavigate: () => {},
      onOpenProject: () => {},
      onToggleLarp: () => {},
      onManageWorkspace: () => {},
    }),
  );
}

test('DashboardView renders overview, telemetry, and quick actions', () => {
  const html = render({
    telemetry: { memoryNodes: 3, activeNotes: 2, relationships: 5 },
    projects: [],
    totalProjects: 0,
    backupStatus: 'Manual backups only',
    queue: { idle: true, activeCount: 0, latestType: null },
    uptime: '2h 3m',
  });
  assert.ok(html.includes('Aurora Workspace'));
  assert.ok(html.includes('ws-1'));
  assert.ok(html.includes('Ready · core up 2h 3m'));
  assert.ok(html.includes('Memory nodes'));
  assert.ok(html.includes('Active notes'));
  assert.ok(html.includes('Recorded relationships'));
  assert.ok(html.includes('role="region"'));
  assert.ok(html.includes('aria-labelledby="dashboard-overview-heading"'));
  assert.ok(html.includes('+ New Project'));
  assert.ok(html.includes('Ask ANANSI'));
  assert.ok(html.includes('LEXI-COLLIDER'));
  assert.ok(html.includes('LARP Telemetry Engine'));
  assert.ok(html.includes('Manage workspace'));
});

test('DashboardView renders the empty-state invitation with no projects', () => {
  const html = render({
    telemetry: { memoryNodes: 0, activeNotes: 0, relationships: 0 },
    projects: [],
    totalProjects: 0,
    backupStatus: 'Manual backups only',
    queue: { idle: true, activeCount: 0, latestType: null },
    uptime: null,
  });
  assert.ok(html.includes('No projects yet'));
  assert.ok(html.includes('create your first project'));
  assert.ok(!html.includes('Continue Pipeline'));
  assert.ok(html.includes('>Ready<'));
});

test('DashboardView renders project cards with excerpts and stage badges', () => {
  const html = render({
    telemetry: { memoryNodes: 1, activeNotes: 0, relationships: 0 },
    projects: [
      {
        id: 'p-1',
        title: 'Harbor App',
        excerpt: 'Offline-first harbor logistics.',
        stageBadge: 'Current stage: Build',
      },
    ],
    totalProjects: 1,
    backupStatus: 'Auto-backup every 24h',
    queue: { idle: false, activeCount: 1, latestType: 'Workspace export' },
    uptime: '45m',
  });
  assert.ok(html.includes('Harbor App'));
  assert.ok(html.includes('Offline-first harbor logistics.'));
  assert.ok(html.includes('Current stage: Build'));
  assert.ok(html.includes('Continue Pipeline'));
  assert.ok(html.includes('aria-label="Dashboard projects"'));
});

test('formatBackupInterval renders days, hours, minutes, seconds', () => {
  assert.equal(formatBackupInterval(86400), '1d');
  assert.equal(formatBackupInterval(172800), '2d');
  assert.equal(formatBackupInterval(3600), '1h');
  assert.equal(formatBackupInterval(43200), '12h');
  assert.equal(formatBackupInterval(1800), '30m');
  assert.equal(formatBackupInterval(90), '90s');
  assert.equal(formatBackupInterval(0), '0s');
});

test('formatBackupStatus distinguishes scheduled from manual', () => {
  assert.equal(
    formatBackupStatus({ enabled: true, intervalSeconds: 86400 }),
    'Auto-backup every 1d',
  );
  assert.equal(
    formatBackupStatus({ enabled: true, intervalSeconds: 43200 }),
    'Auto-backup every 12h',
  );
  assert.equal(
    formatBackupStatus({ enabled: false, intervalSeconds: 86400 }),
    'Manual backups only',
  );
});

const job = (overrides: Record<string, unknown>) =>
  ({
    id: 'j-1',
    checkpoint: '',
    checkpointVersion: 0,
    createdAt: '2026-11-04T00:00:00.000Z',
    log: '',
    maxBytes: 0,
    maxRecords: 0,
    progress: 0,
    selection: { kind: 'workspace' },
    state: 'Completed',
    updatedAt: '2026-11-04T00:00:00.000Z',
    version: 1,
    workspaceId: 'ws-1',
    ...overrides,
  }) as never;

test('deriveQueueStatus is idle without Queued or Running jobs', () => {
  assert.deepEqual(deriveQueueStatus([]), { idle: true, activeCount: 0, latestType: null });
  assert.deepEqual(
    deriveQueueStatus([job({ state: 'Completed' }), job({ state: 'Paused' }), job({ state: 'Failed' })]),
    { idle: true, activeCount: 0, latestType: null },
  );
});

test('deriveQueueStatus counts active jobs and names the latest type', () => {
  const jobs = [
    job({ id: 'old', state: 'Running', createdAt: '2026-11-04T00:00:00.000Z', selection: { kind: 'workspace' } }),
    job({ id: 'new', state: 'Queued', createdAt: '2026-11-04T02:00:00.000Z', selection: { kind: 'project' } }),
    job({ id: 'done', state: 'Completed', createdAt: '2026-11-04T03:00:00.000Z', selection: { kind: 'records' } }),
    job({ id: 'app', state: 'Queued', createdAt: '2026-11-04T01:00:00.000Z', selection: { kind: 'backup', application: true } }),
  ];
  assert.deepEqual(deriveQueueStatus(jobs), {
    idle: false,
    activeCount: 3,
    latestType: 'Project export',
  });
  assert.equal(
    deriveQueueStatus([job({ state: 'Running', selection: { kind: 'backup', application: false } })]).latestType,
    'Workspace backup',
  );
  assert.equal(
    deriveQueueStatus([job({ state: 'Running', selection: { kind: 'records', ids: [] } })]).latestType,
    'Selected records export',
  );
});

test('latestArtifactTimestamp picks the newest parseable timestamp', () => {
  assert.equal(latestArtifactTimestamp([]), null);
  assert.equal(latestArtifactTimestamp(['not-a-date']), null);
  assert.equal(
    latestArtifactTimestamp(['2026-11-01T00:00:00.000Z', '2026-11-03T00:00:00.000Z', 'junk']),
    '2026-11-03T00:00:00.000Z',
  );
});

test('sortProjectsByRecency orders by artifact time then update, ties keep order', () => {
  const projects = [
    { id: 'old', updatedAt: '2026-11-01T00:00:00.000Z', latestArtifactAt: null },
    { id: 'artifact-new', updatedAt: '2026-11-01T00:00:00.000Z', latestArtifactAt: '2026-11-04T00:00:00.000Z' },
    { id: 'updated-new', updatedAt: '2026-11-03T00:00:00.000Z', latestArtifactAt: null },
    { id: 'unparseable', updatedAt: 'garbage', latestArtifactAt: null },
    { id: 'tie-a', updatedAt: '2026-11-02T00:00:00.000Z', latestArtifactAt: null },
    { id: 'tie-b', updatedAt: '2026-11-02T00:00:00.000Z', latestArtifactAt: null },
  ];
  assert.deepEqual(
    sortProjectsByRecency(projects).map((p) => p.id),
    ['artifact-new', 'updated-new', 'tie-a', 'tie-b', 'old', 'unparseable'],
  );
});

test('DashboardView renders health and caps projects at five with a view-all link', () => {
  const six = Array.from({ length: 6 }, (_, i) => ({
    id: `p-${i}`,
    title: `Project ${i}`,
    excerpt: 'Excerpt.',
    stageBadge: 'Current stage: Discover',
  }));
  const html = render({
    telemetry: { memoryNodes: 0, activeNotes: 0, relationships: 0 },
    projects: six.slice(0, 5),
    totalProjects: 6,
    backupStatus: 'Auto-backup every 1d',
    queue: { idle: true, activeCount: 0, latestType: null },
    uptime: '1h',
  });
  assert.ok(html.includes('System &amp; storage health'));
  assert.ok(html.includes('Backup: Auto-backup every 1d'));
  assert.ok(html.includes('Background queue idle'));
  assert.ok(html.includes('View all in Projects'));
  assert.ok(html.includes('Project 4'));
  assert.ok(!html.includes('Project 5'));

  const five = render({
    telemetry: { memoryNodes: 0, activeNotes: 0, relationships: 0 },
    projects: six.slice(0, 5),
    totalProjects: 5,
    backupStatus: 'Manual backups only',
    queue: { idle: false, activeCount: 2, latestType: 'Workspace export' },
    uptime: '1h',
  });
  assert.ok(!five.includes('View all in Projects'));
  assert.ok(five.includes('2 active job(s) · latest: Workspace export'));
});
