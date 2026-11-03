import { test, expect } from '@playwright/test';
import { startSession } from './collider-three-helpers';

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

// §86 walk, steps 1-9: enable -> type & revise -> recording indicator ->
// inspect fragments -> review interpretations -> select fragments ->
// compare responses -> delete session -> verify wiped.
test('ghost-prompt slice 3: §86 walk end to end', async ({ page }) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  try {
    // 1. Explicitly enable Draft Exploration.
    const strip = await enableDraftExploration(page);
    const input = page.getByLabel('Your Question');

    // 2. Type and revise through all four finding triggers. The price
    // edit uses a selection replacement so the `$` stays inside the
    // diff span (`$50` -> `$20`); the other edits use whole-text fills
    // for deterministic diffs.
    await input.click();
    await input.fill('Build a cheap API tool for freelancers with privacy for $50');
    await input.evaluate((el) => {
      const target = el as HTMLInputElement;
      target.setSelectionRange(target.value.length - 3, target.value.length);
    });
    await page.keyboard.type('$20');
    await expect(input).toHaveValue(
      'Build a cheap API tool for freelancers with privacy for $20',
    );
    await input.fill('Build a cheap API tool for freelancers for $20');
    await input.fill('Build a cheap API tool for enterprise for $20');
    await input.fill('Build a cheap simple tool for enterprise for $20');

    // 3. Clear recording indicator.
    await expect(strip.getByText('Draft Exploration ON')).toBeVisible();
    await expect(strip.getByText('Recording', { exact: true })).toBeVisible();

    // 4. Inspect retained draft fragments.
    await strip.getByRole('button', { name: 'Review drafts' }).click();
    const review = strip.locator('section[aria-label="Review My Draft"]');
    await expect(review).toBeVisible();
    const fragmentsTable = strip.locator('section[aria-label="Retained & Deleted Fragments"]');
    await expect(fragmentsTable.getByText('with privacy', { exact: true })).toBeVisible();
    await expect(fragmentsTable.getByText('freelancers', { exact: true })).toBeVisible();

    // 5. Review optional interpretations: all four §57 sentences.
    await expect(
      review.getByText('You changed the requested price several times.'),
    ).toBeVisible();
    await expect(
      review.getByText('You removed a requirement about privacy.'),
    ).toBeVisible();
    await expect(review.getByText('You revised the target audience.')).toBeVisible();
    await expect(
      review.getByText('You replaced a technical term with a broader description.'),
    ).toBeVisible();
    await expect(
      review.getByText('These are objective observations about text changes, not interpretations of intent.'),
    ).toBeVisible();

    // 6. Select which earlier fragments may be used.
    await expect(
      review.getByText('Would you like ANANSI to consider any of these earlier alternatives?'),
    ).toBeVisible();
    await review.getByLabel('Include fragment: with privacy').check();
    await expect(
      review.getByRole('button', { name: 'Compare responses' }),
    ).toBeEnabled();

    // 7. Compare responses.
    await review.getByRole('button', { name: 'Compare responses' }).click();
    const compare = review.locator('section[aria-label="GHOST-PROMPT comparison"]');
    await expect(compare).toBeVisible();
    const panelA = compare.locator('section[aria-label="Final Prompt Only"]');
    const panelB = compare.locator(
      'section[aria-label="Final Prompt + Approved Draft Context"]',
    );
    await expect(panelA.locator('p').first()).toHaveText('Build a cheap simple tool for enterprise for $20');
    await expect(panelB.locator('p').first()).toContainText('Approved draft context');
    await expect(panelB.locator('li').filter({ hasText: 'with privacy' })).toBeVisible();
    await expect(panelB.getByText('freelancers')).toHaveCount(0);
    await expect(compare.getByText('Reference response (offline, not a model answer)')).toHaveCount(2);
    await expect(compare.getByText(/Extra characters:/)).toBeVisible();
    await expect(compare.getByText(/simulated cost delta:/)).toBeVisible();
    await compare.getByRole('button', { name: 'Final Prompt + Draft Context', exact: true }).click();
    await expect(
      compare.getByText('You found Final Prompt + Draft Context more useful.'),
    ).toBeVisible();
    await compare.getByRole('button', { name: 'Combine into an updated prompt' }).click();
    await expect(input).toHaveValue(/Approved draft context/);

    // 8. Delete the session.
    await strip.getByRole('button', { name: 'Stop & discard session' }).click();
    await expect(strip.getByText('Draft history purged.')).toBeVisible();

    // 9. Verify that no deleted draft remains persisted unexpectedly:
    // re-enable and confirm the review shows no fragments, no findings.
    await strip.getByRole('button', { name: 'Enable Draft Exploration' }).click();
    await page
      .getByRole('dialog', { name: 'Enable Draft Exploration consent' })
      .getByRole('button', { name: 'Enable', exact: true })
      .click();
    await expect(strip.getByText('Draft Exploration ON')).toBeVisible();
    await strip.getByRole('button', { name: 'Review drafts' }).click();
    await expect(strip.getByText('No deleted fragments recorded.')).toBeVisible();
    await expect(
      strip.locator('section[aria-label="Review My Draft"]').getByText('No draft findings yet.'),
    ).toBeVisible();
    await expect(session.fakeHits()).toBe(0);
  } finally {
    await session.cleanup();
  }
});

test('ghost-prompt slice 3: unselected fragments never reach Panel B', async ({ page }) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  try {
    const strip = await enableDraftExploration(page);
    const input = page.getByLabel('Your Question');
    await input.click();
    await input.fill('Build a cheap API tool for freelancers with privacy for $50');
    await input.fill('Build a cheap API tool for freelancers for $50');
    await input.fill('Build a cheap API tool for enterprise for $50');
    await strip.getByRole('button', { name: 'Review drafts' }).click();
    const review = strip.locator('section[aria-label="Review My Draft"]');
    await expect(review).toBeVisible();
    // Select only the privacy fragment; the audience fragment stays out.
    await review.getByLabel('Include fragment: with privacy').check();
    await review.getByRole('button', { name: 'Compare responses' }).click();
    const panelB = review.locator(
      'section[aria-label="Final Prompt + Approved Draft Context"]',
    );
    await expect(panelB.locator('li').filter({ hasText: 'with privacy' })).toBeVisible();
    await expect(panelB.getByText('freelancers')).toHaveCount(0);
    await expect(session.fakeHits()).toBe(0);
  } finally {
    await session.cleanup();
  }
});
