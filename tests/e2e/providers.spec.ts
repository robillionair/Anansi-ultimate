import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import http from 'node:http';
import type { Socket } from 'node:net';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * AI Providers browser workflow (M5 UI slice).
 *
 * Real stack throughout: the built web app, the real API binary,
 * the real core authorization boundary, and a synthetic Ollama
 * server on loopback. Synthetic only — this verifies the
 * application workflow, it does not certify any live provider.
 *
 * The browser session is NOT the owner: configure/authorize are
 * refused in the UI (rejected authorization), the workspace's
 * configuration arrives the way it would from the desktop owner —
 * as the persisted configuration file — across an API restart.
 */
test('provider workflow: rejected authorization, persisted config, dispatch, disclosure, streaming cancel, zero-cost lock, error recovery', async ({ page }) => {
  test.setTimeout(240000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-providers-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };
  let pairingFile = '';

  // --- synthetic Ollama-shaped provider -------------------------
  const sockets = new Set<Socket>();
  const fake = http.createServer((req, res) => {
    if (req.method !== 'POST' || req.url !== '/api/chat') {
      res.writeHead(404).end();
      return;
    }
    let body = '';
    req.on('data', (c) => (body += c));
    req.on('end', () => {
      const parsed = JSON.parse(body) as { stream?: boolean };
      res.writeHead(200, { 'Content-Type': 'application/x-ndjson' });
      if (parsed.stream) {
        let i = 0;
        const timer = setInterval(() => {
          i++;
          if (i <= 12) {
            res.write(
              JSON.stringify({
                model: 'x',
                message: { role: 'assistant', content: `tok${i} ` },
                done: false,
              }) + '\n',
            );
          } else {
            clearInterval(timer);
            res.end(
              JSON.stringify({
                model: 'x',
                message: { role: 'assistant', content: '' },
                done: true,
                prompt_eval_count: 5,
                eval_count: 12,
              }) + '\n',
            );
          }
        }, 200);
        res.on('close', () => clearInterval(timer));
      } else {
        res.end(
          JSON.stringify({
            model: 'x',
            message: { role: 'assistant', content: 'fake unary reply' },
            done: true,
            prompt_eval_count: 5,
            eval_count: 3,
          }) + '\n',
        );
      }
    });
  });
  fake.on('connection', (s) => {
    sockets.add(s);
    s.on('close', () => sockets.delete(s));
  });
  await new Promise<void>((r) => fake.listen(0, '127.0.0.1', r));
  const fakePort = (fake.address() as { port: number }).port;
  const fakeUrl = `http://127.0.0.1:${fakePort}`;

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

  try {
    await startApi();
    const workspace = (await call('workspaces', 'create', { name: 'Providers E2E Workspace' })).workspace;
    await call('workspaces', 'settings', { id: workspace.id, reducedMotion: true, onboardingComplete: true });
    await pairAndOpen(workspace.id);

    // 1. Rejected authorization: a browser session cannot
    // configure — the owner boundary is visible, not silent.
    await page.getByRole('button', { name: 'Save configuration' }).click();
    await expect(page.getByRole('alert', { name: 'Configuration error' })).toContainText('Owner authorization required');

    // 2. The owner (desktop) configuration arrives as the
    // persisted file and survives an API restart.
    await stopApi();
    const configDir = path.join(root, 'storage', 'provider-config');
    fs.mkdirSync(configDir, { recursive: true });
    const modelShape = {
      capabilities: { chat: true, streaming: true, vision: false, coding: true, deepReasoning: false, embeddings: false },
      contextWindow: 8192,
      speed: 'balanced',
    };
    fs.writeFileSync(
      path.join(configDir, `${workspace.id}.json`),
      JSON.stringify({
        providers: [
          {
            config: { provider: 'ollama', baseUrl: fakeUrl, credential: null, enabled: true },
            locality: 'local',
            models: [
              { id: 'llama-free', provider: 'ollama', cost: 'free', ...modelShape },
              { id: 'llama-paid', provider: 'ollama', cost: 'paid', ...modelShape },
            ],
          },
        ],
        budget: { maxRequests: 100, maxTotalTokens: 1000000 },
      }),
    );
    await startApi();
    await pairAndOpen(workspace.id);

    // 3. Unary dispatch + disclosure + usage.
    await page.locator('#dispatch-stream').uncheck();
    await page.locator('#dispatch-model').fill('llama-free');
    await page.locator('#dispatch-prompt').fill('Say hello');
    await page.getByRole('button', { name: 'Send', exact: true }).click();
    await expect(page.getByText('fake unary reply')).toBeVisible();
    await expect(page.getByRole('heading', { name: 'View what ANANSI sent' })).toBeVisible();
    await expect(page.getByText('127.0.0.1', { exact: false }).first()).toBeVisible();
    await expect(page.getByText('Total requests (this session of the app)')).toBeVisible();

    // 4. Streaming with a mid-stream cancel.
    await page.locator('#dispatch-stream').check();
    await page.locator('#dispatch-prompt').fill('Count for me');
    await page.getByRole('button', { name: 'Send', exact: true }).click();
    await expect(page.locator('.provider-output')).toContainText('tok3');
    await page.getByRole('button', { name: 'Cancel stream' }).click();
    await expect(page.getByText('Stream cancelled.')).toBeVisible();

    // 5. Zero-Cost Lock refuses the paid model; with the lock
    // off, the missing permit refuses instead; and authorizing
    // from the browser is itself an owner operation.
    await page.locator('#dispatch-stream').uncheck();
    // Pin the paid model: in 'auto' the router may legitimately
    // substitute the free candidate; chooseModel makes the paid
    // dispatch (and its refusals) deterministic.
    await page.locator('#dispatch-mode').selectOption('chooseModel');
    await page.locator('#dispatch-model').fill('llama-paid');
    await page.locator('#dispatch-prompt').fill('Spend money');
    await page.getByRole('button', { name: 'Send', exact: true }).click();
    await expect(page.getByRole('alert', { name: 'Dispatch error' })).toContainText(/Zero-Cost Lock|CostRefused|fails the active privacy\/cost filters/);
    await page.locator('#dispatch-lock').uncheck();
    await page.getByRole('button', { name: 'Send', exact: true }).click();
    await expect(page.getByRole('alert', { name: 'Dispatch error' })).toContainText(/PaidRequiresPermit|permit/i);
    await page.getByRole('button', { name: 'Authorize paid dispatch for this model' }).click();
    await expect(page.getByRole('alert', { name: 'Authorization error' })).toContainText('Owner authorization required');

    // 6. Error recovery: provider down -> visible error; provider
    // back -> the same workflow succeeds again.
    for (const s of sockets) s.destroy();
    await new Promise<void>((r) => fake.close(() => r()));
    await page.locator('#dispatch-model').fill('llama-free');
    await page.locator('#dispatch-prompt').fill('Are you there?');
    await page.locator('#dispatch-mode').selectOption('auto');
    await page.getByRole('button', { name: 'Send', exact: true }).click();
    await expect(page.getByRole('alert', { name: 'Dispatch error' })).toBeVisible();
    await new Promise<void>((r) => fake.listen(fakePort, '127.0.0.1', r));
    await page.getByRole('button', { name: 'Send', exact: true }).click();
    await expect(page.getByText('fake unary reply')).toBeVisible();
  } finally {
    await stopApi();
    for (const s of sockets) s.destroy();
    fake.close();
    fs.rmSync(root, { recursive: true, force: true });
  }
});
