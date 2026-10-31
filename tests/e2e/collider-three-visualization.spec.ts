import { test, expect } from '@playwright/test';
import path from 'node:path';
import {
  clickBeamAHub,
  collideAndSave,
  startSession,
} from './collider-three-helpers';

/**
 * M8 Three.js visualization in the real browser: a
 * saved experiment opens a WebGL canvas built from the
 * exact pinned version (two toroidal beam streams,
 * orbiting fragments, collision marker, output nodes,
 * section-flow edges); raycast clicks on the 3D canvas
 * select nodes; the list alternative stays functional;
 * switching versions clears selection; reduced motion
 * renders identical static frames. Visualization
 * interaction causes no provider traffic.
 */
test('three.js visualization: webgl canvas, raycast select, switch clears, reduced motion static', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  const shotDir = process.env.VIZ_SHOT_DIR;
  try {
    await collideAndSave(session.collider, 'First beam about kitchens.', 'First beam about orbitals.');
    const hitsAfterCollide = session.fakeHits();
    expect(hitsAfterCollide).toBeGreaterThan(0);
    await session.artifacts.getByRole('button', { name: /Collider experiment: First beam about kitchens/ }).click();
    await expect(session.viz.getByRole('heading', { name: 'Experiment visualization' })).toBeVisible();
    // The WebGL canvas renders (not the 2D fallback).
    const three = session.viz.locator('canvas[data-three-canvas]');
    await expect(three).toBeVisible();
    await expect(session.viz.locator('canvas[data-canvas-2d]')).toHaveCount(0);
    await expect(session.viz.getByRole('button', { name: 'Distributed signaling' })).toBeVisible();
    if (shotDir) await three.screenshot({ path: path.join(shotDir, 'viz-three-animated.png') });

    // Pause for determinism, then raycast-pick the Beam A
    // hub on the 3D canvas: exact text, no new traffic.
    await session.viz.getByRole('button', { name: 'Pause motion' }).click();
    await clickBeamAHub(three);
    await expect(session.viz.getByRole('heading', { name: 'Selected: Beam A — Beam A' })).toBeVisible();
    await expect(session.viz.locator('pre', { hasText: 'First beam about kitchens.' })).toBeVisible();
    expect(session.fakeHits()).toBe(hitsAfterCollide);

    // List selection still works in the 3D view.
    await session.viz.getByRole('button', { name: 'Distributed signaling' }).click();
    await expect(session.viz.getByRole('heading', { name: 'Selected: Conceptual primitives — Distributed signaling' })).toBeVisible();

    // Second experiment, then switch back: selection
    // from the second view must not survive the switch.
    await session.openProject();
    await collideAndSave(session.collider, 'Second run beam about gardens.', 'Second run beam about tides.');
    await session.artifacts.getByRole('button', { name: /Collider experiment: Second run beam about gardens/ }).click();
    await expect(session.viz.getByRole('heading', { name: 'Experiment visualization' })).toBeVisible();
    await session.viz.getByRole('button', { name: 'Distributed signaling' }).click();
    await expect(session.viz.getByRole('heading', { name: 'Selected: Conceptual primitives — Distributed signaling' })).toBeVisible();
    await session.artifacts.getByRole('button', { name: /Collider experiment: First beam about kitchens/ }).click();
    await expect(session.viz.getByRole('heading', { name: /^Selected:/ })).toHaveCount(0);

    // Reduced motion: the static note shows, no pause
    // controls, and two frames 600ms apart are identical.
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await session.openProject();
    await session.artifacts.getByRole('button', { name: /Collider experiment: First beam about kitchens/ }).click();
    await expect(session.viz.getByText('Motion reduced — static view.')).toBeVisible();
    await expect(session.viz.getByRole('button', { name: 'Pause motion' })).toHaveCount(0);
    await expect(session.viz.getByRole('button', { name: 'Resume motion' })).toHaveCount(0);
    const canvas = session.viz.locator('canvas[data-three-canvas]');
    await page.waitForTimeout(500);
    const first = await canvas.screenshot();
    await page.waitForTimeout(600);
    const second = await canvas.screenshot();
    expect(first.equals(second)).toBe(true);
    if (shotDir) await canvas.screenshot({ path: path.join(shotDir, 'viz-three-reduced-motion.png') });
  } finally {
    await session.cleanup();
  }
});

test.describe('three.js visualization at high DPI', () => {
  test.use({ deviceScaleFactor: 2 });

  test('high-DPI 3D canvas renders and selects', async ({ page }) => {
    test.setTimeout(240000);
    page.setDefaultTimeout(8000);
    const session = await startSession(page, true);
    const shotDir = process.env.VIZ_SHOT_DIR;
    try {
      await collideAndSave(session.collider, 'Shot beam about kitchens.', 'Shot beam about orbitals.');
      await session.artifacts.getByRole('button', { name: /Collider experiment: Shot beam about kitchens/ }).click();
      await expect(session.viz.getByRole('heading', { name: 'Experiment visualization' })).toBeVisible();
      const three = session.viz.locator('canvas[data-three-canvas]');
      await expect(three).toBeVisible();
      if (shotDir) await three.screenshot({ path: path.join(shotDir, 'viz-three-hidpi-reduced-motion.png') });
      await session.viz.getByRole('button', { name: 'Distributed signaling' }).click();
      await expect(session.viz.getByRole('heading', { name: 'Selected: Conceptual primitives — Distributed signaling' })).toBeVisible();
    } finally {
      await session.cleanup();
    }
  });
});
