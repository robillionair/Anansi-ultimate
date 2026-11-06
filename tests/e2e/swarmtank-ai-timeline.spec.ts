import { test, expect } from '@playwright/test';
import { createServer, type Server } from 'node:http';
import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * SwarmTank v2 Checkpoint 3: time, what-if branches, and timeline
 * playback in regular SwarmTank.
 *
 * (AC-T1) A run with a future end date and a chosen step size completes
 * within its run budget; a longer horizon uses coarser steps and says
 * so. (AC-T2) A scheduled competitor launch is observed by agents in
 * the round it falls due. (AC-T3) From a saved round, changing the price
 * and continuing produces a branch that compares against the original
 * timeline. (AC-T4) The timeline scrubber selects a round; the persona
 * cards show that round's agent action and its cited evidence. (AC-T7)
 * Market setup and playback work in regular SwarmTank.
 *
 * The fake Ollama answers non-streaming dispatches with a valid action
 * (round-robin) and reports realistic token usage. Round-2 actions cite
 * a round-1 interaction so the recorded-interaction path is exercised.
 */

const ACTIONS = ['recommend', 'defer', 'request_evidence', 'reject'] as const;

async function startFake(): Promise<{ server: Server; url: string; hits: () => number }> {
  let hits = 0;
  const server = createServer((req, res) => {
    let body = '';
    req.on('data', c => {
      body += c;
    });
    req.on('end', () => {
      hits += 1;
      const callNumber = hits;
      const action = ACTIONS[(callNumber - 1) % ACTIONS.length];
      const payload: Record<string, unknown> = {
        action,
        rationale: `Fake ${action} for call ${callNumber}.`,
      };
      // Round 2 (calls 4-6) cites a round-1 interaction.
      if (callNumber > 3) payload.priorInteractionRef = 'round-1-agent-1';
      const content = JSON.stringify(payload);
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(
        JSON.stringify({
          model: 'llama-x',
          message: { role: 'assistant', content },
          done: true,
          prompt_eval_count: 800,
          eval_count: 512,
        }),
      );
    });
  });
  await new Promise<void>(r => server.listen(0, '127.0.0.1', r));
  const url = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
  return { server, url, hits: () => hits };
}

type Fixture = {
  page: import('@playwright/test').Page;
  fakeHits: () => number;
  cleanup: () => Promise<void>;
};

async function openSwarmtankAi(page: import('@playwright/test').Page): Promise<Fixture> {
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-ai-timeline-e2e-'));
  const fake = await startFake();
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };
  child = spawn(
    path.resolve('target/debug/anansi-api' + (process.platform === 'win32' ? '.exe' : '')),
    ['--port', '0', '--pairing-root', path.join(root, 'pairing')],
    {
      windowsHide: true,
      env: { ...process.env, ANANSI_STORAGE_ROOT: path.join(root, 'storage') },
      stdio: ['ignore', 'pipe', 'ignore'],
    },
  );
  await new Promise<void>((resolve, reject) => {
    let output = '';
    const timer = setTimeout(() => reject(Error('API startup timeout')), 20000);
    child!.stdout!.on('data', data => {
      output += data.toString();
      const match = output.match(/Pairing file: (.+)\r?\n/);
      if (match) {
        pairing = JSON.parse(fs.readFileSync(match[1].trim(), 'utf8'));
        clearTimeout(timer);
        resolve();
      }
    });
  });
  async function call(group: string, operation: string, input: Record<string, unknown>) {
    const response = await fetch(pairing.endpoint + '/v1/' + group + '/' + operation, {
      method: 'POST',
      headers: {
        Origin: 'http://127.0.0.1:1420',
        Authorization: `Bearer ${pairing.token}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ contractVersion: 1, ...input }),
    });
    return { status: response.status, body: (await response.json()) as Record<string, any> };
  }
  const created = await call('workspaces', 'create', { name: 'AI Timeline Workspace' });
  const workspaceId = created.body.workspace.id as string;
  await call('workspaces', 'settings', { id: workspaceId, reducedMotion: true, onboardingComplete: true });
  fs.mkdirSync(path.join(root, 'storage', 'provider-config'), { recursive: true });
  fs.writeFileSync(
    path.join(root, 'storage', 'provider-config', `${workspaceId}.json`),
    JSON.stringify({
      providers: [
        {
          config: { provider: 'ollama', baseUrl: fake.url, credential: null, enabled: true },
          locality: 'local',
          models: [
            {
              id: 'llama-x',
              provider: 'ollama',
              capabilities: { chat: true, streaming: true, vision: false, coding: true, deepReasoning: false, embeddings: false },
              contextWindow: 8192,
              cost: 'free',
              speed: 'balanced',
            },
          ],
        },
      ],
    }),
  );
  const project = await call('memories', 'create', {
    workspaceId,
    kind: 'project',
    draft: { title: 'Harbor App', body: 'A test project.', privacy: 'private' },
  });
  const projectId = project.body.memory.id as string;
  await page.goto('/');
  await page.getByLabel('Local API address').fill(pairing.endpoint);
  await page.getByLabel('Session token').fill(pairing.token);
  await page.getByRole('button', { name: 'Connect to core' }).click();
  await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled();
  await page.getByLabel('Existing workspace').selectOption(workspaceId);
  await page.getByRole('button', { name: 'Open workspace', exact: true }).click();
  await page.getByRole('button', { name: 'Projects', exact: true }).click();
  await page.getByRole('button', { name: 'Harbor App' }).click();
  await expect(page.getByRole('heading', { name: 'Artifacts', exact: true })).toBeVisible();
  const swarmtank = page.locator('section[aria-labelledby="project-swarmtank-heading"]');
  await expect(swarmtank.getByRole('heading', { name: 'SwarmTank', exact: true })).toBeVisible();
  await swarmtank.getByLabel('SwarmTank description').fill('A scheduling tool for small agencies that automates client reporting.');
  await swarmtank.getByLabel('SwarmTank pricing').fill('$49 per month');
  await swarmtank.getByLabel('SwarmTank monthly price amount').fill('49');
  await swarmtank.getByLabel('SwarmTank segment').fill('small agencies');
  await swarmtank.getByLabel('SwarmTank industry').fill('services');
  await swarmtank.getByRole('radio', { name: /AI simulation/ }).check();
  const ai = swarmtank.locator('section[aria-label="AI simulation"]');
  await expect(ai.getByRole('heading', { name: 'AI simulation' })).toBeVisible();
  return {
    page,
    fakeHits: fake.hits,
    cleanup: async () => {
      if (child && child.exitCode === null) {
        const ended = new Promise(resolve => child!.once('exit', resolve));
        child.kill();
        await ended;
      }
      fake.server.close();
    },
  };
}

function futureDate(days: number): string {
  return new Date(Date.now() + days * 86400000).toISOString().slice(0, 10);
}

test('a run with an end date and step size completes, and a long horizon coarsens the step', async ({ page }) => {
  const fx = await openSwarmtankAi(page);
  try {
    const swarmtank = page.locator('section[aria-labelledby="project-swarmtank-heading"]');
    const ai = swarmtank.locator('section[aria-label="AI simulation"]');
    await expect(ai.getByText(/Estimated budget before Start/)).toBeVisible({ timeout: 15000 });
    // A 90-day horizon at monthly steps: 3 steps, no coarsening.
    // (Raise the round budget to 3 first: the default 2 cannot fit 3 steps.)
    await ai.getByLabel('AI rounds').fill('3');
    await ai.getByLabel('AI end date').fill(futureDate(90));
    await ai.getByLabel('AI step size').selectOption('30');
    await expect(ai.getByText(/Time plan: 3 steps of 30 days\./)).toBeVisible({ timeout: 15000 });
    // A 365-day horizon at weekly steps with a 2-round budget: coarsened.
    await ai.getByLabel('AI rounds').fill('2');
    await ai.getByLabel('AI end date').fill(futureDate(365));
    await ai.getByLabel('AI step size').selectOption('7');
    await expect(ai.getByText(/The horizon forced a coarser step/)).toBeVisible({ timeout: 15000 });
    // Back to the fitting horizon and run it (3 agents x 3 rounds = 9 calls).
    await ai.getByLabel('AI rounds').fill('3');
    await ai.getByLabel('AI end date').fill(futureDate(90));
    await ai.getByLabel('AI step size').selectOption('30');
    await ai.getByRole('button', { name: 'Start AI simulation' }).click();
    await expect(ai.getByText(/AI run — Complete \(9 calls/)).toBeVisible({ timeout: 60000 });
    await ai.getByRole('button', { name: 'View AI run' }).first().click();
    const run = ai.locator('section[aria-label="SwarmTank AI run"]');
    await expect(run.getByText(/Horizon:/)).toBeVisible();
    await expect(run.getByText(/90 days, 30-day steps/)).toBeVisible();
  } finally {
    await fx.cleanup();
  }
});

test('a scheduled competitor launch is observed in its round, and the scrubber shows it', async ({ page }) => {
  const fx = await openSwarmtankAi(page);
  try {
    const swarmtank = page.locator('section[aria-labelledby="project-swarmtank-heading"]');
    const ai = swarmtank.locator('section[aria-label="AI simulation"]');
    await expect(ai.getByText(/Estimated budget before Start/)).toBeVisible({ timeout: 15000 });
    await ai.getByLabel('AI end date').fill(futureDate(60));
    await ai.getByLabel('AI step size').selectOption('30');
    // Schedule a competitor launch in round 2.
    await ai.getByLabel('Scheduled event kind').selectOption('competitor_launch');
    await ai.getByLabel('Scheduled event description').fill('A competitor launches a cheaper tier.');
    await ai.getByLabel('Scheduled event round').fill('2');
    await ai.getByRole('button', { name: 'Add event' }).click();
    await expect(ai.getByText(/Round 2: competitor_launch — A competitor launches a cheaper tier\./)).toBeVisible();
    await ai.getByRole('button', { name: 'Start AI simulation' }).click();
    await expect(ai.getByText(/AI run — Complete \(6 calls/)).toBeVisible({ timeout: 60000 });
    await ai.getByRole('button', { name: 'View AI run' }).first().click();
    const run = ai.locator('section[aria-label="SwarmTank AI run"]');
    await expect(run.getByText(/SwarmTank AI run — Complete/)).toBeVisible();
    // Round 2's actions observed the scheduled event (the run view
    // shows the event in the round detail; the API-level proof that
    // the event id is in the agents' memory windows is in the handoff).
    await run.getByLabel('AI timeline round').fill('2');
    await expect(run.getByText(/Events due: event-1/)).toBeVisible();
    // The timeline scrubber selects round 2; the persona cards show
    // that round's actions and the cited prior interaction.
    await run.getByLabel('AI timeline round').fill('2');
    await expect(run.getByText(/Round 2 —/)).toBeVisible();
    await expect(run.getByText(/Prior: round-1-agent-1/).first()).toBeVisible();
    await run.getByRole('button', { name: /Round 1 \(/ }).click();
    await expect(run.getByText(/Round 1 —/)).toBeVisible();
    await expect(run.getByText(/Fake recommend for call 1\./)).toBeVisible();
  } finally {
    await fx.cleanup();
  }
});

test('a price-change branch compares against the original timeline', async ({ page }) => {
  const fx = await openSwarmtankAi(page);
  try {
    const swarmtank = page.locator('section[aria-labelledby="project-swarmtank-heading"]');
    const ai = swarmtank.locator('section[aria-label="AI simulation"]');
    await expect(ai.getByText(/Estimated budget before Start/)).toBeVisible({ timeout: 15000 });
    await ai.getByRole('button', { name: 'Start AI simulation' }).click();
    await expect(ai.getByText(/AI run — Complete \(6 calls/)).toBeVisible({ timeout: 60000 });
    await ai.getByRole('button', { name: 'View AI run' }).first().click();
    const run = ai.locator('section[aria-label="SwarmTank AI run"]');
    await expect(run.getByText(/SwarmTank AI run — Complete/)).toBeVisible();
    // Branch from round 1 with a price rise to $79.
    await run.getByLabel('Branch price amount').fill('79');
    await run.getByRole('button', { name: 'Start branch' }).click();
    await expect(page.getByText(/Branch started/)).toBeVisible();
    // The branch runs round 2 only (3 new calls).
    await expect(ai.getByText(/AI run — Complete \(3 calls/)).toBeVisible({ timeout: 60000 });
    // The branch is a second saved AI run; open it and compare.
    await expect(ai.getByText(/Saved AI runs/)).toBeVisible();
    const branchButtons = ai.getByRole('button', { name: 'View AI run' });
    await expect(branchButtons).toHaveCount(2);
    // The branch title names it: find the list item with "branch of".
    const branchItem = ai.locator('li', { hasText: 'branch of' });
    await branchItem.getByRole('button', { name: 'View AI run' }).click();
    const branchRun = ai.locator('section[aria-label="SwarmTank AI run"]').last();
    await expect(branchRun.getByText(/Branch of run/)).toBeVisible();
    await branchRun.getByRole('button', { name: 'Compare with original' }).click();
    const comparison = ai.locator('section[aria-label="Branch comparison"]');
    await expect(comparison.getByText(/Conditional simulation results — original timeline beside the branch/)).toBeVisible();
    await expect(comparison.getByText(/Branch price: \$79 per month/)).toBeVisible();
    await expect(comparison.getByText(/Round 1 \(shared — identical to the original\)/)).toBeVisible();
    await expect(comparison.getByText(/Round 2 \(diverged\)/)).toBeVisible();
  } finally {
    await fx.cleanup();
  }
});

test('market setup and AI playback compose in regular SwarmTank', async ({ page }) => {
  const fx = await openSwarmtankAi(page);
  try {
    const swarmtank = page.locator('section[aria-labelledby="project-swarmtank-heading"]');
    const market = swarmtank.locator('section[aria-label="Market"]');
    await market.getByLabel('Search country or territory').fill('Estonia');
    await market.getByLabel('Country or territory', { exact: true }).selectOption({ label: 'Estonia (EE) — researched profile' });
    await expect(market.getByText('Researched profile: Estonia (v1)')).toBeVisible();
    const ai = swarmtank.locator('section[aria-label="AI simulation"]');
    await expect(ai.getByText(/Estimated budget before Start/)).toBeVisible({ timeout: 15000 });
    await ai.getByRole('button', { name: 'Start AI simulation' }).click();
    await expect(ai.getByText(/AI run — Complete \(6 calls/)).toBeVisible({ timeout: 60000 });
    await ai.getByRole('button', { name: 'View AI run' }).first().click();
    const run = ai.locator('section[aria-label="SwarmTank AI run"]');
    await expect(run.getByText(/Market: Estonia/)).toBeVisible();
    // Playback: the scrubber moves between rounds.
    await run.getByLabel('AI timeline round').fill('1');
    await expect(run.getByText(/Round 1 of 2/)).toBeVisible();
  } finally {
    await fx.cleanup();
  }
});
