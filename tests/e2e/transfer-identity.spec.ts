import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M8 acceptance repair, browser identity cases, kept
 * independent of transfer.spec.ts so a failure in the
 * version-switch flow there cannot mask these cases:
 * transfer drafts, selections, pending responses and
 * displayed receipts belong to the exact experiment
 * (and version) being viewed. Editing the brief or
 * changing the selection after approval clears the
 * receipt; switching experiments resets the draft to
 * the newly viewed experiment's default brief.
 * No provider is contacted anywhere in this spec.
 */
function experimentBody(firstCombination: string): string {
  return JSON.stringify({
    format: 'anansi-collider-experiment',
    version: 1,
    beamA: 'Trading signals.',
    beamB: 'Mycelium networks.',
    controls: { creativity: 'balanced', technicalDepth: 'standard', practicality: 'balanced', innovationDistance: 'moderate', budget: '', preferredOutputType: 'concept' },
    result: {
      sourceUnderstanding: 'Both beams describe decentralized coordination systems with local feedback.',
      conceptualPrimitives: ['Distributed signaling', 'Adaptive routing', 'Local feedback'],
      crossDomainMappings: ['Order-book depth resembles nutrient gradients.'],
      combinations: [
        { name: firstCombination, description: 'A routing layer that scores venues by local congestion signals. Hypothesis only.' },
        { name: 'Graceful-degradation mesh', description: 'A trade mesh that reroutes through redundant paths when a venue stalls. Hypothesis only.' },
      ],
      feasibility: {
        hypotheses: ['Local congestion signals can rank venues without a central coordinator.'],
        dependencies: ['A venue feed with latency and fill data'],
        unknownCosts: ['Exchange fees under rerouting are unknown.'],
        risks: ['Stale signals could route into worse fills.'],
        testability: 'Replay historical venue data through the router.',
      },
      productization: {
        conceptBrief: 'A routing advisor that proposes venue choices from local signals. Hypothesis, not a product claim.',
        architecture: 'Signal ingest, local scoring, policy engine.',
        roadmap: 'Phase 1: simulator.',
        testPlan: 'Replay-test the router.',
        prototypeProposal: 'A paper prototype plus a simulator notebook.',
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

test('transfer identity: post-approval edits clear the receipt; switching experiments resets the draft', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-transfer-identity-e2e-'));
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
    await page.getByRole('button', { name: 'Harbor App' }).click();
    await expect(page.getByRole('heading', { name: 'Artifacts' })).toBeVisible();
  }

  async function artifactByType(workspaceId: string, projectId: string, type: string) {
    const list = await call('artifacts', 'list', { workspaceId, projectId });
    return (list.body.artifacts as Array<Record<string, any>>).find(a => a.type === type);
  }

  try {
    await startApi();
    const created = await call('workspaces', 'create', { name: 'Transfer Identity Workspace' });
    const workspaceId = created.body.workspace.id as string;
    await call('workspaces', 'settings', { id: workspaceId, reducedMotion: true, onboardingComplete: true });
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
    await call('artifacts', 'register', {
      workspaceId, projectId, type: 'experiment', title: 'Collider experiment: Alpha',
      body: experimentBody('Alpha order router'), sourceReferences: [], privacy: 'private',
    });
    await call('artifacts', 'register', {
      workspaceId, projectId, type: 'experiment', title: 'Collider experiment: Beta',
      body: experimentBody('Beta order router'), sourceReferences: [], privacy: 'private',
    });

    await pairAndOpen(workspaceId);
    const artifacts = page.locator('section[aria-labelledby="project-artifacts-heading"]');
    await artifacts.getByRole('button', { name: 'Collider experiment: Alpha' }).click();
    await expect(page.getByText('Collider experiment — provider ollama, model llama-x.')).toBeVisible();

    // An unapproved draft on Alpha must not survive switching to Beta.
    await page.getByLabel(/Proposed concept brief/).fill('UNAPPROVED DRAFT FOR ALPHA');
    await artifacts.getByRole('button', { name: 'Collider experiment: Beta' }).click();
    await expect(page.getByLabel(/Proposed concept brief/)).not.toHaveValue('UNAPPROVED DRAFT FOR ALPHA');
    await expect(page.getByLabel(/Proposed concept brief/)).toHaveValue(/Beta order router/);

    // Back on Alpha, the draft is the fresh default, not the old text.
    await artifacts.getByRole('button', { name: 'Collider experiment: Alpha' }).click();
    await expect(page.getByLabel(/Proposed concept brief/)).not.toHaveValue('UNAPPROVED DRAFT FOR ALPHA');
    await expect(page.getByLabel(/Proposed concept brief/)).toHaveValue(/Alpha order router/);

    // Approve on Alpha with an edited brief.
    await page.getByLabel(/Proposed concept brief/).fill('Alpha router, user-edited brief.');
    expect(await artifactByType(workspaceId, projectId, 'concept')).toBeUndefined();
    await page.getByRole('button', { name: 'Approve and send to Product Forge' }).click();
    await expect(page.getByRole('button', { name: 'Open in Product Forge' })).toBeVisible();
    const concept = await artifactByType(workspaceId, projectId, 'concept');
    expect(concept).toBeDefined();

    // Editing the brief after approval clears the receipt.
    await page.getByLabel(/Proposed concept brief/).fill('A DIFFERENT BRIEF THAT HAS NOT BEEN APPROVED');
    await expect(page.getByRole('button', { name: 'Approve and send to Product Forge' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Open in Product Forge' })).toHaveCount(0);

    // Changing the selection also clears the receipt and re-defaults the brief.
    await page.getByLabel(/Proposed concept brief/).fill('Alpha router, user-edited brief.');
    await page.getByRole('button', { name: 'Approve and send to Product Forge' }).click();
    await expect(page.getByRole('button', { name: 'Open in Product Forge' })).toBeVisible();
    await page.getByLabel('Graceful-degradation mesh: A trade mesh that reroutes through redundant paths when a venue stalls. Hypothesis only.').click();
    await expect(page.getByRole('button', { name: 'Approve and send to Product Forge' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Open in Product Forge' })).toHaveCount(0);
    await expect(page.getByLabel(/Proposed concept brief/)).toHaveValue(/Graceful-degradation mesh/);

    // Re-approving the original brief returns the original concept, no duplicate.
    await page.getByLabel('Alpha order router: A routing layer that scores venues by local congestion signals. Hypothesis only.').click();
    await page.getByLabel(/Proposed concept brief/).fill('Alpha router, user-edited brief.');
    await page.getByRole('button', { name: 'Approve and send to Product Forge' }).click();
    await expect(page.getByRole('button', { name: 'Open in Product Forge' })).toBeVisible();
    const concepts = (await call('artifacts', 'list', { workspaceId, projectId })).body.artifacts as Array<Record<string, any>>;
    expect(concepts.filter(a => a.type === 'concept').length).toBe(1);
    expect(concepts.filter(a => a.type === 'concept')[0].artifactId).toBe(concept!.artifactId);
  } finally {
    await stopApi();
  }
});
