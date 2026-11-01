import { test, expect } from '@playwright/test';
import { createHash } from 'node:crypto';
import { startSession } from './collider-three-helpers';

/**
 * M10 SOMNIA Slice 1 vertical slice in the real browser: select a
 * project, supply bounded synthetic input, preview exactly what will
 * be evaluated, approve the run explicitly, inspect decisions,
 * violations and metrics, save the immutable evaluation explicitly,
 * then restart/reopen and backup/fresh-restore with provenance
 * intact. The reference evaluator is deterministic and model-free:
 * no provider traffic is produced at any step, and opening the
 * panel or previewing input creates no artifact.
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

test('SOMNIA Slice 1: preview, approve, run, save, restart, backup/restore', async ({
  page,
}) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  const somnia = page.locator('section[aria-labelledby="project-somnia-heading"]');
  const results = somnia.locator('section[aria-label="SOMNIA evaluation results"]');
  const savedList = somnia.locator('section[aria-label="Saved SOMNIA evaluations"]');
  try {
    const { workspaceId, projectId } = session;
    await session.openProject();

    await expect(savedList.getByText('No saved evaluations in this project.')).toBeVisible();

    // Supply input and preview exactly what will be evaluated.
    await somnia.getByRole('button', { name: 'Load built-in cases' }).click();
    await expect(somnia.getByLabel('SOMNIA replay input')).toHaveValue(/missing-receipt/);
    await somnia.getByRole('button', { name: 'Preview replay input' }).click();
    const preview = somnia.locator('section[aria-label="SOMNIA preview"]');
    await expect(preview.getByText('Scenarios: 5.')).toBeVisible();
    for (const title of [
      'Refund approval without a receipt',
      'Two approval policies disagree',
      'Order lookup times out',
      'Retrieved note tries to override policy',
      'Complete approval with full evidence',
    ]) {
      await expect(preview.getByText(title, { exact: false })).toBeVisible();
    }
    await expect(preview.getByText('Required evidence: receipt', { exact: true })).toBeVisible();

    // Preview alone creates nothing.
    const beforeList = await session.call('artifacts', 'list', { workspaceId, projectId });
    expect(
      (beforeList.body.artifacts as Array<Record<string, unknown>>).filter(
        a => a.type === 'somnia-evaluation',
      ),
    ).toHaveLength(0);

    const hitsBefore = session.fakeHits();

    // Explicit approval runs the deterministic evaluator.
    await somnia.getByRole('button', { name: 'Run evaluation' }).click();
    await expect(results.getByText('Scenarios: 5. Complete: 1. Refuse: 3. Abort: 1.')).toBeVisible();
    await expect(results.getByText('Simulated tool calls: 3.')).toBeVisible();
    await expect(results.getByText('Decision: Refuse')).toHaveCount(3);
    await expect(results.getByText('Decision: Abort')).toHaveCount(1);
    await expect(results.getByText('Decision: Complete')).toHaveCount(1);
    for (const violation of [
      'untrusted-instruction',
      'instruction-conflict',
      'missing-evidence',
      'tool-timeout',
      'constraint-violated',
    ]) {
      await expect(results.getByText(violation, { exact: false }).first()).toBeVisible();
    }
    await expect(results.getByText('Model tokens: Not measured.')).toBeVisible();
    await expect(results.getByText('Model accuracy: Not measured.')).toBeVisible();
    await expect(results.getByText('Production resilience: Not measured.')).toBeVisible();
    const resultsText = await results.innerText();
    const runId = /Run (somnia-[0-9a-f]+)/.exec(resultsText)?.[1];
    expect(runId).toBeTruthy();

    // No provider traffic from the replay at any step.
    expect(session.fakeHits()).toBe(hitsBefore);

    // Explicit save registers one immutable artifact.
    await somnia.getByRole('button', { name: 'Save SOMNIA evaluation' }).click();
    await expect(page.getByText('SOMNIA evaluation saved.')).toBeVisible();
    await expect(savedList.getByText(/SOMNIA evaluation:/)).toBeVisible();
    const afterList = await session.call('artifacts', 'list', { workspaceId, projectId });
    const saved = (afterList.body.artifacts as Array<Record<string, unknown>>).filter(
      a => a.type === 'somnia-evaluation',
    );
    expect(saved).toHaveLength(1);
    const read = await session.call('artifacts', 'read', {
      workspaceId,
      artifactId: saved[0].artifactId,
      version: saved[0].version,
    });
    const rawBody = (read.body as { body: unknown }).body;
    let doc: Record<string, unknown>;
    if (typeof rawBody === 'string') {
      // The registry hash is computed over the exact stored bytes.
      expect(saved[0].contentHash).toBe(sha256(Buffer.from(rawBody)));
      doc = JSON.parse(rawBody) as Record<string, unknown>;
    } else {
      doc = rawBody as Record<string, unknown>;
      expect(String(saved[0].contentHash)).toMatch(/^[0-9a-f]{64}$/);
    }
    expect(doc.format).toBe('anansi-somnia-evaluation');
    expect(doc.version).toBe(1);
    expect(doc.runId).toBe(runId);
    expect(doc.seed).toBe('somnia-builtin-seed');

    // Restart/reopen: reload, reopen the workspace, view the saved
    // evaluation with the same run id and decisions.
    await page.reload();
    await session.pairAndOpen(workspaceId);
    await session.openProject();
    await expect(savedList.getByText(/SOMNIA evaluation:/)).toBeVisible();
    await savedList.getByRole('button', { name: 'View evaluation' }).first().click();
    const reopened = somnia.locator('section[aria-label="SOMNIA evaluation results"]');
    await expect(reopened.getByText(`Run ${runId}`)).toBeVisible();
    await expect(reopened.getByText('Decision: Refuse')).toHaveCount(3);

    // Backup, fresh restore, reopen: provenance intact.
    const restoredWs = await backupAndRestore(session, workspaceId, 'Somnia');
    await session.pairAndOpen(restoredWs);
    await session.openProject();
    const restoredList = somnia.locator('section[aria-label="Saved SOMNIA evaluations"]');
    await expect(restoredList.getByText(/SOMNIA evaluation:/)).toBeVisible();
    await restoredList.getByRole('button', { name: 'View evaluation' }).first().click();
    const restoredView = somnia.locator('section[aria-label="SOMNIA evaluation results"]');
    await expect(restoredView.getByText(`Run ${runId}`)).toBeVisible();
    await expect(restoredView.getByText('Scenarios: 5. Complete: 1. Refuse: 3. Abort: 1.')).toBeVisible();
    expect(session.fakeHits()).toBe(hitsBefore);
  } finally {
    await session.cleanup();
  }
});

test('SOMNIA Slice 1: preview and abandonment create nothing; sealed input refused', async ({
  page,
}) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  const somnia = page.locator('section[aria-labelledby="project-somnia-heading"]');
  const savedList = somnia.locator('section[aria-label="Saved SOMNIA evaluations"]');
  try {
    const { workspaceId, projectId } = session;
    await session.openProject();

    // Preview + run, then abandon before Save: no artifact.
    await somnia.getByRole('button', { name: 'Load built-in cases' }).click();
    await somnia.getByRole('button', { name: 'Preview replay input' }).click();
    await expect(
      somnia.locator('section[aria-label="SOMNIA preview"]').getByText('Scenarios: 5.'),
    ).toBeVisible();
    await somnia.getByRole('button', { name: 'Run evaluation' }).click();
    await expect(
      somnia.locator('section[aria-label="SOMNIA evaluation results"]').getByText('Scenarios: 5.'),
    ).toBeVisible();
    await somnia.getByRole('button', { name: 'Start over' }).click();
    await expect(savedList.getByText('No saved evaluations in this project.')).toBeVisible();
    const list = await session.call('artifacts', 'list', { workspaceId, projectId });
    expect(
      (list.body.artifacts as Array<Record<string, unknown>>).filter(
        a => a.type === 'somnia-evaluation',
      ),
    ).toHaveLength(0);

    // Sealed input: preview refuses, Run stays disabled, nothing saved.
    await somnia.getByLabel('SOMNIA replay input').fill(
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
    );
    await somnia.getByRole('button', { name: 'Preview replay input' }).click();
    await expect(
      somnia.locator('section[aria-label="SOMNIA preview errors"]').getByText(/sealed/),
    ).toBeVisible();
    await expect(somnia.getByRole('button', { name: 'Run evaluation' })).toBeDisabled();

    // Malformed JSON: same refusal shape.
    await somnia.getByLabel('SOMNIA replay input').fill('{not json');
    await somnia.getByRole('button', { name: 'Preview replay input' }).click();
    await expect(
      somnia.locator('section[aria-label="SOMNIA preview errors"]').getByText(/not valid JSON/),
    ).toBeVisible();
    await expect(somnia.getByRole('button', { name: 'Run evaluation' })).toBeDisabled();
    const afterList = await session.call('artifacts', 'list', { workspaceId, projectId });
    expect(
      (afterList.body.artifacts as Array<Record<string, unknown>>).filter(
        a => a.type === 'somnia-evaluation',
      ),
    ).toHaveLength(0);
    expect(session.fakeHits()).toBe(0);
  } finally {
    await session.cleanup();
  }
});
