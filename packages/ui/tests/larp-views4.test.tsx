// LARP Mode Slice 4 unit tests: tokenization, alignment matrix,
// ask-tokens + collision producers, and beam math. Pure modules only
// — no GL objects are constructed, so these run in node.

import test from 'node:test';
import assert from 'node:assert/strict';
import { tokenizeWords, reportAskRun, reportCollision } from '../src/larp-mode/telemetryProducers.ts';
import { alignmentMatrix, latestAskTokens } from '../src/larp-mode/views/tokenAlignment.ts';
import { beamPoint, dopplerColor } from '../src/larp-mode/views/colliderField.ts';
import { AnansiTelemetryStream, sanitizeTelemetryEvent } from '../src/larp-mode/telemetry.ts';

test('tokenize lowercases, strips end punctuation, drops empties', () => {
  assert.deepEqual(tokenizeWords('Hello,  WORLD!  (test)'), ['hello', 'world', 'test']);
  assert.deepEqual(tokenizeWords(''), []);
});

test('alignment matrix matches case-insensitively after normalization', () => {
  const matrix = alignmentMatrix(['Hello', 'world'], ['hello', 'there', 'world']);
  assert.equal(matrix.rows.length, 2);
  assert.equal(matrix.cols.length, 3);
  assert.equal(matrix.matches[0][0], true);
  assert.equal(matrix.matches[0][1], false);
  assert.equal(matrix.matches[1][2], true);
  assert.equal(matrix.matchCount, 2);
});

test('alignment matrix caps rows at 32 and cols at 64', () => {
  const rows = Array.from({ length: 40 }, (_, i) => `r${i}`);
  const cols = Array.from({ length: 80 }, (_, i) => `c${i}`);
  const matrix = alignmentMatrix(rows, cols);
  assert.equal(matrix.rows.length, 32);
  assert.equal(matrix.cols.length, 64);
});

test('latest ask tokens picks the newest ask-tokens event', () => {
  const latest = latestAskTokens([
    { kind: 'ask-tokens', rawTokens: ['a'], compiledTokens: ['b'] },
    { kind: 'compilation' },
    { kind: 'ask-tokens', rawTokens: ['c'], compiledTokens: ['d'] },
  ]);
  assert.deepEqual(latest, { rawTokens: ['c'], compiledTokens: ['d'] });
  assert.equal(latestAskTokens([{ kind: 'compilation' }]), null);
});

test('ask producer pushes ask-tokens only when a compiled prompt exists', () => {
  const stream = new AnansiTelemetryStream();
  reportAskRun(
    'Which database powers Aurora?',
    {
      status: 'answered',
      answer: 'SQLite.',
      manifest: { estimatedTokens: 40, compiledPrompt: 'You are ANANSI. Which database powers Aurora?' },
      citations: [],
    },
    10,
    stream,
  );
  const tokenEvents = stream.snapshot().filter((e) => e.kind === 'ask-tokens');
  assert.equal(tokenEvents.length, 1);
  if (tokenEvents[0].kind !== 'ask-tokens') throw new Error('expected ask-tokens');
  assert.ok(tokenEvents[0].rawTokens.includes('database'));
  assert.ok(tokenEvents[0].compiledTokens.includes('anansi'));

  const empty = new AnansiTelemetryStream();
  reportAskRun(
    'What is the capital of France?',
    {
      status: 'insufficientEvidence',
      answer: '',
      manifest: { estimatedTokens: 0, compiledPrompt: '' },
      citations: [],
    },
    5,
    empty,
  );
  assert.equal(empty.snapshot().filter((e) => e.kind === 'ask-tokens').length, 0);
});

test('collision producer truncates labels; sanitize caps tokens', () => {
  const stream = new AnansiTelemetryStream();
  reportCollision('a'.repeat(200), 'Beam B', stream);
  const events = stream.snapshot();
  assert.equal(events.length, 1);
  assert.equal(events[0].kind, 'collision');
  if (events[0].kind !== 'collision') throw new Error('expected collision');
  assert.equal(events[0].beamA.length, 80);
  assert.equal(events[0].beamB, 'Beam B');
  const dirty = sanitizeTelemetryEvent({
    kind: 'ask-tokens',
    at: 1,
    rawTokens: Array.from({ length: 40 }, (_, i) => `w${i}`),
    compiledTokens: Array.from({ length: 80 }, (_, i) => `c${i}`),
  });
  assert.equal(dirty.kind, 'ask-tokens');
  if (dirty.kind !== 'ask-tokens') throw new Error('expected ask-tokens');
  assert.equal(dirty.rawTokens.length, 32);
  assert.equal(dirty.compiledTokens.length, 64);
});

test('beam endpoints converge at the origin and stay bounded', () => {
  for (const beam of ['A', 'B'] as const) {
    const start = beamPoint(beam, 0);
    const end = beamPoint(beam, 1);
    assert.deepEqual(end, { x: 0, y: 0, z: 0 });
    assert.ok(Math.abs(start.x) > 100);
    const mid = beamPoint(beam, 0.5);
    assert.ok(Number.isFinite(mid.x) && Number.isFinite(mid.y) && Number.isFinite(mid.z));
    assert.ok(Math.hypot(mid.x, mid.y, mid.z) < 150);
  }
});

test('doppler color starts at the beam color and ends white-hot', () => {
  assert.deepEqual(dopplerColor('A', 0), [0, 0.94, 1]);
  assert.deepEqual(dopplerColor('A', 1), [1, 1, 1]);
  assert.deepEqual(dopplerColor('B', 1), [1, 1, 1]);
  const mid = dopplerColor('B', 0.5);
  assert.ok(mid[0] === 1 && mid[1] > 0.54 && mid[1] < 1 && mid[2] > 0.24 && mid[2] < 1);
});
