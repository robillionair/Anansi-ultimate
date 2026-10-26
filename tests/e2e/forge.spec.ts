import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M7 Slice 1 in the real browser application: Product
 * Forge (§§28–33) over the M5 substrate with a synthetic
 * loopback provider. Saved concept -> essential questions
 * -> generated specification -> save draft -> explicit
 * per-version approval -> pinned build -> plan + source
 * package inspected as inert text
 * -> save -> export (.tar verified) -> restart -> reopen.
 * Build/test/preview/deployment must read Unavailable
 * (§32: no execution sandbox in this slice).
 */
function specJson(): string {
  return JSON.stringify({
    title: 'Bean Stock Log',
    summary: 'A tiny page for tracking cafe bean stock.',
    purpose: 'Stop stock-outs.',
    intendedUsers: 'Cafe staff',
    features: ['Add a bean with a starting weight', 'Record usage', 'Warn when stock is low'],
    pages: ['index.html'],
    nonGoals: ['No accounts', 'No server'],
    dataAndState: 'Entries live in localStorage.',
    acceptanceCriteria: ['Open index.html and add a bean', 'Reload keeps entries'],
    openQuestions: ['Which units do staff prefer?'],
  });
}

function packageJson(): string {
  return JSON.stringify({
    plan: {
      summary: 'One page, vanilla JS, localStorage.',
      steps: ['Write index.html shell', 'Add app.js logic'],
      buildNotes: 'No build: open index.html.',
      testingNotes: 'ANANSI has not run this code; check the acceptance criteria manually.',
    },
    files: [
      { path: 'index.html', content: '<!doctype html><title>Bean Stock Log</title><script src="app.js"></script>' },
      { path: 'app.js', content: "console.log('bean log e2e');" },
      { path: 'README.md', content: '# Bean Stock Log\nPrototype. Open index.html. Limitations: no accounts; do not store secrets.' },
    ],
  });
}

function ideaSetBody(): string {
  const concept = (name: string) => ({
    name,
    problem: 'Cafes lose track of bean stock.',
    targetCustomer: 'Independent cafes',
    solution: 'A stock log page.',
    whyUseful: 'It prevents stock-outs.',
    differentiationHypothesis: 'Hypothesis: built for shared counters.',
    revenueModel: 'Subscription.',
    engineeringComplexity: 'low',
    implementationCost: 'about 1-2 weeks for one developer',
    keyRisks: ['Staff may forget to log usage'],
    requiredValidation: 'Shadow one cafe for a week.',
    minimumViableVersion: 'One page with add and subtract.',
  });
  return JSON.stringify({
    topic: 'Cafe tools',
    concepts: [concept('Bean Stock Log'), ...Array.from({ length: 5 }, (_, i) => concept(`Filler concept ${i}`))],
  });
}

test('product forge: concept -> specification -> source package -> export -> restart -> reopen', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-forge-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };

  const fake: Server = createServer((req, res) => {
    let body = '';
    req.on('data', c => { body += c; });
    req.on('end', () => {
      const text = body.includes('product engineer') ? packageJson() : specJson();
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
    await page.getByRole('button', { name: 'Harbor App' }).click();
    await expect(page.getByRole('heading', { name: 'Artifacts' })).toBeVisible();
  }

  try {
    await startApi();
    const created = await call('workspaces', 'create', { name: 'Forge Workspace' });
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
    const project = await call('memories', 'create', {
      workspaceId,
      kind: 'project',
      draft: {
        title: 'Harbor App', body: 'A test project.',
        privacy: 'private', eventDate: null, validFrom: null, validUntil: null,
        confidence: null, sourceReferences: [],
      },
    });
    const projectId = project.body.memory.id as string;
    await call('artifacts', 'register', {
      workspaceId, projectId, type: 'idea', title: 'Idea set: Cafe tools',
      body: ideaSetBody(), sourceReferences: [], privacy: 'private',
    });

    await pairAndOpen(workspaceId);
    const artifacts = page.locator('section[aria-labelledby="project-artifacts-heading"]');
    await artifacts.getByRole('button', { name: 'Idea set: Cafe tools' }).click();
    await expect(page.getByRole('heading', { name: 'Product Forge — develop a concept (§§28–33)' })).toBeVisible();
    await page.getByLabel('Purpose — what should it do?').fill('Track bean stock without a spreadsheet.');
    await page.getByLabel('Intended users').fill('Cafe staff on a shared tablet.');
    await page.getByLabel('Must-have behavior').fill('Add beans, record usage, warn when low.');
    await page.getByRole('button', { name: 'Generate product specification' }).click();
    await expect(page.getByRole('heading', { name: 'Product specification: Bean Stock Log' })).toBeVisible();
    await page.getByRole('button', { name: 'Save specification draft' }).click();
    await expect(page.getByText('Specification draft saved as artifact')).toBeVisible();

    await artifacts.getByRole('button', { name: 'Product specification: Bean Stock Log' }).click();
    await expect(page.getByRole('heading', { name: 'Product Forge — generate plan & source' })).toBeVisible();
    // Generating source requires an explicit backend
    // approval of this exact specification version.
    await expect(page.getByText('Specification version 1: not approved.')).toBeVisible();
    await page.getByRole('button', { name: 'Approve this specification version' }).click();
    await expect(page.getByText('Specification version 1 approved for source generation.')).toBeVisible();
    await page.getByRole('button', { name: 'Generate development plan & source' }).click();
    await expect(page.getByRole('heading', { name: 'Source package: Bean Stock Log' })).toBeVisible();
    await page.getByRole('button', { name: 'Save source package as artifact' }).click();
    await expect(page.getByText('Source package saved as artifact').first()).toBeVisible();

    await artifacts.getByRole('button', { name: 'Source package: Bean Stock Log' }).click();
    await expect(page.getByRole('heading', { name: 'Source package: Bean Stock Log' })).toBeVisible();
    await page.getByRole('button', { name: 'app.js' }).click();
    await expect(page.locator('pre.provider-output')).toContainText('bean log e2e');
    await expect(page.getByText('has not been built, tested, executed or deployed by ANANSI').first()).toBeVisible();
    await expect(page.getByText(/Build: Unavailable — no execution sandbox/).first()).toBeVisible();

    const downloadPromise = page.waitForEvent('download');
    await page.getByRole('button', { name: 'Export source package (.tar)' }).click();
    const download = await downloadPromise;
    const tarPath = await download.path();
    const tar = fs.readFileSync(tarPath!);
    const names: string[] = [];
    let offset = 0;
    while (offset + 512 <= tar.length) {
      const header = tar.subarray(offset, offset + 512);
      if (header.every(b => b === 0)) break;
      const name = header.subarray(0, 100).toString('utf8').replace(/\0.*$/, '');
      const size = parseInt(header.subarray(124, 135).toString('utf8').replace(/\0.*$/, '').trim() || '0', 8);
      names.push(name);
      offset += 512 + Math.ceil(size / 512) * 512;
    }
    expect(names).toEqual(['ANANSI-PACKAGE.json', 'index.html', 'app.js', 'README.md']);
    const manifest = JSON.parse(tar.subarray(512, 512 + parseInt(tar.subarray(124, 135).toString('utf8').replace(/\0.*$/, '').trim(), 8)).toString('utf8'));
    expect(manifest.format).toBe('anansi-source-package-export');
    expect(manifest.files).toHaveLength(3);

    // Restart: the package reopens from persisted storage.
    await stopApi();
    await startApi();
    await pairAndOpen(workspaceId);
    await artifacts.getByRole('button', { name: 'Source package: Bean Stock Log' }).click();
    await expect(page.getByRole('heading', { name: 'Source package: Bean Stock Log' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Development plan' })).toBeVisible();
  } finally {
    await stopApi();
    fake.close();
    fs.rmSync(root, { recursive: true, force: true });
  }
});
