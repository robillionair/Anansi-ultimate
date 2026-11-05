import { test, expect } from '@playwright/test';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
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
  return list;
}

test('MCP client: resources are listed and read only on explicit request, for review only', async ({ page }) => {
  test.setTimeout(300000);
  const session = await openMcp(page);
  try {
    const list = await registerFakeAndConnect(page);
    // Nothing is fetched before the click: no resource content yet.
    await expect(page.getByText('# Guide')).toHaveCount(0);
    await expect(page.getByText('shown here for your review only')).toBeVisible();
    await page.getByRole('button', { name: 'List resources' }).click();
    const resources = page.getByRole('list', { name: 'Resources from Fake MCP' });
    await expect(resources.getByText('Guide', { exact: true })).toBeVisible();
    await expect(resources.getByText('file:///guide.md')).toBeVisible();
    const guide = resources.locator('li', { hasText: 'Guide' });
    await guide.getByRole('button', { name: 'Read' }).click();
    await expect(page.getByText('# Guide')).toBeVisible();
    await expect(page.getByText('Review me.')).toBeVisible();
    await list.getByRole('button', { name: 'Remove' }).click();
    await expect(page.getByText('MCP server removed.')).toBeVisible();
  } finally {
    await session.cleanup();
  }
});

test('MCP client: prompt templates are listed and rendered only on explicit request', async ({ page }) => {
  test.setTimeout(300000);
  const session = await openMcp(page);
  try {
    const list = await registerFakeAndConnect(page);
    await page.getByRole('button', { name: 'List prompts' }).click();
    const prompts = page.getByRole('list', { name: 'Prompt templates from Fake MCP' });
    await expect(prompts.getByText('summarize', { exact: true })).toBeVisible();
    await expect(prompts.getByText('greet', { exact: true })).toBeVisible();
    const summarize = prompts.locator('li', { hasText: 'summarize' }).first();
    // Missing required argument is refused before any fetch.
    await summarize.getByRole('button', { name: 'Get prompt' }).click();
    await expect(summarize.getByText('Missing required argument: text')).toBeVisible();
    await summarize.getByLabel('text (required)').fill('hello world');
    await summarize.getByRole('button', { name: 'Get prompt' }).click();
    await expect(summarize.getByText('Summary of "hello world".')).toBeVisible();
    await list.getByRole('button', { name: 'Remove' }).click();
    await expect(page.getByText('MCP server removed.')).toBeVisible();
  } finally {
    await session.cleanup();
  }
});

type JsonRpc = { jsonrpc: string; id?: number | null; result?: any; error?: { code: number; message: string } };

/** Minimal NDJSON stdio client: queues complete lines from the child. */
class StdioClient {
  private buffer = '';
  private queue: string[] = [];
  private waiters: Array<(line: string) => void> = [];

  constructor(private child: ChildProcessWithoutNullStreams) {
    child.stdout.on('data', (data: Buffer) => {
      this.buffer += data.toString();
      let index: number;
      while ((index = this.buffer.indexOf('\n')) >= 0) {
        const line = this.buffer.slice(0, index).trim();
        this.buffer = this.buffer.slice(index + 1);
        if (line === '') continue;
        const waiter = this.waiters.shift();
        if (waiter) waiter(line);
        else this.queue.push(line);
      }
    });
    child.stderr.on('data', () => {});
  }

  send(value: unknown) {
    this.child.stdin.write(JSON.stringify(value) + '\n');
  }

  async next(): Promise<JsonRpc> {
    const line = this.queue.length > 0
      ? this.queue.shift()!
      : await new Promise<string>((resolve) => this.waiters.push(resolve));
    return JSON.parse(line) as JsonRpc;
  }
}

async function createMemory(session: Awaited<ReturnType<typeof startSession>>, kind: string, title: string, body: string, privacy: string) {
  const created = await session.call('memories', 'create', {
    workspaceId: session.workspaceId,
    kind,
    draft: {
      title, body, privacy, eventDate: null, validFrom: null, validUntil: null,
      confidence: null, sourceReferences: [],
    },
  });
  expect(created.status).toBe(200);
  return created.body.memory.id as string;
}

async function registerArtifact(session: Awaited<ReturnType<typeof startSession>>, projectId: string, type: string, title: string, privacy: string) {
  const registered = await session.call('artifacts', 'register', {
    workspaceId: session.workspaceId,
    projectId,
    type,
    title,
    body: 'Artifact body.',
    privacy,
  });
  expect(registered.status).toBe(200);
}

test('MCP server: notes.search, experiments.list, products.get with strict privacy', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, true);
  try {
    const projectId = await createMemory(session, 'project', 'MCP slice4 probe project', 'Probe project body.', 'standard');
    const privateProjectId = await createMemory(session, 'project', 'MCP slice4 probe private project', 'Private project body.', 'private');
    await createMemory(session, 'note', 'MCP slice4 probe note', 'Probe body text.', 'standard');
    await createMemory(session, 'note', 'MCP slice4 probe private note', 'Private body text.', 'private');
    await registerArtifact(session, projectId, 'experiment', 'Slice4 collision', 'standard');
    await registerArtifact(session, projectId, 'experiment', 'Slice4 hidden collision', 'private');
    await registerArtifact(session, projectId, 'deliverable', 'Slice4 build', 'standard');
    await registerArtifact(session, projectId, 'scenario', 'Slice4 swarm branch', 'standard');

    const binary = path.resolve('target/debug/anansi-api' + (process.platform === 'win32' ? '.exe' : ''));
    const server = spawn(
      binary,
      ['--mcp-server', '--workspace-id', session.workspaceId, '--pairing-root', session.pairingRoot],
      {
        windowsHide: true,
        env: { ...process.env, ANANSI_STORAGE_ROOT: session.storageRoot },
        stdio: ['pipe', 'pipe', 'pipe'],
      },
    );
    const client = new StdioClient(server);
    try {
      client.send({ jsonrpc: '2.0', id: 1, method: 'initialize', params: { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '0' }, _meta: { anansiMcpToken: session.pairingToken } } });
      let response = await client.next();
      expect(response.result.serverInfo.name).toBe('anansi-server');

      // tools/list now returns six tools in the documented order.
      client.send({ jsonrpc: '2.0', id: 2, method: 'tools/list', params: {} });
      response = await client.next();
      expect(response.result.tools.map((t: any) => t.name)).toEqual(['memory.search', 'memory.get', 'notes.search', 'projects.search', 'experiments.list', 'products.get', 'ideas.create']);

      // notes.search: kind fixed to note — the project row never
      // appears, the private note never appears, no bodies.
      client.send({ jsonrpc: '2.0', id: 3, method: 'tools/call', params: { name: 'notes.search', arguments: { query: 'probe' } } });
      response = await client.next();
      expect(response.result.isError).toBe(false);
      const notes = response.result.structuredContent as Array<Record<string, unknown>>;
      expect(notes.some((h) => h.title === 'MCP slice4 probe note')).toBe(true);
      expect(notes.some((h) => h.title === 'MCP slice4 probe private note')).toBe(false);
      expect(notes.some((h) => h.title === 'MCP slice4 probe project')).toBe(false);
      expect(notes.every((h) => !('body' in h))).toBe(true);

      // experiments.list: only the standard experiment; private
      // experiment, deliverable, and scenario are excluded.
      client.send({ jsonrpc: '2.0', id: 4, method: 'tools/call', params: { name: 'experiments.list', arguments: { projectId } } });
      response = await client.next();
      expect(response.result.isError).toBe(false);
      const experiments = response.result.structuredContent.experiments as Array<Record<string, unknown>>;
      expect(experiments.map((e) => e.title)).toEqual(['Slice4 collision']);
      expect(experiments.every((e) => !('body' in e))).toBe(true);
      expect(response.result.structuredContent.projectTitle).toBe('MCP slice4 probe project');

      // products.get: only the deliverable; scenario is not a Forge
      // product artifact.
      client.send({ jsonrpc: '2.0', id: 5, method: 'tools/call', params: { name: 'products.get', arguments: { projectId } } });
      response = await client.next();
      expect(response.result.isError).toBe(false);
      const products = response.result.structuredContent.products as Array<Record<string, unknown>>;
      expect(products.map((e) => e.title)).toEqual(['Slice4 build']);

      // Private, unknown, and malformed project ids.
      client.send({ jsonrpc: '2.0', id: 6, method: 'tools/call', params: { name: 'experiments.list', arguments: { projectId: privateProjectId } } });
      response = await client.next();
      expect(response.result.isError).toBe(true);
      expect(response.result.content[0].text).toBe('Project not found.');
      client.send({ jsonrpc: '2.0', id: 7, method: 'tools/call', params: { name: 'experiments.list', arguments: { projectId: '77777777777777777777777777777777' } } });
      response = await client.next();
      expect(response.result.isError).toBe(true);
      expect(response.result.content[0].text).toBe('Project not found.');
      client.send({ jsonrpc: '2.0', id: 8, method: 'tools/call', params: { name: 'experiments.list', arguments: { projectId: 'not-hex' } } });
      response = await client.next();
      expect(response.error?.code).toBe(-32602);

      // The write tool stays refused.
      client.send({ jsonrpc: '2.0', id: 9, method: 'tools/call', params: { name: 'ideas.create', arguments: {} } });
      response = await client.next();
      expect(response.error?.code).toBe(-32602);
    } finally {
      if (server.exitCode === null) {
        const ended = new Promise((resolve) => server.once('exit', resolve));
        server.kill();
        await ended;
      }
    }
  } finally {
    await session.cleanup();
  }
});
