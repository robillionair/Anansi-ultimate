import { test, expect } from '@playwright/test';
import { startSession } from './collider-three-helpers';
import {
  ghostProjectToNdc,
  layoutGhostDraft,
} from '../../packages/ui/src/ghostPromptThreeLayout';

async function enableDraftExploration(page: import('@playwright/test').Page) {
  await page.getByRole('button', { name: 'Ask ANANSI', exact: true }).click();
  await expect(
    page.locator('section[aria-label="GHOST-PROMPT controls"]'),
  ).toBeVisible();
  const strip = page.locator('section[aria-label="GHOST-PROMPT controls"]');
  await strip.getByRole('button', { name: 'Enable Draft Exploration' }).click();
  const dialog = page.getByRole('dialog', { name: 'Enable Draft Exploration consent' });
  await dialog.getByRole('button', { name: 'Enable', exact: true }).click();
  await expect(strip.getByText('Draft Exploration ON')).toBeVisible();
  return strip;
}

async function typeRevisions(page: import('@playwright/test').Page) {
  const input = page.getByLabel('Your Question');
  await input.click();
  await page.keyboard.type('red fox jumps');
  await page.waitForTimeout(2100);
  await input.evaluate((el) => {
    (el as HTMLInputElement).setSelectionRange(0, 3);
  });
  await page.keyboard.type('blue');
  await expect(input).toHaveValue('blue fox jumps');
  await input.evaluate((el) => {
    (el as HTMLInputElement).setSelectionRange(4, 8);
  });
  await page.keyboard.press('Backspace');
  await expect(input).toHaveValue('blue jumps');
}

test('ghost-prompt slice 2: 3D view renders and node picking opens the detail card', async ({ page }) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  try {
    const strip = await enableDraftExploration(page);
    await typeRevisions(page);
    await strip.getByRole('button', { name: 'Review drafts' }).click();
    const view = page.locator('section[aria-label="GHOST-PROMPT 3D draft view"]');
    await strip.getByRole('button', { name: 'Expand 3D view' }).click();
    await expect(view).toBeVisible();
    const canvas = view.locator('canvas[data-three-canvas]');
    await expect(canvas).toBeVisible();

    // Node positions come from the pure layout module, exactly as the
    // component computes them; the fixed camera math turns them into
    // client pixels for the raycast click.
    const layout = layoutGhostDraft({
      currentText: 'blue jumps',
      fragments: [
        { seq: 14, offset: 0, text: 'red', hash: '' },
        { seq: 18, offset: 4, text: ' fox', hash: '' },
      ],
      pauses: [],
      events: [],
    });
    const red = layout.nodes[0];
    await canvas.scrollIntoViewIfNeeded();
    const box = (await canvas.boundingBox())!;
    const aspect = box.width / box.height;
    const { ndcX, ndcY } = ghostProjectToNdc(red.x, red.y, aspect);
    await page.mouse.click(
      box.x + ((ndcX + 1) / 2) * box.width,
      box.y + ((1 - ndcY) / 2) * box.height,
    );
    const card = view.locator('section[aria-label="Selected fragment"]');
    await expect(card).toBeVisible();
    await expect(card.getByText('red')).toBeVisible();
    await expect(session.fakeHits()).toBe(0);
  } finally {
    await session.cleanup();
  }
});

test('ghost-prompt slice 2: reduced motion is static and context loss falls back to 2D', async ({ page }) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, true);
  try {
    const strip = await enableDraftExploration(page);
    const input = page.getByLabel('Your Question');
    await input.click();
    await page.keyboard.type('red fox');
    await strip.getByRole('button', { name: 'Review drafts' }).click();
    await strip.getByRole('button', { name: 'Expand 3D view' }).click();
    const view = page.locator('section[aria-label="GHOST-PROMPT 3D draft view"]');
    await expect(view).toBeVisible();
    const toggle = view.getByLabel('Reduce motion (static view)');
    await expect(toggle).toBeChecked();
    await expect(view.getByText('Motion reduced — static view.')).toBeVisible();
    await expect(view.locator('canvas[data-three-canvas]')).toHaveCount(0);
    await expect(view.locator('svg')).toBeVisible();
    await expect(view.locator('ol[aria-label="Edit history"]')).toBeVisible();

    await toggle.uncheck();
    const canvas = view.locator('canvas[data-three-canvas]');
    await expect(canvas).toBeVisible();
    await toggle.check();
    await expect(view.getByText('Motion reduced — static view.')).toBeVisible();

    await toggle.uncheck();
    await expect(canvas).toBeVisible();
    await canvas.scrollIntoViewIfNeeded();
    await canvas.evaluate((el) => {
      const target = el as HTMLCanvasElement;
      const context =
        target.getContext('webgl2') ?? target.getContext('webgl');
      const extension = context?.getExtension('WEBGL_lose_context');
      if (!extension) throw Error('WEBGL_lose_context unavailable');
      extension.loseContext();
    });
    await expect(
      view.getByText('WebGL unavailable — showing the accessible 2D view.'),
    ).toBeVisible();
    await expect(view.locator('svg')).toBeVisible();
    await expect(session.fakeHits()).toBe(0);
  } finally {
    await session.cleanup();
  }
});
