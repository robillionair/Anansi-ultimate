import { test, expect } from '@playwright/test';
import fs from 'node:fs';
import { createHash } from 'node:crypto';
import { startSession, collideAndSave } from './collider-three-helpers';

/**
 * M12 Slice 3: the complete §85 pipeline walk as the assignment bounds it
 * (six pipeline steps, not all 17 §85 steps — repository/code/sandbox/
 * preview steps stay M7-gated), plus §62 state recovery: a mid-walk
 * client restart and a full backup/restore with project-binding checks.
 *
 * Provider traffic is the local fake only (opportunity, collider, spec
 * generations route by instruction string in the shared harness). The
 * Launch step itself adds no hits — asserted as a delta, the slice2
 * pattern, not as an absolute zero.
 */
function sha256(data: Buffer): string {
  return createHash('sha256').update(data).digest('hex');
}

function assertZip(bytes: Buffer): void {
  if (bytes[0] !== 0x50 || bytes[1] !== 0x4b || bytes[2] !== 0x03 || bytes[3] !== 0x04) {
    throw new Error('Download is not a ZIP archive (missing PK\\x03\\x04 signature).');
  }
  for (const entry of ['LAUNCH-MANIFEST.json', 'marketing/release-notes.md', 'marketing/show-hn.md']) {
    if (!bytes.includes(Buffer.from(entry, 'utf8'))) {
      throw new Error(`Bundle is missing entry ${entry}.`);
    }
  }
}

/** Status text of the walk step row whose label starts the row. */
async function walkStepText(
  walk: import('@playwright/test').Locator,
  label: string,
): Promise<string> {
  const rows = walk.locator('ol > li');
  const count = await rows.count();
  for (let i = 0; i < count; i++) {
    const text = (await rows.nth(i).innerText()).trim();
    if (text.startsWith(label)) return text;
  }
  throw new Error(`Walk step ${label} not found`);
}

test('product lifecycle slice 3: complete guided walk with a mid-walk restart', async ({ page }) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  const walk = page.locator('section[aria-labelledby="project-walk-heading"]');
  const nav = page.locator('nav[aria-label="Product Creation Pipeline"]');
  const opp = page.locator('section[aria-labelledby="project-opportunities-heading"]');
  const swarmtank = page.locator('section[aria-labelledby="project-swarmtank-heading"]');
  const somnia = page.locator('section[aria-labelledby="project-somnia-heading"]');
  try {
    // Fresh project: Discover Current, the rest Awaiting.
    await expect(walk).toBeVisible();
    expect(await walkStepText(walk, 'Discover')).toContain('Current / In Progress');
    expect(await walkStepText(walk, 'Define')).toContain('Awaiting Preceding Stage');
    expect(await walkStepText(walk, 'Launch')).toContain('Awaiting Preceding Stage');

    // 1. Discover: opportunity set from a problem source, saved as artifact.
    await walk.getByRole('link', { name: 'Continue walk' }).click();
    await expect(opp).toBeInViewport();
    await opp.getByLabel('Source text').fill('Dock offices lose supplier quotes in shared inboxes.');
    await opp.getByRole('button', { name: 'Discover opportunities' }).click();
    await expect(page.getByText('1. Dock quote inbox')).toBeVisible();
    await opp.getByRole('button', { name: 'Save opportunity set as artifact' }).click();
    await expect(opp.getByText(/Saved as artifact/)).toBeVisible();
    expect(await walkStepText(walk, 'Discover')).toContain('Complete');
    expect(await walkStepText(walk, 'Define')).toContain('Current / In Progress');

    // 2. Define: collider concept, then a Forge specification draft.
    await walk.getByRole('link', { name: 'Continue walk' }).click();
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
    await session.artifacts
      .getByRole('button', { name: 'Concept: Mycelial order router' })
      .click();
    await expect(page.getByLabel('Purpose — what should it do?')).toBeVisible();
    await page.getByLabel('Purpose — what should it do?').fill('Route orders to the best venue from local signals.');
    await page.getByLabel('Intended users').fill('Small agencies routing orders across venues.');
    await page.getByLabel('Must-have behavior').fill('Score venues, rank them, record the signals.');
    await page.getByRole('button', { name: 'Generate product specification' }).click();
    await expect(page.getByRole('heading', { name: 'Product specification: Mycelial order router' })).toBeVisible();
    await page.getByRole('button', { name: 'Save specification draft' }).click();
    await expect(page.getByText('Specification draft saved as artifact')).toBeVisible();
    await session.openProject();
    expect(await walkStepText(walk, 'Define')).toContain('Complete');
    expect(await walkStepText(walk, 'Simulate')).toContain('Current / In Progress');

    // 3. Restart checkpoint: full client reload + re-pair rebuilds the
    // pipeline from the server's persisted registry, not client memory.
    await page.reload();
    await session.pairAndOpen(session.workspaceId);
    expect(await walkStepText(walk, 'Discover')).toContain('Complete');
    expect(await walkStepText(walk, 'Define')).toContain('Complete');
    expect(await walkStepText(walk, 'Simulate')).toContain('Current / In Progress');

    // 4. Simulate: SwarmTank across 20 personas, run saved.
    await swarmtank
      .getByLabel('Import from saved experiment')
      .selectOption({ label: 'Collider experiment: First beam about kitchens.' });
    await swarmtank.getByLabel('SwarmTank pricing').fill('$49 per month');
    await swarmtank.getByLabel('SwarmTank monthly price amount').fill('49');
    await swarmtank.getByLabel('SwarmTank segment').fill('small agencies');
    await swarmtank.getByLabel('SwarmTank industry').fill('services');
    await swarmtank
      .getByLabel('SwarmTank value proposition')
      .fill('A routing layer that scores venues by local congestion signals.');
    await swarmtank.getByLabel('SwarmTank population').selectOption('20');
    await swarmtank.getByLabel('SwarmTank seed').fill('e2e-seed-1');
    await swarmtank.getByRole('button', { name: 'Generate personas' }).click();
    await expect(
      swarmtank.locator('section[aria-label="Simulated personas preview"]'),
    ).toBeVisible();
    await swarmtank.getByRole('button', { name: 'Evaluate decisions' }).click();
    await expect(page.getByText('Evaluated and saved the SwarmTank run.')).toBeVisible();
    expect(await walkStepText(walk, 'Simulate')).toContain('Complete');
    expect(await walkStepText(walk, 'Pressure-Test')).toContain('Current / In Progress');

    // 5. Pressure-Test: SOMNIA nightmares evaluated and saved.
    await somnia.getByLabel('SOMNIA nightmare seed').fill('walk-nightmare-seed');
    await somnia.getByLabel('SOMNIA nightmare count').selectOption('10');
    await somnia.getByRole('button', { name: 'Generate SOMNIA nightmares' }).click();
    await somnia.getByRole('button', { name: 'Run evaluation' }).click();
    await somnia.getByRole('button', { name: 'Save SOMNIA evaluation' }).click();
    await expect(page.getByText(/SOMNIA evaluation saved/)).toBeVisible();
    expect(await walkStepText(walk, 'Pressure-Test')).toContain('Complete');

    // 6. Refine: GHOST-PROMPT session in Ask ANANSI, revised, then wiped.
    // Refine never reads Complete — asserted explicitly, not assumed.
    await walk.getByRole('button', { name: 'Go to Refine' }).click();
    const strip = page.locator('section[aria-label="GHOST-PROMPT controls"]');
    await expect(strip).toBeVisible();
    await strip.getByRole('button', { name: 'Enable Draft Exploration' }).click();
    const dialog = page.getByRole('dialog', { name: 'Enable Draft Exploration consent' });
    await dialog.getByRole('button', { name: 'Enable', exact: true }).click();
    await expect(strip.getByText('Draft Exploration ON')).toBeVisible();
    const input = page.getByLabel('Your Question');
    await input.fill('Build a routing tool for small agencies');
    await input.fill('Build a routing tool for small agencies with local signals');
    await strip.getByRole('button', { name: 'Review drafts' }).click();
    const fragments = strip.locator('section[aria-label="Retained & Deleted Fragments"]');
    await expect(fragments).toBeVisible();
    await strip.getByRole('button', { name: 'Stop & discard session' }).click();
    await expect(strip.getByText('Draft history purged.')).toBeVisible();
    await session.openProject();
    expect(await walkStepText(walk, 'Refine')).toContain('Available');
    expect(await walkStepText(walk, 'Refine')).not.toContain('Complete');

    // 7. Launch: studio copy, bundle download, manual launch-material
    // registration (the bundle is download-only by Slice 2 design).
    const hitsBeforeStudio = session.fakeHits();
    await walk.getByRole('button', { name: 'Go to Launch' }).click();
    const studio = page.locator('section[aria-labelledby="project-launch-heading"]');
    await expect(studio).toBeVisible();
    await studio.getByRole('button', { name: 'Generate launch copy' }).click();
    await expect(studio.getByRole('article', { name: 'Show HN preview' })).toBeVisible();
    await expect(studio.getByRole('article', { name: 'Product Hunt preview' })).toBeVisible();
    await expect(studio.getByRole('article', { name: 'X release thread preview' })).toBeVisible();
    await expect(studio.getByRole('article', { name: 'Release notes preview' })).toBeVisible();
    await expect(studio).toContainText('Mycelial order router');
    await expect(studio).toContainText('Prototype for review; not executed, not deployed.');
    const downloadPromise = page.waitForEvent('download');
    await studio.getByRole('button', { name: 'Download Launch Bundle (.zip)' }).click();
    const download = await downloadPromise;
    const downloadPath = await download.path();
    if (!downloadPath) throw new Error('Download produced no file path.');
    assertZip(fs.readFileSync(downloadPath));
    expect(session.fakeHits()).toBe(hitsBeforeStudio);
    await session.artifacts.getByLabel('Artifact type').selectOption('launch-material');
    await session.artifacts.getByLabel('Artifact title').fill('Launch materials');
    await session.artifacts.getByLabel('Artifact text').fill('Launch bundle for the Harbor Walk walkthrough.');
    await session.artifacts.getByRole('button', { name: 'Register artifact' }).click();
    await expect(page.getByText('Artifact registered.')).toBeVisible();
    expect(await walkStepText(walk, 'Launch')).toContain('Complete');
    await expect(walk.getByText('Walk complete')).toBeVisible();
    await expect(nav.locator('li').nth(6)).toContainText('Complete');
  } finally {
    await session.cleanup();
  }
});

test('product lifecycle slice 3: backup and restore preserve pipeline state and project binding', async ({ page }) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  const walk = page.locator('section[aria-labelledby="project-walk-heading"]');
  const swarmtank = page.locator('section[aria-labelledby="project-swarmtank-heading"]');
  const { workspaceId, projectId } = session;
  try {
    await collideAndSave(
      session.collider,
      'First beam about kitchens.',
      'First beam about orbitals.',
    );
    await session.artifacts
      .getByRole('button', { name: /Collider experiment: First beam about kitchens/ })
      .click();
    const build = session.artifacts.locator('section[aria-label="Build this product"]');
    await build.getByLabel('Combination to build').selectOption({ label: 'Mycelial order router' });
    await build.getByRole('button', { name: 'Build This Product' }).click();
    await expect(
      page.getByText('Built: Forge concept created from the approved outputs.'),
    ).toBeVisible();
    await session.openProject();
    await swarmtank
      .getByLabel('Import from saved experiment')
      .selectOption({ label: 'Collider experiment: First beam about kitchens.' });
    await swarmtank.getByLabel('SwarmTank pricing').fill('$49 per month');
    await swarmtank.getByLabel('SwarmTank monthly price amount').fill('49');
    await swarmtank.getByLabel('SwarmTank segment').fill('small agencies');
    await swarmtank.getByLabel('SwarmTank industry').fill('services');
    await swarmtank
      .getByLabel('SwarmTank value proposition')
      .fill('A routing layer that scores venues by local congestion signals.');
    await swarmtank.getByLabel('SwarmTank population').selectOption('20');
    await swarmtank.getByLabel('SwarmTank seed').fill('e2e-seed-1');
    await swarmtank.getByRole('button', { name: 'Generate personas' }).click();
    await swarmtank.getByRole('button', { name: 'Evaluate decisions' }).click();
    await expect(page.getByText('Evaluated and saved the SwarmTank run.')).toBeVisible();

    // Record pre-backup state: walk statuses + artifact type/title list.
    const beforeSteps = [
      await walkStepText(walk, 'Discover'),
      await walkStepText(walk, 'Define'),
      await walkStepText(walk, 'Simulate'),
      await walkStepText(walk, 'Pressure-Test'),
      await walkStepText(walk, 'Launch'),
    ];
    const beforeList = await session.call('artifacts', 'list', { workspaceId, projectId });
    const beforeArtifacts = (beforeList.body.artifacts as Array<Record<string, any>>)
      .map(a => `${a.type}:${a.title}`)
      .sort();

    // Backup, fresh restore, reopen (swarmtank-slice1 pattern).
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
    const landing = await session.call('workspaces', 'create', { name: 'Walk Landing' });
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
      name: 'Walk Restored',
      expectedHash: previewRestore.body.sha256,
      confirm: true,
    });
    expect(restored.status).toBe(200);
    const restoredWs = (restored.body.workspaces as Array<Record<string, any>>)[0].id as string;
    expect(restoredWs).not.toBe(workspaceId);

    await session.pairAndOpen(restoredWs);
    const afterSteps = [
      await walkStepText(walk, 'Discover'),
      await walkStepText(walk, 'Define'),
      await walkStepText(walk, 'Simulate'),
      await walkStepText(walk, 'Pressure-Test'),
      await walkStepText(walk, 'Launch'),
    ];
    expect(afterSteps).toEqual(beforeSteps);
    const restoredList = await session.call('memories', 'list', { workspaceId: restoredWs, includeArchived: true });
    const restoredProjectId = (restoredList.body.memories as Record<string, any>[])
      .find(m => m.title === 'Harbor App')!.id as string;
    const afterList = await session.call('artifacts', 'list', { workspaceId: restoredWs, projectId: restoredProjectId });
    const afterArtifacts = (afterList.body.artifacts as Array<Record<string, any>>)
      .map(a => `${a.type}:${a.title}`)
      .sort();
    expect(afterArtifacts).toEqual(beforeArtifacts);

    // Isolation: a fresh project in the restored workspace starts empty
    // — restored artifacts do not leak into other projects.
    await page.getByRole('button', { name: 'New project' }).click();
    await page.getByLabel('Project title').fill('Isolated Check');
    await page.getByLabel('Description & objectives').fill('A fresh project in the restored workspace.');
    await page.getByRole('button', { name: 'Save project' }).click();
    await expect(page.getByRole('heading', { name: 'Opportunity discovery' })).toBeVisible();
    expect(await walkStepText(walk, 'Discover')).toContain('Current / In Progress');
    await expect(page.getByText('No artifacts registered for this project yet.')).toBeVisible();
  } finally {
    await session.cleanup();
  }
});
