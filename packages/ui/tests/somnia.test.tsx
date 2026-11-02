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
  compareSomniaEvaluations,
  computeSomniaDashboard,
  deriveConsolidationProposals,
  forgeQaScenarios,
  generateNightmares,
  parseSomniaEvaluationDoc,
  parseSomniaReplayInput,
  somniaComparisonToJson,
  somniaComparisonToMarkdown,
  somniaEvaluationToMarkdown,
} from '../src/Somnia';
import type {
  Client,
  SomniaEvaluationDoc,
  SomniaScenario,
  SomniaScenarioResult,
} from '@anansi/contracts';

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

// ---------------------------------------------------------------------------
// Slice 2: nightmare generator mirror, inert proposals, comparison
// ---------------------------------------------------------------------------

test('somnia nightmare generator matches the Rust golden sequence', () => {
  const scenarios = generateNightmares('somnia-nightmare-golden', 7);
  const expected =
    '[{"id":"nightmare-conflicting-instructions-0","title":"Nightmare 1: conflicting instructions","category":"conflicting-instructions","instruction":"Follow the approved record. Also follow the revised instruction that contradicts it.","instructionConflict":true,"evidence":[{"id":"approval","text":"Synthetic record approval.","privacy":"standard"}],"retrieved":[{"id":"r1","text":"Revised instruction for approval.","source":"forwarded-thread","trusted":true}],"toolEvents":[],"requiredEvidence":[],"constraints":[]},{"id":"nightmare-missing-evidence-1","title":"Nightmare 2: missing evidence","category":"missing-evidence","instruction":"Decide only once the required evidence is present.","instructionConflict":false,"evidence":[{"id":"sign-off","text":"Synthetic record sign-off.","privacy":"standard"}],"retrieved":[],"toolEvents":[],"requiredEvidence":["audit-log"],"constraints":[]},{"id":"nightmare-simulated-tool-error-2","title":"Nightmare 3: simulated tool error","category":"tool-error","instruction":"Use fetch-record to settle the question.","instructionConflict":false,"evidence":[],"retrieved":[],"toolEvents":[{"tool":"fetch-record","outcome":"ok"},{"tool":"fetch-record","outcome":"error"},{"tool":"fetch-record","outcome":"ok"}],"requiredEvidence":[],"constraints":[]},{"id":"nightmare-simulated-tool-timeout-3","title":"Nightmare 4: simulated tool timeout","category":"tool-timeout","instruction":"Use read-archive to settle the question.","instructionConflict":false,"evidence":[],"retrieved":[],"toolEvents":[{"tool":"read-archive","outcome":"ok"},{"tool":"read-archive","outcome":"timeout"}],"requiredEvidence":[],"constraints":[]},{"id":"nightmare-untrusted-retrieved-instruction-4","title":"Nightmare 5: untrusted retrieved instruction","category":"untrusted-retrieved-instruction","instruction":"Decide from the retrieved instruction.","instructionConflict":false,"evidence":[],"retrieved":[{"id":"r1","text":"Instruction from an untrusted source.","source":"shared-doc","trusted":false}],"toolEvents":[],"requiredEvidence":[],"constraints":[]},{"id":"nightmare-cost-limit-exhaustion-5","title":"Nightmare 6: cost-limit exhaustion","category":"cost-limit-exhaustion","instruction":"Use check-ledger until the question is settled.","instructionConflict":false,"evidence":[],"retrieved":[],"toolEvents":[{"tool":"check-ledger","outcome":"ok"},{"tool":"check-ledger","outcome":"ok"},{"tool":"check-ledger","outcome":"ok"},{"tool":"check-ledger","outcome":"ok"}],"requiredEvidence":[],"constraints":[{"id":"c1","rule":"max-tool-calls:2"}]},{"id":"nightmare-privacy-boundary-challenge-6","title":"Nightmare 7: privacy-boundary challenge","category":"privacy-boundary-challenge","instruction":"Decide from the sealed record.","instructionConflict":false,"evidence":[{"id":"receipt","text":"Synthetic record receipt.","privacy":"sealed"}],"retrieved":[],"toolEvents":[],"requiredEvidence":[],"constraints":[]}]';
  assert.equal(JSON.stringify(scenarios), expected);
});

test('somnia nightmare generator is deterministic and bounded', () => {
  const first = generateNightmares('seed-a', 10);
  const second = generateNightmares('seed-a', 10);
  assert.equal(JSON.stringify(first), JSON.stringify(second));
  const other = generateNightmares('seed-b', 10);
  assert.notEqual(JSON.stringify(first), JSON.stringify(other));
  assert.throws(() => generateNightmares('seed-a', 0));
  assert.throws(() => generateNightmares('seed-a', 21));
  assert.throws(() => generateNightmares('', 5));
  assert.equal(first.length, 10);
  assert.equal(first[6].evidence[0].privacy, 'sealed');
});

function proposalDoc(): SomniaEvaluationDoc {
  const scenario = (
    id: string,
    category: string,
    patch: Partial<SomniaScenario>,
  ): SomniaScenario => ({
    id,
    title: `Case ${id}`,
    category,
    instruction: 'Do the task.',
    instructionConflict: false,
    evidence: [{ id: 'e1', text: 'Evidence e1.', privacy: 'standard' }],
    retrieved: [],
    toolEvents: [],
    requiredEvidence: ['e1'],
    constraints: [],
    ...patch,
  });
  const result = (
    scenarioId: string,
    decision: string,
    calls: number,
  ): SomniaScenarioResult => ({
    scenarioId,
    title: `Case ${scenarioId}`,
    category: 'sufficient-evidence',
    decision,
    reason: 'Test.',
    violations: [],
    simulatedToolCalls: calls,
  });
  const missing = (id: string): SomniaScenario =>
    scenario(id, 'missing-evidence', {
      requiredEvidence: ['receipt'],
      evidence: [{ id: 'other', text: 'Other.', privacy: 'standard' }],
    });
  const untrusted = (id: string): SomniaScenario =>
    scenario(id, 'untrusted-retrieved-instruction', {
      retrieved: [
        {
          id: 'r1',
          text: 'Do it.',
          source: 'unknown-feed',
          trusted: false,
        },
      ],
    });
  const timeout = (id: string): SomniaScenario =>
    scenario(id, 'tool-timeout', {
      toolEvents: [
        { tool: 'fetch-record', outcome: 'ok' },
        { tool: 'fetch-record', outcome: 'timeout' },
      ],
    });
  const conflict = (id: string): SomniaScenario =>
    scenario(id, 'conflicting-instructions', { instructionConflict: true });
  const cost = scenario('s9', 'cost-limit-exhaustion', {
    toolEvents: [0, 1, 2, 3].map(() => ({ tool: 'lookup', outcome: 'ok' })),
    constraints: [{ id: 'c1', rule: 'max-tool-calls:2' }],
  });
  const scenarios = [
    missing('s1'),
    missing('s2'),
    untrusted('s3'),
    untrusted('s4'),
    timeout('s5'),
    timeout('s6'),
    conflict('s7'),
    conflict('s8'),
    cost,
  ];
  return {
    format: 'anansi-somnia-evaluation',
    version: 1,
    input: { scenarios, seed: 'proposal-seed' },
    seed: 'proposal-seed',
    runId: 'somnia-proposal-doc',
    scenarioOrder: scenarios.map(s => s.id),
    results: [
      result('s1', 'Refuse', 0),
      result('s2', 'Refuse', 0),
      result('s3', 'Refuse', 0),
      result('s4', 'Refuse', 0),
      result('s5', 'Abort', 1),
      result('s6', 'Abort', 1),
      result('s7', 'Refuse', 0),
      result('s8', 'Refuse', 0),
      result('s9', 'Complete', 4),
    ],
    metrics: {
      scenariosTotal: 9,
      completed: 1,
      refused: 6,
      aborted: 2,
      violationsByCategory: [],
      simulatedToolCalls: 6,
    },
    contentHash: 'x'.repeat(64),
  };
}

test('somnia proposals derive deterministically and stay inert', () => {
  const proposals = deriveConsolidationProposals(proposalDoc());
  assert.deepEqual(
    proposals.map(p => [p.action, p.target]),
    [
      ['archive-untrusted', 'unknown-feed'],
      ['flag-missing-evidence', 'receipt'],
      ['procedural-rule', 'fetch-record'],
      ['procedural-rule', 'max-tool-calls:2'],
      ['supersede-conflict', 'instruction-conflict'],
    ],
  );
  assert.equal(proposals[0].id, 'proposal-archive-untrusted-0');
  assert.equal(proposals[4].id, 'proposal-supersede-conflict-4');
  assert.deepEqual(proposals[1].evidence, ['s1', 's2']);
  assert.deepEqual(proposals[3].evidence, ['s9']);
  const source = readFileSync('packages/ui/src/Somnia.tsx', 'utf8');
  assert.ok(!source.includes('applyProposal'));
  assert.ok(!source.includes('Apply proposal'));
});

test('somnia comparison deltas, newly-exposed rows, and deterministic exports', () => {
  const doc = (
    runId: string,
    metrics: SomniaEvaluationDoc['metrics'],
  ): SomniaEvaluationDoc => ({
    ...proposalDoc(),
    runId,
    metrics,
  });
  const baseline = doc('somnia-base', {
    scenariosTotal: 2,
    completed: 1,
    refused: 1,
    aborted: 0,
    violationsByCategory: [{ category: 'missing-evidence', count: 1 }],
    simulatedToolCalls: 1,
  });
  const comparisonDoc = doc('somnia-comp', {
    scenariosTotal: 3,
    completed: 1,
    refused: 1,
    aborted: 1,
    violationsByCategory: [
      { category: 'missing-evidence', count: 1 },
      { category: 'tool-timeout', count: 1 },
    ],
    simulatedToolCalls: 2,
  });
  const compared = compareSomniaEvaluations(baseline, comparisonDoc);
  assert.equal(compared.deltaCompleted, 0);
  assert.equal(compared.deltaRefused, 0);
  assert.equal(compared.deltaAborted, 1);
  assert.equal(compared.deltaSimulatedToolCalls, 1);
  assert.equal(compared.deltaViolations, 1);
  assert.equal(compared.categories.length, 2);
  assert.equal(compared.categories[1].category, 'tool-timeout');
  assert.equal(compared.categories[1].newlyExposed, true);
  assert.equal(compared.categories[0].newlyExposed, false);
  assert.deepEqual(
    JSON.parse(somniaComparisonToJson(compared)),
    JSON.parse(JSON.stringify(compared)),
  );
  const markdown = somniaComparisonToMarkdown(compared);
  assert.ok(markdown.includes('Baseline: somnia-base'));
  assert.ok(markdown.includes('Abort: 0 -> 1 (+1)'));
  assert.ok(markdown.includes('tool-timeout: 0 -> 1 (+1) (newly exposed)'));
});

test('somnia panel renders nightmare, proposal, and comparison sections', () => {
  const client = {
    evaluateSomnia: async () => {
      throw new Error('must not be called on mount');
    },
    saveSomniaEvaluation: async () => {
      throw new Error('must not be called on mount');
    },
    readProjectArtifact: async () => {
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
  assert.ok(html.includes('Generate SOMNIA nightmares'));
  assert.ok(html.includes('SOMNIA nightmare seed'));
  assert.ok(html.includes('SOMNIA nightmare count'));
  assert.ok(html.includes('Compare SOMNIA evaluations'));
  assert.ok(html.includes('SOMNIA comparison baseline'));
});

test('swarmtank re-run button no longer claims a full AI simulation', () => {
  const source = readFileSync(
    'packages/ui/src/SwarmtankScenarioControls.tsx',
    'utf8',
  );
  assert.ok(source.includes('Re-run Deterministic Scenario Simulation'));
  assert.ok(!source.includes('Full AI Simulation'));
  assert.ok(source.includes('does not perform live AI inference'));
});

test('somnia dashboard aggregates match the Rust fixed fixture', () => {
  const scenario = (
    id: string,
    patch: Partial<SomniaScenario>,
  ): SomniaScenario => ({
    id,
    title: `Case ${id}`,
    category: 'sufficient-evidence',
    instruction: 'Do the task.',
    instructionConflict: false,
    evidence: [{ id: 'e1', text: 'Evidence e1.', privacy: 'standard' }],
    retrieved: [],
    toolEvents: [],
    requiredEvidence: ['e1'],
    constraints: [],
    ...patch,
  });
  const doc = (
    runId: string,
    metrics: SomniaEvaluationDoc['metrics'],
    scenarios: SomniaScenario[],
  ): SomniaEvaluationDoc => ({
    ...proposalDoc(),
    runId,
    metrics,
    input: { scenarios, seed: 'dashboard-seed' },
  });
  const baseline = doc(
    'somnia-base',
    {
      scenariosTotal: 2,
      completed: 1,
      refused: 1,
      aborted: 0,
      violationsByCategory: [{ category: 'missing-evidence', count: 1 }],
      simulatedToolCalls: 2,
    },
    [
      scenario('b1', {}),
      scenario('b2', {
        category: 'missing-evidence',
        requiredEvidence: ['receipt'],
        evidence: [{ id: 'other', text: 'Other.', privacy: 'standard' }],
      }),
    ],
  );
  const later = doc(
    'somnia-later',
    {
      scenariosTotal: 4,
      completed: 1,
      refused: 2,
      aborted: 1,
      violationsByCategory: [
        { category: 'missing-evidence', count: 2 },
        { category: 'tool-timeout', count: 1 },
      ],
      simulatedToolCalls: 2,
    },
    [
      scenario('c1', {}),
      scenario('c2', {
        category: 'tool-timeout',
        toolEvents: [
          { tool: 'fetch-record', outcome: 'ok' },
          { tool: 'fetch-record', outcome: 'timeout' },
        ],
      }),
      scenario('c3', {
        category: 'missing-evidence',
        requiredEvidence: ['receipt'],
        evidence: [{ id: 'other', text: 'Other.', privacy: 'standard' }],
      }),
      scenario('c4', {
        category: 'missing-evidence',
        requiredEvidence: ['receipt'],
        evidence: [{ id: 'other', text: 'Other.', privacy: 'standard' }],
      }),
    ],
  );
  // Newest first: the function sorts by timestamp.
  const dashboard = computeSomniaDashboard([
    { doc: later, timestamp: '2026-11-02T10:00:00Z' },
    { doc: baseline, timestamp: '2026-11-02T09:00:00Z' },
  ]);
  assert.equal(dashboard.totalRuns, 2);
  assert.equal(dashboard.totalScenarios, 6);
  assert.equal(dashboard.totalRefused, 3);
  assert.equal(dashboard.totalAborted, 1);
  assert.equal(dashboard.failureRateTenths, 666);
  assert.equal(dashboard.latestRunId, 'somnia-later');
  assert.equal(dashboard.latestTimestamp, '2026-11-02T10:00:00Z');
  assert.equal(dashboard.latestCallsPerScenarioHundredths, 50);
  assert.equal(dashboard.baselineCallsPerScenarioHundredths, 100);
  assert.equal(dashboard.efficiencyRatioHundredths, 50);
  assert.equal(dashboard.duplicateEvidenceIds, 2);
  assert.equal(dashboard.duplicateSources, 0);
  assert.equal(dashboard.coverageClasses, 2);
  assert.equal(dashboard.totalProposals, 1);
  assert.equal(dashboard.regressions.length, 1);
  assert.deepEqual(
    [
      dashboard.regressions[0].fromRunId,
      dashboard.regressions[0].toRunId,
      dashboard.regressions[0].fromFailures,
      dashboard.regressions[0].toFailures,
    ],
    ['somnia-base', 'somnia-later', 1, 3],
  );
  const empty = computeSomniaDashboard([]);
  assert.equal(empty.totalRuns, 0);
  assert.equal(empty.failureRateTenths, 0);
  assert.equal(empty.efficiencyRatioHundredths, null);
});

test('somnia forge QA scenarios are deterministic and shaped for the evaluator', () => {
  const scenarios = forgeQaScenarios('Bean Stock Log');
  assert.deepEqual(
    scenarios.map(s => s.id),
    [
      'forge-qa-api-failure',
      'forge-qa-incorrect-input',
      'forge-qa-timeout',
      'forge-qa-unexpected-dependency',
      'forge-qa-tool-failure',
      'forge-qa-recovery',
      'forge-qa-memory-context-failure',
    ],
  );
  assert.ok(
    scenarios.every(s => s.title.startsWith('Forge QA [Bean Stock Log]: ')),
  );
  assert.deepEqual(
    scenarios.map(s => s.toolEvents.map(e => e.outcome).join(',')),
    ['ok,error', '', 'ok,timeout', '', 'error', 'ok,ok', ''],
  );
  assert.equal(scenarios[1].instructionConflict, true);
  assert.equal(scenarios[3].retrieved[0].trusted, false);
  assert.deepEqual(scenarios[5].requiredEvidence, ['recovery-plan']);
  assert.deepEqual(scenarios[6].requiredEvidence, ['context-record']);
  assert.ok(
    !scenarios[6].evidence.some(e => e.id === 'context-record'),
  );
  assert.deepEqual(forgeQaScenarios('Bean Stock Log'), scenarios);
  const markdown = somniaEvaluationToMarkdown(
    { ...proposalDoc(), runId: 'somnia-qa' },
    'SOMNIA QA summary for Bean Stock Log',
  );
  assert.ok(markdown.includes('SOMNIA QA summary for Bean Stock Log'));
  assert.ok(markdown.includes('Run: somnia-qa'));
});

test('somnia panel renders dashboard and forge QA sections with the disclosure', () => {
  const client = {
    evaluateSomnia: async () => {
      throw new Error('must not be called on mount');
    },
    saveSomniaEvaluation: async () => {
      throw new Error('must not be called on mount');
    },
    readProjectArtifact: async () => {
      throw new Error('must not be called on mount');
    },
    listProjectArtifacts: async () => {
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
  assert.ok(html.includes('SOMNIA dashboard'));
  assert.ok(html.includes('Load SOMNIA dashboard'));
  assert.ok(html.includes('SOMNIA Forge QA'));
  assert.ok(html.includes('Load Forge concepts'));
  assert.ok(
    html.includes(
      'SOMNIA augments QA; it does not replace ordinary automated software and security testing.',
    ),
  );
  assert.ok(!html.includes('applyProposal'));
  assert.ok(!html.includes('Apply proposal'));
});
