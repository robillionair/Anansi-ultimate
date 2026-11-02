import { test, expect } from '@playwright/test';
import { startSession } from './collider-three-helpers';

test('ghost-prompt slice 1: consent, revision capture, review, purge', async ({ page }) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  try {
    await expect(session.fakeHits()).toBe(0);
    // Ask ANANSI lives in the workspace nav, not the project view.
    await page.getByRole('button', { name: 'Ask ANANSI' }).click();
    await expect(page.locator('section[aria-label="GHOST-PROMPT controls"]')).toBeVisible();
    const strip = page.locator('section[aria-label="GHOST-PROMPT controls"]');

    // Step 1: OFF by default.
    await expect(strip.getByText('Draft Exploration ON')).toHaveCount(0);
    await expect(strip.getByRole('button', { name: 'Enable Draft Exploration' })).toBeVisible();

    // Step 2: explicit opt-in through the consent modal.
    await strip.getByRole('button', { name: 'Enable Draft Exploration' }).click();
    const dialog = page.getByRole('dialog', { name: 'Enable Draft Exploration consent' });
    await expect(dialog).toContainText(
      'When enabled, ANANSI can temporarily track revisions made in this editor. Deleted text remains available for your review until you discard it or explicitly save it.',
    );
    await dialog.getByRole('button', { name: 'Enable', exact: true }).click();
    await expect(strip.getByText('Draft Exploration ON')).toBeVisible();
    await expect(strip.getByText('Recording', { exact: true })).toBeVisible();

    // Step 3: type, pause >2s, replace a term, delete a phrase.
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

    // Step 4: review shows the deleted fragments and the pause event.
    await strip.getByRole('button', { name: 'Review drafts' }).click();
    const review = page.locator('section[aria-label="Retained & Deleted Fragments"]');
    await expect(review).toContainText('Retained text: blue jumps');
    const struck = review.locator('table[aria-label="Deleted fragments timeline"] s');
    await expect(struck).toHaveText(['red', ' fox']);
    await expect(review.getByText('deleted').first()).toBeVisible();
    const pauseText =
      (await review.locator('li', { hasText: 'Pause of' }).first().textContent()) ?? '';
    const pauseMs = Number(/Pause of (\d+)ms/.exec(pauseText)?.[1]);
    expect(pauseMs).toBeGreaterThan(2000);

    // Step 5: Stop & discard purges everything; a fresh enable starts empty.
    await strip.getByRole('button', { name: 'Stop & discard session' }).click();
    await expect(strip.getByText('Draft Exploration ON')).toHaveCount(0);
    await expect(strip.getByText('Draft history purged.')).toBeVisible();
    await expect(strip.getByRole('button', { name: 'Enable Draft Exploration' })).toBeVisible();
    await strip.getByRole('button', { name: 'Enable Draft Exploration' }).click();
    await dialog.getByRole('button', { name: 'Enable', exact: true }).click();
    await expect(strip.getByText('Draft Exploration ON')).toBeVisible();
    await strip.getByRole('button', { name: 'Review drafts' }).click();
    await expect(
      page.locator('section[aria-label="Retained & Deleted Fragments"]'),
    ).toContainText('No deleted fragments recorded.');
    await expect(session.fakeHits()).toBe(0);
  } finally {
    await session.cleanup();
  }
});

test('ghost-prompt slice 1: pause and clear keep the session recording', async ({ page }) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  try {
    await page.getByRole('button', { name: 'Ask ANANSI' }).click();
    await expect(page.locator('section[aria-label="GHOST-PROMPT controls"]')).toBeVisible();
    const strip = page.locator('section[aria-label="GHOST-PROMPT controls"]');
    await strip.getByRole('button', { name: 'Enable Draft Exploration' }).click();
    const dialog = page.getByRole('dialog', { name: 'Enable Draft Exploration consent' });
    await dialog.getByRole('button', { name: 'Enable', exact: true }).click();
    await expect(strip.getByText('Draft Exploration ON')).toBeVisible();

    await strip.getByRole('button', { name: 'Pause recording' }).click();
    await expect(strip.getByText('Paused', { exact: true })).toBeVisible();
    const input = page.getByLabel('Your Question');
    await input.click();
    await page.keyboard.type('unrecorded while paused');
    await strip.getByRole('button', { name: 'Resume recording' }).click();
    await expect(strip.getByText('Recording', { exact: true })).toBeVisible();
    await strip.getByRole('button', { name: 'Review drafts' }).click();
    await expect(
      page.locator('section[aria-label="Retained & Deleted Fragments"]'),
    ).toContainText('No deleted fragments recorded.');

    await input.click();
    await page.keyboard.press('ControlOrMeta+A');
    await page.keyboard.type('kept draft');
    await strip.getByRole('button', { name: 'Clear draft history' }).click();
    await expect(strip.getByText('Draft history purged.')).toBeVisible();
    await expect(strip.getByText('Draft Exploration ON')).toBeVisible();
    await expect(
      page.locator('section[aria-label="Retained & Deleted Fragments"]'),
    ).toContainText('No deleted fragments recorded.');
    await expect(session.fakeHits()).toBe(0);
  } finally {
    await session.cleanup();
  }
});
