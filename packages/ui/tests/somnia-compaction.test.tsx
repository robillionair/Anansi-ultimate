// SOMNIA compaction Checkpoint 2: the frozen benchmark module carries all
// 40 cases / 50 questions, the scoring and metrics helpers mirror the Rust
// run module, the canonical experiment JSON matches the server's field
// order (including the omitted empty `supersedes`), the saved-experiment
// parser is strict, and the panel makes no provider call on mount.

import test from 'node:test';
import assert from 'node:assert/strict';
import { renderToStaticMarkup } from 'react-dom/server';
import React from 'react';
import {
  SOMNIA_COMPACTION_BENCHMARK_CASES,
} from '../src/somniaCompactionBenchmark';
import {
  SomniaCompaction,
  canonicalCompactionExperimentJson,
  computeCompactionApprovalHash,
  computeCompactionMetrics,
  isCompactionAbstention,
  parseSomniaCompactionExperiment,
  scoreCompactionReply,
  sha256Hex,
} from '../src/SomniaCompaction';
import type {
  Client,
  CompactionPairResult,
  SomniaCompactionCandidate,
  SomniaCompactionExperiment,
} from '@anansi/contracts';

test('compaction benchmark module carries the frozen 40 cases and 50 questions', () => {
  assert.equal(SOMNIA_COMPACTION_BENCHMARK_CASES.length, 40);
  const questions = SOMNIA_COMPACTION_BENCHMARK_CASES.reduce(
    (total, entry) => total + entry.questions.length,
    0,
  );
  assert.equal(questions, 50);
  const first = SOMNIA_COMPACTION_BENCHMARK_CASES[0];
  assert.equal(first.id, 'case-001');
  assert.equal(first.records.length, 2);
  assert.equal(first.questions[0].expectedAnswer, 'The customs fee is $45.');
  const abstain = SOMNIA_COMPACTION_BENCHMARK_CASES.find(entry => entry.id === 'case-017');
  assert.equal(abstain?.questions[0].expectedAnswer, null);
});

test('compaction abstention detection mirrors the server rule', () => {
  assert.equal(isCompactionAbstention('UNKNOWN'), true);
  assert.equal(isCompactionAbstention('unknown.'), true);
  assert.equal(isCompactionAbstention('UNKNOWN — no such record.'), true);
  assert.equal(isCompactionAbstention(''), true);
  assert.equal(isCompactionAbstention('   '), true);
  assert.equal(isCompactionAbstention('The fee is $45.'), false);
  assert.equal(isCompactionAbstention('UNKNOWNFEE'), false);
  assert.equal(isCompactionAbstention('It is unknown to me whether...'), false);
});

test('compaction scoring mirrors the server matrix', () => {
  const want = 'The customs fee is $45.';
  assert.equal(scoreCompactionReply('The customs fee is $45.', want), 'correct');
  assert.equal(scoreCompactionReply('  The customs fee is $45.  ', want), 'correct');
  assert.equal(scoreCompactionReply('The customs fee is $40.', want), 'incorrect');
  assert.equal(scoreCompactionReply('UNKNOWN', want), 'abstained');
  assert.equal(scoreCompactionReply('', want), 'abstained');
  assert.equal(scoreCompactionReply(null, want), 'failed');
  assert.equal(scoreCompactionReply('UNKNOWN', null), 'correct');
  assert.equal(scoreCompactionReply('The fee is $45.', null), 'incorrect');
  assert.equal(scoreCompactionReply(null, null), 'failed');
});

function pair(overrides: Partial<CompactionPairResult>): CompactionPairResult {
  return {
    pairId: 'case-001:q1',
    caseId: 'case-001',
    questionId: 'q1',
    question: 'What is the fee?',
    expectedAnswer: 'Fee is $45.',
    originalPrompt: 'History:\n[r2] Fee is $45.\n\nQuestion: What is the fee?',
    compactedPrompt: 'History:\n[r2] Fee is $45.\n\nQuestion: What is the fee?',
    originalReply: 'Fee is $45.',
    compactedReply: 'Fee is $45.',
    originalScore: 'correct',
    compactedScore: 'correct',
    originalUsage: { inputTokens: 100, outputTokens: 10, totalTokens: 110 },
    compactedUsage: { inputTokens: 50, outputTokens: 10, totalTokens: 60 },
    failure: null,
    compactionMicros: 120,
    ...overrides,
  };
}

test('compaction metrics count abstentions independently and never zero-fill tokens', () => {
  const abstainPair = pair({
    pairId: 'case-017:q1',
    caseId: 'case-017',
    expectedAnswer: null,
    originalReply: 'UNKNOWN',
    compactedReply: 'UNKNOWN',
    originalScore: 'correct',
    compactedScore: 'correct',
  });
  const metrics = computeCompactionMetrics([abstainPair], 90, 100);
  assert.equal(metrics.correctOriginal, 1);
  assert.equal(metrics.abstentionsOriginal, 1);
  assert.equal(metrics.tokensOriginal, 110);
  assert.equal(metrics.failedPairs, 0);
  const unreported = pair({
    originalUsage: null,
    originalScore: 'failed',
    originalReply: null,
    failure: 'Run cancelled before the original arm.',
  });
  const mixed = computeCompactionMetrics([pair({}), unreported], 240, 200);
  assert.equal(mixed.tokensOriginal, null);
  assert.equal(mixed.tokensReportedOriginalPairs, 1);
  assert.equal(mixed.tokensCompacted, 120);
  assert.equal(mixed.failedPairs, 1);
});

function fixtureCandidate(): SomniaCompactionCandidate {
  return {
    format: 'anansi-somnia-compaction-candidate',
    version: 1,
    inputHash: 'a'.repeat(64),
    retained: {
      trusted: [{
        record: {
          id: 'r2', key: 'harbor.customs-fee', source: 'ledger', revision: 2,
          kind: 'fact', trust: 'trusted', privacy: 'standard',
          text: 'The customs fee is $45.', supersedes: ['r1'],
        },
        status: 'current', detail: '', conflictWith: [],
      }],
      unverified: [],
      conflicts: [],
      deletions: [],
      uncertainties: [],
    },
    excluded: [{
      record: {
        id: 'r1', key: 'harbor.customs-fee', source: 'ledger', revision: 1,
        kind: 'fact', trust: 'trusted', privacy: 'standard',
        text: 'The customs fee is $40.', supersedes: [],
      },
      reason: 'superseded-by:r2',
      detail: '',
    }],
    quarantined: [],
    stats: {
      inputRecords: 2, retainedRecords: 1, excludedRecords: 1,
      quarantinedRecords: 0, inputChars: 44, retainedChars: 23,
    },
    contentHash: 'b'.repeat(64),
  };
}

function fixtureExperiment(): SomniaCompactionExperiment {
  const candidate = fixtureCandidate();
  const pairs = [pair({})];
  return {
    format: 'anansi-somnia-compaction-experiment',
    version: 1,
    runId: 'somnia-compaction-test',
    provider: 'ollama',
    model: 'llama-x',
    locality: 'local',
    inputHash: candidate.inputHash,
    candidate,
    pairs,
    metrics: computeCompactionMetrics(pairs, 120, 500),
    contentHash: '',
  };
}

test('canonical experiment JSON uses the server field order and omits empty supersedes', () => {
  const json = canonicalCompactionExperimentJson(fixtureExperiment());
  // Rust declaration order: experiment, candidate, retained sections, record.
  assert.ok(json.startsWith('{"format":"anansi-somnia-compaction-experiment","version":1,"runId"'));
  assert.ok(json.includes('"contentHash":""'));
  // r1 carries an empty supersedes list in the fixture: it must be absent.
  assert.ok(!json.includes('"id":"r1","key":"harbor.customs-fee","source":"ledger","revision":1,"kind":"fact","trust":"trusted","privacy":"standard","text":"The customs fee is $40.","supersedes"'));
  // r2 carries a non-empty supersedes list: it must be present, verbatim.
  assert.ok(json.includes('"text":"The customs fee is $45.","supersedes":["r1"]'));
  assert.ok(json.includes('"stats":{"inputRecords":2,"retainedRecords":1'));
  assert.ok(json.includes('"metrics":{"pairsTotal":1,"pairsDone":1,"correctOriginal":1'));
});

test('canonical experiment JSON omits absent pair privacy and keeps present privacy', () => {
  // A pair with no disclosed privacy (the standard default) hashes
  // exactly as before this field existed — old saved experiments
  // still validate.
  const without = canonicalCompactionExperimentJson(fixtureExperiment());
  assert.ok(!without.includes('originalPrivacy'));
  assert.ok(!without.includes('compactedPrivacy'));
  // A pair with a private original arm carries the field, so its
  // hash covers what was actually sent.
  const experiment = fixtureExperiment();
  experiment.pairs = [pair({ originalPrivacy: 'private' })];
  const withPrivate = canonicalCompactionExperimentJson(experiment);
  assert.ok(withPrivate.includes('"originalPrivacy":"private"'));
  assert.ok(!withPrivate.includes('compactedPrivacy'));
  assert.notEqual(withPrivate, without);
});

test('sha256 helper matches the known vector', async () => {
  assert.equal(
    await sha256Hex('abc'),
    'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad',
  );
});

test('saved experiment parser accepts the exact format and refuses the rest', () => {
  const valid = fixtureExperiment();
  valid.contentHash = 'c'.repeat(64);
  assert.notEqual(parseSomniaCompactionExperiment(JSON.stringify(valid)), null);
  const wrongFormat = { ...valid, format: 'anansi-somnia-model-comparison' };
  assert.equal(parseSomniaCompactionExperiment(JSON.stringify(wrongFormat)), null);
  const wrongVersion = { ...valid, version: 2 };
  assert.equal(parseSomniaCompactionExperiment(JSON.stringify(wrongVersion)), null);
  const noMetrics = { ...valid };
  delete (noMetrics as Record<string, unknown>).metrics;
  assert.equal(parseSomniaCompactionExperiment(JSON.stringify(noMetrics)), null);
  assert.equal(parseSomniaCompactionExperiment('{broken'), null);
});

test('compaction approval hash binds every reviewed field', async () => {
  const input = {
    cases: [{
      caseId: 'case-001',
      candidateHash: 'a'.repeat(64),
      questions: [{ id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' }],
    }],
    provider: 'ollama',
    model: 'llama-x',
    endpoint: 'http://127.0.0.1:11434',
    locality: 'local',
    cost: 'free',
    cloud: false,
    maxTotalTokens: 20000,
  };
  const base = await computeCompactionApprovalHash(input);
  assert.equal(base.length, 64);
  assert.equal(await computeCompactionApprovalHash(input), base);
  const changed = async (patch: Record<string, unknown>) =>
    computeCompactionApprovalHash({ ...input, ...patch });
  assert.notEqual(await changed({ model: 'aaa-decoy' }), base);
  assert.notEqual(await changed({ model: '' }), base);
  assert.notEqual(await changed({ cloud: true }), base);
  assert.notEqual(await changed({ maxTotalTokens: 50000 }), base);
  // The resolved target's effective endpoint, locality, and cost are
  // bound too: a configuration change to the approved target after
  // approval invalidates the approval.
  assert.notEqual(await changed({ endpoint: 'http://127.0.0.1:9999' }), base);
  assert.notEqual(await changed({ locality: 'cloud' }), base);
  assert.notEqual(await changed({ cost: 'paid' }), base);
  assert.notEqual(
    await computeCompactionApprovalHash({
      ...input,
      cases: [{ ...input.cases[0], candidateHash: 'b'.repeat(64) }],
    }),
    base,
  );
  assert.notEqual(
    await computeCompactionApprovalHash({
      ...input,
      cases: [{
        ...input.cases[0],
        questions: [{ id: 'q1', text: 'What was the fee before?', expectedAnswer: 'The customs fee is $45.' }],
      }],
    }),
    base,
  );
});

test('compaction panel makes no provider call on mount', () => {
  const calls: string[] = [];
  const client = {
    listProjectArtifacts: async () => {
      calls.push('listProjectArtifacts');
      return { contractVersion: 1, artifacts: [] };
    },
    prepareSomniaCompaction: async () => {
      calls.push('prepareSomniaCompaction');
      throw new Error('must not be called on mount');
    },
    startSomniaCompaction: async () => {
      calls.push('startSomniaCompaction');
      throw new Error('must not be called on mount');
    },
  } as unknown as Client;
  const html = renderToStaticMarkup(
    <SomniaCompaction
      client={client}
      workspaceId={'w'.repeat(32)}
      projectId={'p'.repeat(32)}
      onSaved={async () => {}}
      busy={false}
    />,
  );
  assert.ok(html.includes('Automatic compaction'));
  assert.ok(html.includes('case-001'));
  assert.ok(html.includes('case-040'));
  assert.ok(!calls.includes('prepareSomniaCompaction'));
  assert.ok(!calls.includes('startSomniaCompaction'));
});
