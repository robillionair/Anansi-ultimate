import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M8 WP2 in the real browser application: the project
 * picker stays reliable under races, failures, bounds and
 * pagination. Lists page past the 50-row default instead
 * of truncating; list failures show as errors with retry,
 * never as empty; the pre-send review shows privacy
 * labels; cross-picker duplicates disable with a note;
 * count and content bounds block submission with the
 * bound named; failed review reads retry without losing
 * the selection; delayed responses for a superseded
 * project or version never overwrite the newest choice.
 * Route delays and failures below are gated test
 * fixtures, not sleeps in the application.
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

type Harness = {
  root: string;
  child: ChildProcess | undefined;
  pairing: { endpoint: string; token: string };
  fake: Server;
  fakeUrl: string;
  call: (group: string, operation: string, input: Record<string, unknown>) => Promise<{ status: number; body: Record<string, any> }>;
  startApi: () => Promise<void>;
  stopApi: () => Promise<void>;
  seed: (memberCount: number, oversized: boolean) => Promise<{
    workspaceId: string;
    homeId: string;
    sourceId: string;
    memberIds: string[];
  }>;
  openProject: (workspaceId: string, projectTitle: string) => Promise<void>;
};

async function makeHarness(page: import('@playwright/test').Page): Promise<Harness> {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-collider-picker-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };
  const fake: Server = createServer((req, res) => {
    let body = '';
    req.on('data', c => { body += c; });
    req.on('end', () => {
      const text = collisionJson();
      res.writeHead(200, { 'content-type': 'application/x-ndjson' });
      res.end(JSON.stringify({ model: 'llama-x', message: { role: 'assistant', content: text }, done: true, prompt_eval_count: 9, eval_count: 42 }) + '\n');
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
    await expect(page.getByRole('heading', { name: `Edit project: ${projectTitle}`, exact: true })).toBeVisible();
  }

  async function seed(memberCount: number, oversized: boolean) {
    const created = await call('workspaces', 'create', { name: 'Picker Workspace' });
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
    const draft = (title: string, body: string, extra: Record<string, unknown> = {}) => ({
      title, body,
      privacy: 'private', eventDate: null, validFrom: null, validUntil: null,
      confidence: null, sourceReferences: [], ...extra,
    });
    const home = await call('memories', 'create', { workspaceId, kind: 'project', draft: draft('Project Harbor', 'The home project.') });
    const homeId = home.body.memory.id as string;
    const source = await call('memories', 'create', { workspaceId, kind: 'project', draft: draft('Source Harbor', 'Source project record.') });
    const sourceId = source.body.memory.id as string;
    const memberIds: string[] = [];
    for (let i = 1; i <= memberCount; i += 1) {
      const body = oversized && i === 1 ? 'x'.repeat(8001) : `Member body ${i}.`;
      const mem = await call('memories', 'create', { workspaceId, draft: draft(`Member ${i}`, body, { projectId: sourceId }) });
      memberIds.push(mem.body.memory.id as string);
    }
    return { workspaceId, homeId, sourceId, memberIds };
  }

  return { root, child, pairing, fake, fakeUrl, call, startApi, stopApi, seed, openProject };
}

function colliderLocators(page: import('@playwright/test').Page) {
  const collider = page.locator('section[aria-labelledby="project-collider-heading"]');
  return {
    collider,
    projectGroup: collider.getByRole('group', { name: 'Project context (optional)' }),
    memoryGroup: collider.getByRole('group', { name: 'Memory context (optional)' }),
    projectReview: collider.locator('section[aria-label="Project context to send"]'),
    memoryReview: collider.locator('section[aria-label="Memory context to send"]'),
  };
}

test('picker pages past the default list size: 51 members all appear', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const h = await makeHarness(page);
  try {
    await h.startApi();
    const { workspaceId } = await h.seed(51, false);
    await h.openProject(workspaceId, 'Project Harbor');
    const { collider, projectGroup } = colliderLocators(page);
    await expect(collider.getByRole('heading', { name: 'LEXI-COLLIDER' })).toBeVisible();
    await projectGroup.getByLabel('Saved project').selectOption({ label: 'Source Harbor' });
    await expect(projectGroup.getByLabel(/Member 51 · memory/)).toBeVisible();
    await expect(projectGroup.getByLabel(/Member 1 · memory/)).toBeVisible();
  } finally {
    await h.stopApi();
    h.fake.close();
  }
});

test('picker reviews show privacy and cross-picker duplicates disable with a note', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const h = await makeHarness(page);
  try {
    await h.startApi();
    const { workspaceId, homeId } = await h.seed(2, false);
    // A real transfer mints the approval-row artifact in the home
    // project: approval rows cannot be registered via the API,
    // so this is the honest way to put one in the picker.
    const started = await h.call('ai', 'collider-start', {
      workspaceId, projectId: homeId,
      beamA: 'Trading signals', beamB: 'Mycelium networks',
      controls: { creativity: 'balanced', technicalDepth: 'standard', practicality: 'balanced', innovationDistance: 'moderate', budget: '', preferredOutputType: 'concept' },
      zeroCostLock: true,
    });
    const runId = started.body.runId as string;
    let status = '';
    for (let i = 0; i < 100 && status !== 'complete'; i += 1) {
      const polled = await h.call('ai', 'collider-poll', { workspaceId, runId });
      status = polled.body.status as string;
      if (status !== 'complete' && status !== 'invalid' && status !== 'failed') {
        await new Promise(r => setTimeout(r, 100));
      }
    }
    expect(status).toBe('complete');
    const saved = await h.call('ai', 'collider-save', { workspaceId, runId });
    const expId = saved.body.artifact.artifactId as string;
    const transferred = await h.call('ai', 'collider-transfer', {
      workspaceId, projectId: homeId,
      experimentArtifactId: expId, experimentVersion: 1,
      approvedBrief: 'Route venues by local congestion.', combinationIndex: 0,
    });
    expect(transferred.status).toBe(200);
    await h.openProject(workspaceId, 'Project Harbor');
    const { collider, projectGroup, memoryGroup, projectReview, memoryReview } = colliderLocators(page);
    await expect(collider.getByRole('heading', { name: 'LEXI-COLLIDER' })).toBeVisible();
    // Memory picker review shows the privacy label.
    await memoryGroup.getByLabel(/Member 1 · Revision/).check();
    await expect(memoryReview.getByText('Member 1 · revision 1 · private')).toBeVisible();
    // The same memory is disabled in the project picker with a note.
    await projectGroup.getByLabel('Saved project').selectOption({ label: 'Source Harbor' });
    await expect(projectGroup.getByLabel(/Member 1 · memory/)).toBeDisabled();
    await expect(projectGroup.getByText('already selected in memory context')).toBeVisible();
    // Unchecking re-enables it, and the project review shows privacy too.
    await memoryGroup.getByLabel(/Member 1 · Revision/).uncheck();
    await expect(projectGroup.getByLabel(/Member 1 · memory/)).toBeEnabled();
    await projectGroup.getByLabel(/Member 1 · memory/).check();
    await expect(projectReview.getByText('Member 1 · memory revision 1 · private')).toBeVisible();
    // The home project's approval receipt is counted in a note and
    // never offered: only the experiment and concept checkboxes show.
    await projectGroup.getByLabel('Saved project').selectOption({ label: 'Project Harbor (this project)' });
    await expect(projectGroup.getByText(/artifact\(s\) have ineligible types/)).toBeVisible();
    await expect(projectGroup.locator('input[type="checkbox"]')).toHaveCount(2);
  } finally {
    await h.stopApi();
    h.fake.close();
  }
});

test('picker bounds: 8 items maximum, oversized review blocks submission', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const h = await makeHarness(page);
  try {
    await h.startApi();
    const { workspaceId } = await h.seed(9, true);
    await h.openProject(workspaceId, 'Project Harbor');
    const { collider, projectGroup, projectReview } = colliderLocators(page);
    await expect(collider.getByRole('heading', { name: 'LEXI-COLLIDER' })).toBeVisible();
    await projectGroup.getByLabel('Saved project').selectOption({ label: 'Source Harbor' });
    for (let i = 2; i <= 9; i += 1) {
      await projectGroup.getByLabel(new RegExp(`Member ${i} · memory`)).check();
    }
    await expect(projectGroup.getByText('8 items maximum for project context.')).toBeVisible();
    await expect(projectGroup.getByLabel(/Member 1 · memory/)).toBeDisabled();
    // The oversized member cannot be added at the cap; drop one, add it,
    // and the review warns while Collide Ideas disables.
    await projectGroup.getByLabel(/Member 9 · memory/).uncheck();
    await projectGroup.getByLabel(/Member 1 · memory/).check();
    await expect(projectReview.getByText(/exceeds the 8000-character item limit/)).toBeVisible();
    await expect(collider.getByRole('button', { name: 'Collide Ideas' })).toBeDisabled();
  } finally {
    await h.stopApi();
    h.fake.close();
  }
});

test('picker races: delayed superseded responses never overwrite the newest choice', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const h = await makeHarness(page);
  try {
    await h.startApi();
    const { workspaceId, sourceId } = await h.seed(2, false);
    // A second project, so a switch has somewhere to go.
    const other = await h.call('memories', 'create', {
      workspaceId, kind: 'project',
      draft: { title: 'Other Harbor', body: 'Other project.', privacy: 'private', eventDate: null, validFrom: null, validUntil: null, confidence: null, sourceReferences: [] },
    });
    const otherId = other.body.memory.id as string;
    await h.call('memories', 'create', {
      workspaceId,
      draft: { title: 'Other member', body: 'Other body.', privacy: 'private', eventDate: null, validFrom: null, validUntil: null, confidence: null, sourceReferences: [], projectId: otherId },
    });
    await h.openProject(workspaceId, 'Project Harbor');
    const { collider, projectGroup, projectReview } = colliderLocators(page);
    await expect(collider.getByRole('heading', { name: 'LEXI-COLLIDER' })).toBeVisible();

    // Phase 1: the first project's member list is delayed; switching
    // before it lands must not let it overwrite the second project.
    let oldListFulfilled = false;
    await page.route('**/v1/memories/list', async route => {
      const data = route.request().postDataJSON() as Record<string, unknown>;
      if (data?.projectId === sourceId) {
        await new Promise(r => setTimeout(r, 900));
        oldListFulfilled = true;
      }
      await route.continue();
    });
    await projectGroup.getByLabel('Saved project').selectOption({ label: 'Source Harbor' });
    await projectGroup.getByLabel('Saved project').selectOption({ label: 'Other Harbor' });
    await expect(projectGroup.getByLabel(/Other member · memory/)).toBeVisible();
    await expect.poll(() => oldListFulfilled).toBe(true);
    await expect(projectGroup.getByLabel(/Member 1 · memory/)).toHaveCount(0);
    await expect(projectGroup.getByLabel(/Other member · memory/)).toBeVisible();
    await page.unrouteAll({ behavior: 'wait' });

    // Phase 2: the first review read is delayed past a version
    // switch; the late response must not overwrite the new version.
    const racy = await h.call('memories', 'create', {
      workspaceId,
      draft: { title: 'Racy notes', body: 'Racy revision A.', privacy: 'private', eventDate: null, validFrom: null, validUntil: null, confidence: null, sourceReferences: [], projectId: sourceId },
    });
    const racyId = racy.body.memory.id as string;
    const racyRev = racy.body.memory.revisionId as string;
    await h.call('memories', 'revise', {
      workspaceId, id: racyId, expectedRevision: racyRev,
      draft: { title: 'Racy notes', body: 'Racy revision B.', privacy: 'private', eventDate: null, validFrom: null, validUntil: null, confidence: null, sourceReferences: [], projectId: sourceId },
    });
    await projectGroup.getByLabel('Saved project').selectOption({ label: 'Source Harbor' });
    await expect(projectGroup.getByLabel(/Racy notes · memory/)).toBeVisible();
    let firstReadFulfilled = false;
    let readCount = 0;
    await page.route('**/v1/memories/read', async route => {
      readCount += 1;
      if (readCount === 1) {
        await new Promise(r => setTimeout(r, 1200));
        firstReadFulfilled = true;
      }
      await route.continue();
    });
    await projectGroup.getByLabel(/Racy notes · memory/).check();
    await expect(projectGroup.getByLabel('Version for Racy notes', { exact: true })).toBeVisible();
    await projectGroup.getByLabel('Version for Racy notes', { exact: true }).selectOption({ label: 'Revision 1' });
    await expect(projectReview.getByText('Racy notes · memory revision 1 · private')).toBeVisible();
    await expect.poll(() => firstReadFulfilled).toBe(true);
    await expect(projectReview.getByText('Racy notes · memory revision 1 · private')).toBeVisible();
    await expect(projectReview.getByText('Racy revision B.')).toHaveCount(0);
    await page.unrouteAll({ behavior: 'wait' });
  } finally {
    await h.stopApi();
    h.fake.close();
  }
});

test('picker failures: list and review errors retry without losing the selection', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const h = await makeHarness(page);
  try {
    await h.startApi();
    const { workspaceId } = await h.seed(2, false);
    await h.openProject(workspaceId, 'Project Harbor');
    const { collider, projectGroup, projectReview } = colliderLocators(page);
    await expect(collider.getByRole('heading', { name: 'LEXI-COLLIDER' })).toBeVisible();

    // The member list fails once: an alert with retry, not an empty list.
    let listAborted = false;
    await page.route('**/v1/memories/list', async route => {
      const data = route.request().postDataJSON() as Record<string, unknown>;
      if (!listAborted && data?.projectId) {
        listAborted = true;
        await route.abort();
        return;
      }
      await route.continue();
    });
    await projectGroup.getByLabel('Saved project').selectOption({ label: 'Source Harbor' });
    await expect(projectGroup.getByText(/members could not be listed/)).toBeVisible();
    await expect(projectGroup.getByText('This project has no selectable items yet.')).toHaveCount(0);
    await projectGroup.getByRole('button', { name: 'Retry loading project items' }).click();
    await expect(projectGroup.getByLabel(/Member 1 · memory/)).toBeVisible();
    await page.unrouteAll({ behavior: 'wait' });

    // The review read fails once: an alert with retry, same selection.
    let readAborted = false;
    await page.route('**/v1/memories/read', async route => {
      if (!readAborted) {
        readAborted = true;
        await route.abort();
        return;
      }
      await route.continue();
    });
    await projectGroup.getByLabel(/Member 1 · memory/).check();
    await expect(projectReview.getByText(/could not be read/)).toBeVisible();
    await projectReview.getByRole('button', { name: 'Retry loading the project review' }).click();
    await expect(projectReview.getByText('Member 1 · memory revision 1 · private')).toBeVisible();
    await expect(projectGroup.getByLabel(/Member 1 · memory/)).toBeChecked();
    await page.unrouteAll({ behavior: 'wait' });
  } finally {
    await h.stopApi();
    h.fake.close();
  }
});
