import { test, expect } from '@playwright/test';
import { collideAndSave, startSession } from './collider-three-helpers';

/**
 * M8 Three.js visualization without WebGL: the page's
 * canvas getContext is stubbed to refuse every WebGL
 * context (the same in-page interception technique the
 * reviewer probes use), simulating a device without
 * WebGL. The saved-experiment view must then fall back
 * to the resilient Canvas 2D view with the full node
 * list — same version, same selection, same provenance.
 * GPU-disable launch flags alone did NOT remove WebGL
 * in this Chromium (SwiftShader still served contexts),
 * so the stub is what makes the no-WebGL condition
 * deterministic here.
 */
test('webgl failure falls back to Canvas 2D with the full list', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  // Plain-JS string: addInitScript evaluates the source
  // in the page, so TypeScript annotations here would
  // be syntax errors in the browser.
  await page.addInitScript(`
    const originalGetContext = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (type) {
      if (typeof type === 'string' && type.indexOf('webgl') === 0) return null;
      return originalGetContext.apply(this, arguments);
    };
  `);
  const session = await startSession(page, true);
  try {
    await collideAndSave(session.collider, 'Fallback beam about kitchens.', 'Fallback beam about orbitals.');
    await session.artifacts.getByRole('button', { name: /Collider experiment: Fallback beam about kitchens/ }).click();
    await expect(session.viz.getByRole('heading', { name: 'Experiment visualization' })).toBeVisible();
    // No WebGL here: the resilient Canvas 2D view
    // serves the same version with the same list.
    await expect(session.viz.locator('canvas[data-canvas-2d]')).toBeVisible();
    await expect(session.viz.locator('canvas[data-three-canvas]')).toHaveCount(0);
    await session.viz.getByRole('button', { name: 'Distributed signaling' }).click();
    await expect(session.viz.getByRole('heading', { name: 'Selected: Conceptual primitives — Distributed signaling' })).toBeVisible();
  } finally {
    await session.cleanup();
  }
});
