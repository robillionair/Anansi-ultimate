// LARP Mode Slice 2 unit tests: particle layout, bokeh + post shaders,
// composer pass order, bezier math, pulse lifecycle, producers, and the
// estimated badge. Pure modules only — no GL objects are constructed,
// so these run in node. Click/key behavior is e2e-only, as established.

import test from 'node:test';
import assert from 'node:assert/strict';
import { renderToStaticMarkup } from 'react-dom/server';
import { createElement } from 'react';
import {
  clusterCentroids,
  generateParticles,
  mulberry32,
  PARTICLE_COUNT,
  CLUSTER_RADIUS,
} from '../src/larp-mode/views/particleLayout.ts';
import { BOKEH_VERTEX, BOKEH_FRAGMENT } from '../src/larp-mode/views/vaultShaders.ts';
import {
  chromaticAberrationShader,
  filmGrainShader,
  CHROMATIC_OFFSET,
  FILM_GRAIN_AMOUNT,
} from '../src/larp-mode/postprocessing/shaders.ts';
import {
  buildLarpPasses,
  clampPixelRatio,
  LARP_PASS_ORDER,
} from '../src/larp-mode/postprocessing/LarpComposer.ts';
import {
  cubicBezierPoint,
  filamentLayout,
  spawnPulse,
  advancePulses,
} from '../src/larp-mode/views/bezier.ts';
import { AnansiTelemetryStream } from '../src/larp-mode/telemetry.ts';
import { reportAskRun, reportMemorySnapshot, estimateTokens } from '../src/larp-mode/telemetryProducers.ts';
import { TokenDeltaStream } from '../src/larp-mode/TokenDeltaStream.tsx';

test('particle layout is deterministic for a seed and clusters around centroids', () => {
  const centroids = clusterCentroids([
    { id: 'a', kind: 'memory', title: 'Alpha' },
    { id: 'b', kind: 'note', title: 'Beta' },
  ]);
  assert.equal(centroids.length, 2);
  for (const c of centroids) {
    const shell = Math.sqrt(c.x * c.x + c.z * c.z + (c.y / 0.6) * (c.y / 0.6));
    assert.ok(Math.abs(shell - CLUSTER_RADIUS) < 1, `centroid on shell, got ${shell}`);
  }
  const first = generateParticles(7, centroids);
  const second = generateParticles(7, centroids);
  assert.equal(first.positions.length, PARTICLE_COUNT * 3);
  assert.deepEqual(first.positions, second.positions);
  assert.deepEqual(first.scales, second.scales);
  for (let i = 0; i < PARTICLE_COUNT; i++) {
    const x = first.positions[i * 3];
    const y = first.positions[i * 3 + 1];
    const z = first.positions[i * 3 + 2];
    const nearest = Math.min(
      ...centroids.map((c) => Math.hypot(x - c.x, y - c.y, z - c.z)),
    );
    assert.ok(nearest < 150, `particle ${i} near a centroid, got ${nearest}`);
  }
  assert.notDeepEqual(generateParticles(8, centroids).positions, first.positions);
});

test('empty nodes fall back to three centroids; PRNG is deterministic', () => {
  assert.equal(clusterCentroids([]).length, 3);
  const a = mulberry32(42);
  const b = mulberry32(42);
  assert.equal(a(), b());
});

test('bokeh shaders carry the focal-plane uniforms and soft-disc mask', () => {
  assert.ok(BOKEH_VERTEX.includes('uFocalDepth'));
  assert.ok(BOKEH_VERTEX.includes('gl_PointSize'));
  assert.ok(BOKEH_FRAGMENT.includes('gl_PointCoord'));
  assert.ok(BOKEH_FRAGMENT.includes('smoothstep'));
});

test('post shaders carry the specified defaults', () => {
  assert.equal(CHROMATIC_OFFSET, 0.0012);
  assert.equal(chromaticAberrationShader.uniforms.uOffset.value, 0.0012);
  assert.ok(chromaticAberrationShader.fragmentShader.includes('tDiffuse'));
  assert.equal(FILM_GRAIN_AMOUNT, 0.05);
  assert.equal(filmGrainShader.uniforms.uAmount.value, 0.05);
  assert.ok(filmGrainShader.fragmentShader.includes('snoise'));
});

test('composer pass order is render, bloom, aberration, grain, output', () => {
  assert.deepEqual(LARP_PASS_ORDER, ['render', 'bloom', 'aberration', 'grain', 'output']);
  const seen: string[] = [];
  const passes = buildLarpPasses((kind) => {
    seen.push(kind);
    return { kind };
  });
  assert.deepEqual(seen, LARP_PASS_ORDER);
  assert.equal(passes.length, 5);
});

test('pixel ratio clamps to 2 and rejects nonsense', () => {
  assert.equal(clampPixelRatio(1), 1);
  assert.equal(clampPixelRatio(2), 2);
  assert.equal(clampPixelRatio(3), 2);
  assert.equal(clampPixelRatio(0), 1);
  assert.equal(clampPixelRatio(NaN), 1);
});

test('bezier endpoints, midpoint, and funnel layout', () => {
  const p0 = { x: 0, y: 0 };
  const p1 = { x: 0, y: 100 };
  const p2 = { x: 100, y: 0 };
  const p3 = { x: 100, y: 100 };
  assert.deepEqual(cubicBezierPoint(p0, p1, p2, p3, 0), p0);
  assert.deepEqual(cubicBezierPoint(p0, p1, p2, p3, 1), p3);
  assert.deepEqual(cubicBezierPoint(p0, p1, p2, p3, 0.5), { x: 50, y: 50 });
  const layout = filamentLayout(800, 600);
  assert.equal(layout.inputs.length, 4);
  assert.equal(layout.outputs.length, 3);
  assert.deepEqual(layout.singularity, { x: 400, y: 300 });
  assert.deepEqual(layout.blocks.map((b) => b.label), ['L1', 'L2', 'L3']);
  assert.ok(layout.inputs.every((f) => f.points[3] === layout.singularity));
  assert.ok(layout.outputs.every((f) => f.points[0] === layout.singularity));
});

test('pulses advance and expire at the end of a filament', () => {
  const pulse = spawnPulse(1, 2);
  assert.equal(pulse.t, 0);
  const half = advancePulses([pulse], 1200);
  assert.equal(half.length, 1);
  assert.ok(Math.abs(half[0].t - 0.5) < 1e-9);
  assert.equal(half[0].filamentIndex, 2);
  assert.deepEqual(advancePulses(half, 1200), []);
});

test('ask producer: answered run pushes compilation + estimated token-delta', () => {
  const stream = new AnansiTelemetryStream();
  reportAskRun('abcd', { status: 'answered', answer: 'wxyz', manifest: { estimatedTokens: 40 } }, 1234, stream);
  const events = stream.snapshot();
  assert.equal(events.length, 2);
  assert.equal(events[0].kind, 'compilation');
  assert.equal(events[1].kind, 'token-delta');
  if (events[1].kind !== 'token-delta') throw new Error('expected token-delta');
  assert.equal(events[1].rawInputTokens, 1);
  assert.equal(events[1].compiledInputTokens, 40);
  assert.equal(events[1].rawOutputTokens, 1);
  assert.equal(events[1].compiledOutputTokens, 1);
  assert.equal(events[1].latencyMs, 1234);
  assert.equal(events[1].estimated, true);
  assert.equal(estimateTokens(9), 3);
});

test('ask producer: insufficient evidence pushes compilation only; null pushes nothing', () => {
  const stream = new AnansiTelemetryStream();
  reportAskRun('abcd', { status: 'insufficientEvidence', answer: '', manifest: { estimatedTokens: 0 } }, 10, stream);
  assert.equal(stream.snapshot().length, 1);
  reportAskRun('abcd', null, 10, stream);
  assert.equal(stream.snapshot().length, 1);
});

test('memory producer maps kinds and skips empty snapshots', () => {
  const stream = new AnansiTelemetryStream();
  reportMemorySnapshot([], stream);
  assert.equal(stream.snapshot().length, 0);
  reportMemorySnapshot(
    [
      { id: 'm1', kind: 'memory', title: 'Alpha' } as never,
      { id: 'n1', kind: 'note', title: 'Beta' } as never,
      { id: 'p1', kind: 'project', title: 'Gamma' } as never,
    ],
    stream,
  );
  const events = stream.snapshot();
  assert.equal(events.length, 1);
  assert.equal(events[0].kind, 'memory');
  if (events[0].kind !== 'memory') throw new Error('expected memory');
  assert.deepEqual(
    events[0].nodes.map((n) => n.kind),
    ['memory', 'note', 'memory'],
  );
});

test('delta stream badges estimated rows and leaves measured rows plain', () => {
  const html = renderToStaticMarkup(
    createElement(TokenDeltaStream, {
      events: [
        { kind: 'token-delta', at: 2, rawInputTokens: 10, compiledInputTokens: 4, rawOutputTokens: 0, compiledOutputTokens: 0, latencyMs: 5, estimated: true },
        { kind: 'token-delta', at: 1, rawInputTokens: 10, compiledInputTokens: 4, rawOutputTokens: 0, compiledOutputTokens: 0, latencyMs: 5 },
      ],
    }),
  );
  const estCount = (html.match(/est\./g) ?? []).length;
  assert.equal(estCount, 1);
});
