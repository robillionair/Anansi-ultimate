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
    await expect(page.getByText('Tools are listed, never run')).toBeVisible();
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
