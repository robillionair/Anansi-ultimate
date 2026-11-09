import { expect, type Page } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createHash } from 'node:crypto';
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

function extractPromptFromText(content: string): { question: string; history: string } | null {
  const questionMatch = content.match(/\n\nQuestion: ([\s\S]*?)\n\nAnswer with the exact value/);
  if (!questionMatch) return null;
  const history = content.slice('History:\n'.length, content.indexOf('\n\nQuestion: '));
  return { question: questionMatch[1], history };
}

/** The verbatim-extractor answer for one prompt text. */
function verbatimReply(promptText: string): string {
  const prompt = extractPromptFromText(promptText);
  if (!prompt) return 'UNKNOWN';
  const expected = behavior.expected.get(fakeAnswerKey(prompt.question, prompt.history));
  if (expected !== undefined && expected !== null && prompt.history.includes(expected)) {
    return expected;
  }
  return 'UNKNOWN';
}

/**
 * Mirrors the approval hash the UI and the server compute over the
 * reviewed inputs (design §2 D2): canonical JSON of
 * {cases:[{caseId,candidateHash,questions}],provider,model,cloud,
 * maxTotalTokens}, SHA-256 hex. The UI (SomniaCompaction.tsx) and the
 * server (somnia_compaction_run.rs) must produce the identical string
 * for the same inputs; this mirror is the e2e cross-check.
 */
export function computeApprovalHash(input: {
  cases: Array<{ caseId: string; candidateHash: string; questions: Array<{ id: string; text: string; expectedAnswer: string | null }> }>;
  provider: string;
  model: string;
  cloud: boolean;
  maxTotalTokens: number;
}): string {
  const canonical = JSON.stringify({
    cases: input.cases.map(entry => ({
      caseId: entry.caseId,
      candidateHash: entry.candidateHash,
      questions: entry.questions.map(q => ({ id: q.id, text: q.text, expectedAnswer: q.expectedAnswer })),
    })),
    provider: input.provider,
    model: input.model,
    cloud: input.cloud,
    maxTotalTokens: input.maxTotalTokens,
  });
  return createHash('sha256').update(canonical, 'utf8').digest('hex');
}

async function startFake(): Promise<{ server: Server; url: string; hits: () => number; models: () => string[] }> {
  let hits = 0;
  const models: string[] = [];
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
        const parsed = JSON.parse(body || '{}') as { stream?: boolean; model?: string; messages?: Array<{ content?: string }> };
        if (parsed.model) models.push(parsed.model);
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
        const reply = verbatimReply(parsed.messages?.[0]?.content ?? '');
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
  return { server, url, hits: () => hits, models: () => [...models] };
}

/**
 * OpenAI-compatible fake for the cloud path: it answers
 * POST /chat/completions with choices[0].message.content, the shape
 * the OpenAI-compatible adapter (which serves openrouter) parses.
 * The endpoint is loopback, so the provider config declares locality
 * Local (enforce_derived_locality), while the compaction module
 * derives Cloud from the catalog id 'openrouter' — the same split
 * the production code has. No real cloud endpoint is contacted.
 */
async function startCloudFake(): Promise<{ server: Server; url: string; hits: () => number; models: () => string[] }> {
  let hits = 0;
  const models: string[] = [];
  const server = createServer((req, res) => {
    let body = '';
    req.on('data', c => {
      body += c;
    });
    req.on('end', () => {
      hits += 1;
      const respond = () => {
        if (res.destroyed || res.writableEnded) return;
        const parsed = JSON.parse(body || '{}') as { model?: string; messages?: Array<{ content?: string }> };
        if (parsed.model) models.push(parsed.model);
        const content = parsed.messages?.[0]?.content ?? '';
        const reply = verbatimReply(content);
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(
          JSON.stringify({
            id: 'chatcmpl-fake',
            object: 'chat.completion',
            created: 0,
            model: parsed.model ?? 'free-cloud-model',
            choices: [{ index: 0, message: { role: 'assistant', content: reply }, finish_reason: 'stop' }],
            usage: { prompt_tokens: 12, completion_tokens: 4, total_tokens: 16 },
          }),
        );
      };
      if (behavior.delayMs > 0) setTimeout(respond, behavior.delayMs);
      else respond();
    });
  });
  await new Promise<void>(r => server.listen(0, '127.0.0.1', r));
  const url = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
  return { server, url, hits: () => hits, models: () => [...models] };
}

export type CompactionSession = {
  page: Page;
  workspaceId: string;
  projectId: string;
  storageRoot: string;
  fakeHits: () => number;
  /** Models the Ollama fake received, in call order (pinning proof). */
  fakeModels: () => string[];
  cloudFakeHits: () => number;
  /** Models the cloud fake received, in call order (pinning proof). */
  cloudFakeModels: () => string[];
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
  options?: {
    liveEndpoint?: string;
    liveModel?: string;
    alternateModel?: string;
    /** A paid ollama model (cost 'paid') for the free-only refusal tests. */
    paidModel?: string;
    /** An openrouter model served by the OpenAI-compatible fake. */
    cloudModel?: { id: string; cost: 'free' | 'paid' | 'unknown' };
  },
): Promise<CompactionSession> {
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-somnia-compaction-e2e-'));
  const live = options?.liveEndpoint !== undefined;
  // In live mode the provider config points at the authorized endpoint
  // and the fake is never started; every dispatch is a real model call.
  const fake = live ? null : await startFake();
  const cloudFake = live || !options?.cloudModel ? null : await startCloudFake();
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
  const ollamaModels: Array<{ id: string; cost: string }> = [
    { id: live ? liveModel : 'llama-x', cost: 'free' },
    ...(options?.alternateModel ? [{ id: options.alternateModel, cost: 'free' }] : []),
    ...(options?.paidModel ? [{ id: options.paidModel, cost: 'paid' }] : []),
  ];
  const providers: Array<Record<string, unknown>> = [{
    config: { provider: 'ollama', baseUrl: live ? options!.liveEndpoint : fake!.url, credential: null, enabled: true },
    locality: 'local',
    models: ollamaModels.map(entry => ({
      id: entry.id, provider: 'ollama',
      capabilities: { chat: true, streaming: true, vision: false, coding: true, deepReasoning: false, embeddings: false },
      contextWindow: 8192, cost: entry.cost, speed: 'balanced',
    })),
  }];
  if (options?.cloudModel && cloudFake) {
    // The endpoint is loopback, so the config must declare Local
    // (enforce_derived_locality); the compaction module derives
    // Cloud from the catalog id 'openrouter'.
    providers.push({
      config: { provider: 'openrouter', baseUrl: cloudFake.url, credential: null, enabled: true },
      locality: 'local',
      models: [{
        id: options.cloudModel.id, provider: 'openrouter',
        capabilities: { chat: true, streaming: true, vision: false, coding: true, deepReasoning: false, embeddings: false },
        contextWindow: 8192, cost: options.cloudModel.cost, speed: 'balanced',
      }],
    });
  }
  fs.writeFileSync(
    path.join(root, 'storage', 'provider-config', `${workspaceId}.json`),
    JSON.stringify({
      providers,
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
    fakeModels: () => (fake ? fake.models() : []),
    cloudFakeHits: () => (cloudFake ? cloudFake.hits() : 0),
    cloudFakeModels: () => (cloudFake ? cloudFake.models() : []),
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
      if (cloudFake) {
        cloudFake.server.closeAllConnections();
        await new Promise<void>(r => cloudFake.server.close(() => r()));
      }
    },
  };
}
