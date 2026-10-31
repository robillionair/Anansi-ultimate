import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M8 visualization in the real browser application: a
 * saved experiment opens a visualization of its exact
 * pinned version, derived from the parsed document with
 * pipeline-flow relationships only; selecting a node
 * (list path and canvas path) shows its exact source
 * text and provenance; switching experiments or versions
 * clears the selection; restart and backup/fresh
 * restore reopen the same version with identical nodes;
 * visualization interaction causes no provider traffic;
 * reduced motion renders a static view.
 */
function collisionJson(): string {
  return JSON.stringify({
    sourceUnderstanding: 'Both beams describe bounded local work with feedback.',
    conceptualPrimitives: ['Distributed signaling', 'Adaptive routing', 'Local feedback', 'Decentralized coordination', 'Resource optimization'],
    crossDomainMappings: ['Kitchen timing mirrors orbital timing.', 'Signal speed contrasts with patience.'],
    combinations: [
      { name: 'Mycelial order router', description: 'A routing layer that scores venues by local congestion signals. Hypothesis only.' },
      { name: 'Graceful-degradation mesh', description: 'A trade mesh that reroutes through redundant paths when a venue stalls. Hypothesis only.' },
    ],
    feasibility: {
      hypotheses: ['Local congestion signals can rank venues without a central coordinator.'],
      dependencies: ['A venue feed with latency and fill data', 'A simulator for routing policies'],
      unknownCosts: ['Exchange fees under rerouting are unknown.'],
      risks: ['Stale signals could route into worse fills.'],
      testability: 'Testable in simulation first: replay historical venue data through the router.',
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

test('collider visualization: pinned version -> select -> switch clears -> restart -> backup/restore -> reduced motion', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-collider-viz-e2e-'));
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

  async function pairAndOpen(workspaceId: string) {
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
  }

  async function artifactIdByType(workspaceId: string, projectId: string, type: string) {
    const list = await call('artifacts', 'list', { workspaceId, projectId });
    const found = (list.body.artifacts as Array<Record<string, any>>).find(a => a.type === type);
    return found!.artifactId as string;
  }

  const shotDir = process.env.VIZ_SHOT_DIR;
  async function shot(loc: ReturnType<typeof page.locator>, name: string) {
    if (shotDir) await loc.screenshot({ path: path.join(shotDir, name) });
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

    await pairAndOpen(workspaceId);
    const collider = page.locator('section[aria-labelledby="project-collider-heading"]');
    const artifacts = page.locator('section[aria-labelledby="project-artifacts-heading"]');
    await expect(collider.getByRole('heading', { name: 'LEXI-COLLIDER' })).toBeVisible();

    // First experiment, typed beams only.
    await collider.getByLabel('Beam A', { exact: true }).fill('First beam about kitchens.');
    await collider.getByLabel('Beam B', { exact: true }).fill('First beam about orbitals.');
    await collider.getByRole('button', { name: 'Collide Ideas' }).click();
    await expect(collider.getByRole('heading', { name: 'Source understanding' })).toBeVisible();
    const hitsAfterFirstCollide = fakeHits;
    expect(hitsAfterFirstCollide).toBeGreaterThan(0);
    await collider.getByRole('button', { name: 'Save collision as experiment' }).click();
    await expect(collider.getByText(/Saved as artifact .* in this project/)).toBeVisible();
    const experimentId = await artifactIdByType(workspaceId, projectId, 'experiment');
    expect(experimentId).not.toBe('');

    // Open the saved experiment: the visualization shows
    // the exact pinned version with derived nodes.
    await artifacts.getByRole('button', { name: /Collider experiment: First beam about kitchens/ }).click();
    const viz = artifacts.locator('section[aria-labelledby="collider-visualization-heading"]');
    await expect(viz.getByRole('heading', { name: 'Experiment visualization' })).toBeVisible();
    await expect(viz.getByText(new RegExp(`Visualization of experiment ${experimentId} version 1`))).toBeVisible();
    await expect(viz.getByText(/relationships/)).toBeVisible();
    await expect(viz.getByRole('heading', { name: 'Node list (text alternative)' })).toBeVisible();
    await expect(viz.getByRole('button', { name: 'Distributed signaling' })).toBeVisible();
    await expect(viz.getByRole('button', { name: 'Mycelial order router' })).toBeVisible();

    // List selection shows the node's exact source text
    // and pinned provenance; no provider traffic follows.
    await viz.getByRole('button', { name: 'Distributed signaling' }).click();
    await expect(viz.getByRole('heading', { name: 'Selected: Conceptual primitives — Distributed signaling' })).toBeVisible();
    await expect(viz.getByText(new RegExp(`Experiment ${experimentId} version 1`)).first()).toBeVisible();
    expect(fakeHits).toBe(hitsAfterFirstCollide);

    // Canvas selection: pause motion for determinism, then
    // click the Beam A hub (top of the ring).
    const pause = viz.getByRole('button', { name: 'Pause motion' });
    if (await pause.count() > 0) await pause.click();
    const canvas = viz.locator('canvas');
    const dims = await canvas.evaluate(el => ({
      w: (el as HTMLCanvasElement).clientWidth,
      h: (el as HTMLCanvasElement).clientHeight,
    }));
    await canvas.click({
      position: {
        x: dims.w * 0.5,
        y: dims.h * 0.5 - 0.34 * Math.min(dims.w, dims.h),
      },
    });
    await expect(viz.getByRole('heading', { name: 'Selected: Beam A — Beam A' })).toBeVisible();
    await expect(viz.locator('pre', { hasText: 'First beam about kitchens.' })).toBeVisible();
    expect(fakeHits).toBe(hitsAfterFirstCollide);
    // VIS-02 evidence (opt-in): the paused canvas frame,
    // with node labels drawn in contrasting white.
    await shot(canvas, 'viz-paused.png');

    // Second experiment, then switch back: the selection
    // from the second view must not survive the switch.
    await page.getByRole('button', { name: 'Projects', exact: true }).click();
    await page.getByRole('button', { name: 'Harbor App' }).click();
    await collider.getByLabel('Beam A', { exact: true }).fill('Second run beam about gardens.');
    await collider.getByLabel('Beam B', { exact: true }).fill('Second run beam about tides.');
    await collider.getByRole('button', { name: 'Collide Ideas' }).click();
    await expect(collider.getByRole('heading', { name: 'Source understanding' })).toBeVisible();
    await collider.getByRole('button', { name: 'Save collision as experiment' }).click();
    await expect(collider.getByText(/Saved as artifact .* in this project/)).toBeVisible();
    await artifacts.getByRole('button', { name: /Collider experiment: Second run beam about gardens/ }).click();
    await expect(viz.getByRole('heading', { name: 'Experiment visualization' })).toBeVisible();
    await viz.getByRole('button', { name: 'Distributed signaling' }).click();
    await expect(viz.getByRole('heading', { name: 'Selected: Conceptual primitives — Distributed signaling' })).toBeVisible();
    await artifacts.getByRole('button', { name: /Collider experiment: First beam about kitchens/ }).click();
    await expect(viz.getByText(new RegExp(`Visualization of experiment ${experimentId} version 1`))).toBeVisible();
    await expect(viz.getByRole('heading', { name: /^Selected:/ })).toHaveCount(0);

    // Restart/reopen: the same version visualizes again.
    await stopApi();
    await startApi();
    await pairAndOpen(workspaceId);
    await artifacts.getByRole('button', { name: /Collider experiment: First beam about kitchens/ }).click();
    await expect(viz.getByRole('heading', { name: 'Experiment visualization' })).toBeVisible();
    await expect(viz.getByRole('button', { name: 'Distributed signaling' })).toBeVisible();
    await viz.getByRole('button', { name: 'Mycelial order router' }).click();
    await expect(viz.getByRole('heading', { name: 'Selected: Proposed combinations — Mycelial order router' })).toBeVisible();

    // Backup and restore into a fresh workspace, then
    // reopen the restored experiment's visualization.
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
    const artifactBytes = job!.artifactBytes as number;
    const chunks: number[] = [];
    for (let offset = 0; offset < artifactBytes;) {
      const chunk = await call('jobs', 'artifact', { workspaceId, jobId, artifactId: job!.artifactId as string, offset });
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
    await artifacts.getByRole('button', { name: /Collider experiment: First beam about kitchens/ }).click();
    await expect(viz.getByRole('heading', { name: 'Experiment visualization' })).toBeVisible();
    await expect(viz.getByRole('button', { name: 'Distributed signaling' })).toBeVisible();
    await viz.getByRole('button', { name: 'Kitchen timing mirrors orbital timing.' }).click();
    await expect(viz.getByRole('heading', { name: /^Selected: Cross-domain mappings —/ })).toBeVisible();

    // Reduced motion: a static view with no animation
    // control, on the same restored version.
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.getByRole('button', { name: 'Projects', exact: true }).click();
    await page.getByRole('button', { name: 'Harbor App' }).click();
    await artifacts.getByRole('button', { name: /Collider experiment: First beam about kitchens/ }).click();
    await expect(viz.getByText('Motion reduced (system setting) — static view.')).toBeVisible();
    await expect(viz.getByRole('button', { name: 'Pause motion' })).toHaveCount(0);
    await expect(viz.getByRole('button', { name: 'Resume motion' })).toHaveCount(0);
    // VIS-02 evidence (opt-in): the reduced-motion frame.
    await shot(canvas, 'viz-reduced-motion.png');
  } finally {
    await stopApi();
    await new Promise<void>(r => fake.close(() => r()));
  }
});
