// M12 Slice 2: Launch Studio in the real browser. Test 1 pins the
// fresh-project state (Launch Awaiting, Jump opens the studio, Generate
// disabled with no evidence). Test 2 runs the genuine path — collide →
// Build a real Forge concept → SwarmTank evaluate and save — then opens
// the studio, generates deterministic copy, and downloads a bundle whose
// bytes are checked (PK signature + manifest and release-notes entry
// names). The collider step hits the fake provider by design, so the
// studio's zero-traffic claim is measured from the studio step onward:
// no additional fake hits from opening the studio through download.
import { test, expect } from '@playwright/test';
import fs from 'node:fs';
import { collideAndSave, startSession } from './collider-three-helpers';

test('product lifecycle slice 2: fresh project launch row jumps to an empty studio', async ({ page }) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  try {
    const nav = page.locator('nav[aria-label="Product Creation Pipeline"]');
    await expect(nav).toBeVisible();
    const launch = nav.locator('li').nth(6);
    await expect(launch).toContainText('Awaiting Preceding Stage');
    await expect(launch).toContainText('Needs a completed simulation or evaluation artifact.');
    await launch.getByRole('button', { name: 'Jump to Launch Studio' }).click();
    const studio = page.locator('section[aria-labelledby="project-launch-heading"]');
    await expect(studio).toBeVisible();
    await expect(studio).toContainText('No launch evidence artifacts in this project yet.');
    await expect(studio.getByRole('button', { name: 'Generate launch copy' })).toBeDisabled();
    expect(session.fakeHits()).toBe(0);
  } finally {
    await session.cleanup();
  }
});

test('product lifecycle slice 2: concept plus simulation generate copy and a bundle', async ({ page }) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  const swarmtank = page.locator('section[aria-labelledby="project-swarmtank-heading"]');
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
    await expect(build.getByRole('heading', { name: 'Build This Product' })).toBeVisible();
    await build.getByLabel('Combination to build').selectOption({ label: 'Mycelial order router' });
    await build.getByRole('button', { name: 'Build This Product' }).click();
    await expect(
      page.getByText('Built: Forge concept created from the approved outputs.'),
    ).toBeVisible();
    await session.openProject();

    await swarmtank
      .getByLabel('Import from saved experiment')
      .selectOption({ label: 'Collider experiment: First beam about kitchens.' });
    await swarmtank.getByRole('button', { name: 'Import experiment' }).click();
    await swarmtank.getByLabel('SwarmTank pricing').fill('$49 per month');
    await swarmtank.getByLabel('SwarmTank monthly price amount').fill('49');
    await swarmtank.getByLabel('SwarmTank segment').fill('small agencies');
    await swarmtank.getByLabel('SwarmTank industry').fill('services');
    await swarmtank
      .getByLabel('SwarmTank value proposition')
      .fill('Agencies save ten hours a week on reporting.');
    await swarmtank.getByLabel('SwarmTank population').selectOption('20');
    await swarmtank.getByLabel('SwarmTank seed').fill('e2e-seed-1');
    await swarmtank.getByRole('button', { name: 'Generate personas' }).click();
    await expect(
      swarmtank.locator('section[aria-label="Simulated personas preview"]'),
    ).toBeVisible();
    await swarmtank.getByRole('button', { name: 'Evaluate decisions' }).click();
    await expect(page.getByText('Evaluated and saved the SwarmTank run.')).toBeVisible();

    const nav = page.locator('nav[aria-label="Product Creation Pipeline"]');
    const launch = nav.locator('li').nth(6);
    await expect(launch).toContainText('Available');
    const hitsBeforeStudio = session.fakeHits();
    await launch.getByRole('button', { name: 'Open Launch Studio' }).click();
    const studio = page.locator('section[aria-labelledby="project-launch-heading"]');
    await expect(studio).toBeVisible();
    await expect(studio.getByText(/Include concept:/).first()).toBeVisible();
    await expect(studio.getByText(/Include simulation:/).first()).toBeVisible();
    await studio.getByRole('button', { name: 'Generate launch copy' }).click();
    await expect(studio.getByRole('article', { name: 'Show HN preview' })).toBeVisible();
    await expect(studio.getByRole('article', { name: 'Product Hunt preview' })).toBeVisible();
    await expect(studio.getByRole('article', { name: 'X release thread preview' })).toBeVisible();
    await expect(studio.getByRole('article', { name: 'Release notes preview' })).toBeVisible();
    await expect(studio).toContainText('Mycelial order router');
    await expect(studio).toContainText('Prototype for review; not executed, not deployed.');
    await expect(studio).toContainText('(simulated)');

    const downloadPromise = page.waitForEvent('download');
    await studio.getByRole('button', { name: 'Download Launch Bundle (.zip)' }).click();
    const download = await downloadPromise;
    const path = await download.path();
    assertPath(path);
    const bytes = fs.readFileSync(path);
    assertZip(bytes);
    expect(session.fakeHits()).toBe(hitsBeforeStudio);
  } finally {
    await session.cleanup();
  }
});

function assertPath(path: string | null): asserts path is string {
  if (!path) throw new Error('Download produced no file path.');
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
