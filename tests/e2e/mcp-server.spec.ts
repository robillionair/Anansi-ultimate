import { test, expect } from '@playwright/test';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
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

test('MCP server: pairing auth, shareable tools, kind filter, stage metadata', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, true);
  try {
    const noteId = await createMemory(session, 'note', 'MCP server probe note', 'Probe body text.', 'standard');
    const privateId = await createMemory(session, 'note', 'MCP server probe private note', 'Private body text.', 'private');
    await createMemory(session, 'project', 'MCP server probe project', 'Probe project body.', 'standard');

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
      // 1. tools/list before any initialize is refused.
      client.send({ jsonrpc: '2.0', id: 1, method: 'tools/list', params: {} });
      let response = await client.next();
      expect(response.error?.code).toBe(-32001);

      // 2. initialize with a wrong token is refused, then the live
      // pairing token succeeds.
      client.send({ jsonrpc: '2.0', id: 2, method: 'initialize', params: { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '0' }, _meta: { anansiMcpToken: '0'.repeat(64) } } });
      response = await client.next();
      expect(response.error?.code).toBe(-32001);
      client.send({ jsonrpc: '2.0', id: 3, method: 'initialize', params: { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '0' }, _meta: { anansiMcpToken: session.pairingToken } } });
      response = await client.next();
      expect(response.result.protocolVersion).toBe('2025-11-25');
      expect(response.result.serverInfo.name).toBe('anansi-server');
      expect(response.result.capabilities.tools).toBeDefined();

      // 3. tools/list returns exactly the seven approved tools.
      client.send({ jsonrpc: '2.0', id: 4, method: 'tools/list', params: {} });
      response = await client.next();
      expect(response.result.tools.map((t: any) => t.name)).toEqual(['memory.search', 'memory.get', 'notes.search', 'projects.search', 'experiments.list', 'products.get', 'ideas.create']);

      // 4. memory.search finds the standard note, never the private one,
      // and never returns bodies.
      client.send({ jsonrpc: '2.0', id: 5, method: 'tools/call', params: { name: 'memory.search', arguments: { query: 'probe' } } });
      response = await client.next();
      expect(response.result.isError).toBe(false);
      const hits = response.result.structuredContent as Array<Record<string, unknown>>;
      expect(hits.some((h) => h.title === 'MCP server probe note')).toBe(true);
      expect(hits.some((h) => h.title === 'MCP server probe private note')).toBe(false);
      expect(hits.every((h) => !('body' in h))).toBe(true);

      // 5. Kind filter: notes only, then a kind with no hits.
      client.send({ jsonrpc: '2.0', id: 6, method: 'tools/call', params: { name: 'memory.search', arguments: { query: 'probe', kind: 'note' } } });
      response = await client.next();
      expect((response.result.structuredContent as Array<Record<string, unknown>>).some((h) => h.title === 'MCP server probe note')).toBe(true);
      expect((response.result.structuredContent as Array<Record<string, unknown>>).some((h) => h.title === 'MCP server probe project')).toBe(false);
      client.send({ jsonrpc: '2.0', id: 7, method: 'tools/call', params: { name: 'memory.search', arguments: { query: 'probe', kind: 'task' } } });
      response = await client.next();
      expect(response.result.structuredContent).toEqual([]);

      // 6. memory.get returns the body for the standard note; the private
      // note is indistinguishable from a missing id.
      client.send({ jsonrpc: '2.0', id: 8, method: 'tools/call', params: { name: 'memory.get', arguments: { id: noteId } } });
      response = await client.next();
      expect(response.result.isError).toBe(false);
      expect(response.result.structuredContent.body).toBe('Probe body text.');
      client.send({ jsonrpc: '2.0', id: 9, method: 'tools/call', params: { name: 'memory.get', arguments: { id: privateId } } });
      response = await client.next();
      expect(response.result.isError).toBe(true);
      expect(response.result.content[0].text).toBe('Memory not found.');

      // 7. projects.search finds the standard probe project with stage
      // metadata, and never the private Harbor App project.
      client.send({ jsonrpc: '2.0', id: 10, method: 'tools/call', params: { name: 'projects.search', arguments: { query: 'probe' } } });
      response = await client.next();
      expect(response.result.isError).toBe(false);
      const projects = response.result.structuredContent as Array<Record<string, any>>;
      const probe = projects.find((p) => p.title === 'MCP server probe project');
      if (!probe) throw new Error('probe project missing from projects.search');
      expect(probe.stage).toBe('discover');
      expect(probe.stages.length).toBe(7);
      expect(projects.some((p) => p.title === 'Harbor App')).toBe(false);

      // 7b. A title-matching query for the private Harbor App project
      // returns nothing: privacy, not the query, excludes it.
      client.send({ jsonrpc: '2.0', id: 11, method: 'tools/call', params: { name: 'projects.search', arguments: { query: 'Harbor' } } });
      response = await client.next();
      expect(response.result.structuredContent).toEqual([]);

      // 8. Malformed line, unknown tool, unknown id.
      client.sendRaw('not json');
      response = await client.next();
      expect(response.error?.code).toBe(-32700);
      expect(response.id).toBeNull();
      client.send({ jsonrpc: '2.0', id: 12, method: 'tools/call', params: { name: 'ideas.create', arguments: {} } });
      response = await client.next();
      expect(response.error?.code).toBe(-32602);
      client.send({ jsonrpc: '2.0', id: 13, method: 'tools/call', params: { name: 'memory.get', arguments: { id: 'ffffffffffffffffffffffffffffffff' } } });
      response = await client.next();
      expect(response.result.isError).toBe(true);
      expect(response.result.content[0].text).toBe('Memory not found.');

      // 9. Re-initialize under the stateless revision is accepted.
      client.send({ jsonrpc: '2.0', id: 14, method: 'initialize', params: { protocolVersion: '2026-07-28', capabilities: {}, clientInfo: { name: 'e2e', version: '0' }, _meta: { anansiMcpToken: session.pairingToken } } });
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

test('MCP server: refuses to start without a live pairing session', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, true);
  try {
    const emptyPairing = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-mcp-no-pairing-'));
    const binary = path.resolve('target/debug/anansi-api' + (process.platform === 'win32' ? '.exe' : ''));
    const server = spawn(
      binary,
      ['--mcp-server', '--workspace-id', session.workspaceId, '--pairing-root', emptyPairing],
      {
        windowsHide: true,
        env: { ...process.env, ANANSI_STORAGE_ROOT: session.storageRoot },
        stdio: ['pipe', 'pipe', 'pipe'],
      },
    );
    const code = await new Promise<number | null>((resolve) => server.once('exit', resolve));
    expect(code).not.toBe(0);
  } finally {
    await session.cleanup();
  }
});
