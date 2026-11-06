import { test, expect } from '@playwright/test';
import { startSession } from './collider-three-helpers';

/**
 * SwarmTank v2 Checkpoint 1: market selection + profile provenance.
 * (1) The market selector lists the CLDR territories and is searchable;
 * selecting Estonia shows the researched profile (evidence with source
 * and date, editable assumptions, buyer segments). (2) Generating and
 * evaluating with Estonia saves a run whose report labels the market
 * "Estonia — researched profile v1". (3) Selecting a territory with no
 * researched profile (France) opens the Custom assumptions state, and
 * the saved run is labelled Custom assumptions. No provider call is
 * made at any step.
 */

async function openSwarmtank(page: import('@playwright/test').Page) {
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  await session.openProject();
  const swarmtank = page.locator('section[aria-labelledby="project-swarmtank-heading"]');
  await expect(swarmtank.getByRole('heading', { name: 'SwarmTank', exact: true })).toBeVisible();
  await swarmtank.getByLabel('SwarmTank description').fill('A scheduling tool for small agencies that automates client reporting.');
  await swarmtank.getByLabel('SwarmTank pricing').fill('$49 per month');
  await swarmtank.getByLabel('SwarmTank monthly price amount').fill('49');
  await swarmtank.getByLabel('SwarmTank segment').fill('small agencies');
  await swarmtank.getByLabel('SwarmTank industry').fill('services');
  return { session, swarmtank };
}

test('Estonia shows a researched profile and the saved run labels it', async ({ page }) => {
  const { swarmtank } = await openSwarmtank(page);
  const market = swarmtank.locator('section[aria-label="Market"]');
  await expect(market.getByLabel('Search country or territory')).toBeVisible();
  // The searchable list covers the CLDR territories.
  await market.getByLabel('Search country or territory').fill('Estonia');
  await market.getByLabel('Country or territory', { exact: true }).selectOption({ label: 'Estonia (EE) — researched profile' });
  await expect(market.getByText('Researched profile: Estonia (v1)')).toBeVisible();
  await expect(market.getByText('Evidence (each with source and date)')).toBeVisible();
  await expect(market.getByText(/OECD, Financing SMEs and Entrepreneurs 2026/)).toBeVisible();
  await expect(market.getByText('Assumptions (editable — gaps, not evidence)')).toBeVisible();
  await expect(market.getByText('Buyer segments')).toBeVisible();
  await expect(market.getByText(/Micro enterprise \(under 10 employees\)/)).toBeVisible();

  await swarmtank.getByRole('button', { name: 'Generate personas' }).click();
  await expect(page.getByText(/Generated 20 simulated personas/)).toBeVisible();
  await swarmtank.getByRole('button', { name: 'Evaluate decisions' }).click();
  await expect(page.getByText('Evaluated and saved the SwarmTank run.')).toBeVisible();
  const report = swarmtank.locator('section[aria-label="SwarmTank report"]');
  await expect(report.getByText(/Estonia — researched profile v1/)).toBeVisible();
});

test('a territory with no researched profile opens Custom assumptions', async ({ page }) => {
  const { swarmtank } = await openSwarmtank(page);
  const market = swarmtank.locator('section[aria-label="Market"]');
  await market.getByLabel('Search country or territory').fill('France');
  await market.getByLabel('Country or territory', { exact: true }).selectOption({ label: 'France (FR)' });
  await expect(market.getByRole('heading', { name: 'Custom assumptions' })).toBeVisible();
  await expect(
    market.getByText(/No researched profile exists for France/),
  ).toBeVisible();
  await market.getByLabel('Business language').fill('French');
  await market.getByLabel('Typical budget').fill('Mid-market; price-sensitive');

  await swarmtank.getByRole('button', { name: 'Generate personas' }).click();
  await expect(page.getByText(/Generated 20 simulated personas/)).toBeVisible();
  await swarmtank.getByRole('button', { name: 'Evaluate decisions' }).click();
  await expect(page.getByText('Evaluated and saved the SwarmTank run.')).toBeVisible();
  const report = swarmtank.locator('section[aria-label="SwarmTank report"]');
  await expect(report.getByText(/France \(FR\) — Custom assumptions/)).toBeVisible();
  await expect(report.getByText(/2 custom assumption\(s\) recorded/)).toBeVisible();
});

test('the territory search narrows the list', async ({ page }) => {
  const { swarmtank } = await openSwarmtank(page);
  const market = swarmtank.locator('section[aria-label="Market"]');
  await market.getByLabel('Search country or territory').fill('zz-no-such-place');
  await expect(market.getByText('No territory matches that search.')).toBeVisible();
  await market.getByLabel('Search country or territory').fill('United States');
  await market.getByLabel('Country or territory', { exact: true }).selectOption({ label: 'United States (US) — researched profile' });
  await expect(market.getByText('Researched profile: United States (v1)')).toBeVisible();
});
