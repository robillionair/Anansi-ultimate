// LARP Mode Slice 3 unit tests: force layout, graph construction,
// spectrum mapping, and producer field mapping. Pure modules only —
// no GL objects are constructed, so these run in node.

import test from 'node:test';
import assert from 'node:assert/strict';
import { buildGraph, layoutGraph, latestCitedIds, MAX_GRAPH_NODES } from '../src/larp-mode/views/forceLayout.ts';
import { spectrumSeries, compressionPercent } from '../src/larp-mode/views/spectrum.ts';
import { AnansiTelemetryStream, sanitizeTelemetryEvent } from '../src/larp-mode/telemetry.ts';
import { reportAskRun, reportMemorySnapshot } from '../src/larp-mode/telemetryProducers.ts';
import type { MemoryNode } from '../src/larp-mode/telemetry.ts';

const nodes: MemoryNode[] = [
  { id: 'a', kind: 'memory', title: 'Alpha', relationships: ['b', 'missing'], confidence: 0.8 },
  { id: 'b', kind: 'note', title: 'Beta', relationships: [], confidence: null },
  { id: 'c', kind: 'memory', title: 'Gamma', relationships: ['a'], confidence: 0.4 },
];

test('graph matches relationships only to rendered nodes, plus a ring of layout links', () => {
  const graph = buildGraph(nodes, new Set(['b']));
  assert.equal(graph.nodes.length, 3);
  assert.deepEqual(
    graph.nodes.map((n) => n.cited),
    [false, true, false],
  );
  const rel = graph.edges.filter((e) => e.kind === 'relationship');
  assert.equal(rel.length, 2);
  assert.ok(rel.some((e) => (e.source === 'a' && e.target === 'b') || (e.source === 'b' && e.target === 'a')));
  assert.ok(rel.some((e) => (e.source === 'c' && e.target === 'a') || (e.source === 'a' && e.target === 'c')));
  const layout = graph.edges.filter((e) => e.kind === 'layout');
  assert.equal(layout.length, 3);
});

test('graph caps nodes at 64', () => {
  const many: MemoryNode[] = Array.from({ length: 80 }, (_, i) => ({
    id: `n${i}`,
    kind: 'memory' as const,
    title: `Node ${i}`,
  }));
  assert.equal(buildGraph(many, new Set()).nodes.length, MAX_GRAPH_NODES);
});

test('force layout is deterministic, finite, and bounded', () => {
  const graph = buildGraph(nodes, new Set());
  const first = layoutGraph(graph, 7);
  const second = layoutGraph(graph, 7);
  assert.deepEqual([...first.entries()], [...second.entries()]);
  for (const [, p] of first) {
    assert.ok(Number.isFinite(p.x) && Number.isFinite(p.y) && Number.isFinite(p.z));
    assert.ok(Math.hypot(p.x, p.y, p.z) <= 200);
  }
  const other = layoutGraph(graph, 8);
  assert.notDeepEqual([...other.entries()], [...first.entries()]);
});

test('cited ids collect from compilation events only', () => {
  const cited = latestCitedIds([
    { kind: 'compilation', citedIds: ['a', 'b'] },
    { kind: 'compilation' },
    { kind: 'drift' },
  ]);
  assert.deepEqual([...cited].sort(), ['a', 'b']);
});

test('compression percent math and clamping', () => {
  assert.equal(compressionPercent(200, 100, 0, 0), 50);
  assert.equal(compressionPercent(0, 0, 0, 0), 0);
  assert.equal(compressionPercent(100, 300, 0, 0), -100);
});

test('spectrum maps drift, compression, markers, and latest confidences', () => {
  const series = spectrumSeries([
    { kind: 'drift', at: 1, score: 0.25 },
    { kind: 'token-delta', at: 2, rawInputTokens: 200, compiledInputTokens: 100, rawOutputTokens: 0, compiledOutputTokens: 0, latencyMs: 5 },
    { kind: 'compilation', at: 3, label: 'ask-run-completed' },
    { kind: 'memory', at: 4, nodes },
  ]);
  assert.equal(series.drift.length, 1);
  assert.equal(series.drift[0].y, 0.25);
  assert.equal(series.compression.length, 1);
  assert.equal(series.compression[0].y, 50);
  assert.equal(series.markers.length, 1);
  assert.equal(series.markers[0].label, 'ask-run-completed');
  assert.deepEqual(series.confidences, [0.8, 0.4]);
  assert.ok(series.drift[0].x < series.compression[0].x);
  assert.ok(series.compression[0].x < series.markers[0].x);
});

test('spectrum of empty input is empty', () => {
  const series = spectrumSeries([]);
  assert.deepEqual(series, { drift: [], compression: [], markers: [], confidences: [] });
});

test('memory producer maps relationships and confidence', () => {
  const stream = new AnansiTelemetryStream();
  reportMemorySnapshot(
    [
      { id: 'a', kind: 'memory', title: 'Alpha', relationships: ['b'], confidence: 0.8 } as never,
      { id: 'b', kind: 'note', title: 'Beta', relationships: [], confidence: null } as never,
    ],
    stream,
  );
  const events = stream.snapshot();
  assert.equal(events.length, 1);
  assert.equal(events[0].kind, 'memory');
  if (events[0].kind !== 'memory') throw new Error('expected memory');
  assert.deepEqual(events[0].nodes[0].relationships, ['b']);
  assert.equal(events[0].nodes[0].confidence, 0.8);
  assert.equal(events[0].nodes[1].confidence, null);
});

test('ask producer fills cited ids from citations; sanitize caps them', () => {
  const stream = new AnansiTelemetryStream();
  reportAskRun(
    'abcd',
    {
      status: 'answered',
      answer: 'wxyz',
      manifest: { estimatedTokens: 40 },
      citations: [{ memoryId: 'a' }, { memoryId: 'b' }],
    },
    10,
    stream,
  );
  const events = stream.snapshot();
  assert.equal(events[0].kind, 'compilation');
  if (events[0].kind !== 'compilation') throw new Error('expected compilation');
  assert.deepEqual(events[0].citedIds, ['a', 'b']);
  const dirty = sanitizeTelemetryEvent({
    kind: 'compilation',
    at: 1,
    label: 'x',
    citedIds: [...Array.from({ length: 60 }, (_, i) => `id${i}`), 42 as never],
  });
  assert.equal(dirty.kind, 'compilation');
  if (dirty.kind !== 'compilation') throw new Error('expected compilation');
  assert.equal(dirty.citedIds?.length, 50);
  assert.ok(dirty.citedIds?.every((id) => typeof id === 'string'));
});
