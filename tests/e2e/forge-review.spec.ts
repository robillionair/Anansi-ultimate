import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M7 Slice 4 in the real browser application: review a
 * saved source package against its pinned specification's
 * acceptance criteria, save the review as a versioned
 * artifact, revise the review, select findings into
 * editable revision instructions, submit them through
 * the existing revision flow (reject, then accept), and
 * restart/backup/restore with the review still pinning
 * the original package version. Reviewed contents are
 * never executed, previewed, built or tested (§32).
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

function revisedPackageJson(): string {
  return JSON.stringify({
    plan: {
      summary: 'One page, vanilla JS, localStorage, revised.',
      steps: ['Update index.html heading', 'Keep app.js logic'],
      buildNotes: 'No build: open index.html.',
      testingNotes: 'ANANSI has not run this code; check the acceptance criteria manually.',
    },
    files: [
      { path: 'index.html', content: '<!doctype html><title>Bean Stock Log — revised</title><script src="app.js"></script>' },
      { path: 'app.js', content: "console.log('bean log e2e revised');" },
      { path: 'README.md', content: '# Bean Stock Log\nPrototype. Open index.html.' },
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

test('product forge review: assess -> save -> revise -> findings -> revision reject/accept -> restart -> backup/restore', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-forge-review-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };

  const fake: Server = createServer((req, res) => {
    let body = '';
    req.on('data', c => { body += c; });
    req.on('end', () => {
      const text = body.includes('Revision requested') ? revisedPackageJson() : body.includes('product engineer') ? packageJson() : specJson();
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

  async function artifactIdByType(workspaceId: string, projectId: string, type: string) {
    const list = await call('artifacts', 'list', { workspaceId, projectId });
    const found = (list.body.artifacts as Array<Record<string, any>>).find(a => a.type === type);
    return found!.artifactId as string;
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

    const pkgId = await artifactIdByType(workspaceId, projectId, 'source-package');

    // Open the package and review it: criteria verbatim,
    // defaults Not assessed, inspection-only banner.
    await artifacts.getByRole('button', { name: 'Source package: Bean Stock Log' }).click();
    await expect(page.getByRole('heading', { name: 'Review this source package' })).toBeVisible();
    await expect(page.getByText('Inspection only — this review does not establish runtime behavior')).toBeVisible();
    await expect(page.getByText('Criterion 1: Open index.html and add a bean')).toBeVisible();
    await expect(page.getByText('Criterion 2: Reload keeps entries')).toBeVisible();
    await expect(page.getByLabel('Review status for criterion 1')).toHaveValue('not-assessed');
    await expect(page.getByLabel('Review status for criterion 2')).toHaveValue('not-assessed');
    await page.getByLabel('Review status for criterion 1').selectOption('satisfied');
    await page.getByLabel('Review status for criterion 2').selectOption('needs-changes');
    await page.locator('#forge-review-note-1').fill('Reload loses the last entry; persist before unload in app.js.');
    await page.locator('#forge-review-files-1').fill('app.js');
    await page.getByRole('button', { name: 'Save source review' }).click();
    await expect(page.getByText(/Source review saved as version 1/).first()).toBeVisible();

    const reviewId = await artifactIdByType(workspaceId, projectId, 'source-review');
    expect(await versionCount(workspaceId, reviewId)).toBe(1);

    // The saved review shows findings, pins and banner.
    await artifacts.getByRole('button', { name: 'Source review: Bean Stock Log' }).click();
    await expect(page.getByRole('heading', { name: 'Source review: Bean Stock Log' })).toBeVisible();
    await expect(page.getByText('Status: satisfied')).toBeVisible();
    await expect(page.getByText('Status: needs-changes')).toBeVisible();
    await expect(page.locator('pre.provider-output').filter({ hasText: 'Reload loses the last entry' }).first()).toBeVisible();
    await expect(page.getByText(/Pins source package artifact .* version 1/)).toBeVisible();

    // Revise the review as a new version; cancelling a
    // draft must leave no draft behind (regression in
    // the deliverable Cancel pattern).
    await page.getByRole('button', { name: 'Revise this review' }).click();
    await page.locator('#forge-review-note-0').fill('Discard this acceptance-test draft.');
    await page.getByRole('button', { name: 'Cancel editing', exact: true }).click();
    await page.getByRole('button', { name: 'Revise this review' }).click();
    await expect(page.locator('#forge-review-note-0')).toHaveValue('');
    await page.locator('#forge-review-note-0').fill('Add flow present in app.js.');
    await page.getByRole('button', { name: 'Save revised review as new version' }).click();
    await expect(page.getByText(/Review revised as version 2/).first()).toBeVisible();
    expect(await versionCount(workspaceId, reviewId)).toBe(2);

    // Findings -> editable instructions -> revision panel.
    // Criterion 2 (needs-changes) is pre-selected; the
    // satisfied criterion is not included.
    await page.getByRole('button', { name: 'Prepare revision instructions' }).click();
    await expect(page.locator('#forge-review-revision-instructions')).toHaveValue(/Reload keeps entries/);
    await expect(page.locator('#forge-review-revision-instructions')).toHaveValue(/Reload loses the last entry/);
    await expect(page.locator('#forge-review-revision-instructions')).toHaveValue(/app\.js/);
    await expect(page.locator('#forge-review-revision-instructions')).not.toHaveValue(/add a bean/);
    await page.locator('#forge-review-revision-instructions').fill('Revision requested from source review: persist the last entry before unload in app.js.');
    await page.getByRole('button', { name: 'Use in revision panel' }).click();
    await expect(page.getByRole('heading', { name: 'Revise this source package' })).toBeVisible();
    await expect(page.locator('#forge-revision-instructions')).toHaveValue('Revision requested from source review: persist the last entry before unload in app.js.');

    // Reject saves nothing, then accept saves a new
    // package version; the review still pins v1.
    await page.getByRole('button', { name: 'Request revision' }).click();
    await expect(page.getByText('Proposed changes (against version 1)')).toBeVisible();
    await page.getByRole('button', { name: 'Reject revision — save nothing' }).click();
    await expect(page.getByText('Revision rejected. Nothing was saved')).toBeVisible();
    expect(await versionCount(workspaceId, pkgId)).toBe(1);
    await page.getByRole('button', { name: 'Request revision' }).click();
    await expect(page.getByText('Proposed changes (against version 1)')).toBeVisible();
    await page.getByRole('button', { name: 'Accept revision — save as new version' }).click();
    await expect(page.getByText(/Revision saved as version 2/).first()).toBeVisible();
    expect(await versionCount(workspaceId, pkgId)).toBe(2);
    await artifacts.getByRole('button', { name: 'Source review: Bean Stock Log' }).click();
    await expect(page.getByText(/Pins source package artifact .* version 1/)).toBeVisible();

    // Historical review versions stay immutable: v1 must
    // not carry the v2 revision's note.
    await page.getByRole('button', { name: 'View version 1', exact: true }).click();
    await expect(page.getByText('Viewing version 1 of Source review: Bean Stock Log', { exact: false })).toBeVisible();
    await expect(page.locator('pre.provider-output').filter({ hasText: 'Add flow present in app.js.' })).toHaveCount(0);
    await expect(page.locator('pre.provider-output').filter({ hasText: 'Reload loses the last entry' }).first()).toBeVisible();

    // Restart/reopen: the review and its findings persist.
    await stopApi();
    await startApi();
    await pairAndOpen(workspaceId);
    await artifacts.getByRole('button', { name: 'Source review: Bean Stock Log' }).click();
    await expect(page.locator('pre.provider-output').filter({ hasText: 'Reload loses the last entry' }).first()).toBeVisible();

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
    const landing = await call('workspaces', 'create', { name: 'Forge Review Landing' });
    const landingId = landing.body.workspace.id as string;
    const begin = await call('jobs', 'upload-begin', { workspaceId: landingId, bytes: tar.length, sha256, plaintextConsent: true });
    const uploadId = begin.body.id as string;
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
    const restored = await call('jobs', 'restore', { workspaceId: landingId, jobId: importJobId, artifactId: uploadArtifactId, name: 'Forge Review Restored', expectedHash: preview.body.sha256, confirm: true });
    expect(restored.status).toBe(200);
    const restoredWs = (restored.body.workspaces as Array<Record<string, any>>)[0].id as string;
    expect(restoredWs).not.toBe(workspaceId);

    await pairAndOpen(restoredWs);
    await artifacts.getByRole('button', { name: 'Source review: Bean Stock Log' }).click();
    await expect(page.locator('pre.provider-output').filter({ hasText: 'Reload loses the last entry' }).first()).toBeVisible();
    const restoredProjects = await call('memories', 'list', { workspaceId: restoredWs, kind: 'project' });
    const restoredProjectId = (restoredProjects.body.memories as Array<Record<string, any>>)[0].id as string;
    const restoredPkgId = await artifactIdByType(restoredWs, restoredProjectId, 'source-package');
    expect(restoredPkgId).not.toBe(pkgId);
    await expect(page.getByText(new RegExp(`Pins source package artifact ${restoredPkgId} version 1`))).toBeVisible();
  } finally {
    await stopApi();
    fake.close();
    fs.rmSync(root, { recursive: true, force: true });
  }
});
