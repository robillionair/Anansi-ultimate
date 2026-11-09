import { expect, test } from '@playwright/test';
import {
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
  const session = await startCompactionSession(page);
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
    });
    expect(edited.status).toBe(400);
    expect(session.fakeHits()).toBe(hitsBefore);
    // The untouched approval starts cleanly.
    const ok = await session.call('ai', 'somnia-compaction-start', {
      workspaceId: session.workspaceId, projectId: session.projectId, cases: [baseCase],
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
    const started = await session.call('ai', 'somnia-compaction-start', {
      workspaceId: session.workspaceId, projectId: session.projectId,
      cases: [{
        caseId: 'case-001',
        records: CASE_001_RECORDS,
        quarantined: prepared.body.candidate.quarantined,
        candidateHash: prepared.body.candidate.contentHash,
        questions: [{ id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' }],
      }],
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
    const started = await session.call('ai', 'somnia-compaction-start', {
      workspaceId: session.workspaceId, projectId: session.projectId,
      cases: [{
        caseId: 'case-001',
        records: CASE_001_RECORDS,
        quarantined: prepared.body.candidate.quarantined,
        candidateHash: prepared.body.candidate.contentHash,
        questions: [{ id: 'q1', text: 'What is the customs fee now?', expectedAnswer: 'The customs fee is $45.' }],
      }],
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
