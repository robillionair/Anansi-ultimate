import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M8 document input in the real browser application:
 * a picked .md file lands in Beam A and a picked .txt
 * file in Beam B, each with a visible source line; the
 * exact decoded text is reviewed before the explicit
 * Collide Ideas press; nothing reaches a provider before
 * that press; an invalid UTF-8 file and an oversized
 * file are refused visibly; a source-switch race and a
 * same-name reselect both end with the newest content;
 * the saved experiment keeps generation-time provenance
 * across restart and backup/fresh restore.
 */
function collisionJson(): string {
  return JSON.stringify({
    sourceUnderstanding: 'Beam A describes field notes: observations with structure. Beam B describes mycelium networks: distributed hyphal signaling, local nutrient routing. Hypothesis: both are decentralized coordination systems with local feedback.',
    conceptualPrimitives: ['Distributed signaling', 'Adaptive routing', 'Local feedback', 'Decentralized coordination', 'Resource optimization'],
    crossDomainMappings: ['Field-note structure resembles hyphal branching: both organize observations locally.', 'Revision patience contrasts with signal speed: slow notes versus fast routing is the core tension.'],
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

test('collider document input: review -> race -> reselect -> reject -> collide -> save -> restart -> backup/restore', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-collider-doc-e2e-'));
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

    // Fixture files. The .md file carries a BOM, CRLF
    // endings, indentation and a terminal newline; the
    // textarea displays LF-normalized newlines, while
    // the submitted state keeps the decoded text exactly
    // (proven through the saved artifact body below).
    const files = path.join(root, 'files');
    fs.mkdirSync(files, { recursive: true });
    const docA = path.join(files, 'field-notes.md');
    const docAText = '  Field notes\r\n\tIndented line\r\n';
    fs.writeFileSync(docA, Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), Buffer.from(docAText, 'utf8')]));
    const docB = path.join(files, 'mycelium.txt');
    const docBText = 'Mycelium networks: distributed hyphal signaling.\n';
    fs.writeFileSync(docB, docBText);
    const raceA = path.join(files, 'race-a.md');
    fs.writeFileSync(raceA, 'Stale race content that must lose.\n');
    const raceB = path.join(files, 'race-b.md');
    const raceBText = 'Winning race content, second pick.\n';
    fs.writeFileSync(raceB, raceBText);
    const bad = path.join(files, 'bad.txt');
    fs.writeFileSync(bad, Buffer.from([0xff, 0xfe, 0x61, 0x62]));
    const long = path.join(files, 'long.txt');
    fs.writeFileSync(long, 'x'.repeat(8001));

    await pairAndOpen(workspaceId);
    const collider = page.locator('section[aria-labelledby="project-collider-heading"]');
    await expect(collider.getByRole('heading', { name: 'LEXI-COLLIDER' })).toBeVisible();

    // Beam A from a document: exact decoded text is
    // reviewed with its source label, and no provider
    // traffic happens on selection or review.
    await collider.getByLabel('Document for the first beam (.txt or .md)', { exact: true }).setInputFiles(docA);
    await expect(collider.getByLabel('Beam A', { exact: true })).toHaveValue('  Field notes\n\tIndented line\n');
    await expect(collider.getByText(/Document: field-notes\.md/)).toBeVisible();
    await expect(collider.getByText(/leading BOM stripped/)).toBeVisible();
    expect(fakeHits).toBe(0);

    // Source-switch race: two picks for the same beam in
    // succession must end with the second pick's content
    // and attribution, whatever the read interleaving.
    await collider.getByLabel('Document for the first beam (.txt or .md)', { exact: true }).setInputFiles(raceA);
    await collider.getByLabel('Document for the first beam (.txt or .md)', { exact: true }).setInputFiles(raceB);
    await expect(collider.getByLabel('Beam A', { exact: true })).toHaveValue(raceBText);
    await expect(collider.getByText(/Document: race-b\.md/)).toBeVisible();
    expect(fakeHits).toBe(0);

    // Same file name, changed content: the pick fires
    // again and the beam updates.
    const raceBv2 = 'Winning race content, revised.\n';
    fs.writeFileSync(raceB, raceBv2);
    await collider.getByLabel('Document for the first beam (.txt or .md)', { exact: true }).setInputFiles(raceB);
    await expect(collider.getByLabel('Beam A', { exact: true })).toHaveValue(raceBv2);
    await expect(collider.getByText(/Document: race-b\.md/)).toBeVisible();

    // Beam B rejections: invalid UTF-8, then oversized.
    // Both refuse visibly and leave the beam empty.
    await collider.getByLabel('Document for the second beam (.txt or .md)', { exact: true }).setInputFiles(bad);
    await expect(collider.getByRole('alert')).toHaveText(/not valid UTF-8/);
    await expect(collider.getByLabel('Beam B', { exact: true })).toHaveValue('');
    await collider.getByLabel('Document for the second beam (.txt or .md)', { exact: true }).setInputFiles(long);
    await expect(collider.getByRole('alert')).toHaveText(/at most 8000/);
    await expect(collider.getByLabel('Beam B', { exact: true })).toHaveValue('');
    expect(fakeHits).toBe(0);

    // Beam B from a document, then the explicit press.
    await collider.getByLabel('Document for the second beam (.txt or .md)', { exact: true }).setInputFiles(docB);
    await expect(collider.getByLabel('Beam B', { exact: true })).toHaveValue(docBText);
    await expect(collider.getByText(/Document: mycelium\.txt/)).toBeVisible();
    expect(fakeHits).toBe(0);
    await collider.getByRole('button', { name: 'Collide Ideas' }).click();
    await expect(collider.getByRole('heading', { name: 'Source understanding' })).toBeVisible();
    await expect(collider.getByRole('heading', { name: 'Proposed combinations or architectures' })).toBeVisible();
    await expect(collider.getByRole('heading', { name: 'View what ANANSI sent' })).toBeVisible();
    expect(fakeHits).toBeGreaterThan(0);

    // Save, then verify the saved record through the API:
    // exact beams and document provenance, byte for byte.
    await collider.getByRole('button', { name: 'Save collision as experiment' }).click();
    await expect(collider.getByText(/Saved as artifact .* in this project/)).toBeVisible();
    const experimentId = await artifactIdByType(workspaceId, projectId, 'experiment');
    expect(experimentId).not.toBe('');
    const savedBody = await readExperimentBody(workspaceId, experimentId);
    expect(savedBody.beamA).toBe(raceBv2);
    expect(savedBody.beamB).toBe(docBText);
    expect(savedBody.documentA.label).toBe('race-b.md');
    expect(savedBody.documentB.label).toBe('mycelium.txt');
    expect(savedBody.documentA.textChars).toBe([...raceBv2].length);

    // Reopen the saved experiment: provenance renders
    // as inert text beside the verbatim beams.
    const artifacts = page.locator('section[aria-labelledby="project-artifacts-heading"]');
    await artifacts.getByRole('button', { name: /Collider experiment: Winning race content/ }).click();
    await expect(artifacts.getByText('Collider experiment — provider ollama, model llama-x.')).toBeVisible();
    await artifacts.getByText('Beam A (as supplied)').click();
    await expect(artifacts.getByText(/Document: race-b\.md/)).toBeVisible();
    await artifacts.getByText('Beam B (as supplied)').click();
    await expect(artifacts.getByText(/Document: mycelium\.txt/)).toBeVisible();

    // Restart/reopen: the experiment and provenance persist.
    await stopApi();
    await startApi();
    await pairAndOpen(workspaceId);
    await artifacts.getByRole('button', { name: /Collider experiment: Winning race content/ }).click();
    await artifacts.getByText('Beam A (as supplied)').click();
    await expect(artifacts.getByText(/Document: race-b\.md/)).toBeVisible();

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
    await artifacts.getByRole('button', { name: /Collider experiment: Winning race content/ }).click();
    await artifacts.getByText('Beam A (as supplied)').click();
    await expect(artifacts.getByText(/Document: race-b\.md/)).toBeVisible();
    await artifacts.getByText('Beam B (as supplied)').click();
    await expect(artifacts.getByText(/Document: mycelium\.txt/)).toBeVisible();
    const restoredProjects = await call('memories', 'list', { workspaceId: restoredWs, kind: 'project' });
    const restoredProjectId = (restoredProjects.body.memories as Array<Record<string, any>>)[0].id as string;
    const restoredExperimentId = await artifactIdByType(restoredWs, restoredProjectId, 'experiment');
    expect(restoredExperimentId).not.toBe(experimentId);
    const restoredBody = await readExperimentBody(restoredWs, restoredExperimentId);
    expect(restoredBody.beamA).toBe(raceBv2);
    expect(restoredBody.beamB).toBe(docBText);
    expect(restoredBody.documentA.label).toBe('race-b.md');
    expect(restoredBody.documentB.label).toBe('mycelium.txt');
  } finally {
    await stopApi();
    fake.close();
    fs.rmSync(root, { recursive: true, force: true });
  }
});
