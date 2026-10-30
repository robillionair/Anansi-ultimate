import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M8 URL input in the real browser application: a typed
 * URL is fetched by the backend only on the explicit
 * Fetch press; the exact fetched text and its provenance
 * are reviewed before Collide Ideas; nothing reaches a
 * provider before that press; fetch races, source
 * switches, edits, rejections, and cancellation all leave
 * the beam in the newest intended state; the saved
 * experiment keeps generation-time URL provenance across
 * restart and backup/fresh restore. The API runs with
 * the test-only loopback allowance so it can reach the
 * synthetic content server; strict-policy behavior is
 * covered by the Rust tests.
 */
function collisionJson(): string {
  return JSON.stringify({
    sourceUnderstanding: 'Beam A describes a fetched page: observations with structure. Beam B describes a fetched web page: distributed signals. Hypothesis: both are decentralized coordination systems with local feedback.',
    conceptualPrimitives: ['Distributed signaling', 'Adaptive routing', 'Local feedback', 'Decentralized coordination', 'Resource optimization'],
    crossDomainMappings: ['Fetched structure resembles hyphal branching: both organize observations locally.', 'Revision patience contrasts with signal speed: slow pages versus fast routing is the core tension.'],
    combinations: [
      { name: 'Mycelial order router', description: 'A routing layer that scores venues by local congestion signals, like hyphae choosing paths by nutrient gradients. Hypothesis only.' },
      { name: 'Graceful-degradation mesh', description: 'A trade mesh that reroutes through redundant paths when a venue stalls, accepting slower fills to avoid total outages. Hypothesis only.' },
    ],
    feasibility: {
      hypotheses: ['Local congestion signals can rank venues without a central coordinator.'],
      dependencies: ['A venue feed with latency and fill data', 'A simulator for routing policies'],
      unknownCosts: ['Exchange fees under rerouting are unknown.'],
      risks: ['Stale signals could route into worse fills.'],
      testability: 'Testable in simulation first: replay historical venue data through the router and compare fill quality against a static baseline.',
    },
    productization: {
      conceptBrief: 'A routing advisor that proposes venue choices from local signals. Hypothesis, not a product claim.',
      architecture: 'Signal ingest, local scoring, policy engine, simulation harness, audit log. No execution of generated code.',
      roadmap: 'Phase 1: simulator and scoring prototype on paper. Phase 2: replay evaluation. Phase 3: human-reviewed pilot proposal.',
      testPlan: 'Unit-test the scorer on synthetic gradients; replay-test the router.',
      prototypeProposal: 'A paper prototype plus a simulator notebook, reviewed by a human before any live use.',
    },
  });
}

test('collider url input: fetch -> review -> race -> switch -> reject -> cancel -> collide -> save -> restart -> backup/restore', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-collider-url-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };
  let fakeHits = 0;

  const fake: Server = createServer((req, res) => {
    let body = '';
    req.on('data', c => { body += c; });
    req.on('end', () => {
      fakeHits += 1;
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

  // Synthetic content server: plain text, HTML, a slow
  // page (race/cancel), an oversized page, a redirect to
  // a private address, and a non-text type.
  const pageText = 'Fetched page text.\nSecond line.\n';
  const pageHtml = '<html><head><title>Hidden title</title><script>alert(1)</script></head><body><h1>Head &amp; line</h1><p>Body text.</p></body></html>';
  const pageHtmlText = 'Head & line\n\nBody text.';
  const slowText = 'Slow page text.\n';
  const content: Server = createServer((req, res) => {
    const url = req.url ?? '/';
    if (url === '/page.txt') {
      res.writeHead(200, { 'content-type': 'text/plain; charset=utf-8' });
      res.end(pageText);
    } else if (url === '/page.html') {
      res.writeHead(200, { 'content-type': 'text/html' });
      res.end(pageHtml);
    } else if (url === '/slow.txt') {
      setTimeout(() => {
        res.writeHead(200, { 'content-type': 'text/plain' });
        res.end(slowText);
      }, 1500);
    } else if (url === '/big.txt') {
      res.writeHead(200, { 'content-type': 'text/plain' });
      res.end('x'.repeat(9000));
    } else if (url === '/to-private') {
      res.writeHead(302, { Location: 'http://10.0.0.1/' });
      res.end();
    } else if (url === '/binary') {
      res.writeHead(200, { 'content-type': 'application/pdf' });
      res.end('%PDF-1.4');
    } else {
      res.writeHead(404);
      res.end();
    }
  });
  await new Promise<void>(r => content.listen(0, '127.0.0.1', r));
  const contentUrl = `http://127.0.0.1:${(content.address() as { port: number }).port}`;

  async function startApi() {
    child = spawn(
      path.resolve('target/debug/anansi-api' + (process.platform === 'win32' ? '.exe' : '')),
      ['--port', '0', '--pairing-root', path.join(root, 'pairing')],
      { windowsHide: true, env: { ...process.env, ANANSI_STORAGE_ROOT: path.join(root, 'storage'), ANANSI_URL_FETCH_ALLOW_LOOPBACK: '1' }, stdio: ['ignore', 'pipe', 'ignore'] },
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

  async function readExperimentBody(workspaceId: string, artifactId: string) {
    const read = await call('artifacts', 'read', { workspaceId, artifactId, version: 1 });
    expect(read.status).toBe(200);
    return JSON.parse(read.body.body as string) as Record<string, any>;
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

    // A document fixture for the source-switch steps.
    const files = path.join(root, 'files');
    fs.mkdirSync(files, { recursive: true });
    const docA = path.join(files, 'switch-notes.md');
    fs.writeFileSync(docA, 'Switched document text.\n');

    await pairAndOpen(workspaceId);
    const collider = page.locator('section[aria-labelledby="project-collider-heading"]');
    await expect(collider.getByRole('heading', { name: 'LEXI-COLLIDER' })).toBeVisible();

    // Beam A from a URL: exact fetched text is reviewed
    // with its provenance, and no provider traffic
    // happens on entry or on fetch.
    await collider.getByLabel('URL for the first beam').fill(`${contentUrl}/page.txt`);
    await collider.getByRole('button', { name: 'Fetch for the first beam', exact: true }).click();
    await expect(collider.getByLabel('Beam A', { exact: true })).toHaveValue(pageText);
    await expect(collider.getByText(new RegExp(`URL: ${contentUrl.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}/page\\.txt`))).toBeVisible();
    await expect(collider.getByText(/Plain text/)).toBeVisible();
    expect(fakeHits).toBe(0);

    // Fetch race: a slow fetch followed by a fast fetch
    // for the same beam must end with the fast content.
    await collider.getByLabel('URL for the first beam').fill(`${contentUrl}/slow.txt`);
    await collider.getByRole('button', { name: 'Fetch for the first beam', exact: true }).click();
    await collider.getByLabel('URL for the first beam').fill(`${contentUrl}/page.txt`);
    await collider.getByRole('button', { name: 'Fetch for the first beam', exact: true }).click();
    await expect(collider.getByLabel('Beam A', { exact: true })).toHaveValue(pageText);
    await page.waitForTimeout(2000);
    await expect(collider.getByLabel('Beam A', { exact: true })).toHaveValue(pageText);
    expect(fakeHits).toBe(0);

    // Cancel: a slow fetch for Beam B, cancelled before
    // it lands, leaves the beam empty even after the
    // late response arrives.
    await collider.getByLabel('URL for the second beam').fill(`${contentUrl}/slow.txt`);
    await collider.getByRole('button', { name: 'Fetch for the second beam', exact: true }).click();
    await collider.getByRole('button', { name: 'Cancel fetch for the second beam', exact: true }).click();
    await expect(collider.getByLabel('Beam B', { exact: true })).toHaveValue('');
    await page.waitForTimeout(2000);
    await expect(collider.getByLabel('Beam B', { exact: true })).toHaveValue('');
    expect(fakeHits).toBe(0);

    // Beam B rejections: invalid URL, oversized text, a
    // redirect to a private address, and a non-text type.
    // Each refuses visibly and leaves the beam empty.
    await collider.getByLabel('URL for the second beam').fill('not a url');
    await collider.getByRole('button', { name: 'Fetch for the second beam', exact: true }).click();
    await expect(collider.getByRole('alert')).toHaveText(/could not be parsed|must be/);
    await expect(collider.getByLabel('Beam B', { exact: true })).toHaveValue('');
    await collider.getByLabel('URL for the second beam').fill(`${contentUrl}/big.txt`);
    await collider.getByRole('button', { name: 'Fetch for the second beam', exact: true }).click();
    await expect(collider.getByRole('alert')).toHaveText(/1 to 8000/);
    await expect(collider.getByLabel('Beam B', { exact: true })).toHaveValue('');
    await collider.getByLabel('URL for the second beam').fill(`${contentUrl}/to-private`);
    await collider.getByRole('button', { name: 'Fetch for the second beam', exact: true }).click();
    await expect(collider.getByRole('alert')).toHaveText(/not a public destination/);
    await expect(collider.getByLabel('Beam B', { exact: true })).toHaveValue('');
    await collider.getByLabel('URL for the second beam').fill(`${contentUrl}/binary`);
    await collider.getByRole('button', { name: 'Fetch for the second beam', exact: true }).click();
    await expect(collider.getByRole('alert')).toHaveText(/Only plain text and HTML/);
    await expect(collider.getByLabel('Beam B', { exact: true })).toHaveValue('');
    expect(fakeHits).toBe(0);

    // Source switch, both directions: a document pick
    // replaces the URL attribution, and a URL fetch
    // replaces the document attribution.
    await collider.getByLabel('Document for the first beam (.txt or .md)', { exact: true }).setInputFiles(docA);
    await expect(collider.getByLabel('Beam A', { exact: true })).toHaveValue('Switched document text.\n');
    await expect(collider.getByText(/Document: switch-notes\.md/)).toBeVisible();
    await expect(collider.getByText(new RegExp(`URL: ${contentUrl.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}/page\\.txt`))).toHaveCount(0);
    await collider.getByLabel('URL for the first beam').fill(`${contentUrl}/page.txt`);
    await collider.getByRole('button', { name: 'Fetch for the first beam', exact: true }).click();
    await expect(collider.getByLabel('Beam A', { exact: true })).toHaveValue(pageText);
    await expect(collider.getByText(/Document: switch-notes\.md/)).toHaveCount(0);
    await expect(collider.getByText(new RegExp(`URL: ${contentUrl.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}/page\\.txt`))).toBeVisible();

    // Editing a fetched beam clears its attribution.
    await collider.getByLabel('Beam A', { exact: true }).fill('Edited fetched text.\n');
    await expect(collider.getByText(new RegExp(`URL: ${contentUrl.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}/page\\.txt`))).toHaveCount(0);
    await collider.getByLabel('Beam A', { exact: true }).fill(pageText);
    await collider.getByLabel('URL for the first beam').fill(`${contentUrl}/page.txt`);
    await collider.getByRole('button', { name: 'Fetch for the first beam', exact: true }).click();
    await expect(collider.getByLabel('Beam A', { exact: true })).toHaveValue(pageText);

    // Beam B from an HTML page: scripts and head are
    // stripped, entities decoded, blocks separated.
    await collider.getByLabel('URL for the second beam').fill(`${contentUrl}/page.html`);
    await collider.getByRole('button', { name: 'Fetch for the second beam', exact: true }).click();
    await expect(collider.getByLabel('Beam B', { exact: true })).toHaveValue(pageHtmlText);
    await expect(collider.getByText(/Web page/)).toBeVisible();
    await expect(collider.getByText(/conservatively extracted text/)).toBeVisible();
    expect(fakeHits).toBe(0);

    // The explicit press, then save with exact URL
    // provenance through the API.
    await collider.getByRole('button', { name: 'Collide Ideas' }).click();
    await expect(collider.getByRole('heading', { name: 'Source understanding' })).toBeVisible();
    await expect(collider.getByRole('heading', { name: 'Proposed combinations or architectures' })).toBeVisible();
    await expect(collider.getByRole('heading', { name: 'View what ANANSI sent' })).toBeVisible();
    expect(fakeHits).toBeGreaterThan(0);
    await collider.getByRole('button', { name: 'Save collision as experiment' }).click();
    await expect(collider.getByText(/Saved as artifact .* in this project/)).toBeVisible();
    const experimentId = await artifactIdByType(workspaceId, projectId, 'experiment');
    expect(experimentId).not.toBe('');
    const savedBody = await readExperimentBody(workspaceId, experimentId);
    expect(savedBody.beamA).toBe(pageText);
    expect(savedBody.beamB).toBe(pageHtmlText);
    expect(savedBody.urlA.submittedUrl).toBe(`${contentUrl}/page.txt`);
    expect(savedBody.urlA.finalUrl).toBe(`${contentUrl}/page.txt`);
    expect(savedBody.urlA.contentKind).toBe('text');
    expect(savedBody.urlA.extraction).toBe('plain');
    expect(savedBody.urlA.fetchedBytes).toBe(pageText.length);
    expect(savedBody.urlA.textChars).toBe([...pageText].length);
    expect(savedBody.urlB.contentKind).toBe('html');
    expect(savedBody.urlB.extraction).toBe('html-conservative');
    expect(savedBody.urlB.fetchedBytes).toBe(pageHtml.length);

    // Reopen the saved experiment: URL provenance
    // renders as inert text beside the verbatim beams.
    const artifacts = page.locator('section[aria-labelledby="project-artifacts-heading"]');
    await artifacts.getByRole('button', { name: /Collider experiment: Fetched page text/ }).click();
    await expect(artifacts.getByText('Collider experiment — provider ollama, model llama-x.')).toBeVisible();
    await artifacts.getByText('Beam A (as supplied)').click();
    await expect(artifacts.getByText(new RegExp(`URL: ${contentUrl.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}/page\\.txt`))).toBeVisible();
    await artifacts.getByText('Beam B (as supplied)').click();
    await expect(artifacts.getByText(/Web page/)).toBeVisible();

    // Restart/reopen: the experiment and provenance persist.
    await stopApi();
    await startApi();
    await pairAndOpen(workspaceId);
    await artifacts.getByRole('button', { name: /Collider experiment: Fetched page text/ }).click();
    await artifacts.getByText('Beam A (as supplied)').click();
    await expect(artifacts.getByText(new RegExp(`URL: ${contentUrl.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}/page\\.txt`))).toBeVisible();

    // Backup and restore into a fresh workspace, then
    // reopen the restored experiment with identical
    // content and provenance.
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
    const landing = await call('workspaces', 'create', { name: 'Collider Landing' });
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
    const restored = await call('jobs', 'restore', { workspaceId: landingId, jobId: importJobId, artifactId: uploadArtifactId, name: 'Collider Restored', expectedHash: preview.body.sha256, confirm: true });
    expect(restored.status).toBe(200);
    const restoredWs = (restored.body.workspaces as Array<Record<string, any>>)[0].id as string;
    expect(restoredWs).not.toBe(workspaceId);

    await pairAndOpen(restoredWs);
    await artifacts.getByRole('button', { name: /Collider experiment: Fetched page text/ }).click();
    await artifacts.getByText('Beam A (as supplied)').click();
    await expect(artifacts.getByText(new RegExp(`URL: ${contentUrl.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}/page\\.txt`))).toBeVisible();
    await artifacts.getByText('Beam B (as supplied)').click();
    await expect(artifacts.getByText(/Web page/)).toBeVisible();
    const restoredProjects = await call('memories', 'list', { workspaceId: restoredWs, kind: 'project' });
    const restoredProjectId = (restoredProjects.body.memories as Array<Record<string, any>>)[0].id as string;
    const restoredExperimentId = await artifactIdByType(restoredWs, restoredProjectId, 'experiment');
    expect(restoredExperimentId).not.toBe(experimentId);
    const restoredBody = await readExperimentBody(restoredWs, restoredExperimentId);
    expect(restoredBody.beamA).toBe(pageText);
    expect(restoredBody.beamB).toBe(pageHtmlText);
    expect(restoredBody.urlA.submittedUrl).toBe(`${contentUrl}/page.txt`);
    expect(restoredBody.urlB.contentKind).toBe('html');
  } finally {
    await stopApi();
    await new Promise<void>(r => fake.close(() => r()));
    await new Promise<void>(r => content.close(() => r()));
  }
});
