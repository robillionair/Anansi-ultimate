import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/** Independent pre-send consent regressions: immutable reviewed revisions,
 * gated loading and project-switch cleanup. Synthetic loopback only. */
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

for (const scenario of ['review-pinning', 'pending-review'] as const) {
  test('collider memory acceptance: ' + scenario, async ({ page }) => {
    test.setTimeout(240000);
    page.setDefaultTimeout(8000);
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-collider-ctx-e2e-'));
    let child: ChildProcess | undefined;
    let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };
    let fakeHits = 0; let capturedRequest = "";

    const fake: Server = createServer((req, res) => {
      let body = '';
      req.on('data', c => { body += c; });
      req.on('end', () => {
        fakeHits += 1; capturedRequest = body;
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
      expect(project.status).toBe(200);
      await call('memories', 'create', {
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

      const memAId = memA.body.memory.id as string;
      const memARev = memA.body.memory.revisionId as string;

      await openProject(workspaceId, 'Context Harbor');
      const collider = page.locator('section[aria-labelledby="project-collider-heading"]');
      await expect(collider.getByRole('heading', { name: 'LEXI-COLLIDER' })).toBeVisible();

      if (scenario === 'pending-review') {
        let release!: () => void;
        let entered!: () => void;
        const gate = new Promise<void>(resolve => { release = resolve; });
        const responseReady = new Promise<void>(resolve => { entered = resolve; });
        await page.route('**/v1/memories/read', async route => {
          const response = await route.fetch();
          entered();
          await gate;
          await route.fulfill({ response });
        });
        try {
          await collider.getByLabel('Beam A').fill('Trading signals');
          await collider.getByLabel('Beam B').fill('Mycelium networks');
          await collider.getByLabel(/Tide notes/).check();
          await responseReady;
          // A server-controlled response gate proves the review is pending.
          await expect(collider.getByRole('button', { name: 'Collide Ideas' })).toBeDisabled();
          expect(fakeHits).toBe(0);
          await page.getByRole('button', { name: 'Projects', exact: true }).click();
          await page.getByRole('button', { name: 'Other Harbor', exact: true }).click();
          await expect(collider.getByRole('heading', { name: 'LEXI-COLLIDER' })).toBeVisible();
        } finally {
          release();
          await page.unrouteAll({ behavior: 'wait' });
        }
        await expect(collider.locator('section[aria-label="Memory context to send"]')).toHaveCount(0);
        expect(fakeHits).toBe(0);
        // The new project can start a context-free collision, and the late
        // read from the previous project must not leak into its request.
        await collider.getByLabel('Beam A').fill('Trading signals');
        await collider.getByLabel('Beam B').fill('Mycelium networks');
        await collider.getByRole('button', { name: 'Collide Ideas' }).click();
        await expect(collider.getByRole('heading', { name: 'Source understanding' })).toBeVisible();
        expect(fakeHits).toBe(1);
        expect(capturedRequest).not.toContain('Revision B tide observations.');
      } else {
        // The owner reviews B. Another client then creates C; clicking Collide
        // must send the already-reviewed immutable revision, never unseen C.
        await collider.getByLabel(/Tide notes/).check();
        const review = collider.locator('section[aria-label="Memory context to send"]');
        await expect(review.getByText('Tide notes · revision 1')).toBeVisible();
        await review.getByText('Tide notes · revision 1').click();
        await expect(review.getByText('Revision B tide observations.')).toBeVisible();
        const changed = await call('memories', 'revise', {
          workspaceId, id: memAId, expectedRevision: memARev,
          draft: memoryDraft('Tide notes', 'UNREVIEWED REVISION C PRIVATE CANARY'),
        });
        expect(changed.status).toBe(200);
        expect(changed.body.memory.version).toBe(2);
        await expect(review.getByText('Revision B tide observations.')).toBeVisible();
        expect(fakeHits).toBe(0);
        await collider.getByLabel('Beam A').fill('Trading signals');
        await collider.getByLabel('Beam B').fill('Mycelium networks');
        await collider.getByRole('button', { name: 'Collide Ideas' }).click();
        await expect(collider.getByRole('heading', { name: 'Source understanding' })).toBeVisible();
        expect(fakeHits).toBe(1);
        expect(capturedRequest).toContain('Revision B tide observations.');
        expect(capturedRequest).not.toContain('UNREVIEWED REVISION C PRIVATE CANARY');
      }
    } finally {
      await stopApi();
      fake.close();
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

}
