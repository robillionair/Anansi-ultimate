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
  formatUptimeMs,
  projectExcerpt,
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
    uptime: '45m',
  });
  assert.ok(html.includes('Harbor App'));
  assert.ok(html.includes('Offline-first harbor logistics.'));
  assert.ok(html.includes('Current stage: Build'));
  assert.ok(html.includes('Continue Pipeline'));
  assert.ok(html.includes('aria-label="Dashboard projects"'));
});
