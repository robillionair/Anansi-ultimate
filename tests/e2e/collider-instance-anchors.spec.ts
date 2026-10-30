import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * R3 (M8 overnight Windows repair): with the live panel and a saved
 * experiment mounted at the same time, each overview's links must
 * resolve by DOM query to its own result — not merely carry a
 * scoped-looking href — and keyboard navigation from the saved
 * overview must land on the saved heading. The live instance keeps
 * the legacy ids; the saved instance is scoped by artifact id and
 * version.
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

test('collider instance anchors: live and saved targets are distinct by DOM and keyboard', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-collider-anchors-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };

  const fake: Server = createServer((req, res) => {
    let body = '';
    req.on('data', c => { body += c; });
    req.on('end', () => {
      void body;
      res.writeHead(200, { 'content-type': 'application/x-ndjson' });
      res.end(JSON.stringify({ model: 'llama-x', message: { role: 'assistant', content: collisionJson() }, done: true, prompt_eval_count: 9, eval_count: 42 }) + '\n');
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
    const created = await call('workspaces', 'create', { name: 'Anchor Workspace' });
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
      workspaceId, kind: 'project',
      draft: { title: 'Project Harbor', body: 'The home project.', privacy: 'private', eventDate: null, validFrom: null, validUntil: null, confidence: null, sourceReferences: [] },
    });

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
    const liveOverview = collider.locator('section[aria-labelledby="collider-pipeline-heading"]');

    // First collision, saved as the saved instance.
    await collider.getByLabel('Beam A').fill('Trading signals');
    await collider.getByLabel('Beam B').fill('Mycelium networks');
    await collider.getByRole('button', { name: 'Collide Ideas' }).click();
    await expect(liveOverview.getByText('Complete — all six sections passed validation')).toBeVisible();
    await collider.getByRole('button', { name: 'Save collision as experiment' }).click();
    await expect(collider.getByText(/Saved as artifact .* in this project/)).toBeVisible();

    // Second collision in the live panel, so live and saved differ in beams.
    await collider.getByLabel('Beam A').fill('Harbor logistics');
    await collider.getByLabel('Beam B').fill('Seas and tides');
    await collider.getByRole('button', { name: 'Collide Ideas' }).click();
    await expect(liveOverview.getByText('Complete — all six sections passed validation')).toBeVisible();

    // Open the first saved experiment; both instances stay mounted.
    const artifacts = page.locator('section[aria-labelledby="project-artifacts-heading"]');
    await artifacts.getByRole('button', { name: 'Collider experiment: Trading signals' }).click();
    await expect(page.getByText('Collider experiment — provider ollama, model llama-x.')).toBeVisible();
    const savedOverview = page
      .getByRole('region', { name: 'Artifacts' })
      .locator('section[aria-labelledby="collider-pipeline-heading"]');
    await expect(savedOverview.getByText('Complete — all six sections passed validation')).toBeVisible();

    // DOM targets, not href strings: each link's target element must
    // live in its own instance's region.
    const savedTarget = await savedOverview
      .getByRole('link', { name: 'Proposed combinations' })
      .evaluate(link => {
        const href = link.getAttribute('href') ?? '';
        const target = href ? document.querySelector(href) : null;
        return {
          href,
          insideArtifacts: !!target?.closest('section[aria-labelledby="project-artifacts-heading"]'),
          insideLive: !!target?.closest('section[aria-labelledby="project-collider-heading"]'),
        };
      });
    expect(savedTarget.href).toMatch(/^#collider-saved-[0-9a-f]{32}-v1-combinations-heading$/);
    expect(savedTarget.insideArtifacts).toBe(true);
    expect(savedTarget.insideLive).toBe(false);

    const liveTarget = await liveOverview
      .getByRole('link', { name: 'Proposed combinations' })
      .evaluate(link => {
        const href = link.getAttribute('href') ?? '';
        const target = href ? document.querySelector(href) : null;
        return {
          href,
          insideArtifacts: !!target?.closest('section[aria-labelledby="project-artifacts-heading"]'),
          insideLive: !!target?.closest('section[aria-labelledby="project-collider-heading"]'),
        };
      });
    expect(liveTarget.href).toBe('#collider-combinations-heading');
    expect(liveTarget.insideLive).toBe(true);
    expect(liveTarget.insideArtifacts).toBe(false);

    // Keyboard: focus the saved combinations link, Tab to the next
    // saved link, Enter must land on the saved feasibility heading.
    const savedCombinations = savedOverview.getByRole('link', { name: 'Proposed combinations' });
    await savedCombinations.focus();
    await page.keyboard.press('Tab');
    const focused = await page.evaluate(() => document.activeElement?.textContent ?? '');
    expect(focused).toContain('Feasibility');
    await page.keyboard.press('Enter');
    await expect(page).toHaveURL(/#collider-saved-[0-9a-f]{32}-v1-feasibility-heading/);
    const fragmentTarget = await page.evaluate(() => {
      const hash = window.location.hash;
      const target = hash ? document.querySelector(hash) : null;
      return !!target?.closest('section[aria-labelledby="project-artifacts-heading"]');
    });
    expect(fragmentTarget).toBe(true);
  } finally {
    await stopApi();
    fake.close();
  }
});
