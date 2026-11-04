import { test, expect } from '@playwright/test';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import path from 'node:path';
import { startSession } from './collider-three-helpers.ts';

test.beforeEach(async ({ page }) => {
  page.setDefaultTimeout(8000);
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
    // Drain stderr so a chatty child can never block on a full pipe.
    child.stderr.on('data', () => {});
  }

  send(value: unknown) {
    this.child.stdin.write(JSON.stringify(value) + '\n');
  }

  sendRaw(text: string) {
    this.child.stdin.write(text + '\n');
  }

  async next(): Promise<JsonRpc> {
    const line = this.queue.length > 0
      ? this.queue.shift()!
      : await new Promise<string>((resolve) => this.waiters.push(resolve));
    return JSON.parse(line) as JsonRpc;
  }
}

const TOKEN = 'slice3-e2e-token';

test('MCP server: stdio handshake, tools, seeded data, auth', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, true);
  try {
    const created = await session.call('memories', 'create', {
      workspaceId: session.workspaceId,
      kind: 'note',
      draft: {
        title: 'MCP server probe note', body: 'Probe body text.',
        privacy: 'private', eventDate: null, validFrom: null, validUntil: null,
        confidence: null, sourceReferences: [],
      },
    });
    expect(created.status).toBe(200);
    const noteId = created.body.memory.id as string;

    const binary = path.resolve('target/debug/anansi-api' + (process.platform === 'win32' ? '.exe' : ''));
    const server = spawn(
      binary,
      ['--mcp-server', '--workspace-id', session.workspaceId],
      {
        windowsHide: true,
        env: { ...process.env, ANANSI_STORAGE_ROOT: session.storageRoot, ANANSI_MCP_SERVER_TOKEN: TOKEN },
        stdio: ['pipe', 'pipe', 'pipe'],
      },
    );
    const client = new StdioClient(server);
    try {
      // 1. tools/list before any initialize is refused.
      client.send({ jsonrpc: '2.0', id: 1, method: 'tools/list', params: {} });
      let response = await client.next();
      expect(response.error?.code).toBe(-32001);

      // 2. initialize without the token is refused, then with it succeeds.
      client.send({ jsonrpc: '2.0', id: 2, method: 'initialize', params: { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '0' } } });
      response = await client.next();
      expect(response.error?.code).toBe(-32001);
      client.send({ jsonrpc: '2.0', id: 3, method: 'initialize', params: { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '0' }, _meta: { anansiMcpToken: TOKEN } } });
      response = await client.next();
      expect(response.result.protocolVersion).toBe('2025-11-25');
      expect(response.result.serverInfo.name).toBe('ANANSI');
      expect(response.result.capabilities.tools).toBeDefined();

      // 3. tools/list returns exactly the three approved tools.
      client.send({ jsonrpc: '2.0', id: 4, method: 'tools/list', params: {} });
      response = await client.next();
      expect(response.result.tools.map((t: any) => t.name)).toEqual(['memory.search', 'memory.get', 'projects.search']);

      // 4. Seeded memory.search finds the probe note (title, no body).
      client.send({ jsonrpc: '2.0', id: 5, method: 'tools/call', params: { name: 'memory.search', arguments: { query: 'probe' } } });
      response = await client.next();
      expect(response.result.isError).toBe(false);
      const hits = response.result.structuredContent as Array<Record<string, unknown>>;
      expect(hits.some((h) => h.title === 'MCP server probe note')).toBe(true);
      expect(hits.every((h) => !('body' in h))).toBe(true);

      // 5. memory.get returns the body.
      client.send({ jsonrpc: '2.0', id: 6, method: 'tools/call', params: { name: 'memory.get', arguments: { id: noteId } } });
      response = await client.next();
      expect(response.result.isError).toBe(false);
      expect(response.result.structuredContent.body).toBe('Probe body text.');

      // 6. projects.search finds the harness-seeded Harbor App project.
      client.send({ jsonrpc: '2.0', id: 7, method: 'tools/call', params: { name: 'projects.search', arguments: { query: 'Harbor' } } });
      response = await client.next();
      expect(response.result.isError).toBe(false);
      expect((response.result.structuredContent as Array<Record<string, unknown>>).some((p) => p.title === 'Harbor App')).toBe(true);

      // 7. Malformed line, unknown tool, unknown id.
      client.sendRaw('not json');
      response = await client.next();
      expect(response.error?.code).toBe(-32700);
      expect(response.id).toBeNull();
      client.send({ jsonrpc: '2.0', id: 8, method: 'tools/call', params: { name: 'ideas.create', arguments: {} } });
      response = await client.next();
      expect(response.error?.code).toBe(-32602);
      client.send({ jsonrpc: '2.0', id: 9, method: 'tools/call', params: { name: 'memory.get', arguments: { id: 'ffffffffffffffffffffffffffffffff' } } });
      response = await client.next();
      expect(response.result.isError).toBe(true);
      expect(response.result.content[0].text).toBe('Memory not found.');

      // 8. Re-initialize under the stateless revision is accepted.
      client.send({ jsonrpc: '2.0', id: 10, method: 'initialize', params: { protocolVersion: '2026-07-28', capabilities: {}, clientInfo: { name: 'e2e', version: '0' }, _meta: { anansiMcpToken: TOKEN } } });
      response = await client.next();
      expect(response.result.protocolVersion).toBe('2026-07-28');
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

test('MCP server: refuses to start without a token', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, true);
  try {
    const binary = path.resolve('target/debug/anansi-api' + (process.platform === 'win32' ? '.exe' : ''));
    const env: NodeJS.ProcessEnv = { ...process.env, ANANSI_STORAGE_ROOT: session.storageRoot };
    delete env.ANANSI_MCP_SERVER_TOKEN;
    const server = spawn(
      binary,
      ['--mcp-server', '--workspace-id', session.workspaceId],
      { windowsHide: true, env, stdio: ['pipe', 'pipe', 'pipe'] },
    );
    const code = await new Promise<number | null>((resolve) => server.once('exit', resolve));
    expect(code).not.toBe(0);
  } finally {
    await session.cleanup();
  }
});
