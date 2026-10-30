import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M8 WP4 integrated acceptance in the real browser
 * application against the real API and a synthetic
 * provider: project selection -> exact item/version
 * review -> collision -> save -> approved Forge transfer
 * -> restart/reopen -> backup -> fresh restore -> reopen
 * -> second backup/fresh restore, with negatives: a
 * source edit after save never rewrites the snapshot,
 * and an edited brief after approval is not approval.
 * The fake answers collider prompts with a collision
 * document and every other prompt with a product
 * specification, distinguished by the beam text.
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

test('integrated: selection -> review -> collision -> save -> transfer -> restart -> restore -> restore', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-collider-integrated-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };
  let fakeHits = 0;

  const fake: Server = createServer((req, res) => {
    let body = '';
    req.on('data', c => { body += c; });
    req.on('end', () => {
      fakeHits += 1;
      const text = body.includes('Trading signals') ? collisionJson() : specJson();
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

  function writeProviderConfig(workspaceId: string) {
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
    await expect(page.getByRole('heading', { name: `Edit project: ${projectTitle}`, exact: true })).toBeVisible();
  }

  async function backupAndRestore(workspaceId: string, landingName: string) {
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
    const landing = await call('workspaces', 'create', { name: `${landingName} Landing` });
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
    const restored = await call('jobs', 'restore', { workspaceId: landingId, jobId: importJobId, artifactId: uploadArtifactId, name: landingName, expectedHash: preview.body.sha256, confirm: true });
    expect(restored.status).toBe(200);
    const restoredWs = (restored.body.workspaces as Array<Record<string, any>>)[0].id as string;
    expect(restoredWs).not.toBe(workspaceId);
    writeProviderConfig(restoredWs);
    return restoredWs;
  }

  async function forgeSpecFromConcept() {
    await page.getByRole('button', { name: 'Open in Product Forge' }).click();
    await generateSpec();
  }

  async function generateSpec() {
    await expect(page.getByText('Transferred concept — pinned to collider experiment')).toBeVisible();
    const conceptSelect = page.getByLabel('Concept to develop');
    await expect(conceptSelect).toBeVisible();
    expect(await conceptSelect.locator('option').count()).toBe(1);
    await page.getByLabel('Purpose — what should it do?').fill('Route trade flow through venues by local congestion signals.');
    await page.getByLabel('Intended users').fill('Small trading desks.');
    await page.getByLabel('Must-have behavior').fill('Score venues, reroute on stall.');
    await page.getByRole('button', { name: 'Generate product specification' }).click();
    await expect(page.getByRole('heading', { name: 'Product specification: Mycelial order router' })).toBeVisible();
  }

  // The transfer panel is session-scoped: it shows the Forge
  // button only after an approval in the current session, so a
  // restored experiment reaches Forge by re-approving the same
  // brief, which is idempotent (alreadyTransferred, same
  // concept) rather than a duplicate.
  async function reapproveAndForgeSpec(brief: string) {
    await page.getByLabel(/Proposed concept brief/).fill(brief);
    await page.getByRole('button', { name: 'Approve and send to Product Forge' }).click();
    await expect(page.getByRole('button', { name: 'Open in Product Forge' })).toBeVisible();
    await page.getByRole('button', { name: 'Open in Product Forge' }).click();
    await generateSpec();
  }

  try {
    await startApi();
    const created = await call('workspaces', 'create', { name: 'Integrated Workspace' });
    const workspaceId = created.body.workspace.id as string;
    await call('workspaces', 'settings', { id: workspaceId, reducedMotion: true, onboardingComplete: true });
    writeProviderConfig(workspaceId);
    const draft = (title: string, body: string, extra: Record<string, unknown> = {}) => ({
      title, body,
      privacy: 'private', eventDate: null, validFrom: null, validUntil: null,
      confidence: null, sourceReferences: [], ...extra,
    });
    const home = await call('memories', 'create', { workspaceId, kind: 'project', draft: draft('Project Harbor', 'The home project.') });
    const homeId = home.body.memory.id as string;
    void homeId;
    const source = await call('memories', 'create', { workspaceId, kind: 'project', draft: draft('Source Harbor', 'Source project record.') });
    const sourceId = source.body.memory.id as string;
    const mem = await call('memories', 'create', { workspaceId, draft: draft('Tide notes', 'Revision A tide observations.', { projectId: sourceId }) });
    const memId = mem.body.memory.id as string;
    const memRev = mem.body.memory.revisionId as string;
    const revised = await call('memories', 'revise', {
      workspaceId, id: memId, expectedRevision: memRev,
      draft: draft('Tide notes', 'Revision B tide observations.', { projectId: sourceId }),
    });
    expect(revised.status).toBe(200);
    await call('artifacts', 'register', {
      workspaceId, projectId: sourceId,
      type: 'note', title: 'Harbor summary', body: 'Summary draft one.',
      sourceReferences: [], privacy: 'private',
    });

    await openProject(workspaceId, 'Project Harbor');
    const collider = page.locator('section[aria-labelledby="project-collider-heading"]');
    await expect(collider.getByRole('heading', { name: 'LEXI-COLLIDER' })).toBeVisible();
    const projectGroup = collider.getByRole('group', { name: 'Project context (optional)' });
    const review = collider.locator('section[aria-label="Project context to send"]');
    const artifacts = page.locator('section[aria-labelledby="project-artifacts-heading"]');

    // Selection -> exact item/version review -> collision -> save.
    await projectGroup.getByLabel('Saved project').selectOption({ label: 'Source Harbor' });
    await projectGroup.getByLabel(/Tide notes · memory/).check();
    await projectGroup.getByLabel('Version for Tide notes', { exact: true }).selectOption({ label: 'Revision 1' });
    await expect(review.getByText('Tide notes · memory revision 1 · private')).toBeVisible();
    await review.getByText('Tide notes · memory revision 1 · private').click();
    await expect(review.getByText('Revision A tide observations.')).toBeVisible();
    await projectGroup.getByLabel(/Harbor summary · artifact/).check();
    await expect(review.getByText('Harbor summary · artifact version 1 · private')).toBeVisible();
    await collider.getByLabel('Beam A').fill('Trading signals');
    await collider.getByLabel('Beam B').fill('Mycelium networks');
    await collider.getByRole('button', { name: 'Collide Ideas' }).click();
    await expect(collider.getByRole('heading', { name: 'Source understanding' })).toBeVisible();
    expect(fakeHits).toBe(1);
    await collider.getByRole('button', { name: 'Save collision as experiment' }).click();
    await expect(collider.getByText(/Saved as artifact .* in this project/)).toBeVisible();

    // Negative 1: a source edit after save never rewrites the snapshot.
    const memNow = await call('memories', 'list', { workspaceId, id: memId });
    const memRevNow = (memNow.body.memories as Array<Record<string, any>>)[0].revisionId as string;
    const edited = await call('memories', 'revise', {
      workspaceId, id: memId, expectedRevision: memRevNow,
      draft: draft('Tide notes', 'Revision C tide observations.', { projectId: sourceId }),
    });
    expect(edited.status).toBe(200);
    await artifacts.getByRole('button', { name: 'Collider experiment: Trading signals' }).click();
    await expect(page.getByText('Collider experiment — provider ollama, model llama-x.')).toBeVisible();
    const savedContext = page.locator('section[aria-label="Project context"]');
    await expect(savedContext.getByText('Tide notes · memory revision 1 · private')).toBeVisible();
    await savedContext.getByText('Tide notes · memory revision 1 · private').click();
    await expect(savedContext.getByText('Revision A tide observations.')).toBeVisible();
    await expect(savedContext.getByText('Revision C tide observations.')).toHaveCount(0);

    // Approved Forge transfer, with the unapproved-edit negative.
    const approvedBrief = 'Mycelial order router, user-edited: score venues by local congestion.';
    await page.getByLabel(/Proposed concept brief/).fill(approvedBrief);
    await page.getByRole('button', { name: 'Approve and send to Product Forge' }).click();
    await expect(page.getByRole('button', { name: 'Open in Product Forge' })).toBeVisible();
    expect(fakeHits).toBe(1);
    await page.getByLabel(/Proposed concept brief/).fill('A DIFFERENT BRIEF THAT HAS NOT BEEN APPROVED');
    await expect(page.getByRole('button', { name: 'Approve and send to Product Forge' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Open in Product Forge' })).toHaveCount(0);
    await page.getByLabel(/Proposed concept brief/).fill(approvedBrief);
    await page.getByRole('button', { name: 'Approve and send to Product Forge' }).click();
    await expect(page.getByRole('button', { name: 'Open in Product Forge' })).toBeVisible();

    // Forge spec from the transferred concept, then restart/reopen.
    await forgeSpecFromConcept();
    await page.getByRole('button', { name: 'Save specification draft' }).click();
    await expect(page.getByText(/Specification draft saved as artifact/)).toBeVisible();
    await stopApi();
    await startApi();
    await openProject(workspaceId, 'Project Harbor');
    await artifacts.getByRole('button', { name: 'Concept: Mycelial order router' }).click();
    await expect(page.getByText('Transferred concept — pinned to collider experiment')).toBeVisible();
    await expect(page.locator('section[aria-label="Transferred concept"]').locator('pre').filter({ hasText: approvedBrief }).first()).toBeVisible();

    // First backup/fresh restore: experiment snapshot and concept survive,
    // and the restored concept stays Forge-admissible.
    const restoredWs1 = await backupAndRestore(workspaceId, 'Integrated Restored 1');
    await openProject(restoredWs1, 'Project Harbor');
    await artifacts.getByRole('button', { name: 'Collider experiment: Trading signals' }).click();
    await expect(page.getByText('Collider experiment — provider ollama, model llama-x.')).toBeVisible();
    const restoredContext1 = page.locator('section[aria-label="Project context"]');
    await expect(restoredContext1.getByText('Tide notes · memory revision 1 · private')).toBeVisible();
    await restoredContext1.getByText('Tide notes · memory revision 1 · private').click();
    await expect(restoredContext1.getByText('Revision A tide observations.')).toBeVisible();
    // Re-approving the same brief on the restored experiment is
    // idempotent; Forge spec generation then runs against the
    // restored concept (provider config written for the restored
    // id at restore).
    await reapproveAndForgeSpec(approvedBrief);
    await artifacts.getByRole('button', { name: 'Concept: Mycelial order router' }).click();
    await expect(page.getByText('Transferred concept — pinned to collider experiment')).toBeVisible();
    await expect(page.locator('section[aria-label="Transferred concept"]').locator('pre').filter({ hasText: approvedBrief }).first()).toBeVisible();

    // Second backup/fresh restore: the concept stays Forge-admissible.
    const restoredWs2 = await backupAndRestore(restoredWs1, 'Integrated Restored 2');
    await openProject(restoredWs2, 'Project Harbor');
    await artifacts.getByRole('button', { name: 'Collider experiment: Trading signals' }).click();
    await expect(page.getByText('Collider experiment — provider ollama, model llama-x.')).toBeVisible();
    const restoredContext2 = page.locator('section[aria-label="Project context"]');
    await expect(restoredContext2.getByText('Tide notes · memory revision 1 · private')).toBeVisible();
    await reapproveAndForgeSpec(approvedBrief);
    await artifacts.getByRole('button', { name: 'Concept: Mycelial order router' }).click();
    await expect(page.getByText('Transferred concept — pinned to collider experiment')).toBeVisible();
  } finally {
    await stopApi();
    fake.close();
    fs.rmSync(root, { recursive: true, force: true });
  }
});
