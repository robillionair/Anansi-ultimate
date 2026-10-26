import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M7 Slice 2 in the real browser application: revising a
 * saved source package over the M5 substrate with a
 * synthetic loopback provider. Open package -> request
 * revision -> inspect the server-computed diff as inert
 * text -> reject (nothing saved) -> retry -> accept as a
 * new version -> restart/reopen -> backup/fresh restore
 * through the job routes, then reopen the restored
 * workspace in the browser. Generated code is never
 * executed, previewed, built or tested (§32).
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

function revisedPackageJson(): string {
  return JSON.stringify({
    plan: {
      summary: 'One page, vanilla JS, localStorage, plus a notes file.',
      steps: ['Update index.html heading', 'Add notes.txt', 'Keep app.js logic'],
      buildNotes: 'No build: open index.html. The revision kept the specification unchanged.',
      testingNotes: 'ANANSI has not run this code; check the acceptance criteria manually.',
    },
    files: [
      { path: 'index.html', content: '<!doctype html><title>Bean Stock Log - revised</title><script src="app.js"></script>' },
      { path: 'app.js', content: "console.log('bean log e2e revised');" },
      { path: 'README.md', content: '# Bean Stock Log\\nPrototype. Open index.html. Limitations: no accounts; do not store secrets.' },
      { path: 'notes.txt', content: 'Revision notes: heading updated.' },
    ],
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

test('product forge revision: open package -> revise -> inspect diff -> reject -> retry -> accept -> restart -> backup/restore', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-forge-revision-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };

  const fake: Server = createServer((req, res) => {
    let body = '';
    req.on('data', c => { body += c; });
    req.on('end', () => {
      const text = body.includes('Revision request') ? revisedPackageJson() : body.includes('product engineer') ? packageJson() : specJson();
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

  async function packageArtifactId(workspaceId: string, projectId: string) {
    const list = await call('artifacts', 'list', { workspaceId, projectId });
    const pkg = (list.body.artifacts as Array<Record<string, any>>).find(a => a.type === 'source-package');
    return pkg!.artifactId as string;
  }

  async function versionCount(workspaceId: string, artifactId: string) {
    const versions = await call('artifacts', 'versions', { workspaceId, artifactId });
    return (versions.body.versions as unknown[]).length;
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
    await page.getByLabel('Purpose — what should it do?').fill('Track bean stock without a spreadsheet.');
    await page.getByLabel('Intended users').fill('Cafe staff on a shared tablet.');
    await page.getByLabel('Must-have behavior').fill('Add beans, record usage, warn when low.');
    await page.getByRole('button', { name: 'Generate product specification' }).click();
    await expect(page.getByRole('heading', { name: 'Product specification: Bean Stock Log' })).toBeVisible();
    await page.getByRole('button', { name: 'Save specification draft' }).click();
    await expect(page.getByText('Specification draft saved as artifact')).toBeVisible();

    await artifacts.getByRole('button', { name: 'Product specification: Bean Stock Log' }).click();
    await page.getByRole('button', { name: 'Approve this specification version' }).click();
    await expect(page.getByText('Specification version 1 approved for source generation.')).toBeVisible();
    await page.getByRole('button', { name: 'Generate development plan & source' }).click();
    await expect(page.getByRole('heading', { name: 'Source package: Bean Stock Log' })).toBeVisible();
    await page.getByRole('button', { name: 'Save source package as artifact' }).click();
    await expect(page.getByText('Source package saved as artifact').first()).toBeVisible();

    const pkgId = await packageArtifactId(workspaceId, projectId);
    expect(await versionCount(workspaceId, pkgId)).toBe(1);

    // Open the package and request a revision.
    await artifacts.getByRole('button', { name: 'Source package: Bean Stock Log' }).click();
    await expect(page.getByRole('heading', { name: 'Revise this source package' })).toBeVisible();
    await page.getByLabel('Revision instructions — what should change?').fill('Update the heading and add a notes file.');
    await page.getByRole('button', { name: 'Request revision' }).click();
    await expect(page.getByRole('heading', { name: 'Proposed changes (against version 1)' })).toBeVisible();
    await expect(page.getByLabel('Revision added files')).toContainText('notes.txt');
    await expect(page.getByLabel('Revision changed files')).toContainText('index.html');
    await expect(page.getByLabel('Revision changed files')).toContainText('app.js');
    await expect(page.getByText('Generated — not yet saved as a project artifact.').first()).toBeVisible();

    // Reject: nothing is saved.
    await page.getByRole('button', { name: 'Reject revision — save nothing' }).click();
    await expect(page.getByText('Revision rejected. Nothing was saved; the original package version is unchanged.')).toBeVisible();
    expect(await versionCount(workspaceId, pkgId)).toBe(1);

    // Retry and accept: a new version of the same artifact.
    await page.getByRole('button', { name: 'Request revision' }).click();
    await expect(page.getByRole('heading', { name: 'Proposed changes (against version 1)' })).toBeVisible();
    await page.getByRole('button', { name: 'Accept revision — save as new version' }).click();
    await expect(page.getByText(/Revision saved as version 2/).first()).toBeVisible();
    expect(await versionCount(workspaceId, pkgId)).toBe(2);

    // Restart/reopen: the revised version is the current one.
    await stopApi();
    await startApi();
    await pairAndOpen(workspaceId);
    await artifacts.getByRole('button', { name: 'Source package: Bean Stock Log' }).click();
    await expect(page.getByRole('heading', { name: 'Source package: Bean Stock Log' })).toBeVisible();
    await page.getByRole('button', { name: 'notes.txt' }).click();
    await expect(page.locator('pre.provider-output')).toContainText('Revision notes: heading updated.');
    await page.getByRole('button', { name: 'app.js' }).click();
    await expect(page.locator('pre.provider-output')).toContainText('bean log e2e revised');

    // Backup the workspace and restore into a fresh workspace
    // through the same job routes the Backups UI uses, then
    // reopen the restored workspace in the browser.
    const backup = await call('jobs', 'backup', { workspaceId, application: false, plaintextConsent: true });
    expect(backup.status).toBe(200);
    const jobId = backup.body.job.id as string;
    let job: Record<string, any> | undefined;
    for (let i = 0; i < 60 && (!job || job.state !== 'Completed'); i++) {
      await new Promise(r => setTimeout(r, 1000));
      const list = await call('jobs', 'list', { workspaceId });
      job = (list.body.jobs as Array<Record<string, any>>).find(j => j.id === jobId);
    }
    expect(job!.state).toBe('Completed');
    const artifactId = job!.artifactId as string;
    const artifactBytes = job!.artifactBytes as number;
    const chunks: number[] = [];
    for (let offset = 0; offset < artifactBytes;) {
      const chunk = await call('jobs', 'artifact', { workspaceId, jobId, artifactId, offset });
      const bytes = chunk.body.bytes as number[];
      expect(bytes.length).toBeGreaterThan(0);
      chunks.push(...bytes);
      offset += bytes.length;
    }
    expect(chunks.length).toBe(artifactBytes);
    const tar = Buffer.from(chunks);
    const { createHash } = await import('node:crypto');
    const sha256 = createHash('sha256').update(tar).digest('hex');
    const landing = await call('workspaces', 'create', { name: 'Forge Revised Landing' });
    const landingId = landing.body.workspace.id as string;
    const begin = await call('jobs', 'upload-begin', { workspaceId: landingId, bytes: tar.length, sha256, plaintextConsent: true });
    const uploadId = begin.body.id as string;
    // The import job id and artifact id are published on
    // the finished upload-part's job, not on upload-begin;
    // preview/restore address that job, not the upload.
    let uploadArtifactId = '';
    let importJobId = '';
    for (let offset = 0; offset < tar.length; offset += 65536) {
      const end = Math.min(offset + 65536, tar.length);
      const part = await call('jobs', 'upload-part', { workspaceId: landingId, id: uploadId, offset, bytes: Array.from(tar.subarray(offset, end)), finish: end === tar.length });
      if (part.body.job?.artifactId) uploadArtifactId = part.body.job.artifactId as string;
      if (part.body.job?.id) importJobId = part.body.job.id as string;
    }
    expect(uploadArtifactId).not.toBe('');
    expect(importJobId).not.toBe('');
    const preview = await call('jobs', 'preview', { workspaceId: landingId, jobId: importJobId, artifactId: uploadArtifactId });
    expect(preview.status).toBe(200);
    const restored = await call('jobs', 'restore', { workspaceId: landingId, jobId: importJobId, artifactId: uploadArtifactId, name: 'Forge Revised Restored', expectedHash: preview.body.sha256, confirm: true });
    expect(restored.status).toBe(200);
    const restoredWs = (restored.body.workspaces as Array<Record<string, any>>)[0].id as string;
    expect(restoredWs).not.toBe(workspaceId);

    await pairAndOpen(restoredWs);
    await artifacts.getByRole('button', { name: 'Source package: Bean Stock Log' }).click();
    await expect(page.getByRole('heading', { name: 'Source package: Bean Stock Log' })).toBeVisible();
    await page.getByRole('button', { name: 'notes.txt' }).click();
    await expect(page.locator('pre.provider-output')).toContainText('Revision notes: heading updated.');
    await expect(page.getByRole('heading', { name: 'Revise this source package' })).toBeVisible();
  } finally {
    await stopApi();
    fake.close();
    fs.rmSync(root, { recursive: true, force: true });
  }
});
