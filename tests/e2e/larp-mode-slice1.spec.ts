// LARP Mode Slice 1: toggle click and Ctrl+Shift+L transitions, HUD
// structure while ON, and clean unmount back to Original Mode. Click
// behavior is e2e-only (static markup cannot run effects).

import { test, expect } from '@playwright/test';
import { startSession } from './collider-three-helpers';

test.beforeEach(async ({ page }) => {
  page.setDefaultTimeout(8000);
});

test('toggle click enters LARP mode and hotkey exits cleanly', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    await session.pairAndOpen(session.workspaceId);
    const toggle = page.getByLabel('Toggle LARP mode');
    await expect(toggle).toBeVisible();
    await expect(page.getByText('[ LARP: OFF ]', { exact: true })).toBeVisible();

    await toggle.click();
    const hud = page.getByLabel('LARP telemetry frame');
    await expect(hud).toBeVisible();
    await expect(toggle).toHaveAttribute('aria-pressed', 'true');
    await expect(hud.getByText('[ LARP: ACTIVE ]', { exact: true })).toBeVisible();
    await expect(page.getByLabel('Memory allocation matrix')).toBeVisible();
    await expect(page.getByText('Token Compression Delta Stream')).toBeVisible();
    await expect(page.getByLabel('Token velocity coherence waveforms')).toBeVisible();
    for (let n = 1; n <= 6; n++) {
      await expect(page.getByLabel(`LARP view ${n}`)).toBeVisible();
    }
    await expect(page.getByText('No telemetry events yet.')).toBeVisible();

    await page.getByLabel('LARP view 6').click();
    await expect(page.getByLabel('COLLIDER FIELD // CONCEPT RESONANCE ACCELERATOR')).toBeVisible();

    await page.keyboard.press('Control+Shift+L');
    await expect(hud).toHaveCount(0);
    await expect(page.getByText('[ LARP: OFF ]', { exact: true })).toBeVisible();

    // Clean re-mount: a second ON shows the frame again, fresh.
    await toggle.click();
    await expect(page.getByLabel('LARP telemetry frame')).toBeVisible();
    await expect(page.getByLabel('LARP view 1')).toHaveAttribute('aria-pressed', 'true');
  } finally {
    await session.cleanup();
  }
});

test('cold unmount removes all HUD canvases and listeners leave no frame behind', async ({
  page,
}) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    await session.pairAndOpen(session.workspaceId);
    const toggle = page.getByLabel('Toggle LARP mode');
    await toggle.click();
    await expect(page.getByLabel('LARP telemetry frame')).toBeVisible();
    await expect(page.getByLabel('Memory allocation matrix')).toBeVisible();
    await expect(page.getByLabel('Token velocity coherence waveforms')).toBeVisible();

    await page.keyboard.press('Control+Shift+L');
    await expect(page.getByLabel('LARP telemetry frame')).toHaveCount(0);
    await expect(page.getByLabel('Memory allocation matrix')).toHaveCount(0);
    await expect(page.getByLabel('Token velocity coherence waveforms')).toHaveCount(0);
    await expect(page.getByText('[ LARP: OFF ]', { exact: true })).toBeVisible();
  } finally {
    await session.cleanup();
  }
});
