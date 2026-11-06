import { test, expect } from '@playwright/test';
import { createServer, type Server } from 'node:http';
import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * SwarmTank v2 Checkpoint 2: AI simulation through the actual application
 * with a synthetic (fake Ollama) provider.
 *
 * The fake answers non-streaming dispatches with a single JSON object
 * carrying a valid agent action (round-robin across the four actions),
 * and reports realistic token usage (800 prompt + 512 completion) so
 * the budget math is exercised. Behaviour is configurable per test:
 * `malformedCall` makes one call return non-JSON, `delayMs` slows
 * every call so cancellation can land mid-run. No real model, no paid
 * call.
 */

const ACTIONS = ['recommend', 'defer', 'request_evidence', 'reject'] as const;

type FakeBehavior = {
  malformedCall: number | null;
  delayMs: number;
};

const behavior: FakeBehavior = { malformedCall: null, delayMs: 0 };

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
        const parsed = JSON.parse(body || '{}') as { stream?: boolean };
        if (parsed.stream) {
          // Streaming callers get NDJSON chunks (not used by the AI
          // dispatch, which is synchronous).
          const content = JSON.stringify({
            action: 'recommend',
            rationale: 'Streaming fallback.',
          });
          res.writeHead(200, { 'content-type': 'application/x-ndjson' });
          res.end(
            JSON.stringify({
              model: 'llama-x',
              message: { role: 'assistant', content },
              done: true,
              prompt_eval_count: 800,
              eval_count: 512,
            }) + '\n',
          );
          return;
        }
        if (behavior.malformedCall === callNumber) {
          res.writeHead(200, { 'content-type': 'application/json' });
          res.end(
            JSON.stringify({
              model: 'llama-x',
              message: { role: 'assistant', content: 'this is not json' },
              done: true,
              prompt_eval_count: 800,
              eval_count: 512,
            }),
          );
          return;
        }
        const action = ACTIONS[(callNumber - 1) % ACTIONS.length];
        const content = JSON.stringify({
          action,
          rationale: `Fake ${action} for call ${callNumber}.`,
        });
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(
          JSON.stringify({
            model: 'llama-x',
            message: { role: 'assistant', content },
            done: true,
            prompt_eval_count: 800,
            eval_count: 512,
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

type Fixture = {
  page: import('@playwright/test').Page;
  workspaceId: string;
  projectId: string;
  fakeHits: () => number;
  cleanup: () => Promise<void>;
};

async function openSwarmtankAi(page: import('@playwright/test').Page): Promise<Fixture> {
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-swarmtank-ai-e2e-'));
  const fake = await startFake();
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };

  async function startApi() {
    child = spawn(
      path.resolve('target/debug/anansi-api' + (process.platform === 'win32' ? '.exe' : '')),
      ['--port', '0', '--pairing-root', path.join(root, 'pairing')],
      {
        windowsHide: true,
        env: { ...process.env, ANANSI_STORAGE_ROOT: path.join(root, 'storage') },
        stdio: ['ignore', 'pipe', 'ignore'],
      },
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

  async function call(group: string, operation: string, input: Record<string, unknown>) {
    const response = await fetch(pairing.endpoint + '/v1/' + group + '/' + operation, {
      method: 'POST',
      headers: {
        Origin: 'http://127.0.0.1:1420',
        Authorization: `Bearer ${pairing.token}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ contractVersion: 1, ...input }),
    });
    return { status: response.status, body: (await response.json()) as Record<string, any> };
  }

  await startApi();
  const created = await call('workspaces', 'create', { name: 'SwarmTank AI Workspace' });
  const workspaceId = created.body.workspace.id as string;
  await call('workspaces', 'settings', { id: workspaceId, reducedMotion: true, onboardingComplete: true });
  fs.mkdirSync(path.join(root, 'storage', 'provider-config'), { recursive: true });
  fs.writeFileSync(
    path.join(root, 'storage', 'provider-config', `${workspaceId}.json`),
    JSON.stringify({
      providers: [
        {
          config: { provider: 'ollama', baseUrl: fake.url, credential: null, enabled: true },
          locality: 'local',
          models: [
            {
              id: 'llama-x',
              provider: 'ollama',
              capabilities: { chat: true, streaming: true, vision: false, coding: true, deepReasoning: false, embeddings: false },
              contextWindow: 8192,
              cost: 'free',
              speed: 'balanced',
            },
          ],
        },
      ],
    }),
  );
  const project = await call('memories', 'create', {
    workspaceId,
    kind: 'project',
    draft: { title: 'Harbor App', body: 'A test project.', privacy: 'private' },
  });
  const projectId = project.body.memory.id as string;

  await page.goto('/');
  await page.getByLabel('Local API address').fill(pairing.endpoint);
  await page.getByLabel('Session token').fill(pairing.token);
  await page.getByRole('button', { name: 'Connect to core' }).click();
  await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled();
  await page.getByLabel('Existing workspace').selectOption(workspaceId);
  await page.getByRole('button', { name: 'Open workspace', exact: true }).click();
  await page.getByRole('button', { name: 'Projects', exact: true }).click();
  await page.getByRole('button', { name: 'Harbor App' }).click();
  await expect(page.getByRole('heading', { name: 'Artifacts', exact: true })).toBeVisible();

  const swarmtank = page.locator('section[aria-labelledby="project-swarmtank-heading"]');
  await expect(swarmtank.getByRole('heading', { name: 'SwarmTank', exact: true })).toBeVisible();
  await swarmtank.getByLabel('SwarmTank description').fill('A scheduling tool for small agencies that automates client reporting.');
  await swarmtank.getByLabel('SwarmTank pricing').fill('$49 per month');
  await swarmtank.getByLabel('SwarmTank monthly price amount').fill('49');
  await swarmtank.getByLabel('SwarmTank segment').fill('small agencies');
  await swarmtank.getByLabel('SwarmTank industry').fill('services');
  // Switch to AI simulation mode.
  await swarmtank.getByRole('radio', { name: /AI simulation/ }).check();
  const ai = swarmtank.locator('section[aria-label="AI simulation"]');
  await expect(ai.getByRole('heading', { name: 'AI simulation' })).toBeVisible();

  return {
    page,
    workspaceId,
    projectId,
    fakeHits: fake.hits,
    cleanup: async () => {
      if (child && child.exitCode === null) {
        const ended = new Promise(resolve => child!.once('exit', resolve));
        child.kill();
        await ended;
      }
      fake.server.close();
    },
  };
}

test('a small AI run completes through the app with a synthetic provider', async ({ page }) => {
  behavior.malformedCall = null;
  behavior.delayMs = 0;
  const fx = await openSwarmtankAi(page);
  try {
    const swarmtank = page.locator('section[aria-labelledby="project-swarmtank-heading"]');
    const ai = swarmtank.locator('section[aria-label="AI simulation"]');
    // The estimated call budget is shown before Start.
    await expect(ai.getByText(/Estimated budget before Start: ollama\/llama-x \(local\), 6 provider calls/)).toBeVisible({ timeout: 15000 });
    const hitsBefore = fx.fakeHits();
    await ai.getByRole('button', { name: 'Start AI simulation' }).click();
    await expect(page.getByText(/AI simulation started: ollama\/llama-x \(local\), 6 calls estimated/)).toBeVisible();
    // The run completes: 3 agents x 2 rounds = 6 calls, all valid.
    await expect(ai.getByText(/AI run — Complete \(6 calls, 7872 tokens\)/)).toBeVisible({ timeout: 60000 });
    expect(fx.fakeHits() - hitsBefore).toBe(6);
    // Every round shows valid agent actions with rationale.
    await expect(ai.getByText(/Fake recommend for call 1\./)).toBeVisible();
    await expect(ai.getByText(/Fake defer for call 2\./)).toBeVisible();
    // The saved AI run reopens with no further provider call.
    await expect(ai.getByText(/Saved AI runs/)).toBeVisible();
    const hitsAfterRun = fx.fakeHits();
    await ai.getByRole('button', { name: 'View AI run' }).first().click();
    const opened = ai.locator('section[aria-label="SwarmTank AI run"]');
    await expect(opened.getByText(/SwarmTank AI run — Complete/)).toBeVisible();
    // The timeline scrubber defaults to the last round; select round 1.
    await opened.getByLabel('AI timeline round').fill('1');
    await expect(opened.getByText(/Fake recommend for call 1\./)).toBeVisible();
    expect(fx.fakeHits()).toBe(hitsAfterRun);
  } finally {
    await fx.cleanup();
  }
});

test('a malformed model output is recorded and the run continues', async ({ page }) => {
  behavior.malformedCall = 2;
  behavior.delayMs = 0;
  const fx = await openSwarmtankAi(page);
  try {
    const swarmtank = page.locator('section[aria-labelledby="project-swarmtank-heading"]');
    const ai = swarmtank.locator('section[aria-label="AI simulation"]');
    await expect(ai.getByText(/Estimated budget before Start/)).toBeVisible({ timeout: 15000 });
    await ai.getByRole('button', { name: 'Start AI simulation' }).click();
    await expect(ai.getByText(/AI run — Complete \(6 calls/)).toBeVisible({ timeout: 60000 });
    // Call 2 was malformed: that agent's round is recorded as failed
    // and visible, and the run continued with the remaining agents.
    await expect(ai.getByText(/Failed\./).first()).toBeVisible();
    await expect(ai.getByText(/Malformed model output/)).toBeVisible();
    await expect(ai.getByText(/Raw output: this is not json/)).toBeVisible();
    await expect(ai.getByText(/Fake recommend for call 1\./)).toBeVisible();
    await expect(ai.getByText(/Fake request_evidence for call 3\./)).toBeVisible();
  } finally {
    await fx.cleanup();
  }
});

test('budget exhaustion ends the run as budget-exhausted and saves it', async ({ page }) => {
  behavior.malformedCall = null;
  behavior.delayMs = 0;
  const fx = await openSwarmtankAi(page);
  try {
    const swarmtank = page.locator('section[aria-labelledby="project-swarmtank-heading"]');
    const ai = swarmtank.locator('section[aria-label="AI simulation"]');
    await expect(ai.getByText(/Estimated budget before Start/)).toBeVisible({ timeout: 15000 });
    // A 4000-token budget covers round 1 (3 calls x 1312) but not round 2.
    await ai.getByLabel('AI max total tokens').fill('4000');
    await ai.getByRole('button', { name: 'Start AI simulation' }).click();
    await expect(ai.getByText(/AI run — Budget exhausted/)).toBeVisible({ timeout: 60000 });
    await expect(ai.getByText(/The token budget was exhausted/)).toBeVisible();
    // Round 1 completed (3 calls); the run is saved, not lost.
    await expect(ai.getByText(/Saved AI runs/)).toBeVisible();
    await ai.getByRole('button', { name: 'View AI run' }).first().click();
    await expect(ai.getByText(/SwarmTank AI run — Budget exhausted/)).toBeVisible();
  } finally {
    await fx.cleanup();
  }
});

test('cancellation saves the completed rounds', async ({ page }) => {
  behavior.malformedCall = null;
  behavior.delayMs = 3000;
  const fx = await openSwarmtankAi(page);
  try {
    const swarmtank = page.locator('section[aria-labelledby="project-swarmtank-heading"]');
    const ai = swarmtank.locator('section[aria-label="AI simulation"]');
    await expect(ai.getByText(/Estimated budget before Start/)).toBeVisible({ timeout: 15000 });
    await ai.getByRole('button', { name: 'Start AI simulation' }).click();
    await expect(ai.getByText(/AI run — Running/)).toBeVisible({ timeout: 30000 });
    // Let a couple of calls land, then cancel.
    await page.waitForTimeout(2500);
    await ai.getByRole('button', { name: 'Cancel AI simulation' }).click();
    await expect(ai.getByText(/AI run — Cancelled/)).toBeVisible({ timeout: 60000 });
    await expect(ai.getByText(/Saved AI runs/)).toBeVisible();
    await ai.getByRole('button', { name: 'View AI run' }).first().click();
    await expect(ai.getByText(/SwarmTank AI run — Cancelled/)).toBeVisible();
  } finally {
    await fx.cleanup();
  }
});
