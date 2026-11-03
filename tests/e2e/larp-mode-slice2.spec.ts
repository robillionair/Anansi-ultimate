// LARP Mode Slice 2: View 1 particle rendering, View 2 bezier funnel,
// key/button view switching, a live Ask run firing a compilation pulse,
// and clean disposal on exit. Click/key behavior is e2e-only (static
// markup cannot run effects). The Ask leg asks an unrelated question so
// the run completes via the insufficient-evidence path with no provider
// call; the compilation event still fires, which is exactly what the
// funnel's pulses subscribe to.

import { test, expect } from '@playwright/test';
import { startSession } from './collider-three-helpers';

test.beforeEach(async ({ page }) => {
  page.setDefaultTimeout(8000);
});

test('view 1 renders particles, view 2 renders the funnel, keys switch views', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    await session.pairAndOpen(session.workspaceId);
    await page.getByLabel('Toggle LARP mode').click();
    const hud = page.getByLabel('LARP telemetry frame');
    await expect(hud).toBeVisible();

    // View 1 is the default: a real WebGL canvas that has rendered.
    const vault = page.getByLabel('SECOND BRAIN // 3D MEMORY VAULT');
    await expect(vault).toBeVisible();
    await expect(vault.locator('canvas')).toBeVisible();
    await expect
      .poll(async () => Number(await vault.getAttribute('data-frames')), { timeout: 60000 })
      .toBeGreaterThan(0);

    // Button switch to View 2.
    await page.getByLabel('LARP view 2').click();
    const funnel = page.getByLabel('NEURAL FUNNEL // CONTEXT COMPILER');
    await expect(funnel).toBeVisible();
    await expect(funnel.locator('canvas')).toBeVisible();

    // Key switch back to View 1 (focus sits on the view button, a
    // non-editable target, so the switcher must fire).
    await page.keyboard.press('1');
    await expect(page.getByLabel('SECOND BRAIN // 3D MEMORY VAULT')).toBeVisible();
    await expect(page.getByLabel('LARP view 1')).toHaveAttribute('aria-pressed', 'true');

    // Key switch to View 2.
    await page.keyboard.press('2');
    await expect(page.getByLabel('NEURAL FUNNEL // CONTEXT COMPILER')).toBeVisible();

    // OFF disposes the whole HUD, canvases included.
    await page.keyboard.press('Control+Shift+L');
    await expect(hud).toHaveCount(0);
    await expect(page.getByLabel('SECOND BRAIN // 3D MEMORY VAULT')).toHaveCount(0);
    await expect(page.getByLabel('NEURAL FUNNEL // CONTEXT COMPILER')).toHaveCount(0);
  } finally {
    await session.cleanup();
  }
});

test('a live ask run fires a compilation pulse in view 2', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    await session.pairAndOpen(session.workspaceId);
    await page.getByLabel('Toggle LARP mode').click();
    await expect(page.getByLabel('LARP telemetry frame')).toBeVisible();
    await page.getByLabel('LARP view 2').click();
    const funnel = page.getByLabel('NEURAL FUNNEL // CONTEXT COMPILER');
    await expect(funnel).toBeVisible();

    // The HUD overlays the Ask view but is pointer-events none except
    // the view buttons, so the Ask form stays usable underneath it.
    await page.getByRole('button', { name: 'Ask ANANSI', exact: true }).click();
    await page.getByLabel('Your Question').fill('What is the capital of France?');
    await page.getByRole('button', { name: 'Ask Question' }).click();
    await expect(page.locator('.ask-result-section')).toBeVisible({ timeout: 60000 });

    // The completed run pushed a compilation event; View 2 was mounted
    // throughout, so its live subscription spawned a pulse.
    await expect
      .poll(async () => Number(await funnel.getAttribute('data-pulses-fired')), { timeout: 30000 })
      .toBeGreaterThan(0);
  } finally {
    await session.cleanup();
  }
});

test('webgl failure falls back to an explicit vault panel', async ({ page }) => {
  test.setTimeout(300000);
  await page.addInitScript(`
    const orig = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (type, ...args) {
      if (type === 'webgl' || type === 'webgl2' || type === 'experimental-webgl') return null;
      return orig.call(this, type, ...args);
    };
  `);
  const session = await startSession(page, false);
  try {
    await session.pairAndOpen(session.workspaceId);
    await page.getByLabel('Toggle LARP mode').click();
    await expect(page.getByLabel('LARP telemetry frame')).toBeVisible();
    await expect(page.getByText('WebGL unavailable — the memory vault cannot render here.')).toBeVisible();
  } finally {
    await session.cleanup();
  }
});
