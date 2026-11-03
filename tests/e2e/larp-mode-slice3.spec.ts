// LARP Mode Slice 3: attention spider with seeded real records and a
// real relationship, quant drift spectrum, key switching, and clean
// disposal. Click/key behavior is e2e-only (static markup cannot run
// effects). Seeded records are real API records, not fixtures rendered
// as data: the spider's nodes and edges come from the Memory producer
// reading them back.

import { test, expect } from '@playwright/test';
import { startSession } from './collider-three-helpers';

test.beforeEach(async ({ page }) => {
  page.setDefaultTimeout(8000);
});

async function seedSpiderData(session: Awaited<ReturnType<typeof startSession>>): Promise<void> {
  const titles = ['Spider Alpha', 'Spider Beta', 'Spider Gamma'];
  const confidences = [0.8, 0.6, 0.4];
  const ids: string[] = [];
  for (let i = 0; i < titles.length; i++) {
    const created = await session.call('memories', 'create', {
      workspaceId: session.workspaceId,
      kind: 'memory',
      draft: { title: titles[i], body: `Spider fixture ${i}.`, confidence: confidences[i] },
    });
    ids.push(created.body.memory.id as string);
  }
  await session.call('relationships', 'create', {
    workspaceId: session.workspaceId,
    draft: { sourceId: ids[0], targetId: ids[1], predicate: 'relates-to' },
  });
}

test('view 3 renders a real spider and view 4 renders the spectrum', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    await seedSpiderData(session);
    await session.pairAndOpen(session.workspaceId);
    await page.getByRole('button', { name: 'Memory', exact: true }).click();
    await expect(page.getByRole('button', { name: 'Spider Alpha' })).toBeVisible();

    await page.getByLabel('Toggle LARP mode').click();
    await expect(page.getByLabel('LARP telemetry frame')).toBeVisible();

    await page.getByLabel('LARP view 3').click();
    const spider = page.getByLabel('ATTENTION SPIDER // GRAPH DRIFT ENGINE');
    await expect(spider).toBeVisible();
    await expect(spider.locator('canvas')).toBeVisible();
    await expect
      .poll(async () => Number(await spider.getAttribute('data-frames')), { timeout: 60000 })
      .toBeGreaterThan(0);
    await expect
      .poll(async () => Number(await spider.getAttribute('data-nodes')))
      .toBeGreaterThanOrEqual(3);
    await expect
      .poll(async () => Number(await spider.getAttribute('data-edges')))
      .toBeGreaterThanOrEqual(3);
    await expect(spider.getByText('Spider Alpha')).toBeVisible();

    await page.getByLabel('LARP view 4').click();
    const spectrum = page.getByLabel('QUANT DRIFT & COLLIDER SPECTRUM');
    await expect(spectrum).toBeVisible();
    await expect(spectrum.locator('canvas')).toBeVisible();
    await expect(spectrum.getByText('Collision resonance: not measured.')).toBeVisible();
    await expect
      .poll(async () => Number(await spectrum.getAttribute('data-confidences')))
      .toBeGreaterThanOrEqual(1);

    // Hover over the spectrum canvas updates the readout (the HUD is
    // click-through, so the view listens on window + rect checks).
    const box = await spectrum.locator('canvas').boundingBox();
    if (!box) throw new Error('spectrum canvas has no box');
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await expect(spectrum.locator('.larp-spectrum-readout')).not.toHaveText(
      'Hover the spectrum to inspect a sample.',
    );

    // Keys switch back and forth.
    await page.keyboard.press('3');
    await expect(page.getByLabel('ATTENTION SPIDER // GRAPH DRIFT ENGINE')).toBeVisible();
    await page.keyboard.press('4');
    await expect(page.getByLabel('QUANT DRIFT & COLLIDER SPECTRUM')).toBeVisible();

    // OFF disposes the whole HUD, canvases included.
    await page.keyboard.press('Control+Shift+L');
    await expect(page.getByLabel('LARP telemetry frame')).toHaveCount(0);
    await expect(page.getByLabel('ATTENTION SPIDER // GRAPH DRIFT ENGINE')).toHaveCount(0);
    await expect(page.getByLabel('QUANT DRIFT & COLLIDER SPECTRUM')).toHaveCount(0);
  } finally {
    await session.cleanup();
  }
});

test('a live ask run leaves a compilation marker in view 4', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    await session.pairAndOpen(session.workspaceId);
    await page.getByLabel('Toggle LARP mode').click();
    await page.getByLabel('LARP view 4').click();
    const spectrum = page.getByLabel('QUANT DRIFT & COLLIDER SPECTRUM');
    await expect(spectrum).toBeVisible();

    await page.getByRole('button', { name: 'Ask ANANSI', exact: true }).click();
    await page.getByLabel('Your Question').fill('What is the capital of France?');
    await page.getByRole('button', { name: 'Ask Question' }).click();
    await expect(page.locator('.ask-result-section')).toBeVisible({ timeout: 60000 });

    await expect
      .poll(async () => Number(await spectrum.getAttribute('data-markers')), { timeout: 30000 })
      .toBeGreaterThanOrEqual(1);
  } finally {
    await session.cleanup();
  }
});

test('view 3 with no memory nodes shows an explicit empty panel', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    await session.pairAndOpen(session.workspaceId);
    await page.getByLabel('Toggle LARP mode').click();
    await page.getByLabel('LARP view 3').click();
    const spider = page.getByLabel('ATTENTION SPIDER // GRAPH DRIFT ENGINE');
    await expect(spider).toBeVisible();
    await expect(spider.getByText('No memory nodes yet — open Memory to load records.')).toBeVisible();
    await expect(spider.locator('canvas')).toHaveCount(0);
  } finally {
    await session.cleanup();
  }
});
