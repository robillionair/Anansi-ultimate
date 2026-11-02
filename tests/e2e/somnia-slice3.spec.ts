import { test, expect, type Locator, type Page } from '@playwright/test';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { startSession } from './collider-three-helpers';

/**
 * M10 SOMNIA Slice 3 in the real browser: the §50 dashboard
 * aggregates over saved evaluations, the §51 Forge QA bridge
 * (seven QA scenarios from a registered Forge concept, run
 * through the unchanged evaluator, saved and exported), the
 * §87.7 reviewed/dismissed proposal state (local only), the
 * three dashboard action triggers (focus), the §87 walk
 * end to end, and backup/restore survival with identical
 * dashboard aggregates. Everything is local and model-free:
 * zero provider traffic.
 */

function sha256(data: Buffer): string {
  return createHash('sha256').update(data).digest('hex');
}

async function backupAndRestore(
  session: Awaited<ReturnType<typeof startSession>>,
  workspaceId: string,
  name: string,
): Promise<string> {
  const backup = await session.call('jobs', 'backup', {
    workspaceId,
    application: false,
    plaintextConsent: true,
  });
  expect(backup.status).toBe(200);
  const jobId = backup.body.job.id as string;
  let job: Record<string, unknown> | undefined;
  for (let i = 0; i < 60 && (!job || job.state !== 'Completed'); i++) {
    await new Promise(r => setTimeout(r, 1000));
    const list = await session.call('jobs', 'list', { workspaceId });
    job = (list.body.jobs as Array<Record<string, unknown>>).find(j => j.id === jobId);
  }
  expect(job!.state).toBe('Completed');
  const artifactBytes = job!.artifactBytes as number;
  const chunks: number[] = [];
  for (let offset = 0; offset < artifactBytes;) {
    const chunk = await session.call('jobs', 'artifact', {
      workspaceId,
      jobId,
      artifactId: job!.artifactId as string,
      offset,
    });
    const bytes = chunk.body.bytes as number[];
    expect(bytes.length).toBeGreaterThan(0);
    chunks.push(...bytes);
    offset += bytes.length;
  }
  const tar = Buffer.from(chunks);
  const landing = await session.call('workspaces', 'create', { name: `${name} Landing` });
  const landingId = landing.body.workspace.id as string;
  const begin = await session.call('jobs', 'upload-begin', {
    workspaceId: landingId,
    bytes: tar.length,
    sha256: sha256(tar),
    plaintextConsent: true,
  });
  const uploadId = begin.body.id as string;
  let uploadArtifactId = '';
  let importJobId = '';
  for (let offset = 0; offset < tar.length; offset += 65536) {
    const end = Math.min(offset + 65536, tar.length);
    const part = await session.call('jobs', 'upload-part', {
      workspaceId: landingId,
      id: uploadId,
      offset,
      bytes: Array.from(tar.subarray(offset, end)),
      finish: end === tar.length,
    });
    if (part.body.job?.artifactId) uploadArtifactId = part.body.job.artifactId as string;
    if (part.body.job?.id) importJobId = part.body.job.id as string;
  }
  expect(uploadArtifactId).not.toBe('');
  expect(importJobId).not.toBe('');
  const previewRestore = await session.call('jobs', 'preview', {
    workspaceId: landingId,
    jobId: importJobId,
    artifactId: uploadArtifactId,
  });
  const restored = await session.call('jobs', 'restore', {
    workspaceId: landingId,
    jobId: importJobId,
    artifactId: uploadArtifactId,
    expectedHash: previewRestore.body.sha256 as string,
    confirm: true,
    name,
  });
  const restoredWs = restored.body.workspaces[0].id as string;
  expect(restoredWs).not.toBe(workspaceId);
  return restoredWs;
}

async function comparisonKeys(
  comparisonSection: Locator,
): Promise<{ baseline: string; nightmare: string }> {
  const options = await comparisonSection
    .getByLabel('SOMNIA comparison baseline')
    .locator('option')
    .evaluateAll(opts =>
      opts.map(o => ({
        value: (o as HTMLOptionElement).value,
        text: o.textContent ?? '',
      })),
    );
  const nightmare = options.find(o => o.value !== '' && o.text.includes('Nightmare'));
  const baseline = options.find(o => o.value !== '' && !o.text.includes('Nightmare'));
  expect(nightmare).toBeTruthy();
  expect(baseline).toBeTruthy();
  return { baseline: baseline!.value, nightmare: nightmare!.value };
}

async function saveBaselineAndNightmare(page: Page): Promise<void> {
  const somnia = page.locator('section[aria-labelledby="project-somnia-heading"]');
  const results = somnia.locator('section[aria-label="SOMNIA evaluation results"]');
  await somnia.getByRole('button', { name: 'Load built-in cases' }).click();
  await somnia.getByRole('button', { name: 'Preview replay input' }).click();
  await somnia.getByRole('button', { name: 'Run evaluation' }).click();
  await expect(results.getByText('Scenarios: 5. Complete: 1. Refuse: 3. Abort: 1.')).toBeVisible();
  await somnia.getByRole('button', { name: 'Save SOMNIA evaluation' }).click();
  await expect(page.getByText('SOMNIA evaluation saved.')).toBeVisible();

  await somnia.getByLabel('SOMNIA nightmare seed').fill('somnia-nightmare-seed');
  await somnia.getByLabel('SOMNIA nightmare count').selectOption('10');
  await somnia.getByRole('button', { name: 'Generate SOMNIA nightmares' }).click();
  await somnia.getByRole('button', { name: 'Preview replay input' }).click();
  await somnia.getByRole('button', { name: 'Exclude sealed case' }).click();
  await somnia.getByRole('button', { name: 'Preview replay input' }).click();
  await somnia.getByRole('button', { name: 'Run evaluation' }).click();
  await expect(results.getByText('Scenarios: 9. Complete: 1. Refuse: 5. Abort: 3.')).toBeVisible();
  await somnia.getByRole('button', { name: 'Save SOMNIA evaluation' }).click();
  await expect(page.getByText('SOMNIA evaluation saved.')).toBeVisible();
}

test('SOMNIA Slice 3: dashboard, Forge QA, proposal review state, §87 walk', async ({
  page,
}) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  const somnia = page.locator('section[aria-labelledby="project-somnia-heading"]');
  const results = somnia.locator('section[aria-label="SOMNIA evaluation results"]');
  const savedList = somnia.locator('section[aria-label="Saved SOMNIA evaluations"]');
  const proposals = somnia.locator('section[aria-label="SOMNIA consolidation proposals"]');
  const comparison = somnia.locator('section[aria-label="SOMNIA comparison"]');
  const dashboard = somnia.locator('section[aria-label="SOMNIA dashboard"]');
  const forgeQa = somnia.locator('section[aria-label="SOMNIA Forge QA"]');
  try {
    const { workspaceId, projectId } = session;
    await session.openProject();
    await saveBaselineAndNightmare(page);
    const hitsBefore = session.fakeHits();

    // §50 dashboard over the two saved runs.
    await dashboard.getByRole('button', { name: 'Load SOMNIA dashboard' }).click();
    await expect(dashboard.getByText('Failure rate: 85.7% (12 of 14 scenarios refused or aborted).')).toBeVisible();
    await expect(dashboard.getByText('Context efficiency: 1.29× baseline calls per scenario (latest 0.77, baseline 0.60).')).toBeVisible();
    await expect(dashboard.getByText('Pending proposals: 2 of 2 for the latest run.')).toBeVisible();
    await expect(dashboard.getByText('Evaluation coverage: 6 of 7 classes (85.7%).')).toBeVisible();
    await expect(dashboard.getByText('Total scenarios executed: 14.')).toBeVisible();
    await expect(dashboard.getByText('Improvements proposed: 3.')).toBeVisible();
    await expect(dashboard.getByText('failures 4 → 8')).toBeVisible();
    await expect(dashboard.getByText('Failure categories: constraint-violated: 2, instruction-conflict: 3, missing-evidence: 3, tool-error: 2, tool-timeout: 2, untrusted-instruction: 2.')).toBeVisible();

    // Latest run identity comes from the saved records, not from
    // the dashboard code path: read both docs and their records.
    const list = await session.call('artifacts', 'list', { workspaceId, projectId });
    const saved = (list.body.artifacts as Array<Record<string, unknown>>).filter(
      a => a.type === 'somnia-evaluation',
    );
    expect(saved).toHaveLength(2);
    const evidenceCounts = new Map<string, number>();
    const sourceCounts = new Map<string, number>();
    let latestRunId = '';
    let latestTimestamp = '';
    for (const artifact of saved) {
      const read = await session.call('artifacts', 'read', {
        workspaceId,
        artifactId: artifact.artifactId,
        version: artifact.version,
      });
      const rawBody = (read.body as { body: unknown }).body;
      const doc =
        typeof rawBody === 'string'
          ? (JSON.parse(rawBody) as Record<string, unknown>)
          : (rawBody as Record<string, unknown>);
      if ((artifact.timestamp as string) >= latestTimestamp) {
        latestTimestamp = artifact.timestamp as string;
        latestRunId = doc.runId as string;
      }
      const input = doc.input as {
        scenarios: Array<{
          evidence: Array<{ id: string }>;
          retrieved: Array<{ source: string }>;
        }>;
      };
      for (const scenario of input.scenarios) {
        for (const record of scenario.evidence) {
          evidenceCounts.set(record.id, (evidenceCounts.get(record.id) ?? 0) + 1);
        }
        for (const record of scenario.retrieved) {
          sourceCounts.set(record.source, (sourceCounts.get(record.source) ?? 0) + 1);
        }
      }
    }
    const dupEvidence = [...evidenceCounts.values()].filter(n => n > 1).length;
    const dupSources = [...sourceCounts.values()].filter(n => n > 1).length;
    await expect(dashboard.getByText(`Duplicate evidence ids: ${dupEvidence}.`)).toBeVisible();
    await expect(dashboard.getByText(`Duplicate retrieved sources: ${dupSources}.`)).toBeVisible();
    await expect(dashboard.getByText(`Latest run: ${latestTimestamp} (${latestRunId}).`)).toBeVisible();

    // Dashboard action triggers move focus to their sections.
    await dashboard.getByRole('button', { name: 'Run Nightmare Test' }).click();
    await expect(page.locator('#somnia-nightmare-generator')).toBeFocused();
    await dashboard.getByRole('button', { name: 'Review Improvement Proposals' }).click();
    await expect(page.locator('#somnia-proposals')).toBeFocused();
    await expect(proposals.getByText('procedural-rule — max-tool-calls:2')).toBeVisible();
    await dashboard.getByRole('button', { name: 'View Results / Compare' }).click();
    await expect(page.locator('#somnia-comparison')).toBeFocused();

    // §87.7: reviewed + dismissed are view-local only.
    await expect(proposals.getByRole('button', { name: /apply/i })).toHaveCount(0);
    await proposals.getByRole('button', { name: 'Mark as reviewed' }).first().click();
    await expect(proposals.getByRole('button', { name: 'Reviewed' }).first()).toBeVisible();
    // Dismiss the OTHER proposal, so both end up handled and
    // pending drops to zero (reviewed ∪ dismissed covers both).
    await proposals.getByRole('button', { name: 'Dismiss' }).nth(1).click();
    await expect(proposals.getByRole('button', { name: 'Dismissed' })).toHaveCount(1);
    const afterReview = await session.call('artifacts', 'list', { workspaceId, projectId });
    expect(
      (afterReview.body.artifacts as Array<Record<string, unknown>>).filter(
        a => a.type === 'somnia-evaluation',
      ),
    ).toHaveLength(2);
    await dashboard.getByRole('button', { name: 'Load SOMNIA dashboard' }).click();
    await expect(dashboard.getByText('Pending proposals: 0 of 2 for the latest run.')).toBeVisible();

    // §87.9: compare before/after (baseline vs nightmare).
    const keys = await comparisonKeys(comparison);
    await comparison.getByLabel('SOMNIA comparison baseline').selectOption(keys.baseline);
    await comparison.getByLabel('SOMNIA comparison target').selectOption(keys.nightmare);
    await comparison.getByRole('button', { name: 'Compare SOMNIA evaluations' }).click();
    const metricsTable = comparison.locator('table').first();
    await expect(
      metricsTable.locator('tr', { hasText: 'Refuse' }).getByText('+2', { exact: true }),
    ).toBeVisible();

    // §51 Forge QA bridge: register a Forge concept, then run the
    // seven QA scenarios through the unchanged evaluator.
    await session.call('artifacts', 'register', {
      workspaceId,
      projectId,
      type: 'concept',
      title: 'Bean Stock Log',
      body: JSON.stringify({ purpose: 'Track bean stock without a spreadsheet.' }),
      sourceReferences: [],
      privacy: 'private',
    });
    await forgeQa.getByRole('button', { name: 'Load Forge concepts' }).click();
    await expect(forgeQa.getByText('Bean Stock Log (v1)')).toBeVisible();
    await expect(
      forgeQa.getByText(
        'SOMNIA augments QA; it does not replace ordinary automated software and security testing.',
      ),
    ).toBeVisible();
    await forgeQa.getByRole('button', { name: 'Run SOMNIA QA check' }).click();
    await expect(somnia.getByLabel('SOMNIA replay input')).toHaveValue(
      /Forge QA \[Bean Stock Log\]: API failure/,
    );
    await somnia.getByRole('button', { name: 'Preview replay input' }).click();
    await somnia.getByRole('button', { name: 'Run evaluation' }).click();
    await expect(results.getByText('Scenarios: 7. Complete: 1. Refuse: 3. Abort: 3.')).toBeVisible();
    await expect(results.getByText('Simulated tool calls: 4.')).toBeVisible();
    await somnia.getByRole('button', { name: 'Save SOMNIA evaluation' }).click();
    await expect(page.getByText('SOMNIA evaluation saved.')).toBeVisible();
    await expect(savedList.getByRole('button', { name: 'View evaluation' })).toHaveCount(3);

    const qaJsonPromise = page.waitForEvent('download');
    await forgeQa.getByRole('button', { name: 'Export QA summary JSON' }).click();
    const qaJson = await qaJsonPromise;
    const qaJsonPath = await qaJson.path();
    expect(qaJsonPath).not.toBeNull();
    const qaDoc = JSON.parse(await readFile(qaJsonPath as string, 'utf8')) as Record<
      string,
      unknown
    >;
    expect((qaDoc.metrics as Record<string, unknown>).scenariosTotal).toBe(7);
    const qaMdPromise = page.waitForEvent('download');
    await forgeQa.getByRole('button', { name: 'Export QA summary Markdown' }).click();
    const qaMd = await qaMdPromise;
    const qaMdPath = await qaMd.path();
    expect(qaMdPath).not.toBeNull();
    const qaMarkdown = await readFile(qaMdPath as string, 'utf8');
    expect(qaMarkdown).toContain('SOMNIA QA summary for Bean Stock Log');
    expect(qaMarkdown).toContain('Complete: 1');

    expect(session.fakeHits()).toBe(hitsBefore);
  } finally {
    await session.cleanup();
  }
});

test('SOMNIA Slice 3: backup/restore keeps dashboard aggregates identical', async ({
  page,
}) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  const somnia = page.locator('section[aria-labelledby="project-somnia-heading"]');
  const dashboard = somnia.locator('section[aria-label="SOMNIA dashboard"]');
  const savedList = somnia.locator('section[aria-label="Saved SOMNIA evaluations"]');
  try {
    const { workspaceId } = session;
    await session.openProject();
    await saveBaselineAndNightmare(page);

    await dashboard.getByRole('button', { name: 'Load SOMNIA dashboard' }).click();
    await expect(dashboard.getByText('Failure rate: 85.7% (12 of 14 scenarios refused or aborted).')).toBeVisible();
    await expect(dashboard.getByText('Total scenarios executed: 14.')).toBeVisible();
    await expect(dashboard.getByText('Evaluation coverage: 6 of 7 classes (85.7%).')).toBeVisible();

    const restoredWs = await backupAndRestore(session, workspaceId, 'somnia-slice3');
    await session.pairAndOpen(restoredWs);
    await session.openProject();
    await expect(savedList.getByRole('button', { name: 'View evaluation' })).toHaveCount(2);
    await dashboard.getByRole('button', { name: 'Load SOMNIA dashboard' }).click();
    await expect(dashboard.getByText('Failure rate: 85.7% (12 of 14 scenarios refused or aborted).')).toBeVisible();
    await expect(dashboard.getByText('Total scenarios executed: 14.')).toBeVisible();
    await expect(dashboard.getByText('Evaluation coverage: 6 of 7 classes (85.7%).')).toBeVisible();
    await expect(dashboard.getByText('Improvements proposed: 3.')).toBeVisible();
  } finally {
    await session.cleanup();
  }
});
