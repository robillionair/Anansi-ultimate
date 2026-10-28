import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M8 Slice 3 in the real browser application: explicitly
 * selected workspace memories become collision context.
 * Nothing is selected automatically; selecting, reviewing
 * or changing context starts no inference; the pre-send
 * review shows exactly the revisions that will be sent;
 * the saved experiment pins generation-time content, so
 * a later memory edit never changes it; restart and
 * backup/fresh restore preserve the snapshot with
 * remapped references. Provider-request content itself
 * (selected present, unselected absent) is proven by
 * backend tests against the captured request.
 */
function collisionJson(): string {
  return JSON.stringify({
    sourceUnderstanding: 'Beam A describes high-frequency trading: fast signals, order books, latency arbitrage. Beam B describes mycelium networks: distributed hyphal signaling, local nutrient routing, resilience through redundancy. Hypothesis: both are decentralized coordination systems with local feedback.',
    conceptualPrimitives: ['Distributed signaling', 'Adaptive routing', 'Local feedback', 'Decentralized coordination', 'Resource optimization'],
    crossDomainMappings: ['Order-book depth resembles nutrient gradients: both steer flow toward scarcity.', 'Latency arbitrage contrasts with mycelial patience: speed versus resilience is the core tension.', 'Redundant hyphal paths transfer to redundant trade routes that degrade gracefully.'],
    combinations: [
      { name: 'Mycelial order router', description: 'A routing layer that scores venues by local congestion signals, like hyphae choosing paths by nutrient gradients. Hypothesis only.' },
      { name: 'Graceful-degradation mesh', description: 'A trade mesh that reroutes through redundant paths when a venue stalls, accepting slower fills to avoid total outages. Hypothesis only.' },
    ],
    feasibility: {
      hypotheses: ['Local congestion signals can rank venues without a central coordinator.', 'Redundancy costs are bounded by capping parallel routes.'],
      dependencies: ['A venue feed with latency and fill data', 'A simulator for routing policies'],
      unknownCosts: ['Exchange fees under rerouting are unknown.', 'Compute cost of continuous signal scoring is unknown.'],
      risks: ['Stale signals could route into worse fills.', 'Regulatory treatment of adaptive routing is unvalidated.'],
      testability: 'Testable in simulation first: replay historical venue data through the router and compare fill quality against a static baseline.',
    },
    productization: {
      conceptBrief: 'A routing advisor that proposes venue choices from local signals. Hypothesis, not a product claim.',
      architecture: 'Signal ingest, local scoring, policy engine, simulation harness, audit log. No execution of generated code.',
      roadmap: 'Phase 1: simulator and scoring prototype on paper. Phase 2: replay evaluation. Phase 3: human-reviewed pilot proposal.',
      testPlan: 'Unit-test the scorer on synthetic gradients; replay-test the router; red-team stale-signal behavior.',
      prototypeProposal: 'A paper prototype plus a simulator notebook, reviewed by a human before any live use.',
    },
  });
}

test('lexi-collider memory context: selection -> pre-send review -> collision -> save -> restart -> fresh restore', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-collider-ctx-e2e-'));
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

  async function openProject(workspaceId: string, projectTitle: string) {
    await page.goto('/');
    await page.getByLabel('Local API address').fill(pairing.endpoint);
    await page.getByLabel('Session token').fill(pairing.token);
    await page.getByRole('button', { name: 'Connect to core' }).click();
    await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled();
    await page.getByLabel('Existing workspace').selectOption(workspaceId);
    await page.getByRole('button', { name: 'Open workspace', exact: true }).click();
    await page.getByRole('button', { name: 'Projects', exact: true }).click();
    await page.getByRole('button', { name: projectTitle }).click();
    await expect(page.getByRole('heading', { name: 'Artifacts' })).toBeVisible();
  }

  async function artifactIdByType(workspaceId: string, projectId: string, type: string) {
    const list = await call('artifacts', 'list', { workspaceId, projectId });
    const found = (list.body.artifacts as Array<Record<string, any>>).find(a => a.type === type);
    return found!.artifactId as string;
  }

  try {
    await startApi();
    const created = await call('workspaces', 'create', { name: 'Context Workspace' });
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
        title: 'Context Harbor', body: 'A test project.',
        privacy: 'private', eventDate: null, validFrom: null, validUntil: null,
        confidence: null, sourceReferences: [],
      },
    });
    const projectId = project.body.memory.id as string;
    const secondProject = await call('memories', 'create', {
      workspaceId,
      kind: 'project',
      draft: {
        title: 'Other Harbor', body: 'A second project.',
        privacy: 'private', eventDate: null, validFrom: null, validUntil: null,
        confidence: null, sourceReferences: [],
      },
    });
    const memoryDraft = (title: string, body: string) => ({
      title, body,
      privacy: 'private', eventDate: null, validFrom: null, validUntil: null,
      confidence: null, sourceReferences: [],
    });
    const memA = await call('memories', 'create', { workspaceId, draft: memoryDraft('Tide notes', 'Revision B tide observations.') });
    const memB = await call('memories', 'create', { workspaceId, draft: memoryDraft('Fungal notes', 'Hyphal gradients carry nutrients.') });
    await call('memories', 'create', { workspaceId, draft: memoryDraft('Unselected notes', 'This content stays out of the collision.') });
    const memAId = memA.body.memory.id as string;
    const memARev = memA.body.memory.revisionId as string;

    await openProject(workspaceId, 'Context Harbor');
    const collider = page.locator('section[aria-labelledby="project-collider-heading"]');
    await expect(collider.getByRole('heading', { name: 'LEXI-COLLIDER' })).toBeVisible();

    // Picker offers the workspace memories; nothing is selected.
    await expect(collider.getByLabel(/Tide notes/)).toBeVisible();
    await expect(collider.getByLabel(/Fungal notes/)).toBeVisible();
    await expect(collider.getByLabel(/Unselected notes/)).toBeVisible();
    await expect(collider.getByRole('group', { name: 'Memory context (optional)' }).locator('input[type="checkbox"]:checked')).toHaveCount(0);
    await expect(collider.locator('section[aria-label="Memory context to send"]')).toHaveCount(0);

    // Selecting starts no inference; the review shows exactly the revision.
    await collider.getByLabel(/Tide notes/).check();
    const review = collider.locator('section[aria-label="Memory context to send"]');
    await expect(review.getByText('Tide notes · revision 1')).toBeVisible();
    await review.getByText('Tide notes · revision 1').click();
    await expect(review.getByText('Revision B tide observations.')).toBeVisible();
    await expect(review.getByText('Unselected notes')).toHaveCount(0);
    await page.waitForTimeout(500);
    expect(fakeHits).toBe(0);

    // Changing the selection rebuilds the review.
    await collider.getByLabel(/Fungal notes/).check();
    await expect(review.getByText('Fungal notes · revision 1')).toBeVisible();
    await collider.getByLabel(/Fungal notes/).uncheck();
    await expect(review.getByText('Fungal notes · revision 1')).toHaveCount(0);
    await expect(review.getByText('Tide notes · revision 1')).toBeVisible();
    expect(fakeHits).toBe(0);

    // Project switches discard the selection (panel is keyed by project).
    await page.getByRole('button', { name: 'Projects', exact: true }).click();
    await page.getByRole('button', { name: 'Other Harbor' }).click();
    await expect(page.getByRole('heading', { name: 'Artifacts' })).toBeVisible();
    await expect(collider.getByRole('group', { name: 'Memory context (optional)' }).locator('input[type="checkbox"]:checked')).toHaveCount(0);
    await page.getByRole('button', { name: 'Projects', exact: true }).click();
    await page.getByRole('button', { name: 'Context Harbor' }).click();
    await expect(page.getByRole('heading', { name: 'Artifacts' })).toBeVisible();
    await expect(collider.getByRole('group', { name: 'Memory context (optional)' }).locator('input[type="checkbox"]:checked')).toHaveCount(0);

    // Re-select, collide, save.
    await collider.getByLabel(/Tide notes/).check();
    await expect(review.getByText('Tide notes · revision 1')).toBeVisible();
    await collider.getByLabel('Beam A').fill('Trading signals');
    await collider.getByLabel('Beam B').fill('Mycelium networks');
    await collider.getByRole('button', { name: 'Collide Ideas' }).click();
    await expect(collider.getByRole('heading', { name: 'Source understanding' })).toBeVisible();
    expect(fakeHits).toBe(1);
    await collider.getByRole('button', { name: 'Save collision as experiment' }).click();
    await expect(collider.getByText(/Saved as artifact .* in this project/)).toBeVisible();
    const experimentId = await artifactIdByType(workspaceId, projectId, 'experiment');
    expect(experimentId).not.toBe('');

    // The saved view pins generation-time context.
    const artifacts = page.locator('section[aria-labelledby="project-artifacts-heading"]');
    await artifacts.getByRole('button', { name: 'Collider experiment: Trading signals' }).click();
    await expect(page.getByText('Collider experiment — provider ollama, model llama-x.')).toBeVisible();
    const contextSection = page.locator('section[aria-label="Memory context"]');
    await expect(contextSection.getByText('Tide notes · revision 1 · private')).toBeVisible();
    await contextSection.getByText('Tide notes · revision 1 · private').click();
    await expect(contextSection.getByText('Revision B tide observations.')).toBeVisible();

    // Revise the memory, restart, reopen: the snapshot still shows revision B.
    const revised = await call('memories', 'revise', {
      workspaceId, id: memAId, expectedRevision: memARev,
      draft: memoryDraft('Tide notes', 'Revision C tide observations.'),
    });
    expect(revised.status).toBe(200);
    await stopApi();
    await startApi();
    await openProject(workspaceId, 'Context Harbor');
    await artifacts.getByRole('button', { name: 'Collider experiment: Trading signals' }).click();
    await expect(page.getByText('Collider experiment — provider ollama, model llama-x.')).toBeVisible();
    const contextAfterEdit = page.locator('section[aria-label="Memory context"]');
    await expect(contextAfterEdit.getByText('Tide notes · revision 1 · private')).toBeVisible();
    await contextAfterEdit.getByText('Tide notes · revision 1 · private').click();
    await expect(contextAfterEdit.getByText('Revision B tide observations.')).toBeVisible();
    await expect(contextAfterEdit.getByText('Revision C tide observations.')).toHaveCount(0);

    // Backup and restore into a fresh workspace, then reopen there.
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
    const landing = await call('workspaces', 'create', { name: 'Context Landing' });
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
    const restored = await call('jobs', 'restore', { workspaceId: landingId, jobId: importJobId, artifactId: uploadArtifactId, name: 'Context Restored', expectedHash: preview.body.sha256, confirm: true });
    expect(restored.status).toBe(200);
    const restoredWs = (restored.body.workspaces as Array<Record<string, any>>)[0].id as string;
    expect(restoredWs).not.toBe(workspaceId);

    await openProject(restoredWs, 'Context Harbor');
    await artifacts.getByRole('button', { name: 'Collider experiment: Trading signals' }).click();
    await expect(page.getByText('Collider experiment — provider ollama, model llama-x.')).toBeVisible();
    const restoredContext = page.locator('section[aria-label="Memory context"]');
    await expect(restoredContext.getByText('Tide notes · revision 1 · private')).toBeVisible();
    await restoredContext.getByText('Tide notes · revision 1 · private').click();
    await expect(restoredContext.getByText('Revision B tide observations.')).toBeVisible();
    const restoredProjects = await call('memories', 'list', { workspaceId: restoredWs, kind: 'project' });
    const restoredProjectId = (restoredProjects.body.memories as Array<Record<string, any>>).find(m => m.title === 'Context Harbor')!.id as string;
    const restoredExperimentId = await artifactIdByType(restoredWs, restoredProjectId, 'experiment');
    expect(restoredExperimentId).not.toBe(experimentId);
  } finally {
    await stopApi();
    fake.close();
    fs.rmSync(root, { recursive: true, force: true });
  }
});
