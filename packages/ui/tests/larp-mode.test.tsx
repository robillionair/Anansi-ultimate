// LARP Mode Slice 1 tests. Static markup cannot run effects, so click
// behavior and the hotkey listener are covered by e2e; here we cover
// toggle markup states, the hotkey predicate, store + disposal, frame
// throttling with a fake RAF, telemetry routing and validation, matrix
// mapping, velocity math, and HUD structure.

import test from 'node:test';
import assert from 'node:assert/strict';
import { renderToStaticMarkup } from 'react-dom/server';
import React from 'react';
import { LarpToggle } from '../src/larp-mode/LarpToggle.tsx';
import { LarpHud } from '../src/larp-mode/LarpHud.tsx';
import { MemoryMatrix, matrixCellsForNodes, latestMemoryNodes } from '../src/larp-mode/MemoryMatrix.tsx';
import { TokenDeltaStream, compressionDelta, type TokenDeltaEvent } from '../src/larp-mode/TokenDeltaStream.tsx';
import { tokenVelocitySeries } from '../src/larp-mode/CoherenceWaveform.tsx';
import { matchesLarpHotkey } from '../src/larp-mode/hotkey.ts';
import {
  setLarpMode,
  toggleLarpMode,
  getLarpMode,
  registerDisposer,
  disposerCount,
} from '../src/larp-mode/store.ts';
import { createFrameScheduler } from '../src/larp-mode/frameScheduler.ts';
import {
  AnansiTelemetryStream,
  TELEMETRY_RING_LIMIT,
  type TelemetryEvent,
} from '../src/larp-mode/telemetry.ts';

function tokenEvent(at: number, raw: number, compiled: number): TokenDeltaEvent {
  return {
    kind: 'token-delta',
    at,
    rawInputTokens: raw,
    compiledInputTokens: compiled,
    rawOutputTokens: 0,
    compiledOutputTokens: 0,
    latencyMs: 12,
  };
}

// --- Toggle markup states ---

test('toggle OFF renders calm chrome state', () => {
  setLarpMode(false);
  const html = renderToStaticMarkup(<LarpToggle />);
  assert.ok(html.includes('[ LARP: OFF ]'));
  assert.ok(html.includes('aria-pressed="false"'));
  assert.ok(html.includes('larp-toggle-off'));
  assert.ok(!html.includes('larp-toggle-led-on'));
});

test('toggle ON renders active RGB state', () => {
  setLarpMode(true);
  const html = renderToStaticMarkup(<LarpToggle />);
  assert.ok(html.includes('[ LARP: ACTIVE ]'));
  assert.ok(html.includes('aria-pressed="true"'));
  assert.ok(html.includes('larp-toggle-on'));
  assert.ok(html.includes('larp-toggle-led-on'));
  setLarpMode(false);
});

// --- Hotkey predicate ---

test('hotkey matches only Ctrl+Shift+L', () => {
  assert.equal(matchesLarpHotkey({ ctrlKey: true, shiftKey: true, key: 'L' }), true);
  assert.equal(matchesLarpHotkey({ ctrlKey: true, shiftKey: true, key: 'l' }), true);
  assert.equal(matchesLarpHotkey({ ctrlKey: false, shiftKey: true, key: 'L' }), false);
  assert.equal(matchesLarpHotkey({ ctrlKey: true, shiftKey: false, key: 'L' }), false);
  assert.equal(matchesLarpHotkey({ ctrlKey: true, shiftKey: true, key: 'K' }), false);
  assert.equal(matchesLarpHotkey({ ctrlKey: true, shiftKey: true, key: 'Control' }), false);
});

// --- Store toggle + disposal registry ---

test('store toggle flips and disposal runs on OFF only', () => {
  setLarpMode(false);
  let disposed = 0;
  let throwingRan = false;
  registerDisposer(() => {
    disposed++;
  });
  registerDisposer(() => {
    throwingRan = true;
    throw new Error('disposer boom');
  });
  assert.equal(disposerCount(), 2);
  setLarpMode(true);
  assert.equal(getLarpMode(), true);
  assert.equal(disposed, 0);
  toggleLarpMode();
  assert.equal(getLarpMode(), false);
  assert.equal(disposed, 1);
  assert.equal(throwingRan, true);
  assert.equal(disposerCount(), 0);
  // A second OFF is a no-op: no double-dispose.
  setLarpMode(false);
  assert.equal(disposed, 1);
});

// --- Frame scheduler with fake RAF ---

class FakeRaf {
  nowMs = 0;
  private nextId = 1;
  private pending = new Map<number, (time: number) => void>();

  request = (callback: (time: number) => void): number => {
    const id = this.nextId++;
    this.pending.set(id, callback);
    return id;
  };

  cancel = (id: number): void => {
    this.pending.delete(id);
  };

  /** Run every callback pending at flush start, at time t. */
  flush(t: number): void {
    const callbacks = [...this.pending.values()];
    this.pending.clear();
    for (const callback of callbacks) callback(t);
  }
}

test('scheduler ticks every frame while active, throttles when idle, halts when hidden or stopped', () => {
  const raf = new FakeRaf();
  let ticks = 0;
  const scheduler = createFrameScheduler(
    { requestFrame: raf.request, cancelFrame: raf.cancel, now: () => raf.nowMs },
    () => {
      ticks++;
    },
  );
  scheduler.start();
  assert.equal(scheduler.fps(), 60);
  for (let t = 0; t < 160; t += 16) raf.flush(t);
  assert.equal(ticks, 10);
  // Idle: jump past the 5 s threshold, then step 1 ms at a time.
  raf.nowMs = 6000;
  assert.equal(scheduler.fps(), 24);
  ticks = 0;
  for (let t = 6000; t <= 7000; t += 1) raf.flush(t);
  assert.ok(ticks >= 23 && ticks <= 25, `expected ~24 idle ticks, got ${ticks}`);
  // Hidden: zero ticks, fps 0, and resume works.
  scheduler.setHidden(true);
  assert.equal(scheduler.fps(), 0);
  ticks = 0;
  for (let t = 8000; t <= 8100; t += 1) raf.flush(t);
  assert.equal(ticks, 0);
  scheduler.setHidden(false);
  raf.flush(9000);
  assert.ok(ticks >= 1);
  // Stopped: no further ticks even as time advances.
  scheduler.stop();
  ticks = 0;
  for (let t = 10000; t <= 10100; t += 1) raf.flush(t);
  assert.equal(ticks, 0);
  assert.equal(scheduler.isRunning(), false);
});

// --- Telemetry routing + validation ---

test('telemetry routes pushed events to subscribers in order', () => {
  const stream = new AnansiTelemetryStream();
  const seen: TelemetryEvent[] = [];
  const off = stream.subscribe((event) => seen.push(event));
  stream.push(tokenEvent(1, 100, 40));
  stream.push({ kind: 'drift', at: 2, score: 0.125 });
  assert.equal(seen.length, 2);
  assert.equal(seen[0].kind, 'token-delta');
  assert.equal((seen[1] as { score: number }).score, 0.125);
  assert.equal(stream.snapshot().length, 2);
  off();
  stream.push(tokenEvent(3, 50, 20));
  assert.equal(seen.length, 2);
});

test('telemetry ring keeps only the newest 500 events', () => {
  const stream = new AnansiTelemetryStream();
  for (let i = 0; i < TELEMETRY_RING_LIMIT + 5; i++) stream.push(tokenEvent(i, 10, 5));
  const snap = stream.snapshot();
  assert.equal(snap.length, TELEMETRY_RING_LIMIT);
  assert.equal(snap[0].at, 5);
  assert.equal(snap[snap.length - 1].at, TELEMETRY_RING_LIMIT + 4);
});

test('telemetry sanitizes drift and token counts instead of throwing', () => {
  const stream = new AnansiTelemetryStream();
  stream.push({ kind: 'drift', at: 1, score: 1.5 });
  stream.push({ kind: 'drift', at: 2, score: -0.2 });
  stream.push({ kind: 'drift', at: 3, score: Number.NaN });
  stream.push(tokenEvent(4, -5, -1));
  const snap = stream.snapshot();
  assert.equal((snap[0] as { score: number }).score, 1);
  assert.equal((snap[1] as { score: number }).score, 0);
  assert.equal((snap[2] as { score: number }).score, 0);
  const last = snap[3] as Extract<TelemetryEvent, { kind: 'token-delta' }>;
  assert.equal(last.rawInputTokens, 0);
  assert.equal(last.compiledInputTokens, 0);
});

// --- Matrix mapping ---

test('matrix maps node kinds to colors and defaults to evicted', () => {
  const cells = matrixCellsForNodes([
    { id: 'a', kind: 'memory', title: 'Rule' },
    { id: 'b', kind: 'note', title: 'Note' },
    { id: 'c', kind: 'artifact', title: 'Spec' },
  ]);
  assert.equal(cells[0], 'pinned');
  assert.equal(cells[1], 'active');
  assert.equal(cells[2], 'delta');
  assert.ok(cells.slice(3).every((cell) => cell === 'evicted'));
  assert.deepEqual(matrixCellsForNodes([]).slice(0, 4), ['evicted', 'evicted', 'evicted', 'evicted']);
  const nodes = latestMemoryNodes([
    { kind: 'memory', at: 1, nodes: [{ id: 'x', kind: 'note', title: 'Old' }] },
    tokenEvent(2, 10, 5),
    { kind: 'memory', at: 3, nodes: [{ id: 'y', kind: 'artifact', title: 'New' }] },
  ]);
  assert.deepEqual(nodes, [{ id: 'y', kind: 'artifact', title: 'New' }]);
});

// --- Delta + velocity math ---

test('compression delta and velocity series handle edges', () => {
  assert.equal(compressionDelta(tokenEvent(0, 100, 40)), '-60.0%');
  assert.equal(compressionDelta(tokenEvent(0, 0, 0)), 'n/a');
  const series = tokenVelocitySeries([
    tokenEvent(0, 0, 0),
    tokenEvent(1000, 1000, 1000),
    tokenEvent(2000, 3000, 3000),
  ]);
  assert.deepEqual(series.raw, [1000, 2000]);
  assert.deepEqual(series.compiled, [1000, 2000]);
  const sameTime = tokenVelocitySeries([tokenEvent(5, 0, 0), tokenEvent(5, 500, 250)]);
  assert.ok(Number.isFinite(sameTime.raw[0]));
  assert.deepEqual(tokenVelocitySeries([tokenEvent(0, 10, 5)]), { raw: [], compiled: [] });
});

// --- Widget + HUD static markup ---

test('HUD renders frame regions, widgets, view buttons 1-6, and empty states', () => {
  const stream = new AnansiTelemetryStream();
  const html = renderToStaticMarkup(<LarpHud stream={stream} />);
  assert.ok(html.includes('aria-label="LARP telemetry frame"'));
  assert.ok(html.includes('L1 / L2 / L3 Memory Allocation'));
  assert.ok(html.includes('Token Compression Delta Stream'));
  assert.ok(html.includes('Coherence Waveforms'));
  assert.ok(html.includes('No telemetry events yet.'));
  assert.ok(html.includes('No memory snapshot yet.'));
  for (let n = 1; n <= 6; n++) assert.ok(html.includes(`aria-label="LARP view ${n}"`));
  assert.ok(html.includes('arrives in Slice 2.'));
});

test('HUD renders pushed events instead of empty states', () => {
  const stream = new AnansiTelemetryStream();
  stream.push(tokenEvent(1, 200, 50));
  stream.push({ kind: 'memory', at: 2, nodes: [{ id: 'm', kind: 'memory', title: 'Rule' }] });
  const html = renderToStaticMarkup(<LarpHud stream={stream} />);
  assert.ok(!html.includes('No telemetry events yet.'));
  assert.ok(html.includes('200 → 50'));
  assert.ok(html.includes('-75.0%'));
  assert.ok(html.includes('12 ms'));
});

test('matrix and delta widgets render their shells', () => {
  const matrix = renderToStaticMarkup(<MemoryMatrix nodes={[]} />);
  assert.ok(matrix.includes('Memory allocation matrix'));
  assert.ok(matrix.includes('Pinned Rules'));
  const delta = renderToStaticMarkup(<TokenDeltaStream events={[]} />);
  assert.ok(delta.includes('Token Compression Delta Stream'));
});
