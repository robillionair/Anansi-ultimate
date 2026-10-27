import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M8 Slice 2 in the real browser application: an exact
 * saved collider experiment transfers one selected
 * combination to Product Forge only on explicit
 * approval, with the brief edited by the user; the
 * transferred concept opens the existing Forge spec
 * workflow with a single concept; duplicate submission
 * creates no duplicate; restart and backup/fresh
 * restore keep the pinned provenance. No provider is
 * contacted by opening, selecting or editing; the fake
 * serves only the later, separately approved spec
 * generation.
 */
function experimentBody(): string {
  return JSON.stringify({
    format: 'anansi-collider-experiment',
    version: 1,
    beamA: 'Trading signals.',
    beamB: 'Mycelium networks.',
    controls: { creativity: 'balanced', technicalDepth: 'standard', practicality: 'balanced', innovationDistance: 'moderate', budget: '', preferredOutputType: 'concept' },
    result: {
      sourceUnderstanding: 'Both beams describe decentralized coordination systems with local feedback.',
      conceptualPrimitives: ['Distributed signaling', 'Adaptive routing', 'Local feedback'],
      crossDomainMappings: ['Order-book depth resembles nutrient gradients: both steer flow toward scarcity.', 'Redundant hyphal paths transfer to redundant trade routes that degrade gracefully.'],
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
    },
    provider: 'ollama',
    model: 'llama-x',
    disclosure: {
      provider: 'ollama', model: 'llama-x', endpointHost: '127.0.0.1',
      local: true, routingMode: 'auto', routingReason: 'test',
      messageCount: 3, segmentCount: 4, redactionCount: 0,
      estimatedInputTokens: 100, costClass: 'free', usage: null, attempts: [],
    },
  });
}

function specJson(): string {
  return JSON.stringify({
    title: 'Mycelial order router',
    summary: 'A routing advisor that proposes venue choices from local signals.',
    purpose: 'Route trade flow through venues by local congestion signals.',
    intendedUsers: 'Small trading desks',
    features: ['Score venues by local congestion', 'Reroute when a venue stalls', 'Show the chosen route with its reason'],
    pages: ['index.html'],
    nonGoals: ['No live trading', 'No accounts'],
    dataAndState: 'Signals live in memory only.',
    acceptanceCriteria: ['Open index.html and see venue scores', 'A stalled venue reroutes'],
    openQuestions: ['Which venues first?'],
  });
}

test('collider transfer: preview and edit save nothing -> approve creates one concept -> forge spec -> restart -> backup/restore', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-transfer-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };
  let fakeHits = 0;

  const fake: Server = createServer((req, res) => {
    let body = '';
    req.on('data', c => { body += c; });
    req.on('end', () => {
      fakeHits += 1;
      if (res.destroyed || res.writableEnded) return;
      const text = specJson();
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

  async function artifactByType(workspaceId: string, projectId: string, type: string) {
    const list = await call('artifacts', 'list', { workspaceId, projectId });
    return (list.body.artifacts as Array<Record<string, any>>).find(a => a.type === type);
  }

  try {
    await startApi();
    const created = await call('workspaces', 'create', { name: 'Transfer Workspace' });
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
    const registered = await call('artifacts', 'register', {
      workspaceId, projectId, type: 'experiment', title: 'Collider experiment: Trading signals',
      body: experimentBody(), sourceReferences: [], privacy: 'private',
    });
    const experimentId = registered.body.artifact.artifactId as string;

    await pairAndOpen(workspaceId);
    const artifacts = page.locator('section[aria-labelledby="project-artifacts-heading"]');
    await artifacts.getByRole('button', { name: 'Collider experiment: Trading signals' }).click();
    await expect(page.getByText('Collider experiment — provider ollama, model llama-x.')).toBeVisible();

    // Preview and edit save nothing and contact no provider.
    await expect(page.getByRole('button', { name: 'Approve and send to Product Forge' })).toBeVisible();
    await page.getByLabel(/Proposed concept brief/).fill('Mycelial order router, user-edited: score venues by local congestion.');
    await page.waitForTimeout(500);
    expect(fakeHits).toBe(0);
    expect(await artifactByType(workspaceId, projectId, 'concept')).toBeUndefined();

    // Explicit approval creates exactly one concept.
    await page.getByRole('button', { name: 'Approve and send to Product Forge' }).click();
    await expect(page.getByRole('button', { name: 'Open in Product Forge' })).toBeVisible();
    expect(fakeHits).toBe(0);
    const concept = await artifactByType(workspaceId, projectId, 'concept');
    expect(concept).toBeDefined();
    const conceptId = concept!.artifactId as string;

    // Duplicate submission through the API returns the same artifact.
    const duplicate = await call('ai', 'collider-transfer', {
      workspaceId, projectId, experimentArtifactId: experimentId, experimentVersion: 1,
      combinationIndex: 0, approvedBrief: 'Mycelial order router, user-edited: score venues by local congestion.',
    });
    expect(duplicate.status).toBe(200);
    expect(duplicate.body.alreadyTransferred).toBe(true);
    expect(duplicate.body.artifact.artifactId).toBe(conceptId);
    const concepts = (await call('artifacts', 'list', { workspaceId, projectId })).body.artifacts as Array<Record<string, any>>;
    expect(concepts.filter(a => a.type === 'concept').length).toBe(1);

    // Opening the concept shows the Forge spec workflow with one concept.
    await page.getByRole('button', { name: 'Open in Product Forge' }).click();
    await expect(page.getByText('Transferred concept — pinned to collider experiment')).toBeVisible();
    await expect(page.getByText('Mycelial order router, user-edited: score venues by local congestion.')).toBeVisible();
    const conceptSelect = page.getByLabel('Concept to develop');
    await expect(conceptSelect).toBeVisible();
    expect(await conceptSelect.locator('option').count()).toBe(1);
    await expect(conceptSelect.locator('option').first()).toHaveText('Mycelial order router');

    // Spec generation stays a separate explicit action on the fake provider.
    await page.getByLabel('Purpose — what should it do?').fill('Route trade flow through venues by local congestion signals.');
    await page.getByLabel('Intended users').fill('Small trading desks.');
    await page.getByLabel('Must-have behavior').fill('Score venues, reroute on stall.');
    await page.getByRole('button', { name: 'Generate product specification' }).click();
    await expect(page.getByRole('heading', { name: 'Product specification: Mycelial order router' })).toBeVisible();
    await page.getByRole('button', { name: 'Save specification draft' }).click();
    await expect(page.getByText(/Specification draft saved as artifact/)).toBeVisible();
    expect(fakeHits).toBeGreaterThan(0);

    // Restart/reopen: the concept and its provenance persist.
    await stopApi();
    await startApi();
    await pairAndOpen(workspaceId);
    await artifacts.getByRole('button', { name: 'Concept: Mycelial order router' }).click();
    await expect(page.getByText('Transferred concept — pinned to collider experiment')).toBeVisible();
    await expect(page.getByText('Mycelial order router, user-edited: score venues by local congestion.')).toBeVisible();

    // Backup and restore into a fresh workspace, then reopen the concept.
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
    const landing = await call('workspaces', 'create', { name: 'Transfer Landing' });
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
    const restored = await call('jobs', 'restore', { workspaceId: landingId, jobId: importJobId, artifactId: uploadArtifactId, name: 'Transfer Restored', expectedHash: preview.body.sha256, confirm: true });
    expect(restored.status).toBe(200);
    const restoredWs = (restored.body.workspaces as Array<Record<string, any>>)[0].id as string;
    expect(restoredWs).not.toBe(workspaceId);

    await pairAndOpen(restoredWs);
    await artifacts.getByRole('button', { name: 'Concept: Mycelial order router' }).click();
    await expect(page.getByText('Transferred concept — pinned to collider experiment')).toBeVisible();
    await expect(page.getByText('Mycelial order router, user-edited: score venues by local congestion.')).toBeVisible();
    const restoredProjects = await call('memories', 'list', { workspaceId: restoredWs, kind: 'project' });
    const restoredProjectId = (restoredProjects.body.memories as Array<Record<string, any>>)[0].id as string;
    const restoredConcept = await artifactByType(restoredWs, restoredProjectId, 'concept');
    expect(restoredConcept).toBeDefined();
    expect(restoredConcept!.artifactId).not.toBe(conceptId);
  } finally {
    await stopApi();
    fake.close();
    fs.rmSync(root, { recursive: true, force: true });
  }
});
