import { expect, test } from '@playwright/test';
import { startSession } from './collider-three-helpers';

test('LARP SOMNIA instruments follow replay and provider activity', async ({ page }, testInfo) => {
  test.setTimeout(180_000);
  page.setDefaultTimeout(8000);
  await page.setViewportSize({ width: 1500, height: 950 });
  const session = await startSession(page, false);
  let releaseDispatch!: () => void;
  const hold = new Promise<void>(resolve => { releaseDispatch = resolve; });
  try {
    const hits = session.fakeHits();
    await page.getByLabel('Toggle LARP mode').click();
    await page.keyboard.press('9');
    const chamber = page.getByRole('region', { name: 'SOMNIA chamber' });
    const controls = page.getByLabel('SOMNIA controls');
    await expect(chamber.getByText('STANDBY', { exact: true })).toBeVisible();
    await controls.getByLabel('Project', { exact: true }).selectOption({ label: 'Harbor App' });
    await controls.getByRole('button', { name: 'Load built-in cases' }).click();
    await controls.getByRole('button', { name: 'Preview replay input' }).click();
    await expect(chamber.getByText('REVIEW READY', { exact: true })).toBeVisible();
    await controls.getByRole('button', { name: 'Run evaluation' }).click();
    await expect(chamber.getByText('EVALUATION COMPLETE', { exact: true })).toBeVisible();
    await chamber.getByRole('button', { name: 'Findings' }).click();
    await expect(chamber.getByRole('region', { name: 'Findings instrument' })).toContainText('recorded violations');
    await chamber.getByRole('button', { name: 'Pause motion' }).click();
    await expect(chamber).toHaveAttribute('data-paused', 'true');
    const canvas = chamber.locator('canvas[data-interactive="true"]');
    await expect(canvas).toBeVisible();
    const distance = (await canvas.getAttribute('data-camera-distance'))!;
    const box = (await canvas.boundingBox())!;
    await page.mouse.move(box.x + box.width * .50, box.y + box.height * .44);
    await page.mouse.wheel(0, 250);
    await expect(canvas).not.toHaveAttribute('data-camera-distance', distance, { timeout: 8000 });
    const position = (await canvas.getAttribute('data-camera-position'))!;
    await page.mouse.down();
    await page.mouse.move(box.x + box.width * .56, box.y + box.height * .49, { steps: 4 });
    await page.mouse.up();
    await expect(canvas).not.toHaveAttribute('data-camera-position', position, { timeout: 8000 });
    await chamber.getByRole('button', { name: 'Reset view' }).click();
    await expect(canvas).toHaveAttribute('data-camera-distance', distance);
    expect(session.fakeHits()).toBe(hits);

    const compare = controls.getByRole('region', { name: 'SOMNIA model comparison' });
    await compare.getByLabel('Model ID').fill('llama-x');
    await compare.getByLabel('Original replay prompt').fill('Original synthetic evidence.');
    await compare.getByLabel('Proposed replay prompt').fill('Proposed synthetic evidence.');
    await compare.getByRole('button', { name: 'Review model comparison' }).click();
    await chamber.getByRole('button', { name: 'Model', exact: true }).click();
    await page.route('**/v1/ai/providers-dispatch', async route => { await hold; await route.continue(); });
    await compare.getByRole('button', { name: 'Run approved model comparison' }).click();
    await expect(chamber.getByText('MODEL REPLAY', { exact: true })).toBeVisible();
    await expect(chamber).toHaveAttribute('data-active', 'true');
    await chamber.getByRole('button', { name: 'Resume motion' }).click();
    await page.screenshot({ path: testInfo.outputPath('larp-somnia-active.png') });
    releaseDispatch();
    await expect(chamber.getByText('COMPARISON COMPLETE', { exact: true })).toBeVisible();
    await expect(chamber.getByRole('region', { name: 'Model instrument' })).toContainText('16 original · 16 proposed tokens');
    expect(session.fakeHits()).toBe(hits + 2);
    await controls.evaluate(element => { element.scrollTop = 0; });
    await page.screenshot({ path: testInfo.outputPath('larp-somnia-chamber.png') });
  } finally {
    releaseDispatch();
    await session.cleanup();
  }
});

test('SOMNIA replay controls work without WebGL', async ({ page }) => {
  test.setTimeout(60_000);
  await page.addInitScript(`
    const getContext = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function(type, ...args) {
      return type.startsWith('webgl') ? null : getContext.call(this, type, ...args);
    };
  `);
  const session = await startSession(page, true);
  try {
    await page.getByLabel('Toggle LARP mode').click();
    await page.getByLabel('LARP view 9', { exact: true }).click();
    const chamber = page.getByRole('region', { name: 'SOMNIA chamber' });
    const controls = page.getByLabel('SOMNIA controls');
    await expect(chamber.locator('svg.somnia-optics')).toBeVisible();
    await expect(chamber.locator('canvas')).toHaveCount(0);
    await controls.getByLabel('Project', { exact: true }).selectOption({ label: 'Harbor App' });
    await controls.getByRole('button', { name: 'Load built-in cases' }).click();
    await controls.getByRole('button', { name: 'Preview replay input' }).click();
    await controls.getByRole('button', { name: 'Run evaluation' }).click();
    await expect(chamber.getByText('EVALUATION COMPLETE', { exact: true })).toBeVisible();
  } finally {
    await session.cleanup();
  }
});
