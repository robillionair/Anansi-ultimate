// LARP Mode Slice 4: attention heatmap from a genuine answered Ask run,
// collider field from a real LEXI-COLLIDER generation, the all-views
// cycle, and empty states. Click/key behavior is e2e-only (static
// markup cannot run effects). The heatmap's sequences are the real raw
// and compiled prompts (whitespace words, not provider tokens); the
// collider's discharge comes from a real collision event, not a stub.

import { test, expect } from '@playwright/test';
import { startSession, collideAndSave } from './collider-three-helpers';

test.beforeEach(async ({ page }) => {
  page.setDefaultTimeout(8000);
});

test('view 5 renders a genuine token alignment matrix after an answered run', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    await session.call('memories', 'create', {
      workspaceId: session.workspaceId,
      kind: 'memory',
      draft: {
        title: 'Project Aurora Storage Architecture',
        body: '# Architecture Plan\nWe decided to adopt SQLite with FTS5 for local-first search.',
      },
    });
    await session.pairAndOpen(session.workspaceId);

    await page.getByRole('button', { name: 'Ask ANANSI', exact: true }).click();
    await page.getByLabel('Your Question').fill('Which database powers Aurora offline knowledge?');
    await page.getByRole('button', { name: 'Ask Question' }).click();
    await expect(page.locator('.ask-result-section')).toBeVisible({ timeout: 60000 });
    await expect(page.locator('.answer-text')).not.toBeEmpty();

    await page.getByLabel('Toggle LARP mode').click();
    await page.getByLabel('LARP view 5').click();
    const heatmap = page.getByLabel('ATTENTION TENSOR HEATMAP // TOKEN ATTENTION MATRIX');
    await expect(heatmap).toBeVisible();
    await expect(heatmap.locator('canvas')).toBeVisible();
    await expect
      .poll(async () => Number(await heatmap.getAttribute('data-raw-tokens')))
      .toBeGreaterThanOrEqual(1);
    await expect
      .poll(async () => Number(await heatmap.getAttribute('data-compiled-tokens')))
      .toBeGreaterThanOrEqual(1);
    await expect
      .poll(async () => Number(await heatmap.getAttribute('data-matches')))
      .toBeGreaterThanOrEqual(1);
    await expect(heatmap.getByText('No attention heads measured')).toBeVisible();

    const box = await heatmap.locator('canvas').boundingBox();
    if (!box) throw new Error('heatmap canvas has no box');
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await expect(heatmap.locator('.larp-spectrum-readout')).not.toHaveText(
      'Hover the matrix to inspect a cell.',
    );
  } finally {
    await session.cleanup();
  }
});

test('view 6 discharges on a real collider generation', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    await session.pairAndOpen(session.workspaceId);
    await collideAndSave(session.collider, 'Quantum lattice memory', 'Ocean current logistics');

    await page.getByLabel('Toggle LARP mode').click();
    await page.getByLabel('LARP view 6').click();
    const field = page.getByLabel('COLLIDER FIELD // CONCEPT RESONANCE ACCELERATOR');
    await expect(field).toBeVisible();
    await expect(field.locator('canvas')).toBeVisible();
    await expect
      .poll(async () => Number(await field.getAttribute('data-frames')), { timeout: 60000 })
      .toBeGreaterThan(0);
    await expect
      .poll(async () => Number(await field.getAttribute('data-collisions')), { timeout: 30000 })
      .toBeGreaterThanOrEqual(1);
    await expect(field.getByText(/Quantum lattice memory/)).toBeVisible();
  } finally {
    await session.cleanup();
  }
});

test('all six views cycle via keys and buttons, then cold unmount', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    await session.pairAndOpen(session.workspaceId);
    await page.getByLabel('Toggle LARP mode').click();
    const labels = [
      'SECOND BRAIN // 3D MEMORY VAULT',
      'NEURAL FUNNEL // CONTEXT COMPILER',
      'ATTENTION SPIDER // GRAPH DRIFT ENGINE',
      'QUANT DRIFT & COLLIDER SPECTRUM',
      'ATTENTION TENSOR HEATMAP // TOKEN ATTENTION MATRIX',
      'COLLIDER FIELD // CONCEPT RESONANCE ACCELERATOR',
    ];
    for (let n = 1; n <= 6; n++) {
      await page.keyboard.press(String(n));
      await expect(page.getByLabel(labels[n - 1])).toBeVisible();
    }
    for (let n = 6; n >= 1; n--) {
      await page.getByLabel(`LARP view ${n}`).click();
      await expect(page.getByLabel(labels[n - 1])).toBeVisible();
    }
    await page.keyboard.press('Control+Shift+L');
    await expect(page.getByLabel('LARP telemetry frame')).toHaveCount(0);
    for (const label of labels) {
      await expect(page.getByLabel(label)).toHaveCount(0);
    }
  } finally {
    await session.cleanup();
  }
});

test('views 5 and 6 show explicit empty states with no data', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    await session.pairAndOpen(session.workspaceId);
    await page.getByLabel('Toggle LARP mode').click();
    await page.getByLabel('LARP view 5').click();
    const heatmap = page.getByLabel('ATTENTION TENSOR HEATMAP // TOKEN ATTENTION MATRIX');
    await expect(heatmap).toBeVisible();
    await expect(heatmap.getByText('No compiled prompt yet — ask a question in Ask ANANSI.')).toBeVisible();
    await expect(heatmap.locator('canvas')).toHaveCount(0);

    await page.getByLabel('LARP view 6').click();
    const field = page.getByLabel('COLLIDER FIELD // CONCEPT RESONANCE ACCELERATOR');
    await expect(field).toBeVisible();
    await expect(field.getByText('No collisions yet — run LEXI-COLLIDER in a project.')).toBeVisible();
  } finally {
    await session.cleanup();
  }
});
