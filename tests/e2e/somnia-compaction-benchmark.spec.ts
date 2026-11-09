import { expect, test } from '@playwright/test';
import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import {
  fakeAnswerKey,
  renderHistory,
  setFakeDelayMs,
  setFakeExpectedAnswers,
  startCompactionSession,
} from './somnia-compaction-helpers';

/**
 * SOMNIA compaction Checkpoint 2: the frozen 40-case benchmark through
 * automatic compaction with the synthetic provider.
 *
 * The fake is a verbatim extractor: it replies with the expected answer
 * when the arm's history contains it verbatim, and UNKNOWN otherwise
 * (or when the history should not state it). Accuracy through this fake
 * is synthetic by construction — it measures whether the compacted
 * history still contains the answer string, not model quality. The
 * spec asserts pipeline properties (50 pairs, 100 dispatches, no failed
 * pairs, tokens reported for every pair) and writes the full per-pair
 * raw results for the handoff report. Saving is proven by the workflow
 * spec; this spec reports the rerun.
 */

type FixtureCase = {
  id: string;
  category: string;
  records: Array<Record<string, unknown>>;
  questions: Array<{ id: string; text: string; expectedAnswer: string | null }>;
};

async function loadFixtureCases(): Promise<FixtureCase[]> {
  const dir = join('crates/core/tests/fixtures/somnia-compaction');
  const files = (await readdir(dir)).filter(name => /^case-\d{3}\.json$/.test(name)).sort();
  const cases: FixtureCase[] = [];
  for (const file of files) {
    cases.push(JSON.parse(await readFile(join(dir, file), 'utf8')) as FixtureCase);
  }
  return cases;
}

test('frozen benchmark reruns through automatic compaction (synthetic by default, live with ANANSI_SOMNIA_LIVE_ENDPOINT)', async ({ page }, testInfo) => {
  test.setTimeout(300_000);
  setFakeDelayMs(0);
  // Live mode follows the model-comparison spec's pattern: with
  // ANANSI_SOMNIA_LIVE_ENDPOINT set, the run uses that authorized
  // Ollama endpoint and model instead of the fake, and the report is
  // labelled live. Without it the synthetic proof runs.
  const liveEndpoint = process.env.ANANSI_SOMNIA_LIVE_ENDPOINT;
  const liveModel = liveEndpoint ? (process.env.ANANSI_SOMNIA_LIVE_MODEL ?? 'qwen2.5:3b') : 'llama-x';
  const live = liveEndpoint !== undefined;
  const session = await startCompactionSession(
    page,
    live ? { liveEndpoint, liveModel } : undefined,
  );
  try {
    const cases = await loadFixtureCases();
    expect(cases.length).toBe(40);
    const totalQuestions = cases.reduce((total, entry) => total + entry.questions.length, 0);
    expect(totalQuestions).toBe(50);

    // Prepare every case first (no provider traffic) and register the
    // fake's answer key for both arms: all records, and retained only.
    const expectedEntries: Array<[string, string | null]> = [];
    const startCases: Array<Record<string, unknown>> = [];
    for (const fixtureCase of cases) {
      const envelope = JSON.stringify({
        format: 'anansi-somnia-compaction-input',
        version: 1,
        records: fixtureCase.records,
      });
      const prepared = await session.call('ai', 'somnia-compaction-prepare', {
        workspaceId: session.workspaceId,
        projectId: session.projectId,
        inputText: envelope,
        inputFormat: 'json',
      });
      expect(prepared.status).toBe(200);
      const candidate = prepared.body.candidate;
      const retainedRecords: Array<Record<string, unknown>> = [];
      for (const section of ['trusted', 'unverified', 'conflicts', 'deletions', 'uncertainties'] as const) {
        for (const entry of candidate.retained[section] as Array<{ record: Record<string, unknown> }>) {
          retainedRecords.push(entry.record);
        }
      }
      for (const question of fixtureCase.questions) {
        expectedEntries.push([fakeAnswerKey(question.text, renderHistory(fixtureCase.records)), question.expectedAnswer]);
        expectedEntries.push([fakeAnswerKey(question.text, renderHistory(retainedRecords)), question.expectedAnswer]);
      }
      const excludedRecords = (candidate.excluded as Array<{ record: Record<string, unknown> }>).map(entry => entry.record);
      startCases.push({
        caseId: fixtureCase.id,
        records: [...retainedRecords, ...excludedRecords],
        quarantined: candidate.quarantined,
        candidateHash: candidate.contentHash,
        questions: fixtureCase.questions.map(question => ({
          id: question.id,
          text: question.text,
          expectedAnswer: question.expectedAnswer,
        })),
      });
    }
    setFakeExpectedAnswers(expectedEntries);
    const hitsBefore = session.fakeHits();

    const started = await session.call('ai', 'somnia-compaction-start', {
      workspaceId: session.workspaceId,
      projectId: session.projectId,
      cases: startCases,
      // A live 100-call run estimates ~50k tokens against the per-call
      // estimate (prompt chars/4 + 384), so it needs the full budget;
      // the synthetic run keeps the 20k default.
      ...(live ? { provider: 'ollama', model: liveModel, maxTotalTokens: 200000 } : {}),
    });
    expect(started.status).toBe(200);
    expect(started.body.pairsTotal).toBe(50);
    expect(started.body.estimatedCalls).toBe(100);
    const runId = started.body.runId as string;

    let poll = await session.call('ai', 'somnia-compaction-poll', {
      workspaceId: session.workspaceId, projectId: session.projectId, runId,
    });
    const deadline = Date.now() + 240_000;
    while (poll.body.status === 'running' && Date.now() < deadline) {
      await new Promise(resolve => setTimeout(resolve, 1000));
      poll = await session.call('ai', 'somnia-compaction-poll', {
        workspaceId: session.workspaceId, projectId: session.projectId, runId,
      });
    }
    expect(poll.body.status).toBe('complete');
    expect(poll.body.pairsTotal).toBe(50);
    expect(poll.body.pairsDone).toBe(50);
    expect(poll.body.callsMade).toBe(100);
    if (!live) {
      expect(session.fakeHits() - hitsBefore).toBe(100);
    }
    const pairs = poll.body.pairs as Array<{
      pairId: string; caseId: string; questionId: string; question: string;
      expectedAnswer: string | null; originalReply: string | null; compactedReply: string | null;
      originalScore: string; compactedScore: string;
      originalUsage: { totalTokens: number } | null; compactedUsage: { totalTokens: number } | null;
      failure: string | null; compactionMicros: number;
    }>;
    expect(pairs.length).toBe(50);
    const failed = pairs.filter(pair => pair.originalScore === 'failed' || pair.compactedScore === 'failed');
    const correctOriginal = pairs.filter(pair => pair.originalScore === 'correct').length;
    const correctCompacted = pairs.filter(pair => pair.compactedScore === 'correct').length;
    const abstentionsOriginal = pairs.filter(pair => pair.originalScore === 'abstained').length;
    const abstentionsCompacted = pairs.filter(pair => pair.compactedScore === 'abstained').length;
    const tokensOriginal = pairs.reduce((total, pair) => total + (pair.originalUsage?.totalTokens ?? 0), 0);
    const tokensCompacted = pairs.reduce((total, pair) => total + (pair.compactedUsage?.totalTokens ?? 0), 0);
    const reportedOriginalPairs = pairs.filter(pair => pair.originalUsage?.totalTokens !== null && pair.originalUsage?.totalTokens !== undefined).length;
    const reportedCompactedPairs = pairs.filter(pair => pair.compactedUsage?.totalTokens !== null && pair.compactedUsage?.totalTokens !== undefined).length;
    if (!live) {
      // Synthetic proof: every dispatch lands, nothing fails, every
      // pair reports the fake's 16 tokens.
      expect(failed.length).toBe(0);
      const allReported = pairs.every(pair => pair.originalUsage?.totalTokens === 16 && pair.compactedUsage?.totalTokens === 16);
      expect(allReported).toBe(true);
      expect(tokensOriginal).toBe(800);
      expect(tokensCompacted).toBe(800);
    }
    expect(poll.body.compactionMicrosTotal).toBeGreaterThan(0);
    expect(poll.body.totalLatencyMs).toBeGreaterThan(0);

    const byCategory: Record<string, { pairs: number; correctOriginal: number; correctCompacted: number; abstentionsOriginal: number; abstentionsCompacted: number; failedPairs: number }> = {};
    for (const fixtureCase of cases) {
      const casePairs = pairs.filter(pair => pair.caseId === fixtureCase.id);
      byCategory[fixtureCase.category] = byCategory[fixtureCase.category] ?? {
        pairs: 0, correctOriginal: 0, correctCompacted: 0,
        abstentionsOriginal: 0, abstentionsCompacted: 0, failedPairs: 0,
      };
      const entry = byCategory[fixtureCase.category];
      entry.pairs += casePairs.length;
      entry.correctOriginal += casePairs.filter(pair => pair.originalScore === 'correct').length;
      entry.correctCompacted += casePairs.filter(pair => pair.compactedScore === 'correct').length;
      entry.abstentionsOriginal += casePairs.filter(pair => pair.originalScore === 'abstained').length;
      entry.abstentionsCompacted += casePairs.filter(pair => pair.compactedScore === 'abstained').length;
      entry.failedPairs += casePairs.filter(pair => pair.originalScore === 'failed' || pair.compactedScore === 'failed').length;
    }

    const report = {
      provider: live ? `live ollama ${liveModel} at ${liveEndpoint}` : 'synthetic-verbatim-extractor (fake ollama llama-x)',
      accuracyLabel: live ? 'live' : 'synthetic',
      cases: cases.length,
      pairsTotal: 50,
      callsMade: 100,
      correctOriginal,
      correctCompacted,
      abstentionsOriginal,
      abstentionsCompacted,
      failedPairs: failed.length,
      tokensOriginal,
      tokensCompacted,
      tokensReportedOriginalPairs: reportedOriginalPairs,
      tokensReportedCompactedPairs: reportedCompactedPairs,
      compactionMicrosTotal: poll.body.compactionMicrosTotal,
      totalLatencyMs: poll.body.totalLatencyMs,
      byCategory,
      pairs: pairs.map(pair => ({
        pairId: pair.pairId,
        caseId: pair.caseId,
        question: pair.question,
        expectedAnswer: pair.expectedAnswer,
        originalReply: pair.originalReply,
        compactedReply: pair.compactedReply,
        originalScore: pair.originalScore,
        compactedScore: pair.compactedScore,
        compactionMicros: pair.compactionMicros,
      })),
    };
    await testInfo.attach('somnia-compaction-benchmark-synthetic.json', {
      body: JSON.stringify(report, null, 2),
      contentType: 'application/json',
    });
    // Also write the raw report to disk: the handoff ships it as evidence.
    const { writeFile } = await import('node:fs/promises');
    await writeFile(
      testInfo.outputPath('somnia-compaction-benchmark-synthetic.json'),
      JSON.stringify(report, null, 2),
    );
    // The report is the deliverable of this rerun; log the headline so
    // the run output carries it too.
    console.log(
      `SOMNIA compaction benchmark (${live ? 'live' : 'synthetic'}): ${correctOriginal}/50 original, ` +
      `${correctCompacted}/50 compacted, ${failed.length} failed pairs, ` +
      `${tokensOriginal + tokensCompacted} tokens, ${poll.body.compactionMicrosTotal} µs compaction.`,
    );
  } finally {
    await session.cleanup();
  }
});
