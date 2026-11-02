import { test, expect, type Locator } from '@playwright/test';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { startSession } from './collider-three-helpers';

/**
 * M10 SOMNIA Slice 2 in the real browser: seeded nightmare
 * generation, sealed-case exclusion by explicit user action only,
 * per-class decisions, inert consolidation proposals (displayed,
 * never applied), before/after comparison with deterministic
 * exports, and backup/fresh-restore survival of both saved
 * evaluations with identical recomputed proposals and comparison.
 * Everything is local and model-free: zero provider traffic.
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
  expect(previewRestore.status).toBe(200);
  const restored = await session.call('jobs', 'restore', {
    workspaceId: landingId,
    jobId: importJobId,
    artifactId: uploadArtifactId,
    name: `${name} Restored`,
    expectedHash: previewRestore.body.sha256,
    confirm: true,
  });
  expect(restored.status).toBe(200);
  const restoredWs = (restored.body.workspaces as Array<Record<string, unknown>>)[0].id as string;
  expect(restoredWs).not.toBe(workspaceId);
  return restoredWs;
}

/**
 * Resolve the comparison select values from the DOM: the nightmare
 * run is the saved evaluation whose title contains a nightmare case
 * title; the baseline is the other saved evaluation. No list-order
 * or remapped-id assumption is made.
 */
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

test('SOMNIA Slice 2: nightmares, inert proposals, comparison, exports', async ({
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
  try {
    const { workspaceId, projectId } = session;
    await session.openProject();

    // Baseline: the built-in evaluation, saved first so the
    // comparison selects have a deterministic order.
    await somnia.getByRole('button', { name: 'Load built-in cases' }).click();
    await somnia.getByRole('button', { name: 'Preview replay input' }).click();
    await somnia.getByRole('button', { name: 'Run evaluation' }).click();
    await expect(results.getByText('Scenarios: 5. Complete: 1. Refuse: 3. Abort: 1.')).toBeVisible();
    await somnia.getByRole('button', { name: 'Save SOMNIA evaluation' }).click();
    await expect(page.getByText('SOMNIA evaluation saved.')).toBeVisible();

    const hitsBefore = session.fakeHits();

    // Nightmares: seed + count 10, then preview refuses on the
    // sealed privacy-boundary case and Run stays disabled.
    await somnia.getByLabel('SOMNIA nightmare seed').fill('somnia-nightmare-seed');
    await somnia.getByLabel('SOMNIA nightmare count').selectOption('10');
    await somnia.getByRole('button', { name: 'Generate SOMNIA nightmares' }).click();
    await expect(somnia.getByLabel('SOMNIA replay input')).toHaveValue(
      /nightmare-privacy-boundary-challenge-6/,
    );
    await somnia.getByRole('button', { name: 'Preview replay input' }).click();
    const previewErrors = somnia.locator('section[aria-label="SOMNIA preview errors"]');
    await expect(
      previewErrors.locator('li').filter({ hasText: 'sealed' }),
    ).toBeVisible();
    await expect(somnia.getByRole('button', { name: 'Run evaluation' })).toBeDisabled();

    // Only an explicit exclusion removes the sealed case; the
    // preview count drops by exactly one and the rest can run.
    await somnia.getByRole('button', { name: 'Exclude sealed case' }).click();
    await somnia.getByRole('button', { name: 'Preview replay input' }).click();
    const preview = somnia.locator('section[aria-label="SOMNIA preview"]');
    await expect(preview.getByText('Scenarios: 9.')).toBeVisible();
    await expect(somnia.getByLabel('SOMNIA replay input')).not.toHaveValue(
      /nightmare-privacy-boundary-challenge-6/,
    );

    // Per-class decisions for the remaining nine nightmares.
    await somnia.getByRole('button', { name: 'Run evaluation' }).click();
    await expect(results.getByText('Scenarios: 9. Complete: 1. Refuse: 5. Abort: 3.')).toBeVisible();
    await expect(results.getByText('Simulated tool calls: 7.')).toBeVisible();
    await expect(results.getByText('Decision: Refuse')).toHaveCount(5);
    await expect(results.getByText('Decision: Abort')).toHaveCount(3);
    await expect(results.getByText('Decision: Complete')).toHaveCount(1);
    for (const violation of [
      'instruction-conflict',
      'missing-evidence',
      'tool-error',
      'tool-timeout',
      'untrusted-instruction',
      'constraint-violated',
    ]) {
      await expect(results.getByText(violation, { exact: false }).first()).toBeVisible();
    }
    expect(session.fakeHits()).toBe(hitsBefore);

    await somnia.getByRole('button', { name: 'Save SOMNIA evaluation' }).click();
    await expect(page.getByText('SOMNIA evaluation saved.')).toBeVisible();
    await expect(savedList.getByRole('button', { name: 'View evaluation' })).toHaveCount(2);

    // Inert proposals on the saved nightmare run: exactly the two
    // threshold-biting proposals for this seed and count, shown
    // with rationale and proposed change, and no apply control.
    // The saved list order is not assumed: the nightmare run is
    // the item whose title contains a nightmare case title.
    await savedList
      .locator('li', { hasText: 'Nightmare' })
      .getByRole('button', { name: 'View evaluation' })
      .click();
    await expect(proposals.getByText('Review only — nothing is changed.')).toBeVisible();
    await expect(proposals.getByText('procedural-rule — max-tool-calls:2')).toBeVisible();
    await expect(proposals.getByText('supersede-conflict — instruction-conflict')).toBeVisible();
    await expect(proposals.getByRole('button', { name: /apply/i })).toHaveCount(0);
    await proposals.getByRole('button', { name: 'Mark as reviewed' }).first().click();
    await expect(proposals.getByRole('button', { name: 'Reviewed' }).first()).toBeVisible();

    // Comparison: baseline (built-ins) vs nightmare run, selected
    // by resolved option values rather than list order.
    const keys = await comparisonKeys(comparison);
    await comparison.getByLabel('SOMNIA comparison baseline').selectOption(keys.baseline);
    await comparison.getByLabel('SOMNIA comparison target').selectOption(keys.nightmare);
    await comparison.getByRole('button', { name: 'Compare SOMNIA evaluations' }).click();
    const metricsTable = comparison.locator('table').first();
    await expect(
      metricsTable.locator('tr', { hasText: 'Refuse' }).getByText('+2', { exact: true }),
    ).toBeVisible();
    await expect(
      metricsTable.locator('tr', { hasText: 'Abort' }).getByText('+2', { exact: true }),
    ).toBeVisible();
    await expect(
      metricsTable.locator('tr', { hasText: 'Simulated tool calls' }).getByText('+4', { exact: true }),
    ).toBeVisible();
    const matrixTable = comparison.locator('table').nth(1);
    const toolErrorRow = matrixTable.locator('tr', { hasText: 'tool-error' });
    await expect(toolErrorRow.getByText('(newly exposed)')).toBeVisible();
    await expect(toolErrorRow.getByText('+2', { exact: true })).toBeVisible();

    // Deterministic exports.
    const jsonDownloadPromise = page.waitForEvent('download');
    await comparison.getByRole('button', { name: 'Export comparison JSON' }).click();
    const jsonDownload = await jsonDownloadPromise;
    const jsonPath = await jsonDownload.path();
    expect(jsonPath).not.toBeNull();
    const exported = JSON.parse(await readFile(jsonPath as string, 'utf8')) as Record<
      string,
      unknown
    >;
    expect(exported.deltaRefused).toBe(2);
    expect(exported.deltaAborted).toBe(2);
    expect(exported.deltaSimulatedToolCalls).toBe(4);
    const exportedCategories = exported.categories as Array<Record<string, unknown>>;
    const toolError = exportedCategories.find(c => c.category === 'tool-error');
    expect(toolError?.newlyExposed).toBe(true);
    expect(toolError?.delta).toBe(2);

    const mdDownloadPromise = page.waitForEvent('download');
    await comparison.getByRole('button', { name: 'Export comparison Markdown' }).click();
    const mdDownload = await mdDownloadPromise;
    const mdPath = await mdDownload.path();
    expect(mdPath).not.toBeNull();
    const markdown = await readFile(mdPath as string, 'utf8');
    expect(markdown).toContain('Abort: 1 -> 3 (+2)');
    expect(markdown).toContain('tool-error: 0 -> 2 (+2) (newly exposed)');

    expect(session.fakeHits()).toBe(hitsBefore);
    const list = await session.call('artifacts', 'list', { workspaceId, projectId });
    expect(
      (list.body.artifacts as Array<Record<string, unknown>>).filter(
        a => a.type === 'somnia-evaluation',
      ),
    ).toHaveLength(2);
  } finally {
    await session.cleanup();
  }
});

test('SOMNIA Slice 2: backup/restore keeps proposals and comparison identical', async ({
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
  try {
    const { workspaceId } = session;
    await session.openProject();

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

    const hitsBefore = session.fakeHits();
    const restoredWs = await backupAndRestore(session, workspaceId, 'SomniaSlice2');
    await session.pairAndOpen(restoredWs);
    await session.openProject();

    await expect(savedList.getByRole('button', { name: 'View evaluation' })).toHaveCount(2);
    await savedList
      .locator('li', { hasText: 'Nightmare' })
      .getByRole('button', { name: 'View evaluation' })
      .click();
    await expect(proposals.getByText('procedural-rule — max-tool-calls:2')).toBeVisible();
    await expect(proposals.getByText('supersede-conflict — instruction-conflict')).toBeVisible();
    await expect(proposals.getByRole('button', { name: /apply/i })).toHaveCount(0);

    const restoredKeys = await comparisonKeys(comparison);
    await comparison.getByLabel('SOMNIA comparison baseline').selectOption(restoredKeys.baseline);
    await comparison.getByLabel('SOMNIA comparison target').selectOption(restoredKeys.nightmare);
    await comparison.getByRole('button', { name: 'Compare SOMNIA evaluations' }).click();
    const metricsTable = comparison.locator('table').first();
    await expect(
      metricsTable.locator('tr', { hasText: 'Refuse' }).getByText('+2', { exact: true }),
    ).toBeVisible();
    const matrixTable = comparison.locator('table').nth(1);
    await expect(
      matrixTable.locator('tr', { hasText: 'tool-error' }).getByText('(newly exposed)'),
    ).toBeVisible();
    expect(session.fakeHits()).toBe(hitsBefore);
  } finally {
    await session.cleanup();
  }
});
