import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

// VIS-02 screenshot evidence at deviceScaleFactor 2:
// the visualization canvas at high DPI, paused and
// under reduced motion, with contrasting labels. The
// screenshots are opt-in (VIZ_SHOT_DIR); without it
// this spec still asserts the workflow renders.
test.use({ deviceScaleFactor: 2 });

function collisionJson(): string {
  return JSON.stringify({
    sourceUnderstanding: 'Both beams describe bounded local work with feedback.',
    conceptualPrimitives: ['Distributed signaling', 'Adaptive routing', 'Local feedback'],
    crossDomainMappings: ['Kitchen timing mirrors orbital timing.', 'Signal speed contrasts with patience.'],
    combinations: [
      { name: 'Mycelial order router', description: 'A routing layer that scores venues by local congestion signals. Hypothesis only.' },
      { name: 'Graceful-degradation mesh', description: 'A trade mesh that reroutes through redundant paths. Hypothesis only.' },
    ],
    feasibility: {
      hypotheses: ['Local signals can rank venues.'],
      dependencies: [],
      unknownCosts: [],
      risks: ['Stale signals could route into worse fills.'],
      testability: 'Testable in simulation first.',
    },
    productization: {
      conceptBrief: 'A routing advisor. Hypothesis, not a product claim.',
      architecture: 'Signal ingest, local scoring, policy engine.',
      roadmap: 'Phase 1: simulator. Phase 2: replay evaluation.',
      testPlan: 'Unit-test the scorer on synthetic gradients.',
      prototypeProposal: 'A paper prototype, reviewed by a human first.',
    },
  });
}

test('collider visualization screenshots at high DPI', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-collider-viz-shot-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };

  const fake: Server = createServer((req, res) => {
    let body = '';
    req.on('data', c => { body += c; });
    req.on('end', () => {
      const text = collisionJson();
      const third = Math.ceil(text.length / 3);
      const chunks = [text.slice(0, third), text.slice(third, third * 2), text.slice(third * 2)];
      res.writeHead(200, { 'content-type': 'application/x-ndjson' });
      for (const chunk of chunks) {
        res.write(JSON.stringify({ model: 'llama-x', message: { role: 'assistant', content: chunk }, done: false }) + '\n');
      }
      res.end(JSON.stringify({ model: 'llama-x', message: { role: 'assistant', content: '' }, done: true, prompt_eval_count: 9, eval_count: 42 }) + '\n');
    });
  });
  await new Promise<void>(r => fake.listen(0, '127.0.0.1', r));
  const fakeUrl = `http://127.0.0.1:${(fake.address() as { port: number }).port}`;
  const shotDir = process.env.VIZ_SHOT_DIR;
  async function shot(loc: ReturnType<typeof page.locator>, name: string) {
    if (shotDir) await loc.screenshot({ path: path.join(shotDir, name) });
  }

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

  try {
    await startApi();
    const created = await call('workspaces', 'create', { name: 'Collider Workspace' });
    const workspaceId = created.body.workspace.id as string;
    await call('workspaces', 'settings', { id: workspaceId, reducedMotion: true, onboardingComplete: true });
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
    await call('memories', 'create', {
      workspaceId,
      kind: 'project',
      draft: {
        title: 'Harbor App', body: 'A test project.',
        privacy: 'private', eventDate: null, validFrom: null, validUntil: null,
        confidence: null, sourceReferences: [],
      },
    });

    await page.goto('/');
    await page.getByLabel('Local API address').fill(pairing.endpoint);
    await page.getByLabel('Session token').fill(pairing.token);
    await page.getByRole('button', { name: 'Connect to core' }).click();
    await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled();
    await page.getByLabel('Existing workspace').selectOption(workspaceId);
    await page.getByRole('button', { name: 'Open workspace', exact: true }).click();
    await page.getByRole('button', { name: 'Projects', exact: true }).click();
    await page.getByRole('button', { name: 'Harbor App' }).click();
    await expect(page.getByRole('heading', { name: 'Artifacts' })).toBeVisible();

    const collider = page.locator('section[aria-labelledby="project-collider-heading"]');
    const artifacts = page.locator('section[aria-labelledby="project-artifacts-heading"]');
    await collider.getByLabel('Beam A', { exact: true }).fill('Shot beam about kitchens.');
    await collider.getByLabel('Beam B', { exact: true }).fill('Shot beam about orbitals.');
    await collider.getByRole('button', { name: 'Collide Ideas' }).click();
    await expect(collider.getByRole('heading', { name: 'Source understanding' })).toBeVisible();
    await collider.getByRole('button', { name: 'Save collision as experiment' }).click();
    await expect(collider.getByText(/Saved as artifact .* in this project/)).toBeVisible();

    await artifacts.getByRole('button', { name: /Collider experiment: Shot beam about kitchens/ }).click();
    const viz = artifacts.locator('section[aria-labelledby="collider-visualization-heading"]');
    await expect(viz.getByRole('heading', { name: 'Experiment visualization' })).toBeVisible();
    const canvas = viz.locator('canvas');
    const pause = viz.getByRole('button', { name: 'Pause motion' });
    if (await pause.count() > 0) await pause.click();
    await shot(canvas, 'viz-hidpi-paused.png');

    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.getByRole('button', { name: 'Projects', exact: true }).click();
    await page.getByRole('button', { name: 'Harbor App' }).click();
    await artifacts.getByRole('button', { name: /Collider experiment: Shot beam about kitchens/ }).click();
    await expect(viz.getByText('Motion reduced — static view.')).toBeVisible();
    await shot(canvas, 'viz-hidpi-reduced-motion.png');
  } finally {
    await stopApi();
    await new Promise<void>(r => fake.close(() => r()));
  }
});
