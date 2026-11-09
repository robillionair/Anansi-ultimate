import { expect, type Page } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * SOMNIA compaction Checkpoint 2: e2e harness with a synthetic (fake
 * Ollama) provider that behaves like a verbatim extractor.
 *
 * The fake answers each dispatch from the prompt it was given: it looks
 * up the expected answer for (question text + the record ids in that
 * arm's history), replies with it when the history contains it verbatim,
 * and replies UNKNOWN otherwise (or when the history should not state
 * it). Usage is 12 input + 4 output = 16 tokens per call, matching the
 * shared harness. Behaviour is configurable per test: `delayMs` slows
 * every call so cancellation can land mid-run. No real model, no paid
 * call. Accuracy through this fake is synthetic by construction.
 */

type FakeBehavior = {
  delayMs: number;
  expected: Map<string, string | null>;
  wrongModelCall: number | null;
};

const behavior: FakeBehavior = { delayMs: 0, expected: new Map(), wrongModelCall: null };

/** Every test resets the fake first: the behavior is module-level. */
export function resetFakeBehavior(): void {
  behavior.delayMs = 0;
  behavior.expected = new Map();
  behavior.wrongModelCall = null;
}

export function setFakeExpectedAnswers(entries: Array<[string, string | null]>): void {
  behavior.expected = new Map(entries);
}

export function setFakeDelayMs(ms: number): void {
  behavior.delayMs = ms;
}

/** Makes one dispatch (by call number) report a different model. */
export function setFakeWrongModelCall(callNumber: number | null): void {
  behavior.wrongModelCall = callNumber;
}

/**
 * Mirrors the server's render_history exactly (canonical order by key,
 * source, revision, id; one `[id] key (source, rev n): text` line per
 * record). The fake keys its answer map on the rendered history text so
 * two cases that share a question string can never collide.
 */
export function renderHistory(records: Array<Record<string, unknown>>): string {
  const sorted = [...records].sort((a, b) => {
    const key = (record: Record<string, unknown>, field: string) => String(record[field] ?? '');
    for (const field of ['key', 'source']) {
      const cmp = key(a, field).localeCompare(key(b, field));
      if (cmp !== 0) return cmp;
    }
    const rev = Number(a.revision ?? 0) - Number(b.revision ?? 0);
    if (rev !== 0) return rev;
    return key(a, 'id').localeCompare(key(b, 'id'));
  });
  return sorted
    .map(record => `[${record.id}] ${record.key} (${record.source}, rev ${record.revision}): ${record.text}`)
    .join('\n');
}

export function fakeAnswerKey(question: string, history: string): string {
  return `${question}\n${history}`;
}

function extractPrompt(body: string): { question: string; history: string } | null {
  let parsed: { messages?: Array<{ content?: string }> };
  try {
    parsed = JSON.parse(body);
  } catch {
    return null;
  }
  const content = parsed.messages?.[0]?.content ?? '';
  const questionMatch = content.match(/\n\nQuestion: ([\s\S]*?)\n\nAnswer with the exact value/);
  if (!questionMatch) return null;
  const history = content.slice('History:\n'.length, content.indexOf('\n\nQuestion: '));
  return { question: questionMatch[1], history };
}

async function startFake(): Promise<{ server: Server; url: string; hits: () => number }> {
  let hits = 0;
  const server = createServer((req, res) => {
    let body = '';
    req.on('data', c => {
      body += c;
    });
    req.on('end', () => {
      hits += 1;
      const callNumber = hits;
      const respond = () => {
        if (res.destroyed || res.writableEnded) return;
        const parsed = JSON.parse(body || '{}') as { stream?: boolean; model?: string };
        if (parsed.stream) {
          res.writeHead(200, { 'content-type': 'application/x-ndjson' });
          res.end(
            JSON.stringify({
              model: 'llama-x',
              message: { role: 'assistant', content: 'UNKNOWN' },
              done: true,
              prompt_eval_count: 12,
              eval_count: 4,
            }) + '\n',
          );
          return;
        }
        const prompt = extractPrompt(body);
        let reply = 'UNKNOWN';
        if (prompt) {
          const expected = behavior.expected.get(fakeAnswerKey(prompt.question, prompt.history));
          if (expected !== undefined && expected !== null && prompt.history.includes(expected)) {
            reply = expected;
          }
        }
        // A wrong-model call reports a different model in the same
        // shape the real provider would; the run must refuse the pair.
        const model = behavior.wrongModelCall === callNumber ? 'other-model' : parsed.model;
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(
          JSON.stringify({
            model,
            message: { role: 'assistant', content: reply },
            done: true,
            prompt_eval_count: 12,
            eval_count: 4,
          }),
        );
      };
      if (behavior.delayMs > 0) setTimeout(respond, behavior.delayMs);
      else respond();
    });
  });
  await new Promise<void>(r => server.listen(0, '127.0.0.1', r));
  const url = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
  return { server, url, hits: () => hits };
}

export type CompactionSession = {
  page: Page;
  workspaceId: string;
  projectId: string;
  storageRoot: string;
  fakeHits: () => number;
  restartApi: () => Promise<void>;
  repair: () => Promise<void>;
  call: (group: string, operation: string, input: Record<string, unknown>) => Promise<{ status: number; body: Record<string, any> }>;
  openProject: () => Promise<void>;
  openProjectByTitle: (title: string) => Promise<void>;
  createProject: (title: string) => Promise<string>;
  panel: ReturnType<Page['locator']>;
  cleanup: () => Promise<void>;
};

export async function startCompactionSession(
  page: Page,
  options?: { liveEndpoint?: string; liveModel?: string; alternateModel?: string },
): Promise<CompactionSession> {
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-somnia-compaction-e2e-'));
  const live = options?.liveEndpoint !== undefined;
  // In live mode the provider config points at the authorized endpoint
  // and the fake is never started; every dispatch is a real model call.
  const fake = live ? null : await startFake();
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };

  async function startApi() {
    child = spawn(
      path.resolve('target/debug/anansi-api' + (process.platform === 'win32' ? '.exe' : '')),
      ['--port', '0', '--pairing-root', path.join(root, 'pairing')],
      { windowsHide: true, env: { ...process.env, ANANSI_STORAGE_ROOT: path.join(root, 'storage') }, stdio: ['ignore', 'pipe', 'ignore'] },
    );
    await new Promise<void>((resolve, reject) => {
      let output = '';
      const timer = setTimeout(() => reject(Error('API startup timeout')), 20000);
      child!.stdout!.on('data', data => {
        output += data.toString();
        const match = output.match(/Pairing file: (.+)\r?\n/);
        if (match) {
          pairing = JSON.parse(fs.readFileSync(match[1].trim(), 'utf8'));
          clearTimeout(timer);
          resolve();
        }
      });
    });
  }

  async function restartApi() {
    if (child && child.exitCode === null && child.signalCode === null) {
      const exited = new Promise<void>((resolve, reject) => {
        const timer = setTimeout(() => reject(Error('API did not exit after SIGKILL')), 10000);
        child!.once('exit', () => {
          clearTimeout(timer);
          resolve();
        });
      });
      child.kill('SIGKILL');
      await exited;
    }
    await startApi();
  }

  async function call(group: string, operation: string, input: Record<string, unknown>) {
    const response = await fetch(pairing.endpoint + '/v1/' + group + '/' + operation, {
      method: 'POST',
      headers: { Origin: 'http://127.0.0.1:1420', Authorization: `Bearer ${pairing.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ contractVersion: 1, ...input }),
    });
    return { status: response.status, body: (await response.json()) as Record<string, any> };
  }

  await startApi();
  const created = await call('workspaces', 'create', { name: 'SOMNIA Compaction Workspace' });
  const workspaceId = created.body.workspace.id as string;
  await call('workspaces', 'settings', { id: workspaceId, reducedMotion: true, onboardingComplete: true });
  fs.mkdirSync(path.join(root, 'storage', 'provider-config'), { recursive: true });
  const liveModel = options?.liveModel ?? 'qwen2.5:3b';
  fs.writeFileSync(
    path.join(root, 'storage', 'provider-config', `${workspaceId}.json`),
    JSON.stringify({
      providers: [{
        config: { provider: 'ollama', baseUrl: live ? options!.liveEndpoint : fake!.url, credential: null, enabled: true },
        locality: 'local',
        models: [live ? liveModel : 'llama-x', ...(options?.alternateModel ? [options.alternateModel] : [])].map(id => ({
          id, provider: 'ollama',
          capabilities: { chat: true, streaming: true, vision: false, coding: true, deepReasoning: false, embeddings: false },
          contextWindow: 8192, cost: 'free', speed: 'balanced',
        })),
      }],
      budget: { maxRequests: 500, maxTotalTokens: 500000 },
    }),
  );
  const project = await call('memories', 'create', {
    workspaceId,
    kind: 'project',
    draft: { title: 'Harbor App', body: 'A test project.', privacy: 'private', eventDate: null, validFrom: null, validUntil: null, confidence: null, sourceReferences: [] },
  });
  const projectId = project.body.memory.id as string;

  async function openWorkspace(ws: string) {
    await page.goto('/');
    await page.getByLabel('Local API address').fill(pairing.endpoint);
    await page.getByLabel('Session token').fill(pairing.token);
    await page.getByRole('button', { name: 'Connect to core' }).click();
    await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled();
    await page.getByLabel('Existing workspace').selectOption(ws);
    await page.getByRole('button', { name: 'Open workspace', exact: true }).click();
  }

  async function openProjectByTitle(title: string) {
    await page.getByRole('button', { name: 'Projects', exact: true }).click();
    // The list is fetched when the view opens; a project created through
    // the API after that needs an explicit refresh.
    await page.getByRole('button', { name: 'Refresh projects' }).click();
    await page.getByRole('button', { name: title }).click();
    await expect(page.getByRole('heading', { name: 'Artifacts', exact: true })).toBeVisible();
  }

  await openWorkspace(workspaceId);
  await openProjectByTitle('Harbor App');

  const panel = page.getByRole('region', { name: 'SOMNIA automatic compaction' });
  await expect(panel).toBeVisible();

  return {
    page,
    workspaceId,
    projectId,
    storageRoot: path.join(root, 'storage'),
    fakeHits: () => (fake ? fake.hits() : 0),
    restartApi,
    // A restart mints a fresh pairing token, so the browser must pair
    // again against the current API child before it can open a project.
    repair: async () => {
      await openWorkspace(workspaceId);
      await openProjectByTitle('Harbor App');
    },
    call,
    openProject: () => openProjectByTitle('Harbor App'),
    openProjectByTitle,
    createProject: async (title: string) => {
      const createdProject = await call('memories', 'create', {
        workspaceId,
        kind: 'project',
        draft: { title, body: 'A second test project.', privacy: 'private', eventDate: null, validFrom: null, validUntil: null, confidence: null, sourceReferences: [] },
      });
      return createdProject.body.memory.id as string;
    },
    panel,
    cleanup: async () => {
      if (child && child.exitCode === null && child.signalCode === null) {
        child.kill();
        await Promise.race([
          new Promise(resolve => child!.once('exit', resolve)),
          new Promise(resolve => setTimeout(() => {
            child!.kill('SIGKILL');
            resolve(null);
          }, 10000)),
        ]);
      }
      if (fake) {
        fake.server.closeAllConnections();
        await new Promise<void>(r => fake.server.close(() => r()));
      }
    },
  };
}
