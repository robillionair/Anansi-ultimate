import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M8 WP3 in the real browser application: the pipeline
 * overview shows only real run states. Before any run it
 * reads not started with plain-text sections; during a
 * slow stream it reads generating with stream characters
 * (never per-section progress); after validation it reads
 * complete with working section links; keyboard Tab and
 * Enter move through those links; the saved experiment
 * view shows the same complete overview. No animation
 * exists anywhere in the overview.
 */
function collisionJson(): string {
  return JSON.stringify({
    sourceUnderstanding: 'Two domains.',
    conceptualPrimitives: ['a', 'b', 'c'],
    crossDomainMappings: ['m1', 'm2'],
    combinations: [
      { name: 'one', description: 'first' },
      { name: 'two', description: 'second' },
    ],
    feasibility: {
      hypotheses: ['h'],
      dependencies: [],
      unknownCosts: [],
      risks: ['r'],
      testability: 'testable',
    },
    productization: {
      conceptBrief: 'brief',
      architecture: 'arch',
      roadmap: 'road',
      testPlan: 'plan',
      prototypeProposal: 'proto',
    },
  });
}

test('collider pipeline overview: states, links, keyboard, saved view', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-collider-overview-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };

  // Slow drip: first chunk at once, the rest after 1.5 s,
  // so the generating state is observable mid-stream.
  const fake: Server = createServer((req, res) => {
    let body = '';
    req.on('data', c => { body += c; });
    req.on('end', () => {
      const text = collisionJson();
      const first = text.slice(0, 40);
      const rest = text.slice(40);
      res.writeHead(200, { 'content-type': 'application/x-ndjson' });
      res.write(JSON.stringify({ model: 'llama-x', message: { role: 'assistant', content: first }, done: false }) + '\n');
      setTimeout(() => {
        res.write(JSON.stringify({ model: 'llama-x', message: { role: 'assistant', content: rest }, done: false }) + '\n');
        res.end(JSON.stringify({ model: 'llama-x', message: { role: 'assistant', content: '' }, done: true, prompt_eval_count: 9, eval_count: 42 }) + '\n');
      }, 1500);
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

  try {
    await startApi();
    const created = await call('workspaces', 'create', { name: 'Overview Workspace' });
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
    const home = await call('memories', 'create', {
      workspaceId, kind: 'project',
      draft: { title: 'Project Harbor', body: 'The home project.', privacy: 'private', eventDate: null, validFrom: null, validUntil: null, confidence: null, sourceReferences: [] },
    });
    const homeId = home.body.memory.id as string;
    void homeId;

    await page.goto('/');
    await page.getByLabel('Local API address').fill(pairing.endpoint);
    await page.getByLabel('Session token').fill(pairing.token);
    await page.getByRole('button', { name: 'Connect to core' }).click();
    await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled();
    await page.getByLabel('Existing workspace').selectOption(workspaceId);
    await page.getByRole('button', { name: 'Open workspace', exact: true }).click();
    await page.getByRole('button', { name: 'Projects', exact: true }).click();
    await page.getByRole('button', { name: 'Project Harbor' }).click();
    await expect(page.getByRole('heading', { name: 'Edit project: Project Harbor', exact: true })).toBeVisible();

    const collider = page.locator('section[aria-labelledby="project-collider-heading"]');
    await expect(collider.getByRole('heading', { name: 'LEXI-COLLIDER' })).toBeVisible();
    const overview = collider.locator('section[aria-labelledby="collider-pipeline-heading"]');

    // Not started: plain-text sections, no dead links, no animation.
    await expect(overview.getByText('Not started — press Collide Ideas')).toBeVisible();
    await expect(overview.getByText('Source understanding — Not started')).toBeVisible();
    await expect(overview.locator('a[href^="#collider-"]')).toHaveCount(0);
    await expect(overview.locator('animate')).toHaveCount(0);

    // Generating: slow stream keeps the run observable mid-flight.
    await collider.getByLabel('Beam A').fill('Trading signals');
    await collider.getByLabel('Beam B').fill('Mycelium networks');
    await collider.getByRole('button', { name: 'Collide Ideas' }).click();
    await expect(overview.getByText(/Generating — one provider request is in flight/)).toBeVisible();
    await expect(overview.getByText(/stream characters received/)).toBeVisible();
    await expect(overview.getByText(/no per-section progress/)).toBeVisible();

    // Complete: section links work, and keyboard moves through them.
    await expect(overview.getByText('Complete — all six sections passed validation')).toBeVisible();
    await expect(collider.getByRole('heading', { name: 'Source understanding' })).toBeVisible();
    const combinationsLink = overview.getByRole('link', { name: 'Proposed combinations' });
    await combinationsLink.click();
    await expect(page).toHaveURL(/#collider-combinations-heading/);
    const understandingLink = overview.getByRole('link', { name: 'Source understanding' });
    await understandingLink.click();
    await expect(page).toHaveURL(/#collider-understanding-heading/);
    // Focus without activating: fragment navigation moves the
    // sequential-focus starting point to the target, so Tab
    // after a click would leave the overview entirely.
    await understandingLink.focus();
    await page.keyboard.press('Tab');
    const focused = await page.evaluate(() => document.activeElement?.textContent ?? '');
    expect(focused).toContain('Conceptual primitives');
    await page.keyboard.press('Enter');
    await expect(page).toHaveURL(/#collider-primitives-heading/);

    // Save, then the saved view shows the same complete overview.
    await collider.getByRole('button', { name: 'Save collision as experiment' }).click();
    await expect(collider.getByText(/Saved as artifact .* in this project/)).toBeVisible();
    const artifacts = page.locator('section[aria-labelledby="project-artifacts-heading"]');
    await artifacts.getByRole('button', { name: 'Collider experiment: Trading signals' }).click();
    await expect(page.getByText('Collider experiment — provider ollama, model llama-x.')).toBeVisible();
    // The panel's own overview stays mounted, so scope to the artifact view.
    const savedOverview = page
      .getByRole('region', { name: 'Artifacts' })
      .locator('section[aria-labelledby="collider-pipeline-heading"]');
    await expect(savedOverview.getByText('Complete — all six sections passed validation')).toBeVisible();
    await expect(savedOverview.getByRole('link', { name: 'Proposed combinations' })).toBeVisible();
    // Independent acceptance probe: a saved overview must navigate to its
    // own source/version, rather than the still-mounted live panel.
    const targetInsideSaved = await savedOverview
      .getByRole('link', { name: 'Proposed combinations' })
      .evaluate(link => {
        const href = link.getAttribute('href');
        const target = href ? document.querySelector(href) : null;
        return !!target?.closest('section[aria-labelledby="project-artifacts-heading"]');
      });
    expect(targetInsideSaved, 'saved experiment section link resolves outside its artifact view').toBe(true);

  } finally {
    await stopApi();
    fake.close();
  }
});
