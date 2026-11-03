// HOME Command Dashboard Slice 1: landing on connection, quick
// actions, project-card jump to the Guided Walk, and the empty state.
// Navigation buttons and quick actions share labels (e.g. Ask ANANSI),
// so every click is scoped to its region. Click behavior is e2e-only
// (static markup cannot run effects).

import { test, expect } from '@playwright/test';
import { startSession } from './collider-three-helpers';

test.beforeEach(async ({ page }) => {
  page.setDefaultTimeout(8000);
});

function nav(page: import('@playwright/test').Page) {
  return page.getByRole('navigation', { name: 'Main navigation' });
}

function quickActions(page: import('@playwright/test').Page) {
  return page.getByRole('region', { name: 'Quick actions' });
}

function projectHub(page: import('@playwright/test').Page) {
  return page.getByRole('region', { name: 'Recent & active projects' });
}

test('connection lands on the Dashboard with workspace overview', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    await session.openWorkspace(session.workspaceId);
    await expect(page.getByRole('heading', { name: 'Dashboard', exact: true })).toBeVisible();
    await expect(page.getByText('Collider Workspace')).toBeVisible();
    await expect(page.getByText(/Ready/)).toBeVisible();
    await expect(quickActions(page).getByRole('button', { name: '+ New Project' })).toBeVisible();
    await expect(quickActions(page).getByRole('button', { name: 'Ask ANANSI' })).toBeVisible();
    await expect(quickActions(page).getByRole('button', { name: 'LEXI-COLLIDER' })).toBeVisible();
    await expect(quickActions(page).getByRole('button', { name: /LARP Telemetry Engine/ })).toBeVisible();
    await expect(nav(page).getByRole('button', { name: 'Dashboard', exact: true })).toHaveAttribute(
      'aria-current',
      'page',
    );
  } finally {
    await session.cleanup();
  }
});

test('quick actions jump to Ask, LARP, and project creation', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    await session.openWorkspace(session.workspaceId);
    await quickActions(page).getByRole('button', { name: 'Ask ANANSI' }).click();
    await expect(page.getByLabel('Your Question')).toBeVisible();

    await nav(page).getByRole('button', { name: 'Dashboard', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Dashboard', exact: true })).toBeVisible();

    await quickActions(page).getByRole('button', { name: /LARP Telemetry Engine/ }).click();
    await expect(page.getByLabel('LARP telemetry frame')).toBeVisible();

    await quickActions(page).getByRole('button', { name: '+ New Project' }).click();
    await expect(page.getByRole('heading', { name: 'Create a project' })).toBeVisible();
  } finally {
    await session.cleanup();
  }
});

test('project card Continue Pipeline jumps to the Guided Walk', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    await session.call('memories', 'create', {
      workspaceId: session.workspaceId,
      kind: 'project',
      draft: {
        title: 'Dashboard Harbor',
        body: 'Harbor logistics for dashboard testing.',
      },
    });
    await session.openWorkspace(session.workspaceId);
    const card = projectHub(page).locator('li', { hasText: 'Dashboard Harbor' });
    await expect(card).toBeVisible();
    await card.getByRole('button', { name: 'Continue Pipeline' }).click();
    await expect(page.getByRole('heading', { name: 'Guided Walk' })).toBeVisible();
    await expect(page.getByText('Edit project: Dashboard Harbor')).toBeVisible();
  } finally {
    await session.cleanup();
  }
});

test('a workspace with no projects shows the empty-state invitation', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    const created = await session.call('workspaces', 'create', { name: 'Empty Dashboard' });
    const emptyId =
      (created.body.workspace as { id: string } | undefined)?.id ??
      (created.body.id as string);
    await session.openWorkspace(emptyId);
    // A fresh workspace is not onboarded yet, so the onboarding flow
    // shows first; completing it lands on the Dashboard.
    await page.getByLabel('I have reviewed these privacy limits').check();
    await page.getByRole('button', { name: 'Continue to AI setup' }).click();
    await page.getByRole('button', { name: 'Skip AI setup and enter ANANSI' }).click();
    await expect(page.getByRole('heading', { name: 'Dashboard', exact: true })).toBeVisible();
    await expect(page.getByText('No projects yet')).toBeVisible();
    await expect(projectHub(page).getByRole('button', { name: 'Continue Pipeline' })).toHaveCount(0);
  } finally {
    await session.cleanup();
  }
});

function healthSection(page: import('@playwright/test').Page) {
  return page.getByRole('region', { name: 'System & storage health' });
}

test('health section shows manual backups and jumps to Backups', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    await session.openWorkspace(session.workspaceId);
    await expect(healthSection(page).getByText('Manual backups only')).toBeVisible();
    await healthSection(page).getByRole('button', { name: 'Backups' }).click();
    await expect(page.getByRole('heading', { name: 'Your backups', exact: true })).toBeVisible();
  } finally {
    await session.cleanup();
  }
});

test('health section shows an idle background queue and jumps to Jobs', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, false);
  try {
    await session.openWorkspace(session.workspaceId);
    await expect(healthSection(page).getByText('Background queue idle')).toBeVisible();
    await healthSection(page).getByRole('button', { name: 'Jobs' }).click();
    await expect(page.getByRole('heading', { name: 'Your jobs and exports', exact: true })).toBeVisible();
  } finally {
    await session.cleanup();
  }
});
