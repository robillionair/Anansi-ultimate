import { test, expect } from '@playwright/test';
import { createServer, type Server } from 'node:http';
import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * SwarmTank v2 Checkpoint 3 in LARP scene 08.
 *
 * (AC-T4) The timeline scrubber selects a round; the persona cards show
 * that round's agent action and its cited evidence or prior event.
 * (AC-T5) Recorded agent interactions and decorative wiring are visually
 * and textually distinct in the LARP view. (AC-T6) Reduced motion and
 * the accessible report view still work on the AI run.
 *
 * The fake Ollama answers non-streaming dispatches with a valid action;
 * round-2 actions cite a round-1 interaction so the recorded-interaction
 * links exist in the matrix.
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

test('LARP scene 08 shows the AI run matrix, scrubber, and recorded interactions', async ({ page }, testInfo) => {
  test.setTimeout(300000);
  await page.setViewportSize({ width: 1500, height: 1000 });
  const rendererErrors: string[] = [];
  page.on('pageerror', error => rendererErrors.push(error.message));
  page.on('console', message => {
    if (message.type() === 'error' && /WebGLProgram|shader/i.test(message.text())) {
      rendererErrors.push(message.text());
    }
  });
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-larp-ai-e2e-'));
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
  try {
    const created = await call('workspaces', 'create', { name: 'LARP AI Workspace' });
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
    void projectId;

    await page.goto('/');
    await page.getByLabel('Local API address').fill(pairing.endpoint);
    await page.getByLabel('Session token').fill(pairing.token);
    await page.getByRole('button', { name: 'Connect to core' }).click();
    await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled();
    await page.getByLabel('Existing workspace').selectOption(workspaceId);
    await page.getByRole('button', { name: 'Open workspace', exact: true }).click();

    // Enter LARP scene 08.
    await page.getByLabel('Toggle LARP mode').click();
    await page.keyboard.press('8');
    const console = page.getByLabel('SwarmTank controls');
    await console.getByLabel('Project', { exact: true }).selectOption({ label: 'Harbor App' });
    await console.getByLabel('SwarmTank description').fill('A scheduling tool for small agencies.');
    await console.getByLabel('SwarmTank pricing').fill('$49 per month');
    await console.getByLabel('SwarmTank monthly price amount').fill('49');
    await console.getByLabel('SwarmTank segment').fill('small agencies');
    await console.getByLabel('SwarmTank industry').fill('services');
    await console.getByRole('radio', { name: /AI simulation/ }).check();
    const ai = console.locator('section[aria-label="AI simulation"]');
    await expect(ai.getByText(/Estimated budget before Start/)).toBeVisible({ timeout: 15000 });
    await ai.getByRole('button', { name: 'Start AI simulation' }).click();
    await expect(ai.getByText(/AI run — Complete \(6 calls/)).toBeVisible({ timeout: 60000 });
    await ai.getByRole('button', { name: 'View AI run' }).first().click();
    const run = ai.locator('section[aria-label="SwarmTank AI run"]');
    await expect(run.getByText(/SwarmTank AI run — Complete/)).toBeVisible();

    // The cinematic AI matrix appears in LARP with a timeline scrubber.
    const matrix = ai.locator('section[aria-label="AI persona matrix"]');
    await expect(matrix).toBeVisible();
    await expect(matrix.getByText(/SWARM \/ AI DECISION MATRIX/)).toBeVisible();
    await expect(matrix.getByText(/Conditional simulation result/)).toBeVisible();
    // Decorative wiring (visual only) and recorded interactions (gold
    // lines from the saved run) are both present and labelled.
    await expect(matrix.locator('.wire-filament').first()).toBeAttached();
    await expect(matrix.locator('.wire-recorded').first()).toBeAttached();
    await expect(matrix.getByText(/WIRING IS VISUAL ONLY/)).toBeVisible();
    await expect(matrix.getByText(/RECORDED INTERACTIONS \(GOLD LINES\) ARE FROM THE SAVED RUN/)).toBeVisible();
    // The scrubber selects round 2; the persona cards show that round's
    // action and its cited prior interaction.
    await matrix.getByLabel('AI matrix timeline round').fill('2');
    await expect(matrix.getByText(/Round 2 of 2/)).toBeVisible();
    await expect(matrix.getByText(/Prior: round-1-agent-1/).first()).toBeVisible();
    await matrix.getByLabel('AI matrix timeline round').fill('1');
    await expect(matrix.getByText(/Round 1 of 2/)).toBeVisible();
    // Selecting a persona card shows its round action and citation.
    await matrix.locator('.larp-matrix-card').first().click();
    await expect(matrix.getByRole('region', { name: 'Selected AI persona' })).toBeVisible();
    await expect(matrix.getByText(/Cited evidence:/)).toBeVisible();
    // Reduced motion: the matrix is in its static view (no animated signal).
    await expect(matrix).toHaveAttribute('data-static', 'true');
    await expect(matrix.locator('.wire-signal').first()).toHaveCSS('animation-name', 'none');
    // The accessible report view (the run list below the matrix) is present.
    await expect(ai.getByText(/SwarmTank AI run — Complete/)).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath('larp-ai-matrix.png') });
    expect(rendererErrors).toEqual([]);
  } finally {
    if (child && child.exitCode === null) {
      const ended = new Promise(resolve => child!.once('exit', resolve));
      child.kill();
      await ended;
    }
    fake.server.close();
  }
});
