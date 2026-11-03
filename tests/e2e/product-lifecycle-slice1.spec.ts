// M12 Slice 1: the pipeline navigator derives stage status from the
// project's artifact registry and links each stage to its tool.
// Test 1 pins the fresh-project state; test 2 registers a Forge
// concept and a SwarmTank run through the existing manual-registration
// form and asserts the pipeline advances and the stage links land on
// the right tools. Zero provider traffic throughout (manual
// registration only; fakeHits() === 0).
import { test, expect } from '@playwright/test';
import { startSession } from './collider-three-helpers';

test('product lifecycle slice 1: fresh project starts at Discover', async ({ page }) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  try {
    const nav = page.locator('nav[aria-label="Product Creation Pipeline"]');
    await expect(nav).toBeVisible();
    const current = nav.locator('li[aria-current="step"]');
    await expect(current).toContainText('Discover');
    await expect(current).toContainText('Current / In Progress');
    // Rows are in pipeline order: 0 Discover, 1 Invent, 2 Build,
    // 3 Simulate, 4 Pressure test, 5 Refine, 6 Launch.
    const build = nav.locator('li').nth(2);
    await expect(build).toContainText('Awaiting Preceding Stage');
    await expect(build).toContainText('Needs at least one Forge concept artifact.');
    await expect(build.getByRole('link', { name: 'Jump to Artifacts (Product Forge)' })).toBeVisible();
    const refine = nav.locator('li').nth(5);
    await expect(refine).toContainText('Available');
    const launch = nav.locator('li').nth(6);
    await expect(launch).toContainText('Awaiting Preceding Stage');
    await expect(launch.getByRole('button', { name: 'Launch Studio (Slice 2)' })).toBeDisabled();
    await expect(launch).toContainText('Launch Studio arrives in Slice 2.');
    expect(session.fakeHits()).toBe(0);
  } finally {
    await session.cleanup();
  }
});

test('product lifecycle slice 1: concept and simulation advance the pipeline and link to tools', async ({ page }) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  try {
    const nav = page.locator('nav[aria-label="Product Creation Pipeline"]');
    const artifacts = session.artifacts;
    await expect(nav).toBeVisible();

    async function register(type: string, title: string) {
      await artifacts.getByLabel('Artifact type').selectOption(type);
      await artifacts.getByLabel('Artifact title').fill(title);
      await artifacts.getByLabel('Artifact text').fill(`Body for ${title}.`);
      await artifacts.getByRole('button', { name: 'Register artifact' }).click();
      // The notice is set at the end of the submit handler, after the
      // list reload, so waiting for it (not just the list row) proves
      // the full registration — including the navigator's state feed —
      // has settled before the pipeline assertions run.
      await expect(page.getByText('Artifact registered.')).toBeVisible();
      await expect(artifacts.getByText(title).first()).toBeVisible();
    }

    await register('concept', 'Test concept');
    await register('simulation', 'Test run');

    const invent = nav.locator('li').nth(1);
    await expect(invent).toContainText('Complete');
    const simulate = nav.locator('li').nth(3);
    await expect(simulate).toContainText('Complete');
    const pressure = nav.locator('li').nth(4);
    await expect(pressure).toHaveAttribute('aria-current', 'step');
    await expect(pressure).toContainText('Current / In Progress');
    const launch = nav.locator('li').nth(6);
    await expect(launch).toContainText('Available');

    // Stage links land on the right tools, with the project pre-selected
    // (the navigator lives in the open project view).
    await simulate.getByRole('link', { name: 'Open SwarmTank' }).click();
    await expect(page).toHaveURL(/#project-swarmtank-heading$/);
    await expect(page.locator('#project-swarmtank-heading')).toBeVisible();
    await invent.getByRole('link', { name: 'Open LEXI-COLLIDER' }).click();
    await expect(page).toHaveURL(/#project-collider-heading$/);
    await expect(page.locator('#project-collider-heading')).toBeVisible();

    // Refine jumps to the Ask ANANSI view, where GHOST-PROMPT lives.
    await nav.locator('li').nth(5).getByRole('button', { name: 'Open Ask ANANSI (GHOST-PROMPT)' }).click();
    await expect(page.getByLabel('Your Question')).toBeVisible();
    expect(session.fakeHits()).toBe(0);
  } finally {
    await session.cleanup();
  }
});
