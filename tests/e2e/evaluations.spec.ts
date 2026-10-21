import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M6 Slice 3 in the real browser application: transparent
 * concept evaluation & comparison. No model is involved.
 *
 * Open a saved idea set -> select concepts -> assess under
 * the standard rubric -> save the evaluation -> reopen it ->
 * restart -> reopen again -> revise the rubric to v2 ->
 * a second evaluation cites v2 while the first still shows
 * the rubric as originally used.
 */
function concept(name: string) {
  return {
    name, problem: 'A problem.', targetCustomer: 'Cafes', solution: 'A tool.',
    whyUseful: 'Saves time.', differentiationHypothesis: 'A hypothesis.',
    revenueModel: 'Subscription.', engineeringComplexity: 'medium',
    implementationCost: '3-5 weeks', keyRisks: ['Adoption'],
    requiredValidation: 'Interviews.', minimumViableVersion: 'One workflow.',
  };
}

test('evaluation: saved idea set -> assess -> save -> reopen -> rubric v2 -> history unchanged', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-eval-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };

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
  }

  try {
    await startApi();
    const created = await call('workspaces', 'create', { name: 'Eval Workspace' });
    const workspaceId = created.body.workspace.id as string;
    await call('workspaces', 'settings', { id: workspaceId, reducedMotion: true, onboardingComplete: true });
    const project = await call('memories', 'create', {
      workspaceId, kind: 'project',
      draft: {
        title: 'Harbor App', body: 'A test project.', privacy: 'private',
        eventDate: null, validFrom: null, validUntil: null, confidence: null, sourceReferences: [],
      },
    });
    const projectId = project.body.memory.id as string;
    const ideaBody = JSON.stringify({
      topic: 'harbor logistics',
      concepts: ['Alpha', 'Beta', 'Gamma', 'Delta', 'Epsilon'].map(concept),
    });
    await call('artifacts', 'register', {
      workspaceId, projectId, type: 'idea', title: 'Idea set: harbor logistics',
      body: ideaBody, sourceReferences: [], privacy: 'private',
    });

    await pairAndOpen(workspaceId);
    await page.getByRole('button', { name: /Harbor App/ }).first().click();
    await page.getByRole('button', { name: 'Idea set: harbor logistics', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Compare & evaluate' })).toBeVisible();

    // Select two concepts and assess the first under the
    // standard rubric; the second stays unassessed.
    await page.getByLabel('Alpha', { exact: true }).check();
    await page.getByLabel('Gamma', { exact: true }).check();
    await page.getByLabel('Problem clarity score').first().selectOption('5');
    await page.getByLabel('Rationale').first().fill('Very sharp problem statement.');
    await expect(page.getByText(/Preview summary: 5 \(assessed 1 of 5 dimensions\)/)).toBeVisible();

    await page.getByRole('button', { name: 'Save evaluation' }).click();
    await expect(page.getByText(/Evaluation saved as artifact/)).toBeVisible();
    // The standard rubric became a citable artifact, and the
    // evaluation joined the project's artifacts.
    await expect(page.getByRole('button', { name: /Standard evaluation rubric/ })).toBeVisible();
    await page.getByRole('button', { name: /Evaluation: Idea set/ }).click();
    await expect(page.getByText(/Evaluation of idea artifact/)).toBeVisible();
    await expect(page.getByText(/Weighted summary:/).first()).toBeVisible();
    await expect(page.getByText(/not market evidence/).first()).toBeVisible();
    await expect(page.getByText(/MVP feasibility: not assessed/).first()).toBeVisible();

    // Restart: the saved comparison reopens intact.
    await stopApi();
    await startApi();
    await pairAndOpen(workspaceId);
    await page.getByRole('button', { name: /Harbor App/ }).first().click();
    await page.getByRole('button', { name: /Evaluation: Idea set/ }).click();
    await expect(page.getByText(/Problem clarity — weight 1\./)).toBeVisible();
    await expect(page.getByText(/assessed 1 of 5 dimensions/)).toBeVisible();

    // Rubric v2: heavier problem-clarity. History must not move.
    await page.getByRole('button', { name: 'Idea set: harbor logistics', exact: true }).click();
    await page.getByLabel('Rubric', { exact: true }).selectOption({ label: 'Standard evaluation rubric (v1)' });
    await page.getByLabel('Weight').first().fill('3');
    await page.getByRole('button', { name: 'Save rubric as new version' }).click();
    await expect(page.getByText(/Rubric saved as version 2/)).toBeVisible();

    await page.getByLabel('Alpha', { exact: true }).check();
    await page.getByLabel('Problem clarity score').first().selectOption('5');
    await page.getByLabel('Customer specificity score').first().selectOption('4');
    await page.getByRole('button', { name: 'Save evaluation' }).click();
    await expect(page.getByText(/Evaluation saved as artifact/)).toBeVisible();

    // Two evaluations now exist; each must render with the
    // rubric snapshot it was saved under (v1: weight 1, v2:
    // weight 3) — revising the rubric must not rewrite
    // history. The artifact list is ordered by artifact id,
    // so which button comes first varies between runs: open
    // both and check each against its own cited rubric
    // version instead of assuming a position.
    const evaluationButtons = page.getByRole('button', { name: /Evaluation: Idea set/ });
    await expect(evaluationButtons).toHaveCount(2);
    const citedRubricVersion = async () => {
      const text = await page.getByText(/Evaluation of idea artifact/).innerText();
      const match = text.match(/under rubric artifact \w+ \(version (\d+)\)/);
      return match ? Number(match[1]) : -1;
    };
    const seenRubricVersions = new Set<number>();
    for (const index of [0, 1]) {
      await evaluationButtons.nth(index).click();
      // The detail view swaps asynchronously; wait until it
      // cites a rubric version not seen yet (the two
      // evaluations cite different versions by construction).
      await expect
        .poll(
          async () => {
            const v = await citedRubricVersion();
            return v > 0 && !seenRubricVersions.has(v) ? v : -1;
          },
          { timeout: 15000 },
        )
        .toBeGreaterThan(0);
      const rubricVersion = await citedRubricVersion();
      seenRubricVersions.add(rubricVersion);
      await expect(
        page.getByText(new RegExp(`Problem clarity — weight ${rubricVersion === 1 ? '1' : '3'}\\.`)),
      ).toBeVisible();
    }
    expect([...seenRubricVersions].sort()).toEqual([1, 2]);
  } finally {
    await stopApi();
  }
});
