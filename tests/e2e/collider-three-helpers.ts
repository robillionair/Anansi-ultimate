import { expect, type Page } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * Shared harness for the Three.js visualization specs:
 * a synthetic local provider, a temp API, and the
 * collide-and-save flow. No real provider, no network
 * beyond loopback.
 *
 * M12 Slice 3: the fake routes on the server-side instruction strings
 * (verified in Rust source, all distinct) so the lifecycle walk can
 * drive provider-backed panels through the same harness: opportunity
 * discovery and Forge spec drafting alongside the collider. The
 * collider instruction ("You are ANANSI's LEXI-COLLIDER engine.")
 * contains neither marker, so existing collider specs are unaffected.
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

// M12 Slice 3: opportunity-set fixture for the lifecycle walk, in the
// opportunities.spec.ts shape. Each concept cites 'supplied-1', the
// single problem source the walk supplies (no memory source).
function opportunityJson(): string {
  const concept = (name: string) => ({
    name,
    problem: 'Dock offices lose supplier quotes in shared inboxes.',
    targetCustomer: 'Small marina operators',
    solution: 'A quote inbox with deadlines.',
    whyUseful: 'It stops missed renewals.',
    differentiationHypothesis: 'Hypothesis: built for shared dock-office inboxes.',
    revenueModel: 'Subscription.',
    engineeringComplexity: 'medium',
    implementationCost: 'about 3-5 weeks for one developer',
    keyRisks: ['Adoption may be slow'],
    requiredValidation: 'Interview five marina operators.',
    minimumViableVersion: 'One inbox end to end.',
    sourceReferences: ['supplied-1'],
    assumptions: ['Quotes arrive mostly by email.'],
  });
  return JSON.stringify({
    concepts: [
      concept('Dock quote inbox'),
      ...Array.from({ length: 5 }, (_, i) => concept(`Filler opportunity ${i}`)),
    ],
  });
}

// M12 Slice 3: Forge spec fixture for the lifecycle walk, in the
// forge.spec.ts shape, titled after the walk's collider concept.
function walkSpecJson(): string {
  return JSON.stringify({
    title: 'Mycelial order router',
    summary: 'A routing layer that scores venues by local congestion signals.',
    purpose: 'Route orders to the best venue from local signals.',
    intendedUsers: 'Small agencies routing orders across venues',
    features: [
      'Score venues by local congestion signals',
      'Propose a ranked venue list for each order',
      'Record the signals behind each proposal',
      'Replay past signals through the scorer',
    ],
    pages: ['index.html'],
    nonGoals: ['No accounts', 'No server', 'No live execution'],
    dataAndState: 'Proposals and signals live in localStorage.',
    acceptanceCriteria: [
      'Open index.html and see a ranked venue list',
      'Reload keeps the recorded signals',
    ],
    openQuestions: ['Which venue feeds are available?'],
  });
}

type Session = {
  cleanup: () => Promise<void>;
  collider: ReturnType<Page['locator']>;
  artifacts: ReturnType<Page['locator']>;
  viz: ReturnType<Page['locator']>;
  fakeHits: () => number;
  artifactIdByType: (workspaceId: string, projectId: string, type: string) => Promise<string>;
  openProject: () => Promise<void>;
  workspaceId: string;
  projectId: string;
  call: (group: string, operation: string, input: Record<string, unknown>) => Promise<{ status: number; body: Record<string, any> }>;
  pairAndOpen: (ws: string) => Promise<void>;
  openWorkspace: (ws: string) => Promise<void>;
};

export async function startSession(page: Page, reducedMotion: boolean): Promise<Session> {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-collider-three-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };
  let fakeHits = 0;

  const fake: Server = createServer((req, res) => {
    let body = '';
    req.on('data', c => { body += c; });
    req.on('end', () => {
      fakeHits += 1;
      // Route on the server-side instruction strings: opportunity
      // discovery and Forge spec drafting share this fake with the
      // collider in the lifecycle walk (M12 Slice 3). Anything else
      // keeps the collider fixture.
      const text = body.includes('opportunity discovery engine')
        ? opportunityJson()
        : body.includes('product architect')
          ? walkSpecJson()
          : collisionJson();
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

  async function call(group: string, operation: string, input: Record<string, unknown>) {
    const response = await fetch(pairing.endpoint + '/v1/' + group + '/' + operation, {
      method: 'POST',
      headers: { Origin: 'http://127.0.0.1:1420', Authorization: `Bearer ${pairing.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ contractVersion: 1, ...input }),
    });
    return { status: response.status, body: await response.json() as Record<string, any> };
  }

  await startApi();
  const created = await call('workspaces', 'create', { name: 'Collider Workspace' });
  const workspaceId = created.body.workspace.id as string;
  await call('workspaces', 'settings', { id: workspaceId, reducedMotion, onboardingComplete: true });
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

  async function openWorkspace(ws: string) {
    await page.goto('/');
    await page.getByLabel('Local API address').fill(pairing.endpoint);
    await page.getByLabel('Session token').fill(pairing.token);
    await page.getByRole('button', { name: 'Connect to core' }).click();
    await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled();
    await page.getByLabel('Existing workspace').selectOption(ws);
    await page.getByRole('button', { name: 'Open workspace', exact: true }).click();
  }

  async function pairAndOpen(ws: string) {
    await openWorkspace(ws);
    await page.getByRole('button', { name: 'Projects', exact: true }).click();
    await page.getByRole('button', { name: 'Harbor App' }).click();
    await expect(page.getByRole('heading', { name: 'Artifacts', exact: true })).toBeVisible();
  }

  await pairAndOpen(workspaceId);
  const collider = page.locator('section[aria-labelledby="project-collider-heading"]');
  const artifacts = page.locator('section[aria-labelledby="project-artifacts-heading"]');
  const viz = artifacts.locator('section[aria-labelledby="collider-visualization-heading"]');

  return {
    cleanup: async () => {
      if (child && child.exitCode === null) {
        const ended = new Promise((resolve) => child!.once('exit', resolve));
        child.kill();
        await ended;
      }
      await new Promise<void>(r => fake.close(() => r()));
    },
    collider,
    artifacts,
    viz,
    fakeHits: () => fakeHits,
    artifactIdByType: async (ws: string, pid: string, type: string) => {
      const list = await call('artifacts', 'list', { workspaceId: ws, projectId: pid });
      const found = (list.body.artifacts as Array<Record<string, any>>).find(a => a.type === type);
      return found!.artifactId as string;
    },
    openProject: async () => {
      await page.getByRole('button', { name: 'Projects', exact: true }).click();
      await page.getByRole('button', { name: 'Harbor App' }).click();
      await expect(page.getByRole('heading', { name: 'Artifacts', exact: true })).toBeVisible();
    },
    workspaceId,
    projectId,
    call,
    pairAndOpen,
    openWorkspace,
  };
}

export async function collideAndSave(
  collider: ReturnType<Page['locator']>,
  beamA: string,
  beamB: string,
) {
  await collider.getByLabel('Beam A', { exact: true }).fill(beamA);
  await collider.getByLabel('Beam B', { exact: true }).fill(beamB);
  await collider.getByRole('button', { name: 'Collide Ideas' }).click();
  await expect(collider.getByRole('heading', { name: 'Source understanding' })).toBeVisible();
  await collider.getByRole('button', { name: 'Save collision as experiment' }).click();
  await expect(collider.getByText(/Saved as artifact .* in this project/)).toBeVisible();
}

/** Click the Beam A hub on the fixed 3D camera (fov 45, distance 900). */
export async function clickBeamAHub(canvas: ReturnType<Page['locator']>) {
  const dims = await canvas.evaluate(el => ({
    w: (el as HTMLCanvasElement).clientWidth,
    h: (el as HTMLCanvasElement).clientHeight,
  }));
  const ring = 0.34 * Math.min(dims.w, dims.h);
  const ndcY = ring / (900 * Math.tan(Math.PI / 8));
  await canvas.click({
    position: { x: dims.w * 0.5, y: dims.h * ((1 - ndcY) / 2) },
  });
}

