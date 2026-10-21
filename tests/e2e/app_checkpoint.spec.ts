import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import http from 'node:http';
import type { Socket } from 'node:net';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * Integrated application checkpoint on the merged M2–M5 tree.
 *
 * One continuous browser session against the real API binary:
 * open a workspace, create + edit a memory and find it via
 * search, then use the M5 provider workflow (configure arrives
 * as the owner's persisted configuration, the desktop-side
 * step), dispatch, stream + cancel, inspect disclosure/usage,
 * restart the API, and dispatch again from the persisted
 * configuration. The provider is a synthetic loopback server:
 * this verifies application integration, NOT real-model
 * behavior.
 */
test('integrated checkpoint: workspace, memory lifecycle + search, provider stream/cancel/disclosure, restart persistence', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-checkpoint-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };

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
            res.write(JSON.stringify({ model: 'x', message: { role: 'assistant', content: `tok${i} ` }, done: false }) + '\n');
          } else {
            clearInterval(timer);
            res.end(JSON.stringify({ model: 'x', message: { role: 'assistant', content: '' }, done: true, prompt_eval_count: 5, eval_count: 12 }) + '\n');
          }
        }, 200);
        res.on('close', () => clearInterval(timer));
      } else {
        res.end(JSON.stringify({ model: 'x', message: { role: 'assistant', content: 'checkpoint reply' }, done: true, prompt_eval_count: 5, eval_count: 3 }) + '\n');
      }
    });
  });
  fake.on('connection', (s) => {
    sockets.add(s);
    s.on('close', () => sockets.delete(s));
  });
  await new Promise<void>((r) => fake.listen(0, '127.0.0.1', r));
  const fakeUrl = `http://127.0.0.1:${(fake.address() as { port: number }).port}`;

  async function startApi() {
    child = spawn(
      path.resolve('target/debug/anansi-api' + (process.platform === 'win32' ? '.exe' : '')),
      ['--port', '0', '--pairing-root', path.join(root, 'pairing')],
      { windowsHide: true, env: { ...process.env, ANANSI_STORAGE_ROOT: path.join(root, 'storage') }, stdio: ['ignore', 'pipe', 'ignore'] },
    );
    await new Promise<void>((resolve, reject) => {
      let output = '';
      const timer = setTimeout(() => reject(Error('API startup timeout')), 20000);
      child!.stdout!.on('data', (data) => {
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
      headers: { Origin: 'http://127.0.0.1:1420', Authorization: `Bearer ${pairing.token}`, 'Content-Type': 'application/json' },
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
    await expect(page.getByRole('heading', { level: 1, name: 'Checkpoint Workspace' })).toBeVisible();
  }

  try {
    await startApi();
    const workspace = (await call('workspaces', 'create', { name: 'Checkpoint Workspace' })).workspace;
    await call('workspaces', 'settings', { id: workspace.id, reducedMotion: true, onboardingComplete: true });

    // --- Memory lifecycle, entirely in the UI -------------------
    await pairAndOpen(workspace.id);
    await page.getByRole('button', { name: 'Memory', exact: true }).click();
    await page.getByRole('button', { name: 'New memory' }).click();
    await page.getByLabel('Memory title').fill('Harbor migration notes');
    await page.getByLabel('Memory text').fill('The harbor migration uses codeword quixotic7741 for the cutover.');
    await page.getByRole('button', { name: 'Save memory' }).click();
    await expect(page.getByRole('button', { name: 'Harbor migration notes' })).toBeVisible();
    // Edit: open it, revise the body, save again.
    await page.getByRole('button', { name: 'Harbor migration notes' }).click();
    await page.getByLabel('Memory text').fill('The harbor migration uses codeword quixotic7741 and lantern2210 for the cutover.');
    await page.getByRole('button', { name: 'Save memory' }).click();
    await expect(page.getByText(/revision 2/i).first()).toBeVisible();
    // Search finds the edited body term.
    await page.getByRole('button', { name: 'Search', exact: true }).click();
    await page.getByRole('radio', { name: 'Full-body search (includes Markdown text)' }).check();
    await page.getByLabel('Search query').fill('lantern2210');
    await page.locator('section[aria-labelledby="search-title"]').getByRole('button', { name: 'Search knowledge', exact: true }).click();
    await expect(page.getByText('Harbor migration notes').first()).toBeVisible();

    // --- Provider configuration arrives owner-side, persists ---
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
        providers: [{
          config: { provider: 'ollama', baseUrl: fakeUrl, credential: null, enabled: true },
          locality: 'local',
          models: [{ id: 'llama-free', provider: 'ollama', cost: 'free', ...modelShape }],
        }],
        budget: { maxRequests: 100, maxTotalTokens: 1000000 },
      }),
    );
    await startApi();
    await pairAndOpen(workspace.id);
    await page.getByRole('button', { name: 'AI Providers', exact: true }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'AI providers' })).toBeVisible();

    // Unary dispatch + disclosure + usage.
    await page.locator('#dispatch-stream').uncheck();
    await page.locator('#dispatch-model').fill('llama-free');
    await page.locator('#dispatch-prompt').fill('Summarize the harbor notes');
    await page.getByRole('button', { name: 'Send', exact: true }).click();
    await expect(page.getByText('checkpoint reply')).toBeVisible();
    await expect(page.getByRole('heading', { name: 'View what ANANSI sent' })).toBeVisible();

    // Streamed answer, cancelled mid-stream, terminal state shown.
    await page.locator('#dispatch-stream').check();
    await page.locator('#dispatch-prompt').fill('Count for me');
    await page.getByRole('button', { name: 'Send', exact: true }).click();
    await expect(page.locator('.provider-output')).toContainText('tok3');
    await page.getByRole('button', { name: 'Cancel stream' }).click();
    await expect(page.getByText('Stream cancelled.')).toBeVisible();

    // Restart: configuration persists; dispatch works with no
    // reconfiguration step.
    await stopApi();
    await startApi();
    await pairAndOpen(workspace.id);
    await page.getByRole('button', { name: 'AI Providers', exact: true }).click();
    await page.locator('#dispatch-stream').uncheck();
    await page.locator('#dispatch-model').fill('llama-free');
    await page.locator('#dispatch-prompt').fill('After restart');
    await page.getByRole('button', { name: 'Send', exact: true }).click();
    await expect(page.getByText('checkpoint reply')).toBeVisible();
  } finally {
    await stopApi();
    for (const s of sockets) s.destroy();
    fake.close();
    fs.rmSync(root, { recursive: true, force: true });
  }
});
