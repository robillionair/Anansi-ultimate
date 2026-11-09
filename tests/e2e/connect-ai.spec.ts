import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import http from 'node:http';
import type { Socket } from 'node:net';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * Connect AI (API-key setup) browser workflow.
 *
 * Real stack throughout: the built web app, the real API binary,
 * the real core authorization boundary, and synthetic providers on
 * loopback. Synthetic only — this verifies the application
 * workflow, it does not certify any live provider.
 *
 * The browser session is NOT the owner: reference generation and
 * saving are refused with a desktop direction, while the setup
 * status (connected providers, budget, credential stored vs
 * verified) stays readable. The owner side of the add is proven
 * at the API level by the Rust workflow tests; here the add
 * arrives the way it would from the desktop owner — as the
 * persisted configuration file — across an API restart.
 */
test('connect AI: guided flow prefill, browser refusal with desktop direction, restart keeps both providers and budget', async ({ page }) => {
  test.setTimeout(240000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-connect-ai-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };
  let pairingFile = '';

  // --- synthetic providers (hit counters; B must never be called) ---
  const sockets = new Set<Socket>();
  let fakeAHits = 0;
  let fakeBHits = 0;
  function startFake(counter: () => void, reply: (body: string) => string) {
    const server = http.createServer((req, res) => {
      if (req.method !== 'POST') {
        res.writeHead(404).end();
        return;
      }
      let body = '';
      req.on('data', (c) => (body += c));
      req.on('end', () => {
        counter();
        res.writeHead(200, { 'Content-Type': 'application/x-ndjson' });
        res.end(reply(body) + '\n');
      });
    });
    server.on('connection', (s) => {
      sockets.add(s);
      s.on('close', () => sockets.delete(s));
    });
    return server;
  }
  const fakeA = startFake(
    () => {
      fakeAHits += 1;
    },
    () =>
      JSON.stringify({
        model: 'llama-free',
        message: { role: 'assistant', content: 'fake unary reply' },
        done: true,
        prompt_eval_count: 5,
        eval_count: 3,
      }),
  );
  const fakeB = startFake(
    () => {
      fakeBHits += 1;
    },
    () =>
      JSON.stringify({
        choices: [{ message: { role: 'assistant', content: 'fake openai reply' } }],
        usage: { prompt_tokens: 5, completion_tokens: 3, total_tokens: 8 },
      }),
  );
  await new Promise<void>((r) => fakeA.listen(0, '127.0.0.1', r));
  await new Promise<void>((r) => fakeB.listen(0, '127.0.0.1', r));
  const fakeAUrl = `http://127.0.0.1:${(fakeA.address() as { port: number }).port}`;
  const fakeBUrl = `http://127.0.0.1:${(fakeB.address() as { port: number }).port}`;

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
      child!.once('error', reject);
      child!.stdout!.on('data', (data) => {
        output += data.toString();
        const match = output.match(/Pairing file: (.+)\r?\n/);
        if (match) {
          pairingFile = match[1].trim();
          pairing = JSON.parse(fs.readFileSync(pairingFile, 'utf8'));
          clearTimeout(timer);
          resolve();
        }
      });
    });
  }

  async function stopApi() {
    if (child && child.exitCode === null) {
      const ended = new Promise((resolve) => child!.once('exit', resolve));
      child.kill();
      await ended;
    }
    child = undefined;
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
    expect(response.ok).toBe(true);
    return response.json();
  }

  async function pairAndOpen(workspaceId: string) {
    await page.goto('/');
    await page.getByLabel('Local API address').fill(pairing.endpoint);
    await page.getByLabel('Session token').fill(pairing.token);
    await page.getByRole('button', { name: 'Connect to core' }).click();
    await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled();
    await page.getByLabel('Existing workspace').selectOption(workspaceId);
    await page.getByRole('button', { name: 'Open workspace', exact: true }).click();
    await page.getByRole('button', { name: 'AI Providers', exact: true }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'AI providers' })).toBeVisible();
  }

  const modelShape = {
    capabilities: { chat: true, streaming: true, vision: false, coding: true, deepReasoning: false, embeddings: false },
    contextWindow: 8192,
    speed: 'balanced',
  };

  try {
    await startApi();
    const workspace = (await call('workspaces', 'create', { name: 'Connect AI E2E Workspace' })).workspace;
    await call('workspaces', 'settings', { id: workspace.id, reducedMotion: true, onboardingComplete: true });

    // 1. Provider A with a budget arrives as the persisted config
    // (owner-equivalent) and survives an API restart.
    await stopApi();
    const configDir = path.join(root, 'storage', 'provider-config');
    fs.mkdirSync(configDir, { recursive: true });
    fs.writeFileSync(
      path.join(configDir, `${workspace.id}.json`),
      JSON.stringify({
        providers: [
          {
            config: { provider: 'ollama', baseUrl: fakeAUrl, credential: null, enabled: true },
            locality: 'local',
            models: [
              { id: 'llama-free', provider: 'ollama', cost: 'free', ...modelShape },
              { id: 'llama-paid', provider: 'ollama', cost: 'paid', ...modelShape },
            ],
          },
        ],
        budget: { maxRequests: 5, maxTotalTokens: 100000 },
      }),
    );
    await startApi();
    await pairAndOpen(workspace.id);

    // The connected list shows A, the budget, and "not yet verified".
    const connected = page.locator('[aria-labelledby="providers-connected-title"]');
    await expect(connected).toContainText('Ollama (local)');
    await expect(connected).toContainText('max 5 requests');
    await expect(connected).toContainText('not yet verified');
    // Zero-Cost Lock is on by default.
    await expect(page.locator('#dispatch-lock')).toBeChecked();

    // 2. A dispatch verifies the connection in this session.
    await page.locator('#dispatch-stream').uncheck();
    await page.locator('#dispatch-model').fill('llama-free');
    await page.locator('#dispatch-prompt').fill('Say hello');
    await page.getByRole('button', { name: 'Send', exact: true }).click();
    await expect(page.getByText('fake unary reply')).toBeVisible();
    await page.getByRole('button', { name: 'Refresh connected providers' }).click();
    await expect(connected).toContainText('verified in this session');

    // 3. Guided flow as a browser: endpoint prefill, then the
    // credential reference is refused with a desktop direction.
    await page.getByRole('button', { name: 'OpenRouter', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Step 2 — Endpoint' })).toBeVisible();
    await expect(page.locator('#connect-base-url')).toHaveValue('https://openrouter.ai/api/v1');
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Credential setup' })).toBeVisible();
    await page.getByRole('button', { name: 'Generate reference' }).click();
    await expect(page.getByRole('alert', { name: 'Connect AI error' })).toContainText('desktop app');
    // The flow still lets the user continue without a credential
    // and reach review — where saving is refused the same way.
    await page.getByRole('button', { name: 'Continue without a credential' }).click();
    await expect(page.getByRole('heading', { name: 'Step 4 — Choose a model' })).toBeVisible();
    await expect(page.locator('#connect-model-cost-0')).toHaveValue('unknown');
    await page.locator('#connect-model-0').fill('gpt-x');
    await page.getByRole('button', { name: 'Review', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Step 5 — Review and save' })).toBeVisible();
    await expect(page.locator('[aria-labelledby="providers-connect-title"]')).toContainText('gpt-x (unknown)');
    await expect(page.locator('[aria-labelledby="providers-connect-title"]')).toContainText(
      'Your existing budget and other providers are unchanged',
    );
    await page.getByRole('button', { name: 'Save connection', exact: true }).click();
    await expect(page.getByRole('alert', { name: 'Connect AI error' })).toContainText('Owner authorization required');
    await expect(page.getByRole('alert', { name: 'Connect AI error' })).toContainText('still usable');

    // 4. The owner-side add of B arrives as the merged persisted
    // config across a restart: both providers and the budget.
    await stopApi();
    fs.writeFileSync(
      path.join(configDir, `${workspace.id}.json`),
      JSON.stringify({
        providers: [
          {
            config: { provider: 'ollama', baseUrl: fakeAUrl, credential: null, enabled: true },
            locality: 'local',
            models: [
              { id: 'llama-free', provider: 'ollama', cost: 'free', ...modelShape },
              { id: 'llama-paid', provider: 'ollama', cost: 'paid', ...modelShape },
            ],
          },
          {
            config: {
              provider: 'openai',
              baseUrl: fakeBUrl,
              credential: { provider: 'openai', reference: 'openai-seeded-ref' },
              enabled: true,
              customEndpoint: true,
            },
            locality: 'local',
            models: [{ id: 'gpt-x', provider: 'openai', cost: 'unknown', ...modelShape }],
          },
        ],
        budget: { maxRequests: 5, maxTotalTokens: 100000 },
      }),
    );
    await startApi();
    await pairAndOpen(workspace.id);
    await expect(connected).toContainText('Ollama (local)');
    await expect(connected).toContainText('OpenAI');
    await expect(connected).toContainText('gpt-x (unknown)');
    await expect(connected).toContainText('max 5 requests');
    // Credential stored vs verified stay separate: the reference
    // is listed, no key was ever stored from this browser, and
    // the connection is not yet verified.
    await expect(connected).toContainText('openai-seeded-ref — not stored');
    await expect(connected).toContainText('not yet verified');

    // Setup made zero inference calls to B.
    expect(fakeBHits).toBe(0);
    expect(fakeAHits).toBeGreaterThan(0);
  } finally {
    await stopApi();
    for (const s of sockets) s.destroy();
    fakeA.close();
    fakeB.close();
    fs.rmSync(root, { recursive: true, force: true });
  }
});
