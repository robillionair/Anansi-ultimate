import { expect, test } from '@playwright/test';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import {
  computeApprovalHash,
  fakeAnswerKey,
  renderHistory,
  resetFakeBehavior,
  setFakeDelayMs,
  setFakeExpectedAnswers,
  setFakeWrongModelCall,
  startCompactionSession,
} from './somnia-compaction-helpers';

/**
 * SOMNIA compaction Checkpoint 2: the full workflow through the app with
 * a synthetic provider — import, prepare, review, approve, compare, save,
 * restart, reopen — plus cancellation and project scoping. No real model,
 * no paid call. Accuracy through the synthetic provider is labelled
 * synthetic wherever it is reported.
 */

const CASE_001_RECORDS = [
  {
    id: 'r1', key: 'harbor.customs-fee', source: 'ledger', revision: 1,
    kind: 'fact', trust: 'trusted', privacy: 'standard',
    text: 'The customs fee is $40.',
  },
  {
    id: 'r2', key: 'harbor.customs-fee', source: 'ledger', revision: 2,
    kind: 'fact', trust: 'trusted', privacy: 'standard',
    text: 'The customs fee is $45.', supersedes: ['r1'],
  },
];

/** Same shape as CASE_001, but the superseded record is Private: the
 * original arm's history contains a Private record, the compacted
 * arm's does not. */
const PRIVATE_CASE_RECORDS = [
  {
    id: 'r1', key: 'harbor.customs-fee', source: 'ledger', revision: 1,
    kind: 'fact', trust: 'trusted', privacy: 'private',
    text: 'The customs fee is $40.',
  },
  {
    id: 'r2', key: 'harbor.customs-fee', source: 'ledger', revision: 2,
    kind: 'fact', trust: 'trusted', privacy: 'standard',
    text: 'The customs fee is $45.', supersedes: ['r1'],
  },
];

const SEALED_CASE_RECORDS = [
  {
    id: 'r1', key: 'harbor.customs-fee', source: 'ledger', revision: 1,
    kind: 'fact', trust: 'trusted', privacy: 'sealed',
    text: 'The customs fee is $40.',
  },
];

test('SOMNIA compaction runs the approved comparison and saves it privately', async ({ page }, testInfo) => {
  test.setTimeout(240_000);
  resetFakeBehavior();
  setFakeDelayMs(0);
  // The synthetic provider answers the fee question from either history
  // and abstains on the question the history should not state.
  const originalHistory = renderHistory(CASE_001_RECORDS);
  const compactedHistory = renderHistory([CASE_001_RECORDS[1]]);
  setFakeExpectedAnswers([
    [fakeAnswerKey('What is the customs fee now?', originalHistory), 'The customs fee is $45.'],
    [fakeAnswerKey('What is the customs fee now?', compactedHistory), 'The customs fee is $45.'],
    [fakeAnswerKey('What was the customs fee before the update?', originalHistory), null],
    [fakeAnswerKey('What was the customs fee before the update?', compactedHistory), null],
  ]);
  // Auto would try this model before the explicitly chosen llama-x.
  const session = await startCompactionSession(page, { alternateModel: 'aaa-decoy' });
  try {
    const panel = session.panel;
    // No run exists before approval, and the run button is absent.
    await expect(panel.getByRole('button', { name: 'Run approved comparison' })).toHaveCount(0);
    const hitsBefore = session.fakeHits();

    await panel.getByLabel('Frozen benchmark case').selectOption('case-001');
    // Add a second, missing-fact question: the history should not state it.
    await panel.getByLabel(/Questions/).fill(JSON.stringify([
      { id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' },
      { id: 'q2', text: 'What was the customs fee before the update?', expectedAnswer: null },
    ], null, 2));
    await panel.getByRole('button', { name: 'Prepare candidate' }).click();
    const review = panel.getByRole('region', { name: 'Compacted candidate review' });
    await expect(review).toBeVisible();
    await expect(review.getByText('Retained 1')).toBeVisible();
    await expect(review.getByText('Excluded 1')).toBeVisible();
    await review.getByText('Retained · trusted (1)').click();
    await expect(review.getByText('The customs fee is $45.')).toBeVisible();
    await review.getByText('Excluded (1)').click();
    await expect(review.getByText('superseded-by:r2', { exact: false })).toBeVisible();
    // Reviewing dispatches nothing.
    expect(session.fakeHits()).toBe(hitsBefore);

    await panel.getByLabel('Model ID').fill('llama-x');
    await review.getByRole('button', { name: 'Approve reviewed inputs' }).click();
    await expect(review.getByText(/Approved candidate .* with 2 questions/)).toBeVisible();
    expect(session.fakeHits()).toBe(hitsBefore);

    await panel.getByRole('button', { name: 'Run approved comparison' }).click();
    const results = panel.getByRole('region', { name: 'Compaction run results' });
    await expect(results.getByText(/Status: complete/)).toBeVisible({ timeout: 60_000 });
    // Two pairs, two dispatches each, 16 synthetic tokens per call.
    expect(session.fakeHits() - hitsBefore).toBe(4);
    await expect(results.getByText(/4 calls · 64 tokens used/)).toBeVisible();
    await expect(results.getByText(/Correct: original 2\/2, compacted 2\/2/)).toBeVisible();
    await expect(results.getByText(/Failed pairs: 0/)).toBeVisible();

    await results.getByRole('button', { name: 'Save compaction experiment to project' }).click();
    await expect(results.getByText('Saved private compaction experiment to project artifacts.')).toBeVisible();
    const list = await session.call('artifacts', 'list', { workspaceId: session.workspaceId, projectId: session.projectId });
    const matching = list.body.artifacts.filter((item: { type: string }) => item.type === 'somnia-compaction-experiment');
    expect(matching.length).toBe(1);
    expect(matching[0].privacy).toBe('private');
    const read = await session.call('artifacts', 'read', { workspaceId: session.workspaceId, artifactId: matching[0].artifactId, version: matching[0].version });
    expect(read.status).toBe(200);
    const saved = JSON.parse(read.body.body);
    expect(saved.format).toBe('anansi-somnia-compaction-experiment');
    expect(saved.version).toBe(1);
    expect(saved.metrics.pairsTotal).toBe(2);
    expect(saved.metrics.correctOriginal).toBe(2);
    expect(saved.metrics.correctCompacted).toBe(2);
    expect(saved.metrics.tokensOriginal).toBe(32);
    expect(saved.metrics.tokensCompacted).toBe(32);
    expect(saved.metrics.failedPairs).toBe(0);
    expect(typeof saved.contentHash).toBe('string');
    expect(saved.contentHash).toMatch(/^[0-9a-f]{64}$/);

    // Restart the API: the saved experiment reopens with no further
    // provider call, and the in-flight run registry is gone.
    const hitsAfterSave = session.fakeHits();
    await session.restartApi();
    await session.repair();
    const reopened = session.panel.getByRole('region', { name: 'Saved compaction experiments' });
    await expect(reopened).toBeVisible();
    await reopened.getByRole('button', { name: 'View experiment' }).click();
    const viewing = session.panel.getByRole('region', { name: 'Saved compaction experiment' });
    await expect(viewing.getByText(/Correct: original 2\/2, compacted 2\/2/)).toBeVisible();
    expect(session.fakeHits()).toBe(hitsAfterSave);
    const pollAfterRestart = await session.call('ai', 'somnia-compaction-poll', {
      workspaceId: session.workspaceId, projectId: session.projectId, runId: saved.runId,
    });
    expect(pollAfterRestart.status).toBe(404);
    await testInfo.attach('saved-experiment.json', { body: JSON.stringify(saved, null, 2), contentType: 'application/json' });
    const { writeFile } = await import('node:fs/promises');
    await writeFile(
      testInfo.outputPath('saved-experiment.json'),
      JSON.stringify(saved, null, 2),
    );
  } finally {
    await session.cleanup();
  }
});

test('SOMNIA compaction refuses a start whose approval does not match the reviewed inputs', async ({ page }) => {
  test.setTimeout(120_000);
  resetFakeBehavior();
  setFakeDelayMs(0);
  setFakeExpectedAnswers([]);
  const session = await startCompactionSession(page);
  try {
    const envelope = JSON.stringify({ format: 'anansi-somnia-compaction-input', version: 1, records: CASE_001_RECORDS });
    const prepared = await session.call('ai', 'somnia-compaction-prepare', {
      workspaceId: session.workspaceId, projectId: session.projectId, inputText: envelope, inputFormat: 'json',
    });
    expect(prepared.status).toBe(200);
    const candidate = prepared.body.candidate;
    const hitsBefore = session.fakeHits();
    // Resolve the target first: the approval hash binds the resolved
    // provider/model/endpoint/locality/cost, never a blank default.
    const resolved = await session.resolveTarget();
    expect(resolved.status).toBe(200);
    const target = resolved.body as { provider: string; model: string; endpoint: string; locality: string; cost: string };
    expect(target.provider).toBe('ollama');
    expect(target.model).toBe('llama-x');
    const hashFor = (candidateHash: string) =>
      computeApprovalHash({
        cases: [{ caseId: 'case-001', candidateHash, questions: baseCase.questions }],
        provider: target.provider, model: target.model,
        endpoint: target.endpoint, locality: target.locality, cost: target.cost,
        cloud: false, maxTotalTokens: 20000,
      });
    const baseCase = {
      caseId: 'case-001',
      records: CASE_001_RECORDS,
      quarantined: candidate.quarantined,
      candidateHash: candidate.contentHash,
      questions: [{ id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' }],
    };
    // A tampered hash is refused before any dispatch.
    const tamperedHash = await session.call('ai', 'somnia-compaction-start', {
      workspaceId: session.workspaceId, projectId: session.projectId,
      cases: [{ ...baseCase, candidateHash: '0'.repeat(64) }],
      provider: target.provider, model: target.model,
      approvalHash: hashFor('0'.repeat(64)),
    });
    expect(tamperedHash.status).toBe(400);
    expect(session.fakeHits()).toBe(hitsBefore);
    // An edited record with the reviewed hash is refused the same way.
    const editedRecords = CASE_001_RECORDS.map(record =>
      record.id === 'r2' ? { ...record, text: 'The customs fee is $99.' } : record,
    );
    const edited = await session.call('ai', 'somnia-compaction-start', {
      workspaceId: session.workspaceId, projectId: session.projectId,
      cases: [{ ...baseCase, records: editedRecords }],
      provider: target.provider, model: target.model,
      approvalHash: hashFor(candidate.contentHash),
    });
    expect(edited.status).toBe(400);
    expect(session.fakeHits()).toBe(hitsBefore);
    // The untouched approval starts cleanly.
    const ok = await session.call('ai', 'somnia-compaction-start', {
      workspaceId: session.workspaceId, projectId: session.projectId, cases: [baseCase],
      provider: target.provider, model: target.model,
      approvalHash: hashFor(candidate.contentHash),
    });
    expect(ok.status).toBe(200);
    expect(ok.body.pairsTotal).toBe(1);
  } finally {
    await session.cleanup();
  }
});

test('SOMNIA compaction cancellation stops the run without dropping the dispatched call', async ({ page }) => {
  test.setTimeout(120_000);
  resetFakeBehavior();
  setFakeDelayMs(3000);
  const originalHistory = renderHistory(CASE_001_RECORDS);
  const compactedHistory = renderHistory([CASE_001_RECORDS[1]]);
  setFakeExpectedAnswers([
    [fakeAnswerKey('What is the customs fee now?', originalHistory), 'The customs fee is $45.'],
    [fakeAnswerKey('What is the customs fee now?', compactedHistory), 'The customs fee is $45.'],
  ]);
  const session = await startCompactionSession(page);
  try {
    const panel = session.panel;
    await panel.getByLabel('Frozen benchmark case').selectOption('case-001');
    await panel.getByRole('button', { name: 'Prepare candidate' }).click();
    const review = panel.getByRole('region', { name: 'Compacted candidate review' });
    await expect(review).toBeVisible();
    await panel.getByLabel('Model ID').fill('llama-x');
    await review.getByRole('button', { name: 'Approve reviewed inputs' }).click();
    await panel.getByRole('button', { name: 'Run approved comparison' }).click();
    const results = panel.getByRole('region', { name: 'Compaction run results' });
    await expect(results.getByText(/Status: running/)).toBeVisible({ timeout: 30_000 });
    const hitsBeforeCancel = session.fakeHits();
    await results.getByRole('button', { name: 'Cancel compaction run' }).click();
    await expect(results.getByText(/Status: cancelled/)).toBeVisible({ timeout: 30_000 });
    // Exactly one dispatch landed (the original arm of the first pair);
    // the compacted arm never dispatched, and the pair is recorded as
    // failed with the cancellation reason instead of being dropped.
    expect(session.fakeHits() - hitsBeforeCancel).toBeLessThanOrEqual(1);
    await expect(results.getByText(/Run cancelled before the compacted arm/)).toBeVisible();
  } finally {
    await session.cleanup();
  }
});

test('LARP scene 09 shows the compaction flow with no provider traffic on entry', async ({ page }) => {
  test.setTimeout(120_000);
  resetFakeBehavior();
  setFakeDelayMs(0);
  setFakeExpectedAnswers([]);
  const session = await startCompactionSession(page);
  try {
    await page.setViewportSize({ width: 1500, height: 950 });
    const hitsBefore = session.fakeHits();
    await page.getByLabel('Toggle LARP mode').click();
    await page.keyboard.press('9');
    const chamber = page.getByRole('region', { name: 'SOMNIA chamber' });
    const controls = page.getByLabel('SOMNIA controls');
    await expect(chamber.getByText('STANDBY', { exact: true })).toBeVisible();
    await controls.getByLabel('Project', { exact: true }).selectOption({ label: 'Harbor App' });
    // The console (SOMNIA controls) holds the chamber's panel; the
    // chamber region itself holds only the instruments.
    const compaction = controls.getByRole('region', { name: 'SOMNIA automatic compaction' });
    await expect(compaction).toBeVisible();
    await expect(compaction.getByRole('button', { name: 'Prepare candidate' })).toBeVisible();
    await expect(compaction.getByLabel('Frozen benchmark case')).toBeVisible();
    // Entering the scene mounts the panel but dispatches nothing.
    expect(session.fakeHits()).toBe(hitsBefore);
  } finally {
    await session.cleanup();
  }
});

test('SOMNIA compaction runs are scoped to their project', async ({ page }) => {
  test.setTimeout(120_000);
  resetFakeBehavior();
  setFakeDelayMs(0);
  const originalHistory = renderHistory(CASE_001_RECORDS);
  const compactedHistory = renderHistory([CASE_001_RECORDS[1]]);
  setFakeExpectedAnswers([
    [fakeAnswerKey('What is the customs fee now?', originalHistory), 'The customs fee is $45.'],
    [fakeAnswerKey('What is the customs fee now?', compactedHistory), 'The customs fee is $45.'],
  ]);
  const session = await startCompactionSession(page);
  try {
    const envelope = JSON.stringify({ format: 'anansi-somnia-compaction-input', version: 1, records: CASE_001_RECORDS });
    const prepared = await session.call('ai', 'somnia-compaction-prepare', {
      workspaceId: session.workspaceId, projectId: session.projectId, inputText: envelope, inputFormat: 'json',
    });
    expect(prepared.status).toBe(200);
    const resolved = await session.resolveTarget();
    expect(resolved.status).toBe(200);
    const target = resolved.body as { provider: string; model: string; endpoint: string; locality: string; cost: string };
    const started = await session.call('ai', 'somnia-compaction-start', {
      workspaceId: session.workspaceId, projectId: session.projectId,
      cases: [{
        caseId: 'case-001',
        records: CASE_001_RECORDS,
        quarantined: prepared.body.candidate.quarantined,
        candidateHash: prepared.body.candidate.contentHash,
        questions: [{ id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' }],
      }],
      provider: target.provider, model: target.model,
      approvalHash: computeApprovalHash({
        cases: [{
          caseId: 'case-001',
          candidateHash: prepared.body.candidate.contentHash,
          questions: [{ id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' }],
        }],
        provider: target.provider, model: target.model,
        endpoint: target.endpoint, locality: target.locality, cost: target.cost,
        cloud: false, maxTotalTokens: 20000,
      }),
    });
    expect(started.status).toBe(200);
    const runId = started.body.runId as string;
    const secondProjectId = await session.createProject('Second Harbor');
    // Polling or cancelling the run from another project is not found.
    const foreignPoll = await session.call('ai', 'somnia-compaction-poll', {
      workspaceId: session.workspaceId, projectId: secondProjectId, runId,
    });
    expect(foreignPoll.status).toBe(404);
    const foreignCancel = await session.call('ai', 'somnia-compaction-cancel', {
      workspaceId: session.workspaceId, projectId: secondProjectId, runId,
    });
    expect(foreignCancel.status).toBe(404);
    // The owning project still sees it.
    const ownPoll = await session.call('ai', 'somnia-compaction-poll', {
      workspaceId: session.workspaceId, projectId: session.projectId, runId,
    });
    expect(ownPoll.status).toBe(200);
    // In the UI, the second project shows no run and no saved experiments.
    await session.openProjectByTitle('Second Harbor');
    await expect(session.panel.getByRole('region', { name: 'Compaction run results' })).toHaveCount(0);
    await expect(session.panel.getByRole('region', { name: 'Saved compaction experiments' })).toHaveCount(0);
  } finally {
    await session.cleanup();
  }
});

test('SOMNIA compaction refuses a pair when the provider reports a different model', async ({ page }) => {
  test.setTimeout(120_000);
  resetFakeBehavior();
  setFakeDelayMs(0);
  const originalHistory = renderHistory(CASE_001_RECORDS);
  const compactedHistory = renderHistory([CASE_001_RECORDS[1]]);
  setFakeExpectedAnswers([
    [fakeAnswerKey('What is the customs fee now?', originalHistory), 'The customs fee is $45.'],
    [fakeAnswerKey('What is the customs fee now?', compactedHistory), 'The customs fee is $45.'],
  ]);
  // The first dispatch (original arm) reports another model; the second
  // (compacted arm) reports the requested one.
  setFakeWrongModelCall(1);
  const session = await startCompactionSession(page);
  try {
    const envelope = JSON.stringify({ format: 'anansi-somnia-compaction-input', version: 1, records: CASE_001_RECORDS });
    const prepared = await session.call('ai', 'somnia-compaction-prepare', {
      workspaceId: session.workspaceId, projectId: session.projectId, inputText: envelope, inputFormat: 'json',
    });
    expect(prepared.status).toBe(200);
    const resolved = await session.resolveTarget();
    expect(resolved.status).toBe(200);
    const target = resolved.body as { provider: string; model: string; endpoint: string; locality: string; cost: string };
    const started = await session.call('ai', 'somnia-compaction-start', {
      workspaceId: session.workspaceId, projectId: session.projectId,
      cases: [{
        caseId: 'case-001',
        records: CASE_001_RECORDS,
        quarantined: prepared.body.candidate.quarantined,
        candidateHash: prepared.body.candidate.contentHash,
        questions: [{ id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' }],
      }],
      provider: target.provider, model: target.model,
      approvalHash: computeApprovalHash({
        cases: [{
          caseId: 'case-001',
          candidateHash: prepared.body.candidate.contentHash,
          questions: [{ id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' }],
        }],
        provider: target.provider, model: target.model,
        endpoint: target.endpoint, locality: target.locality, cost: target.cost,
        cloud: false, maxTotalTokens: 20000,
      }),
    });
    expect(started.status).toBe(200);
    const runId = started.body.runId as string;
    let poll = await session.call('ai', 'somnia-compaction-poll', {
      workspaceId: session.workspaceId, projectId: session.projectId, runId,
    });
    const deadline = Date.now() + 60_000;
    while (poll.body.status === 'running' && Date.now() < deadline) {
      await new Promise(resolve => setTimeout(resolve, 500));
      poll = await session.call('ai', 'somnia-compaction-poll', {
        workspaceId: session.workspaceId, projectId: session.projectId, runId,
      });
    }
    expect(poll.body.status).toBe('complete');
    expect(poll.body.callsMade).toBe(2);
    expect(poll.body.pairs.length).toBe(1);
    const pair = poll.body.pairs[0];
    expect(pair.originalScore).toBe('failed');
    expect(pair.failure).toContain('The provider reported a different model.');
    // The good arm's reply is still recorded; the pair is failed, not completed.
    expect(pair.compactedScore).toBe('correct');
    expect(pair.compactedReply).toBe('The customs fee is $45.');
  } finally {
    await session.cleanup();
  }
});

test('SOMNIA compaction runs with a blank Model ID against the first configured model', async ({ page }) => {
  test.setTimeout(240_000);
  resetFakeBehavior();
  setFakeDelayMs(0);
  const originalHistory = renderHistory(CASE_001_RECORDS);
  const compactedHistory = renderHistory([CASE_001_RECORDS[1]]);
  setFakeExpectedAnswers([
    [fakeAnswerKey('What is the customs fee now?', originalHistory), 'The customs fee is $45.'],
    [fakeAnswerKey('What is the customs fee now?', compactedHistory), 'The customs fee is $45.'],
  ]);
  // Two configured models: llama-x is first in configuration order,
  // aaa-decoy second. The panel promises a blank Model ID selects the
  // first configured model; the run must use llama-x, not the decoy.
  const session = await startCompactionSession(page, { alternateModel: 'aaa-decoy' });
  try {
    const panel = session.panel;
    const hitsBefore = session.fakeHits();
    await panel.getByLabel('Frozen benchmark case').selectOption('case-001');
    await panel.getByLabel(/Questions/).fill(JSON.stringify([
      { id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' },
    ], null, 2));
    await panel.getByRole('button', { name: 'Prepare candidate' }).click();
    const review = panel.getByRole('region', { name: 'Compacted candidate review' });
    await expect(review).toBeVisible();
    // The Model ID field is deliberately left blank here.
    await expect(panel.getByLabel('Model ID')).toHaveValue('');
    await review.getByRole('button', { name: 'Approve reviewed inputs' }).click();
    await expect(review.getByText(/Approved candidate .* with 1 question/)).toBeVisible();
    await panel.getByRole('button', { name: 'Run approved comparison' }).click();
    const results = panel.getByRole('region', { name: 'Compaction run results' });
    await expect(results.getByText(/Status: complete/)).toBeVisible({ timeout: 60_000 });
    expect(session.fakeHits() - hitsBefore).toBe(2);
    // Every dispatch went to the first configured model, not the decoy.
    expect(session.fakeModels()).toEqual(['llama-x', 'llama-x']);
    const heading = await results.getByRole('heading').textContent();
    const runId = heading!.replace('Comparison run ', '').trim();
    const poll = await session.call('ai', 'somnia-compaction-poll', {
      workspaceId: session.workspaceId, projectId: session.projectId, runId,
    });
    expect(poll.body.provider).toBe('ollama');
    expect(poll.body.model).toBe('llama-x');
    expect(poll.body.locality).toBe('local');
  } finally {
    await session.cleanup();
  }
});

test('SOMNIA compaction runs a free cloud model with explicit approval and no permit', async ({ page }) => {
  test.setTimeout(240_000);
  resetFakeBehavior();
  setFakeDelayMs(0);
  const originalHistory = renderHistory(CASE_001_RECORDS);
  const compactedHistory = renderHistory([CASE_001_RECORDS[1]]);
  setFakeExpectedAnswers([
    [fakeAnswerKey('What is the customs fee now?', originalHistory), 'The customs fee is $45.'],
    [fakeAnswerKey('What is the customs fee now?', compactedHistory), 'The customs fee is $45.'],
  ]);
  const session = await startCompactionSession(page, { cloudModel: { id: 'free-cloud-model', cost: 'free' } });
  try {
    const panel = session.panel;
    const hitsBefore = session.fakeHits();
    const cloudHitsBefore = session.cloudFakeHits();
    await panel.getByLabel('Frozen benchmark case').selectOption('case-001');
    await panel.getByLabel(/Questions/).fill(JSON.stringify([
      { id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' },
    ], null, 2));
    await panel.getByRole('button', { name: 'Prepare candidate' }).click();
    const review = panel.getByRole('region', { name: 'Compacted candidate review' });
    await expect(review).toBeVisible();
    await panel.getByLabel('Provider ID').fill('openrouter');
    await panel.getByLabel('Model ID').fill('free-cloud-model');
    await panel.getByLabel('Authorize cloud transmission for this comparison').check();
    await review.getByRole('button', { name: 'Approve reviewed inputs' }).click();
    await expect(review.getByText(/Approved candidate .* with 1 question/)).toBeVisible();
    await panel.getByRole('button', { name: 'Run approved comparison' }).click();
    const results = panel.getByRole('region', { name: 'Compaction run results' });
    await expect(results.getByText(/Status: complete/)).toBeVisible({ timeout: 60_000 });
    // Both arms went to the cloud fake; the local fake saw nothing.
    expect(session.cloudFakeHits() - cloudHitsBefore).toBe(2);
    expect(session.fakeHits() - hitsBefore).toBe(0);
    expect(session.cloudFakeModels()).toEqual(['free-cloud-model', 'free-cloud-model']);
    const heading = await results.getByRole('heading').textContent();
    const runId = heading!.replace('Comparison run ', '').trim();
    const poll = await session.call('ai', 'somnia-compaction-poll', {
      workspaceId: session.workspaceId, projectId: session.projectId, runId,
    });
    expect(poll.body.provider).toBe('openrouter');
    expect(poll.body.model).toBe('free-cloud-model');
    expect(poll.body.locality).toBe('cloud');
    expect(poll.body.cost).toBe('free');
  } finally {
    await session.cleanup();
  }
});

test('SOMNIA compaction refuses billable targets and supplied permits before traffic', async ({ page }) => {
  test.setTimeout(120_000);
  resetFakeBehavior();
  setFakeDelayMs(0);
  setFakeExpectedAnswers([]);
  const session = await startCompactionSession(page, {
    paidModel: 'paid-model',
    cloudModel: { id: 'paid-cloud-model', cost: 'paid' },
  });
  try {
    const envelope = JSON.stringify({ format: 'anansi-somnia-compaction-input', version: 1, records: CASE_001_RECORDS });
    const prepared = await session.call('ai', 'somnia-compaction-prepare', {
      workspaceId: session.workspaceId, projectId: session.projectId, inputText: envelope, inputFormat: 'json',
    });
    expect(prepared.status).toBe(200);
    const candidate = prepared.body.candidate;
    const questions = [{ id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' }];
    const baseCase = {
      caseId: 'case-001',
      records: CASE_001_RECORDS,
      quarantined: candidate.quarantined,
      candidateHash: candidate.contentHash,
      questions,
    };
    const hashFor = async (provider: string, model: string, cloud: boolean, maxTotalTokens = 20000) => {
      const resolved = await session.resolveTarget(provider, model);
      expect(resolved.status).toBe(200);
      const target = resolved.body as { provider: string; model: string; endpoint: string; locality: string; cost: string };
      return computeApprovalHash({
        cases: [{ caseId: 'case-001', candidateHash: candidate.contentHash, questions }],
        provider: target.provider, model: target.model,
        endpoint: target.endpoint, locality: target.locality, cost: target.cost,
        cloud, maxTotalTokens,
      });
    };
    const startWith = (extra: Record<string, unknown>) =>
      session.call('ai', 'somnia-compaction-start', {
        workspaceId: session.workspaceId, projectId: session.projectId, cases: [baseCase], ...extra,
      });
    // Control: the free local target with a valid approval starts.
    const hitsBefore = session.fakeHits();
    const ok = await startWith({
      provider: 'ollama', model: 'llama-x', cloud: false, maxTotalTokens: 20000,
      approvalHash: await hashFor('ollama', 'llama-x', false),
    });
    expect(ok.status).toBe(200);
    const runId = ok.body.runId as string;
    let poll = await session.call('ai', 'somnia-compaction-poll', {
      workspaceId: session.workspaceId, projectId: session.projectId, runId,
    });
    const deadline = Date.now() + 60_000;
    while (poll.body.status === 'running' && Date.now() < deadline) {
      await new Promise(resolve => setTimeout(resolve, 500));
      poll = await session.call('ai', 'somnia-compaction-poll', {
        workspaceId: session.workspaceId, projectId: session.projectId, runId,
      });
    }
    expect(poll.body.status).toBe('complete');
    const hitsAfterControl = session.fakeHits();
    expect(hitsAfterControl - hitsBefore).toBe(2);
    // A billable local model is refused at start, before any dispatch.
    const paidLocal = await startWith({
      provider: 'ollama', model: 'paid-model', cloud: false, maxTotalTokens: 20000,
      approvalHash: await hashFor('ollama', 'paid-model', false),
    });
    expect(paidLocal.status).toBe(400);
    expect(String(paidLocal.body.message ?? paidLocal.body.error ?? '')).toMatch(/free/i);
    expect(session.fakeHits()).toBe(hitsAfterControl);
    // A billable cloud model is refused the same way, with no cloud hit.
    const cloudHitsBefore = session.cloudFakeHits();
    const paidCloud = await startWith({
      provider: 'openrouter', model: 'paid-cloud-model', cloud: true, maxTotalTokens: 20000,
      approvalHash: await hashFor('openrouter', 'paid-cloud-model', true),
    });
    expect(paidCloud.status).toBe(400);
    expect(String(paidCloud.body.message ?? paidCloud.body.error ?? '')).toMatch(/free/i);
    expect(session.cloudFakeHits()).toBe(cloudHitsBefore);
    // A supplied single-use permit does not authorize a comparison.
    const permitted = await startWith({
      provider: 'ollama', model: 'llama-x', cloud: false, maxTotalTokens: 20000,
      permitId: 'permit-test', approvalHash: await hashFor('ollama', 'llama-x', false),
    });
    expect(permitted.status).toBe(400);
    expect(session.fakeHits()).toBe(hitsAfterControl);
  } finally {
    await session.cleanup();
  }
});

test('SOMNIA compaction refuses a start whose approval hash does not match the request', async ({ page }) => {
  test.setTimeout(120_000);
  resetFakeBehavior();
  setFakeDelayMs(0);
  setFakeExpectedAnswers([]);
  const session = await startCompactionSession(page, { alternateModel: 'aaa-decoy' });
  try {
    const envelope = JSON.stringify({ format: 'anansi-somnia-compaction-input', version: 1, records: CASE_001_RECORDS });
    const prepared = await session.call('ai', 'somnia-compaction-prepare', {
      workspaceId: session.workspaceId, projectId: session.projectId, inputText: envelope, inputFormat: 'json',
    });
    expect(prepared.status).toBe(200);
    const candidate = prepared.body.candidate;
    const questions = [{ id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' }];
    const baseCase = {
      caseId: 'case-001',
      records: CASE_001_RECORDS,
      quarantined: candidate.quarantined,
      candidateHash: candidate.contentHash,
      questions,
    };
    const resolved = await session.resolveTarget('ollama', 'llama-x');
    expect(resolved.status).toBe(200);
    const target = resolved.body as { provider: string; model: string; endpoint: string; locality: string; cost: string };
    const validHash = computeApprovalHash({
      cases: [{ caseId: 'case-001', candidateHash: candidate.contentHash, questions }],
      provider: target.provider, model: target.model,
      endpoint: target.endpoint, locality: target.locality, cost: target.cost,
      cloud: false, maxTotalTokens: 20000,
    });
    const startWith = (extra: Record<string, unknown>) =>
      session.call('ai', 'somnia-compaction-start', {
        workspaceId: session.workspaceId, projectId: session.projectId, cases: [baseCase], ...extra,
      });
    const hitsBefore = session.fakeHits();
    // A start without an approval hash is refused: approval is
    // mandatory, not optional. (Before the fix this call was
    // accepted — the server bound nothing.)
    const noHash = await startWith({ provider: 'ollama', model: 'llama-x', cloud: false, maxTotalTokens: 20000 });
    expect(noHash.status).toBe(400);
    expect(session.fakeHits()).toBe(hitsBefore);
    // The untouched approval starts cleanly.
    const ok = await startWith({
      provider: 'ollama', model: 'llama-x', cloud: false, maxTotalTokens: 20000, approvalHash: validHash,
    });
    expect(ok.status).toBe(200);
    const runId = ok.body.runId as string;
    let poll = await session.call('ai', 'somnia-compaction-poll', {
      workspaceId: session.workspaceId, projectId: session.projectId, runId,
    });
    const deadline = Date.now() + 60_000;
    while (poll.body.status === 'running' && Date.now() < deadline) {
      await new Promise(resolve => setTimeout(resolve, 500));
      poll = await session.call('ai', 'somnia-compaction-poll', {
        workspaceId: session.workspaceId, projectId: session.projectId, runId,
      });
    }
    expect(poll.body.status).toBe('complete');
    const hitsAfterControl = session.fakeHits();
    expect(hitsAfterControl - hitsBefore).toBe(2);
    // One field changed at a time, same approval hash: all refused
    // before traffic.
    const tamperedQuestions = await startWith({
      provider: 'ollama', model: 'llama-x', cloud: false, maxTotalTokens: 20000, approvalHash: validHash,
      cases: [{ ...baseCase, questions: [{ id: 'q1', text: 'What is the customs fee after the update?', expectedAnswer: 'The customs fee is $45.' }] }],
    });
    expect(tamperedQuestions.status).toBe(400);
    expect(session.fakeHits()).toBe(hitsAfterControl);
    const tamperedTarget = await startWith({
      provider: 'ollama', model: 'aaa-decoy', cloud: false, maxTotalTokens: 20000, approvalHash: validHash,
    });
    expect(tamperedTarget.status).toBe(400);
    expect(session.fakeHits()).toBe(hitsAfterControl);
    const tamperedBudget = await startWith({
      provider: 'ollama', model: 'llama-x', cloud: false, maxTotalTokens: 50000, approvalHash: validHash,
    });
    expect(tamperedBudget.status).toBe(400);
    expect(session.fakeHits()).toBe(hitsAfterControl);
    const tamperedCandidate = await startWith({
      provider: 'ollama', model: 'llama-x', cloud: false, maxTotalTokens: 20000, approvalHash: validHash,
      cases: [{ ...baseCase, candidateHash: '0'.repeat(64) }],
    });
    expect(tamperedCandidate.status).toBe(400);
    expect(session.fakeHits()).toBe(hitsAfterControl);
  } finally {
    await session.cleanup();
  }
});

/** Waits for a compaction run to leave `running` (max 60 s). */
async function waitForRun(
  session: Awaited<ReturnType<typeof startCompactionSession>>,
  runId: string,
) {
  let poll = await session.call('ai', 'somnia-compaction-poll', {
    workspaceId: session.workspaceId, projectId: session.projectId, runId,
  });
  const deadline = Date.now() + 60_000;
  while (poll.body.status === 'running' && Date.now() < deadline) {
    await new Promise(resolve => setTimeout(resolve, 500));
    poll = await session.call('ai', 'somnia-compaction-poll', {
      workspaceId: session.workspaceId, projectId: session.projectId, runId,
    });
  }
  return poll;
}

test('SOMNIA compaction review shows the exact resolved target for a blank Model ID', async ({ page }) => {
  test.setTimeout(240_000);
  resetFakeBehavior();
  setFakeDelayMs(0);
  setFakeExpectedAnswers([
    [fakeAnswerKey('What is the customs fee now?', renderHistory(CASE_001_RECORDS)), 'The customs fee is $45.'],
    [fakeAnswerKey('What is the customs fee now?', renderHistory([CASE_001_RECORDS[1]])), 'The customs fee is $45.'],
  ]);
  // Two configured models: llama-x is first, aaa-decoy second. The
  // Model ID field is deliberately left blank: the review must show
  // the exact resolved target, never "first configured model".
  const session = await startCompactionSession(page, { alternateModel: 'aaa-decoy' });
  try {
    const panel = session.panel;
    await panel.getByLabel('Frozen benchmark case').selectOption('case-001');
    await panel.getByLabel(/Questions/).fill(JSON.stringify([
      { id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' },
    ], null, 2));
    await panel.getByRole('button', { name: 'Prepare candidate' }).click();
    const review = panel.getByRole('region', { name: 'Compacted candidate review' });
    await expect(review).toBeVisible();
    await expect(panel.getByLabel('Model ID')).toHaveValue('');
    await review.getByRole('button', { name: 'Approve reviewed inputs' }).click();
    const approved = review.getByText(/Approved candidate/);
    await expect(approved).toBeVisible();
    // The approved line names the resolved target exactly.
    await expect(approved).toContainText('ollama / llama-x');
    await expect(approved).toContainText('local');
    await expect(approved).toContainText('free');
    await panel.getByRole('button', { name: 'Run approved comparison' }).click();
    const results = panel.getByRole('region', { name: 'Compaction run results' });
    await expect(results.getByText(/Status: complete/)).toBeVisible({ timeout: 60_000 });
    expect(session.fakeModels()).toEqual(['llama-x', 'llama-x']);
  } finally {
    await session.cleanup();
  }
});

test('SOMNIA compaction refuses a stale approval after the configured default changes', async ({ page }) => {
  test.setTimeout(240_000);
  resetFakeBehavior();
  setFakeDelayMs(0);
  setFakeExpectedAnswers([
    [fakeAnswerKey('What is the customs fee now?', renderHistory(CASE_001_RECORDS)), 'The customs fee is $45.'],
    [fakeAnswerKey('What is the customs fee now?', renderHistory([CASE_001_RECORDS[1]])), 'The customs fee is $45.'],
  ]);
  const session = await startCompactionSession(page, { alternateModel: 'aaa-decoy' });
  try {
    const envelope = JSON.stringify({ format: 'anansi-somnia-compaction-input', version: 1, records: CASE_001_RECORDS });
    const prepared = await session.call('ai', 'somnia-compaction-prepare', {
      workspaceId: session.workspaceId, projectId: session.projectId, inputText: envelope, inputFormat: 'json',
    });
    expect(prepared.status).toBe(200);
    const candidate = prepared.body.candidate;
    const questions = [{ id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' }];
    const baseCase = {
      caseId: 'case-001',
      records: CASE_001_RECORDS,
      quarantined: candidate.quarantined,
      candidateHash: candidate.contentHash,
      questions,
    };
    const approve = async () => {
      const resolved = await session.resolveTarget();
      expect(resolved.status).toBe(200);
      const target = resolved.body as { provider: string; model: string; endpoint: string; locality: string; cost: string };
      return {
        target,
        approvalHash: computeApprovalHash({
          cases: [{ caseId: 'case-001', candidateHash: candidate.contentHash, questions }],
          provider: target.provider, model: target.model,
          endpoint: target.endpoint, locality: target.locality, cost: target.cost,
          cloud: false, maxTotalTokens: 20000,
        }),
      };
    };
    const startWith = (approval: { target: { provider: string; model: string }; approvalHash: string }) =>
      session.call('ai', 'somnia-compaction-start', {
        workspaceId: session.workspaceId, projectId: session.projectId, cases: [baseCase],
        provider: approval.target.provider, model: approval.target.model,
        cloud: false, maxTotalTokens: 20000, approvalHash: approval.approvalHash,
      });
    // Control: the resolved default (llama-x) starts cleanly.
    const approval = await approve();
    expect(approval.target.model).toBe('llama-x');
    const hitsBefore = session.fakeHits();
    const ok = await startWith(approval);
    expect(ok.status).toBe(200);
    const poll = await waitForRun(session, ok.body.runId as string);
    expect(poll.body.status).toBe('complete');
    expect(session.fakeHits() - hitsBefore).toBe(2);
    expect(session.fakeModels().slice(-2)).toEqual(['llama-x', 'llama-x']);
    // Reorder only: the approved target itself did not change, so the
    // same approval still starts — and still uses llama-x, not the
    // new first model. A changed default must never redirect an arm.
    const reordered = JSON.parse(fs.readFileSync(session.providerConfigPath, 'utf8')) as {
      providers: Array<{ config: { provider: string }; models: Array<Record<string, unknown>> }>;
    };
    const ollama = reordered.providers.find(entry => entry.config.provider === 'ollama')!;
    ollama.models.reverse();
    fs.writeFileSync(session.providerConfigPath, JSON.stringify(reordered));
    await session.restartApi();
    const afterReorder = await startWith(approval);
    expect(afterReorder.status).toBe(200);
    const reorderPoll = await waitForRun(session, afterReorder.body.runId as string);
    expect(reorderPoll.body.status).toBe('complete');
    expect(session.fakeModels().slice(-2)).toEqual(['llama-x', 'llama-x']);
    // Remove the approved model: the old approval must refuse before
    // traffic — it cannot silently run the new first model.
    const removed = JSON.parse(fs.readFileSync(session.providerConfigPath, 'utf8')) as {
      providers: Array<{ config: { provider: string }; models: Array<Record<string, any>> }>;
    };
    const ollama2 = removed.providers.find(entry => entry.config.provider === 'ollama')!;
    ollama2.models = ollama2.models.filter((model: Record<string, any>) => model.id !== 'llama-x');
    fs.writeFileSync(session.providerConfigPath, JSON.stringify(removed));
    await session.restartApi();
    const hitsBeforeRemoval = session.fakeHits();
    const stale = await startWith(approval);
    expect(stale.status).toBe(400);
    expect(session.fakeHits()).toBe(hitsBeforeRemoval);
    // A fresh review resolves the new default and runs it.
    const fresh = await approve();
    expect(fresh.target.model).toBe('aaa-decoy');
    const freshOk = await startWith(fresh);
    expect(freshOk.status).toBe(200);
    const freshPoll = await waitForRun(session, freshOk.body.runId as string);
    expect(freshPoll.body.status).toBe('complete');
    expect(session.fakeModels().slice(-2)).toEqual(['aaa-decoy', 'aaa-decoy']);
  } finally {
    await session.cleanup();
  }
});

test('SOMNIA compaction refuses a stale approval after the configured endpoint changes', async ({ page }) => {
  test.setTimeout(240_000);
  resetFakeBehavior();
  setFakeDelayMs(0);
  setFakeExpectedAnswers([
    [fakeAnswerKey('What is the customs fee now?', renderHistory(CASE_001_RECORDS)), 'The customs fee is $45.'],
    [fakeAnswerKey('What is the customs fee now?', renderHistory([CASE_001_RECORDS[1]])), 'The customs fee is $45.'],
  ]);
  const session = await startCompactionSession(page);
  // A second loopback endpoint, hit-counted: the reconfigured target
  // must never be dispatched to under the old approval.
  let secondHits = 0;
  const second: Server = createServer((_req, res) => {
    secondHits += 1;
    res.writeHead(200, { 'content-type': 'application/json' });
    res.end(JSON.stringify({
      model: 'llama-x', message: { role: 'assistant', content: 'UNKNOWN' },
      done: true, prompt_eval_count: 12, eval_count: 4,
    }));
  });
  await new Promise<void>(r => second.listen(0, '127.0.0.1', r));
  const secondUrl = `http://127.0.0.1:${(second.address() as { port: number }).port}`;
  try {
    const envelope = JSON.stringify({ format: 'anansi-somnia-compaction-input', version: 1, records: CASE_001_RECORDS });
    const prepared = await session.call('ai', 'somnia-compaction-prepare', {
      workspaceId: session.workspaceId, projectId: session.projectId, inputText: envelope, inputFormat: 'json',
    });
    expect(prepared.status).toBe(200);
    const candidate = prepared.body.candidate;
    const questions = [{ id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' }];
    const baseCase = {
      caseId: 'case-001',
      records: CASE_001_RECORDS,
      quarantined: candidate.quarantined,
      candidateHash: candidate.contentHash,
      questions,
    };
    const approve = async () => {
      const resolved = await session.resolveTarget();
      expect(resolved.status).toBe(200);
      const target = resolved.body as { provider: string; model: string; endpoint: string; locality: string; cost: string };
      return {
        target,
        approvalHash: computeApprovalHash({
          cases: [{ caseId: 'case-001', candidateHash: candidate.contentHash, questions }],
          provider: target.provider, model: target.model,
          endpoint: target.endpoint, locality: target.locality, cost: target.cost,
          cloud: false, maxTotalTokens: 20000,
        }),
      };
    };
    const startWith = (approval: { target: { provider: string; model: string }; approvalHash: string }) =>
      session.call('ai', 'somnia-compaction-start', {
        workspaceId: session.workspaceId, projectId: session.projectId, cases: [baseCase],
        provider: approval.target.provider, model: approval.target.model,
        cloud: false, maxTotalTokens: 20000, approvalHash: approval.approvalHash,
      });
    const approval = await approve();
    const firstEndpoint = approval.target.endpoint;
    const ok = await startWith(approval);
    expect(ok.status).toBe(200);
    const poll = await waitForRun(session, ok.body.runId as string);
    expect(poll.body.status).toBe('complete');
    // Point the same provider/model at the second endpoint.
    const moved = JSON.parse(fs.readFileSync(session.providerConfigPath, 'utf8')) as {
      providers: Array<{ config: { provider: string; baseUrl: string } }>;
    };
    const ollama = moved.providers.find(entry => entry.config.provider === 'ollama')!;
    expect(ollama.config.baseUrl).toBe(firstEndpoint);
    ollama.config.baseUrl = secondUrl;
    fs.writeFileSync(session.providerConfigPath, JSON.stringify(moved));
    await session.restartApi();
    const hitsBefore = session.fakeHits();
    const stale = await startWith(approval);
    expect(stale.status).toBe(400);
    expect(session.fakeHits()).toBe(hitsBefore);
    expect(secondHits).toBe(0);
    // A fresh review binds the new endpoint and runs against it.
    const fresh = await approve();
    expect(fresh.target.endpoint).toBe(secondUrl);
    const freshOk = await startWith(fresh);
    expect(freshOk.status).toBe(200);
    const freshPoll = await waitForRun(session, freshOk.body.runId as string);
    expect(freshPoll.body.status).toBe('complete');
    expect(secondHits).toBe(2);
  } finally {
    await new Promise<void>(r => second.close(() => r()));
    await session.cleanup();
  }
});

test('SOMNIA compaction labels each arm with the strongest privacy of its history', async ({ page }) => {
  test.setTimeout(240_000);
  resetFakeBehavior();
  setFakeDelayMs(0);
  const originalHistory = renderHistory(PRIVATE_CASE_RECORDS);
  const compactedHistory = renderHistory([PRIVATE_CASE_RECORDS[1]]);
  setFakeExpectedAnswers([
    [fakeAnswerKey('What is the customs fee now?', originalHistory), 'The customs fee is $45.'],
    [fakeAnswerKey('What is the customs fee now?', compactedHistory), 'The customs fee is $45.'],
  ]);
  const session = await startCompactionSession(page, { alternateModel: 'aaa-decoy' });
  try {
    const envelope = JSON.stringify({ format: 'anansi-somnia-compaction-input', version: 1, records: PRIVATE_CASE_RECORDS });
    const prepared = await session.call('ai', 'somnia-compaction-prepare', {
      workspaceId: session.workspaceId, projectId: session.projectId, inputText: envelope, inputFormat: 'json',
    });
    expect(prepared.status).toBe(200);
    const candidate = prepared.body.candidate;
    expect(candidate.stats.retainedRecords).toBe(1);
    expect(candidate.stats.excludedRecords).toBe(1);
    const questions = [{ id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' }];
    const resolved = await session.resolveTarget();
    expect(resolved.status).toBe(200);
    const target = resolved.body as { provider: string; model: string; endpoint: string; locality: string; cost: string };
    const started = await session.call('ai', 'somnia-compaction-start', {
      workspaceId: session.workspaceId, projectId: session.projectId,
      cases: [{
        caseId: 'case-001',
        records: PRIVATE_CASE_RECORDS,
        quarantined: candidate.quarantined,
        candidateHash: candidate.contentHash,
        questions,
      }],
      provider: target.provider, model: target.model,
      cloud: false, maxTotalTokens: 20000,
      approvalHash: computeApprovalHash({
        cases: [{ caseId: 'case-001', candidateHash: candidate.contentHash, questions }],
        provider: target.provider, model: target.model,
        endpoint: target.endpoint, locality: target.locality, cost: target.cost,
        cloud: false, maxTotalTokens: 20000,
      }),
    });
    expect(started.status).toBe(200);
    const poll = await waitForRun(session, started.body.runId as string);
    expect(poll.body.status).toBe('complete');
    expect(poll.body.callsMade).toBe(2);
    expect(session.fakeModels()).toEqual(['llama-x', 'llama-x']);
    const pair = poll.body.pairs[0];
    // The original arm renders the Private record: labelled private.
    expect(pair.originalPrivacy).toBe('private');
    // The compacted arm renders only the standard record: standard,
    // disclosed as absent (the default needs no marker).
    expect(pair.compactedPrivacy).toBeUndefined();
    expect(pair.originalScore).toBe('correct');
    expect(pair.compactedScore).toBe('correct');
  } finally {
    await session.cleanup();
  }
});

test('SOMNIA compaction sends Private history to the cloud only with explicit approval', async ({ page }) => {
  test.setTimeout(240_000);
  resetFakeBehavior();
  setFakeDelayMs(0);
  const originalHistory = renderHistory(PRIVATE_CASE_RECORDS);
  const compactedHistory = renderHistory([PRIVATE_CASE_RECORDS[1]]);
  setFakeExpectedAnswers([
    [fakeAnswerKey('What is the customs fee now?', originalHistory), 'The customs fee is $45.'],
    [fakeAnswerKey('What is the customs fee now?', compactedHistory), 'The customs fee is $45.'],
  ]);
  const session = await startCompactionSession(page, { cloudModel: { id: 'free-cloud-model', cost: 'free' } });
  try {
    const envelope = JSON.stringify({ format: 'anansi-somnia-compaction-input', version: 1, records: PRIVATE_CASE_RECORDS });
    const prepared = await session.call('ai', 'somnia-compaction-prepare', {
      workspaceId: session.workspaceId, projectId: session.projectId, inputText: envelope, inputFormat: 'json',
    });
    expect(prepared.status).toBe(200);
    const candidate = prepared.body.candidate;
    const questions = [{ id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' }];
    const baseCase = {
      caseId: 'case-001',
      records: PRIVATE_CASE_RECORDS,
      quarantined: candidate.quarantined,
      candidateHash: candidate.contentHash,
      questions,
    };
    const resolved = await session.resolveTarget('openrouter', 'free-cloud-model');
    expect(resolved.status).toBe(200);
    const target = resolved.body as { provider: string; model: string; endpoint: string; locality: string; cost: string };
    expect(target.locality).toBe('cloud');
    const hashFor = (cloud: boolean) =>
      computeApprovalHash({
        cases: [{ caseId: 'case-001', candidateHash: candidate.contentHash, questions }],
        provider: target.provider, model: target.model,
        endpoint: target.endpoint, locality: target.locality, cost: target.cost,
        cloud, maxTotalTokens: 20000,
      });
    // Authorized: the Private history goes to the cloud fake with the
    // segment labelled private, and the run completes.
    const cloudHitsBefore = session.cloudFakeHits();
    const ok = await session.call('ai', 'somnia-compaction-start', {
      workspaceId: session.workspaceId, projectId: session.projectId, cases: [baseCase],
      provider: target.provider, model: target.model,
      cloud: true, maxTotalTokens: 20000, approvalHash: hashFor(true),
    });
    expect(ok.status).toBe(200);
    const poll = await waitForRun(session, ok.body.runId as string);
    expect(poll.body.status).toBe('complete');
    expect(session.cloudFakeHits() - cloudHitsBefore).toBe(2);
    expect(session.cloudFakeModels()).toEqual(['free-cloud-model', 'free-cloud-model']);
    expect(poll.body.pairs[0].originalPrivacy).toBe('private');
    expect(poll.body.pairs[0].compactedPrivacy).toBeUndefined();
    // Unauthorized: the cloud flag flipped after approval (hash over
    // cloud=false, request cloud=true) is refused with zero cloud hits.
    const cloudHitsAfter = session.cloudFakeHits();
    const tampered = await session.call('ai', 'somnia-compaction-start', {
      workspaceId: session.workspaceId, projectId: session.projectId, cases: [baseCase],
      provider: target.provider, model: target.model,
      cloud: true, maxTotalTokens: 20000, approvalHash: hashFor(false),
    });
    expect(tampered.status).toBe(400);
    expect(session.cloudFakeHits()).toBe(cloudHitsAfter);
  } finally {
    await session.cleanup();
  }
});

test('SOMNIA compaction refuses Sealed records before any provider traffic', async ({ page }) => {
  test.setTimeout(120_000);
  resetFakeBehavior();
  setFakeDelayMs(0);
  setFakeExpectedAnswers([]);
  const session = await startCompactionSession(page);
  try {
    const envelope = JSON.stringify({ format: 'anansi-somnia-compaction-input', version: 1, records: SEALED_CASE_RECORDS });
    const prepared = await session.call('ai', 'somnia-compaction-prepare', {
      workspaceId: session.workspaceId, projectId: session.projectId, inputText: envelope, inputFormat: 'json',
    });
    expect(prepared.status).toBe(400);
    const hitsBefore = session.fakeHits();
    const started = await session.call('ai', 'somnia-compaction-start', {
      workspaceId: session.workspaceId, projectId: session.projectId,
      cases: [{
        caseId: 'case-001',
        records: SEALED_CASE_RECORDS,
        quarantined: [],
        candidateHash: '0'.repeat(64),
        questions: [{ id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' }],
      }],
      provider: 'ollama', model: 'llama-x',
      cloud: false, maxTotalTokens: 20000,
      approvalHash: '0'.repeat(64),
    });
    expect(started.status).toBe(400);
    expect(session.fakeHits()).toBe(hitsBefore);
  } finally {
    await session.cleanup();
  }
});
