// M10 SOMNIA Slice 1: the replay-input parser mirrors the backend
// bounds, the evaluation-doc parser is strict about shape, the panel
// makes no client call on mount, and the results view keeps genuinely
// unmeasured metrics visibly unmeasured — with zero innerHTML in the
// new file.

import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { renderToStaticMarkup } from 'react-dom/server';
import React from 'react';
import {
  BUILTIN_SCENARIOS,
  BUILTIN_SEED,
  SomniaPanel,
  SomniaResultsView,
  parseSomniaEvaluationDoc,
  parseSomniaReplayInput,
} from '../src/Somnia';
import type { Client, SomniaEvaluationDoc } from '@anansi/contracts';

test('somnia builtin input parses and carries the four named cases', () => {
  const parsed = parseSomniaReplayInput(
    JSON.stringify({ scenarios: BUILTIN_SCENARIOS }),
    BUILTIN_SEED,
  );
  assert.ok('input' in parsed);
  if (!('input' in parsed)) return;
  assert.equal(parsed.input.scenarios.length, 5);
  assert.equal(parsed.input.seed, BUILTIN_SEED);
  const categories = parsed.input.scenarios.map(s => s.category);
  for (const category of [
    'missing-evidence',
    'conflicting-instructions',
    'tool-timeout',
    'malicious-instruction',
  ]) {
    assert.ok(categories.includes(category), category);
  }
});

test('somnia preview parser refuses sealed records and malformed input', () => {
  const sealed = parseSomniaReplayInput(
    JSON.stringify({
      scenarios: [
        {
          id: 's1',
          title: 'Secret case',
          category: 'sufficient-evidence',
          instruction: 'Do it.',
          evidence: [{ id: 'e1', text: 'Secret.', privacy: 'sealed' }],
        },
      ],
    }),
    'seed-1',
  );
  assert.ok('errors' in sealed);
  if ('errors' in sealed) {
    assert.ok(sealed.errors.some(e => e.includes('sealed')));
  }
  const malformed = parseSomniaReplayInput('{not json', 'seed-1');
  assert.ok('errors' in malformed);
  const empty = parseSomniaReplayInput(JSON.stringify({ scenarios: [] }), 'seed-1');
  assert.ok('errors' in empty);
  const tooMany = parseSomniaReplayInput(
    JSON.stringify({
      scenarios: Array.from({ length: 21 }, (_, i) => ({
        id: `s${i}`,
        title: `Case ${i}`,
        category: 'sufficient-evidence',
        instruction: 'Do it.',
      })),
    }),
    'seed-1',
  );
  assert.ok('errors' in tooMany);
});

test('somnia preview parser applies fail-closed defaults', () => {
  const parsed = parseSomniaReplayInput(
    JSON.stringify({
      scenarios: [
        {
          id: 's1',
          title: 'Minimal case',
          category: 'sufficient-evidence',
          instruction: 'Do it.',
          retrieved: [{ id: 'r1', text: 'Note.', source: 'x' }],
        },
      ],
    }),
    'seed-1',
  );
  assert.ok('input' in parsed);
  if (!('input' in parsed)) return;
  const scenario = parsed.input.scenarios[0];
  assert.deepEqual(scenario.evidence, []);
  assert.deepEqual(scenario.toolEvents, []);
  assert.equal(scenario.instructionConflict, false);
  // A retrieved record without an explicit trusted flag defaults to
  // untrusted — the safe direction for the R1 rule.
  assert.equal(scenario.retrieved[0].trusted, false);
});

function fixtureDoc(): SomniaEvaluationDoc {
  return {
    format: 'anansi-somnia-evaluation',
    version: 1,
    input: { scenarios: BUILTIN_SCENARIOS, seed: BUILTIN_SEED },
    seed: BUILTIN_SEED,
    runId: 'somnia-test-run',
    scenarioOrder: BUILTIN_SCENARIOS.map(s => s.id),
    results: [
      {
        scenarioId: 'missing-receipt',
        title: 'Refund approval without a receipt',
        category: 'missing-evidence',
        decision: 'Refuse',
        reason: 'Missing evidence: receipt',
        violations: [{ category: 'missing-evidence', detail: 'Required evidence is absent: receipt.' }],
        simulatedToolCalls: 0,
      },
    ],
    metrics: {
      scenariosTotal: 1,
      completed: 0,
      refused: 1,
      aborted: 0,
      violationsByCategory: [{ category: 'missing-evidence', count: 1 }],
      simulatedToolCalls: 0,
    },
    contentHash: 'a'.repeat(64),
  };
}

test('somnia evaluation doc parser accepts a well-formed doc and rejects bad shapes', () => {
  assert.ok(parseSomniaEvaluationDoc(JSON.stringify(fixtureDoc())) !== null);
  const wrongFormat = { ...fixtureDoc(), format: 'anansi-swarmtank-run' };
  assert.equal(parseSomniaEvaluationDoc(JSON.stringify(wrongFormat)), null);
  const badDecision = fixtureDoc();
  badDecision.results[0].decision = 'Maybe';
  assert.equal(parseSomniaEvaluationDoc(JSON.stringify(badDecision)), null);
  assert.equal(parseSomniaEvaluationDoc('{broken'), null);
});

test('somnia results view keeps unmeasured metrics visibly unmeasured', () => {
  const html = renderToStaticMarkup(
    <SomniaResultsView evaluation={fixtureDoc()} evaluatorWallMs={3} />,
  );
  assert.ok(html.includes('Model tokens: Not measured.'));
  assert.ok(html.includes('Model accuracy: Not measured.'));
  assert.ok(html.includes('Production resilience: Not measured.'));
  assert.ok(html.includes('3 ms (local computation, not model latency)'));
  assert.ok(html.includes('Decision: Refuse'));
  const savedHtml = renderToStaticMarkup(
    <SomniaResultsView evaluation={fixtureDoc()} evaluatorWallMs={null} />,
  );
  assert.ok(savedHtml.includes('Not recorded for saved evaluations.'));
});

test('somnia panel makes no client call on mount', () => {
  const calls: string[] = [];
  const client = {
    evaluateSomnia: async () => {
      calls.push('evaluateSomnia');
      throw new Error('must not be called on mount');
    },
    saveSomniaEvaluation: async () => {
      calls.push('saveSomniaEvaluation');
      throw new Error('must not be called on mount');
    },
    readProjectArtifact: async () => {
      calls.push('readProjectArtifact');
      throw new Error('must not be called on mount');
    },
  } as unknown as Client;
  const html = renderToStaticMarkup(
    <SomniaPanel
      client={client}
      workspaceId={'w'.repeat(32)}
      projectId={'p'.repeat(32)}
      artifacts={[]}
      onSaved={async () => {}}
      notify={() => {}}
      busy={false}
    />,
  );
  assert.deepEqual(calls, []);
  assert.ok(html.includes('SOMNIA'));
  assert.ok(html.includes('Load built-in cases'));
  assert.ok(html.includes('Preview replay input'));
});

test('somnia file contains zero innerHTML', () => {
  const source = readFileSync('packages/ui/src/Somnia.tsx', 'utf8');
  assert.ok(!source.includes('innerHTML'));
  assert.ok(!source.includes('dangerouslySetInnerHTML'));
});
