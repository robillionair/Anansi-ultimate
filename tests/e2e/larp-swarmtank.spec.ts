import { expect, test } from '@playwright/test';
import { startSession } from './collider-three-helpers';

test('LARP SwarmTank runs a simulation, explores its arena, and reopens the saved report', async ({ page }, testInfo) => {
  test.setTimeout(300000);
  await page.setViewportSize({ width: 1500, height: 1000 });
  const rendererErrors: string[] = [];
  page.on('pageerror', error => rendererErrors.push(error.message));
  page.on('console', message => {
    if (message.type() === 'error' && /WebGLProgram|shader/i.test(message.text())) {
      rendererErrors.push(message.text());
    }
  });
  const session = await startSession(page, true);
  try {
    const hits = session.fakeHits();
    await page.getByLabel('Toggle LARP mode').click();
    await page.keyboard.press('8');
    const console = page.getByLabel('SwarmTank controls');
    await console.getByLabel('Project', { exact: true }).selectOption({ label: 'Harbor App' });
    await console.getByLabel('SwarmTank description').fill('A scheduling app for small agencies.');
    await console.getByLabel('SwarmTank pricing').fill('$49 per month');
    await console.getByLabel('SwarmTank monthly price amount').fill('49');
    await console.getByLabel('SwarmTank segment').fill('small agencies');
    await console.getByLabel('SwarmTank industry').fill('services');
    await console.getByLabel('SwarmTank value proposition').fill('Save ten hours a week on reporting.');
    await console.getByRole('button', { name: 'Generate personas' }).click();
    await expect(console.getByRole('heading', { name: 'Simulated personas (preview, not saved)' })).toBeVisible();
    await console.getByRole('button', { name: 'Evaluate decisions' }).click();
    const arena = page.getByRole('region', { name: 'SwarmTank 3D arena' });
    const canvas = arena.locator('canvas[data-interactive="true"]');
    await expect(canvas).toBeVisible();
    await expect(arena.getByLabel('Reduce motion (static arena view)')).toBeChecked();
    const matrixToggle = arena.getByRole('button', { name: 'Persona matrix' });
    const matrix = arena.getByRole('region', { name: 'Persona matrix' });
    await expect(matrix.locator('.larp-matrix-card')).toHaveCount(12);
    await expect(matrix.locator('.wire-filament').first()).toBeAttached();
    await matrix.getByRole('button', { name: 'Next persona cards' }).click();
    await expect(matrix.locator('.larp-matrix-card')).toHaveCount(8);
    await matrix.getByRole('button', { name: 'Previous persona cards' }).click();
    await matrix.locator('.larp-matrix-card').first().click();
    await expect(matrix.locator('.larp-matrix-card').first()).toHaveAttribute('aria-pressed', 'true');
    await expect(arena.getByRole('region', { name: 'Selected persona' })).toBeVisible();
    await expect(matrix.locator('g[data-active="true"]').first()).toBeAttached();
    await expect(matrix.locator('.wire-signal').first()).toHaveCSS('animation-name', 'none');
    await console.evaluate(element => { element.scrollTop = 0; });
    await page.screenshot({ path: testInfo.outputPath('swarmtank-matrix.png') });
    await arena.getByRole('button', { name: 'Close persona details' }).click();
    await matrixToggle.click();
    await expect(matrix).toHaveCount(0);
    await canvas.hover();
    const distance = await canvas.getAttribute('data-camera-distance');
    await page.mouse.wheel(0, 350);
    await expect(canvas).not.toHaveAttribute('data-camera-distance', distance!);
    const box = (await canvas.boundingBox())!;
    const position = await canvas.getAttribute('data-camera-position');
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2 + 95, box.y + box.height / 2 + 35, { steps: 5 });
    await page.mouse.up();
    await expect(canvas).not.toHaveAttribute('data-camera-position', position!);
    await arena.getByRole('button', { name: 'Reset arena camera' }).click();
    await expect(canvas).toHaveAttribute('data-camera-distance', distance!);
    await console.getByRole('button', { name: /Select .* in arena/ }).first().click();
    await expect(arena.getByRole('region', { name: 'Selected persona' })).toBeVisible();
    await arena.getByRole('button', { name: 'Close persona details' }).click();
    const outcomes = arena.getByLabel('Simulated decisions');
    const baseline = await outcomes.innerText();
    const price = console.getByLabel('Scenario monthly price (exact value)');
    await price.fill('490');
    await price.press('Tab');
    await expect(outcomes).not.toHaveText(baseline);
    await console.evaluate(element => { element.scrollTop = 0; });
    await page.screenshot({ path: testInfo.outputPath('swarmtank-larp.png') });

    await page.getByLabel('LARP view 1', { exact: true }).click();
    await page.getByLabel('LARP view 8', { exact: true }).click();
    await console.getByLabel('Project', { exact: true }).selectOption({ label: 'Harbor App' });
    await console.getByRole('button', { name: 'View report' }).click();
    await expect(canvas).toBeVisible();
    expect(session.fakeHits()).toBe(hits);
    expect(rendererErrors).toEqual([]);
  } finally {
    await session.cleanup();
  }
});
