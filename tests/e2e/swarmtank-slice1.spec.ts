import { test, expect } from '@playwright/test';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import { collideAndSave, startSession } from './collider-three-helpers';

/**
 * M9 SwarmTank Slice 1 vertical slice in the real
 * browser: collide → Build a real Forge concept →
 * import the experiment into SwarmTank → Generate
 * Personas → Evaluate Decisions → View Report →
 * Apply Recommendations to Forge → Backup/Restore
 * → Export. Generation and evaluation are
 * deterministic backend operations: no provider
 * traffic is produced by SwarmTank at any step,
 * and the saved run survives a fresh restore with
 * its report intact.
 */

function sha256(data: Buffer): string {
  return createHash('sha256').update(data).digest('hex');
}

test('SwarmTank Slice 1: input, personas, decisions, report, Forge transfer, restore, export', async ({
  page,
}) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  const swarmtank = page.locator('section[aria-labelledby="project-swarmtank-heading"]');
  const preview = swarmtank.locator('section[aria-label="Simulated personas preview"]');
  const reportSection = swarmtank.locator('section[aria-label="SwarmTank report"]');
  try {
    const { workspaceId, projectId } = session;
    // A real Forge concept via collide → Build, so
    // the later transfer revises genuine provenance
    // (restore refuses fabricated experiment pins).
    await collideAndSave(
      session.collider,
      'First beam about kitchens.',
      'First beam about orbitals.',
    );
    await session.artifacts
      .getByRole('button', { name: /Collider experiment: First beam about kitchens/ })
      .click();
    const build = session.artifacts.locator('section[aria-label="Build this product"]');
    await expect(build.getByRole('heading', { name: 'Build This Product' })).toBeVisible();
    await build.getByLabel('Combination to build').selectOption({ label: 'Mycelial order router' });
    await build.getByRole('button', { name: 'Build This Product' }).click();
    await expect(
      page.getByText('Built: Forge concept created from the approved outputs.'),
    ).toBeVisible();
    await session.openProject();

    // Input, imported from the saved experiment: the
    // concept brief becomes the description and the
    // primitives become features; unstated fields
    // stay empty until filled below.
    await swarmtank
      .getByLabel('Import from saved experiment')
      .selectOption({ label: 'Collider experiment: First beam about kitchens.' });
    await swarmtank.getByRole('button', { name: 'Import experiment' }).click();
    await expect(swarmtank.getByLabel('SwarmTank description')).toHaveValue(
      'A routing advisor that proposes venue choices from local signals. Hypothesis, not a product claim.',
    );
    await expect(
      page.getByText('Imported from the saved experiment. Fields the record does not state stay empty.'),
    ).toBeVisible();
    await swarmtank.getByLabel('SwarmTank pricing').fill('$49 per month');
    await swarmtank.getByLabel('SwarmTank monthly price amount').fill('49');
    await swarmtank.getByLabel('SwarmTank segment').fill('small agencies');
    await swarmtank.getByLabel('SwarmTank industry').fill('services');
    await swarmtank
      .getByLabel('SwarmTank value proposition')
      .fill('Agencies save ten hours a week on reporting.');
    await swarmtank.getByLabel('SwarmTank population').selectOption('20');
    await swarmtank.getByLabel('SwarmTank seed').fill('e2e-seed-1');

    const hitsBefore = session.fakeHits();

    // Generate: preview only, nothing saved yet.
    await swarmtank.getByRole('button', { name: 'Generate personas' }).click();
    await expect(
      preview.getByRole('heading', { name: 'Simulated personas (preview, not saved)' }),
    ).toBeVisible();
    await expect(preview.getByText('Simulated persona.')).toHaveCount(20);
    await expect(
      swarmtank.locator('section[aria-label="Saved SwarmTank runs"]'),
    ).toHaveCount(0);
    const previewFirstName = await preview.locator('li').first().locator('strong').innerText();

    // Evaluate: the same seed reproduces the preview
    // population, now saved as a run with a report.
    await swarmtank.getByRole('button', { name: 'Evaluate decisions' }).click();
    await expect(page.getByText('Evaluated and saved the SwarmTank run.')).toBeVisible();
    await expect(reportSection.getByRole('heading', { name: 'SwarmTank report' })).toBeVisible();
    await expect(reportSection.getByText(/Conversion rate: \d+%/)).toBeVisible();
    await expect(reportSection.getByText(/Top objections:/)).toBeVisible();
    await expect(reportSection.getByText(/Top confusion points:/)).toBeVisible();
    await expect(reportSection.getByText(previewFirstName)).toBeVisible();
    await expect(
      swarmtank.getByText(/SwarmTank run: A routing advisor/),
    ).toBeVisible();

    // No provider traffic from generation or evaluation.
    expect(session.fakeHits()).toBe(hitsBefore);

    // Apply Recommendations to Forge: revises the
    // registered concept to version 2 with the run's
    // revision note.
    await swarmtank
      .getByLabel('Apply recommendations to Forge concept')
      .selectOption({ label: 'Concept: Mycelial order router' });
    await swarmtank
      .getByRole('button', { name: 'Apply Recommendations to Forge' })
      .click();
    await expect(
      page.getByText(/Applied: Forge concept revised to version 2 with the SwarmTank revision note/),
    ).toBeVisible();
    const conceptList = await session.call('artifacts', 'list', { workspaceId, projectId });
    const concept = (conceptList.body.artifacts as Array<Record<string, any>>).find(
      artifact => artifact.type === 'concept',
    );
    expect(concept!.version).toBe(2);
    const conceptRead = await session.call('artifacts', 'read', {
      workspaceId,
      artifactId: concept!.artifactId,
      version: 2,
    });
    const conceptDoc = JSON.parse(conceptRead.body.body as string) as Record<string, any>;
    expect(conceptDoc.swarmtankAddendum).toBeTruthy();
    expect(conceptDoc.swarmtankAddendum.topObjections.length).toBeLessThanOrEqual(3);

    // Backup, fresh restore, reopen: the run and its
    // report survive with the same content.
    const backup = await session.call('jobs', 'backup', {
      workspaceId,
      application: false,
      plaintextConsent: true,
    });
    expect(backup.status).toBe(200);
    const jobId = backup.body.job.id as string;
    let job: Record<string, any> | undefined;
    for (let i = 0; i < 60 && (!job || job.state !== 'Completed'); i++) {
      await new Promise(r => setTimeout(r, 1000));
      const list = await session.call('jobs', 'list', { workspaceId });
      job = (list.body.jobs as Array<Record<string, any>>).find(j => j.id === jobId);
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
    expect(chunks.length).toBe(artifactBytes);
    const tar = Buffer.from(chunks);
    const landing = await session.call('workspaces', 'create', { name: 'SwarmTank Landing' });
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
      name: 'SwarmTank Restored',
      expectedHash: previewRestore.body.sha256,
      confirm: true,
    });
    expect(restored.status).toBe(200);
    const restoredWs = (restored.body.workspaces as Array<Record<string, any>>)[0].id as string;
    expect(restoredWs).not.toBe(workspaceId);

    await session.pairAndOpen(restoredWs);
    const restoredRuns = swarmtank.locator('section[aria-label="Saved SwarmTank runs"]');
    await expect(
      restoredRuns.getByText(/SwarmTank run: A routing advisor/),
    ).toBeVisible();
    await restoredRuns.getByRole('button', { name: 'View report' }).first().click();
    await expect(reportSection.getByRole('heading', { name: 'SwarmTank report' })).toBeVisible();
    await expect(reportSection.getByText(/Conversion rate: \d+%/)).toBeVisible();
    await expect(reportSection.getByText(previewFirstName)).toBeVisible();

    // Export the restored run's report as JSON.
    const downloadPromise = page.waitForEvent('download');
    await reportSection.getByRole('button', { name: 'Export report' }).click();
    const download = await downloadPromise;
    const downloadPath = await download.path();
    const exported = JSON.parse(fs.readFileSync(downloadPath!, 'utf8')) as Record<string, any>;
    expect(exported.disclaimer).toMatch(/simulated personas, not real customers/);
    expect(exported.report.total).toBe(20);
    expect(typeof exported.report.conversionRate).toBe('number');
    expect(exported.report.topObjections.length).toBeLessThanOrEqual(3);
    expect(exported.report.topConfusionPoints.length).toBeLessThanOrEqual(3);
  } finally {
    await session.cleanup();
  }
});
