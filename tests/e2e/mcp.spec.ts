import { test, expect } from '@playwright/test';
import path from 'node:path';
import { startSession } from './collider-three-helpers.ts';

test.beforeEach(async ({ page }) => {
  page.setDefaultTimeout(8000);
});

async function openMcp(page: import('@playwright/test').Page) {
  const session = await startSession(page, false);
  await session.openWorkspace(session.workspaceId);
  await page.getByRole('button', { name: 'MCP', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Register an MCP server', exact: true })).toBeVisible();
  return session;
}

test('MCP navigation shows an empty registry', async ({ page }) => {
  test.setTimeout(300000);
  const session = await openMcp(page);
  try {
    await expect(page.getByText('No MCP servers configured yet.')).toBeVisible();
    await expect(page.getByText('A tool runs only after you approve that exact call')).toBeVisible();
  } finally {
    await session.cleanup();
  }
});

test('MCP stdio registration probes a real local server', async ({ page }) => {
  test.setTimeout(300000);
  const session = await openMcp(page);
  try {
    await page.getByLabel('Server name').fill('Fake MCP');
    await page.getByLabel('Command').fill('node');
    await page
      .getByLabel('Arguments (one per line)')
      .fill(path.resolve('tests/e2e/mcp-fake-server.mjs'));
    await page.getByRole('button', { name: 'Register server' }).click();
    await expect(page.getByText('MCP server registered.')).toBeVisible();
    const list = page.getByRole('list', { name: 'MCP servers' });
    await expect(list.getByText('Fake MCP')).toBeVisible();
    await list.getByRole('button', { name: 'Connect' }).click();
    await expect(page.getByText('Healthy via ping')).toBeVisible();
    await expect(page.getByText('protocol 2025-11-25')).toBeVisible();
    const tools = page.getByRole('list', { name: 'Tools from Fake MCP' });
    await expect(tools.getByText('search', { exact: true })).toBeVisible();
    await expect(tools.getByText('q (string, required)')).toBeVisible();
    await expect(tools.getByText('Read-only')).toBeVisible();
    await expect(tools.getByText('Destructive')).toBeVisible();
    const prompts = page.getByRole('list', { name: 'Prompts from Fake MCP' });
    await expect(prompts.getByText('summarize', { exact: true })).toBeVisible();
    await list.getByRole('button', { name: 'Remove' }).click();
    await expect(page.getByText('MCP server removed.')).toBeVisible();
    await expect(page.getByText('No MCP servers configured yet.')).toBeVisible();
  } finally {
    await session.cleanup();
  }
});

test('MCP SSE probe is explicitly refused', async ({ page }) => {
  test.setTimeout(300000);
  const session = await openMcp(page);
  try {
    await page.getByLabel('Server name').fill('Remote');
    await page.getByLabel('Transport').selectOption('sse');
    await page.getByLabel('Server URL').fill('https://example.test/mcp');
    await page.getByRole('button', { name: 'Register server' }).click();
    await expect(page.getByText('MCP server registered.')).toBeVisible();
    await page.getByRole('list', { name: 'MCP servers' }).getByRole('button', { name: 'Connect' }).click();
    await expect(page.getByText('Not healthy:')).toBeVisible();
    await expect(page.getByText('Only local stdio servers can be connected.')).toBeVisible();
  } finally {
    await session.cleanup();
  }
});

async function registerFakeAndConnect(page: import('@playwright/test').Page) {
  await page.getByLabel('Server name').fill('Fake MCP');
  await page.getByLabel('Command').fill('node');
  await page
    .getByLabel('Arguments (one per line)')
    .fill(path.resolve('tests/e2e/mcp-fake-server.mjs'));
  await page.getByRole('button', { name: 'Register server' }).click();
  await expect(page.getByText('MCP server registered.')).toBeVisible();
  const list = page.getByRole('list', { name: 'MCP servers' });
  await list.getByRole('button', { name: 'Connect' }).click();
  await expect(page.getByText('Healthy via ping')).toBeVisible();
  return page.getByRole('list', { name: 'Tools from Fake MCP' });
}

test('MCP tool call requires authorization and returns results', async ({ page }) => {
  test.setTimeout(300000);
  const session = await openMcp(page);
  try {
    const tools = await registerFakeAndConnect(page);
    const searchItem = tools.locator('li').filter({ hasText: 'Search memory.' });
    await searchItem.getByRole('button', { name: 'Call tool' }).click();
    await searchItem.getByLabel('q (string, required)').fill('cats');
    await searchItem.getByRole('button', { name: 'Review and authorize' }).click();
    const dialog = page.getByRole('dialog', { name: 'Authorize search' });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByText('Fake MCP')).toBeVisible();
    await expect(dialog.getByText('"q": "cats"')).toBeVisible();
    await dialog.getByRole('button', { name: 'Authorize and Call Tool' }).click();
    await expect(page.getByText('Tool call Success: search')).toBeVisible();
    await expect(page.getByText('Search results for')).toBeVisible();
    const history = page.getByRole('list', { name: 'Recent tool calls' });
    await expect(history.getByText('Fake MCP — search — Success')).toBeVisible();
  } finally {
    await session.cleanup();
  }
});

test('MCP tool call cancel dispatches nothing', async ({ page }) => {
  test.setTimeout(300000);
  const session = await openMcp(page);
  try {
    const tools = await registerFakeAndConnect(page);
    const writeItem = tools.locator('li').filter({ hasText: 'Write a note.' });
    await writeItem.getByRole('button', { name: 'Call tool' }).click();
    await writeItem.getByLabel('text (string)').fill('hello');
    await writeItem.getByRole('button', { name: 'Review and authorize' }).click();
    const dialog = page.getByRole('dialog', { name: 'Authorize write' });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByText('Warning: this tool is marked destructive.')).toBeVisible();
    await dialog.getByRole('button', { name: 'Cancel' }).click();
    await expect(page.getByRole('dialog', { name: 'Authorize write' })).toHaveCount(0);
    await expect(page.getByText('Tool call Success')).toHaveCount(0);
    await expect(page.getByText('Tool call Error')).toHaveCount(0);
    await expect(page.getByRole('heading', { name: 'Recent tool calls' })).toHaveCount(0);
  } finally {
    await session.cleanup();
  }
});

test('MCP registration validates the form', async ({ page }) => {
  test.setTimeout(300000);
  const session = await openMcp(page);
  try {
    await page.getByRole('button', { name: 'Register server' }).click();
    await expect(page.getByText('Server name must be 1 to 100')).toBeVisible();
    await expect(page.getByText('No MCP servers configured yet.')).toBeVisible();
  } finally {
    await session.cleanup();
  }
});
