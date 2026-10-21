import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M6 Slice 2 in the real browser application: project-scoped
 * idea generation (SS21-22) over the M5 provider substrate
 * with a synthetic loopback provider.
 *
 * Open project -> fill the S22 brief -> explicitly approve a
 * context memory -> generate 6 concepts -> 3 shown, expand
 * for all -> inspect the disclosure -> save as an immutable
 * artifact -> restart -> reopen -> the saved set renders as
 * concepts again.
 */
function ideaJson(count: number): string {
  const concepts = Array.from({ length: count }, (_, i) => ({
    name: `Concept ${i}`,
    problem: 'A problem worth solving.',
    targetCustomer: 'Independent cafes',
    solution: 'A focused tool.',
    whyUseful: 'It saves time weekly.',
    differentiationHypothesis: 'Hypothesis: incumbents ignore this niche.',
    revenueModel: 'Subscription.',
    engineeringComplexity: 'medium',
    implementationCost: 'about 3-5 weeks for one developer',
    keyRisks: ['Adoption may be slow'],
    requiredValidation: 'Interview five customers.',
    minimumViableVersion: 'One workflow end to end.',
  }));
  return JSON.stringify({ concepts });
}

test('idea generation: project -> explicit context -> generate -> 3 + expand -> disclosure -> save -> restart -> reopen', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-ideas-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };

  // Synthetic streaming provider that also records what it
  // was sent (context discipline is asserted on the wire).
  const sentBodies: string[] = [];
  const text = ideaJson(6);
  const third = Math.ceil(text.length / 3);
  const chunks = [text.slice(0, third), text.slice(third, third * 2), text.slice(third * 2)];
  const fake: Server = createServer((req, res) => {
    let body = '';
    req.on('data', c => { body += c; });
    req.on('end', () => {
      sentBodies.push(body);
      res.writeHead(200, { 'content-type': 'application/x-ndjson' });
      for (const chunk of chunks) {
        res.write(JSON.stringify({ model: 'llama-x', message: { role: 'assistant', content: chunk }, done: false }) + '\n');
      }
      res.end(JSON.stringify({ model: 'llama-x', message: { role: 'assistant', content: '' }, done: true, prompt_eval_count: 9, eval_count: 42 }) + '\n');
    });
  });
  await new Promise<void>(r => fake.listen(0, '127.0.0.1', r));
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
    return { status: response.status, body: await response.json() as Record<string, any> };
  }

  async function pairAndOpen(workspaceId: string) {
    await page.goto('/');
    await page.getByLabel('Local API address').fill(pairing.endpoint);
    await page.getByLabel('Session token').fill(pairing.token);
    await page.getByRole('button', { name: 'Connect to core' }).click();
    await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled();
    await page.getByLabel('Existing workspace').selectOption(workspaceId);
    await page.getByRole('button', { name: 'Open workspace', exact: true }).click();
    await page.getByRole('button', { name: 'Projects' }).click();
  }

  try {
    await startApi();
    const created = await call('workspaces', 'create', { name: 'Ideas Workspace' });
    const workspaceId = created.body.workspace.id as string;
    await call('workspaces', 'settings', { id: workspaceId, reducedMotion: true, onboardingComplete: true });
    await call('memories', 'create', {
      workspaceId,
      draft: {
        title: 'Field notes', body: 'The approved e2e context phrase.',
        privacy: 'private', eventDate: null, validFrom: null, validUntil: null,
        confidence: null, sourceReferences: [],
      },
    });
    // Owner-side provider configuration via the persisted file
    // (the desktop-equivalent step; browser configuration is
    // owner-gated by design and covered elsewhere).
    fs.mkdirSync(path.join(root, 'storage', 'provider-config'), { recursive: true });
    fs.writeFileSync(path.join(root, 'storage', 'provider-config', `${workspaceId}.json`), JSON.stringify({
      providers: [{
        config: { provider: 'ollama', baseUrl: fakeUrl, credential: null, enabled: true },
        locality: 'local',
        models: [{
          id: 'llama-x', provider: 'ollama',
          capabilities: { chat: true, streaming: true, vision: false, coding: true, deepReasoning: false, embeddings: false },
          contextWindow: 8192, cost: 'free', speed: 'balanced',
        }],
      }],
      budget: { maxRequests: 50, maxTotalTokens: 500000 },
    }));

    await pairAndOpen(workspaceId);
    await page.getByRole('button', { name: 'New project' }).click();
    await page.getByLabel('Project title').fill('Harbor App');
    await page.getByLabel('Description & objectives').fill('A test project.');
    await page.getByRole('button', { name: 'Save project' }).click();
    await expect(page.getByRole('heading', { name: 'Idea generation' })).toBeVisible();

    await page.getByLabel('Industry or topic').fill('harbor logistics');
    await page.getByLabel('Intended customer').fill('marina operators');
    await page.getByLabel('Available budget').fill('small');
    await page.getByLabel('Preferred technology').fill('rust');
    await page.getByLabel('Business or personal objective').fill('find a product');
    await page.getByLabel(/Field notes/).check();

    await page.getByRole('button', { name: 'Generate product ideas' }).click();
    await expect(page.getByText('1. Concept 0')).toBeVisible();
    await expect(page.getByText('3. Concept 2')).toBeVisible();
    await expect(page.getByText('6. Concept 5')).toHaveCount(0);
    await page.getByRole('button', { name: 'Show all 6 concepts' }).click();
    await expect(page.getByText('6. Concept 5')).toBeVisible();
    expect(sentBodies.join('')).toContain('approved e2e context phrase');
    await expect(page.getByRole('heading', { name: 'View what ANANSI sent' })).toBeVisible();

    await page.getByRole('button', { name: 'Explore' }).first().click();
    await expect(page.getByText(/Differentiation hypothesis:/)).toBeVisible();

    await page.getByRole('button', { name: 'Save idea set as artifact' }).click();
    await expect(page.getByText(/Saved as artifact/)).toBeVisible();
    await expect(page.getByRole('button', { name: /Idea set: harbor logistics/ })).toBeVisible();

    // Restart: the saved set survives as a registry artifact
    // and renders as concepts, not raw JSON.
    await stopApi();
    await startApi();
    await pairAndOpen(workspaceId);
    await page.getByRole('button', { name: /Harbor App/ }).first().click();
    await page.getByRole('button', { name: /Idea set: harbor logistics/ }).click();
    await expect(page.getByText(/Model-generated hypotheses/)).toBeVisible();
    await expect(page.getByText('1. Concept 0')).toBeVisible();
  } finally {
    await stopApi();
    fake.close();
  }
});
