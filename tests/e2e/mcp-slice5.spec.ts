import { test, expect } from '@playwright/test';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import path from 'node:path';
import { startSession } from './collider-three-helpers.ts';

test.beforeEach(async ({ page }) => {
  page.setDefaultTimeout(8000);
});

type JsonRpc = { jsonrpc: string; id?: number | null; result?: any; error?: { code: number; message: string } };

/** Minimal NDJSON stdio client, mirroring mcp-server.spec.ts. */
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

function spawnServer(session: Awaited<ReturnType<typeof startSession>>) {
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
  return { server, client: new StdioClient(server) };
}

async function initialize(client: StdioClient, session: Awaited<ReturnType<typeof startSession>>) {
  client.send({ jsonrpc: '2.0', id: 1, method: 'initialize', params: { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '0' }, _meta: { anansiMcpToken: session.pairingToken } } });
  const response = await client.next();
  expect(response.result.protocolVersion).toBe('2025-11-25');
  expect(response.result.serverInfo.name).toBe('anansi-server');
}

async function callTool(client: StdioClient, id: number, name: string, args: Record<string, unknown>) {
  client.send({ jsonrpc: '2.0', id, method: 'tools/call', params: { name, arguments: args } });
  return client.next();
}

async function listArtifacts(session: Awaited<ReturnType<typeof startSession>>, projectId: string) {
  const listed = await session.call('artifacts', 'list', { workspaceId: session.workspaceId, projectId });
  expect(listed.status).toBe(200);
  return listed.body.artifacts as Array<{ artifactId: string; type: string; title: string; privacy: string }>;
}

async function readArtifact(session: Awaited<ReturnType<typeof startSession>>, artifactId: string) {
  const read = await session.call('artifacts', 'read', { workspaceId: session.workspaceId, artifactId });
  expect(read.status).toBe(200);
  return read.body.body as string;
}

test('MCP server: ideas.create writes a bound idea artifact into the project registry', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, true);
  try {
    const projectId = await createMemory(session, 'project', 'Slice5 ideas project', 'Project body.', 'standard');
    const { server, client } = spawnServer(session);
    try {
      await initialize(client, session);
      // tools/list now carries seven tools, ideas.create last.
      client.send({ jsonrpc: '2.0', id: 2, method: 'tools/list', params: {} });
      let response = await client.next();
      expect(response.result.tools.map((t: any) => t.name)).toEqual(['memory.search', 'memory.get', 'notes.search', 'projects.search', 'experiments.list', 'products.get', 'ideas.create']);
      // Tagged idea: artifact id reported, tags echoed.
      response = await callTool(client, 3, 'ideas.create', { projectId, title: 'Solar kiosk', description: 'A kiosk idea.', tags: ['solar', 'kiosk'] });
      expect(response.result.isError).toBe(false);
      const artifactId = response.result.structuredContent.artifactId as string;
      expect(typeof artifactId).toBe('string');
      expect(response.result.structuredContent.type).toBe('idea');
      expect(response.result.structuredContent.tags).toEqual(['solar', 'kiosk']);
      // The registry proves persistence: the idea is bound to this
      // project, standard like its project, body verbatim plus tags.
      const artifacts = await listArtifacts(session, projectId);
      const tagged = artifacts.find((a) => a.artifactId === artifactId);
      expect(tagged).toBeDefined();
      expect(tagged!.type).toBe('idea');
      expect(tagged!.title).toBe('Solar kiosk');
      expect(tagged!.privacy).toBe('standard');
      expect(await readArtifact(session, artifactId)).toBe('A kiosk idea.\n\nTags: solar, kiosk');
      // Untagged idea: body is exactly the description.
      response = await callTool(client, 4, 'ideas.create', { projectId, title: 'Untagged idea', description: 'No tags here.' });
      expect(response.result.isError).toBe(false);
      const untaggedId = response.result.structuredContent.artifactId as string;
      expect(await readArtifact(session, untaggedId)).toBe('No tags here.');
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

test('MCP server: ideas.create refuses invalid targets and arguments, creating nothing', async ({ page }) => {
  test.setTimeout(300000);
  const session = await startSession(page, true);
  try {
    const privateId = await createMemory(session, 'project', 'Slice5 hidden project', 'Project body.', 'private');
    const { server, client } = spawnServer(session);
    try {
      await initialize(client, session);
      // Private project, unknown project: identical not-found.
      let response = await callTool(client, 2, 'ideas.create', { projectId: privateId, title: 'Hidden idea', description: 'Must not land.' });
      expect(response.result.isError).toBe(true);
      expect(response.result.content[0].text).toBe('Project not found.');
      response = await callTool(client, 3, 'ideas.create', { projectId: '77777777777777777777777777777777', title: 'Unknown', description: 'Must not land.' });
      expect(response.result.isError).toBe(true);
      expect(response.result.content[0].text).toBe('Project not found.');
      // Malformed arguments: all -32602.
      const bad = [
        { projectId: 'not-hex', title: 'Bad id', description: 'No.' },
        { projectId: privateId, description: 'No title.' },
        { projectId: privateId, title: '   ', description: 'Blank title.' },
        { projectId: privateId, title: 't'.repeat(121), description: 'Overlong title.' },
        { projectId: privateId, title: 'No desc.' },
        { projectId: privateId, title: 'Blank desc.', description: '  ' },
        { projectId: privateId, title: 'Bad tags', description: 'Tags not an array.', tags: 'solar' },
      ];
      let id = 4;
      for (const args of bad) {
        response = await callTool(client, id++, 'ideas.create', args);
        expect(response.error?.code).toBe(-32602);
      }
      // Nothing was created anywhere: the private project holds no
      // artifacts at all.
      expect(await listArtifacts(session, privateId)).toEqual([]);
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
