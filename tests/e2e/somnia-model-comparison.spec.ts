import { expect, test } from '@playwright/test';
import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { startSession } from './collider-three-helpers';

test('SOMNIA approves, dispatches and saves a paired model replay in the app', async ({ page }, testInfo) => {
  test.setTimeout(240_000);
  const session = await startSession(page, false);
  try {
    const endpoint = process.env.ANANSI_SOMNIA_LIVE_ENDPOINT;
    const model = endpoint ? (process.env.ANANSI_SOMNIA_LIVE_MODEL ?? 'qwen2.5:3b') : 'llama-x';
    if (endpoint) {
      await writeFile(join(session.storageRoot, 'provider-config', session.workspaceId + '.json'), JSON.stringify({
        providers: [{
          config: { provider: 'ollama', baseUrl: endpoint, credential: null, enabled: true },
          locality: 'local',
          models: [{
            id: model, provider: 'ollama', contextWindow: 4096, cost: 'free', speed: 'balanced',
            capabilities: { chat: true, streaming: true, vision: false, coding: false, deepReasoning: false, embeddings: false },
          }],
        }],
        budget: { maxRequests: 4, maxTotalTokens: 20000 },
      }));
      await session.restartApi();
      await session.pairAndOpen(session.workspaceId);
    }
    await session.openProject();
    const panel = page.getByRole('region', { name: 'SOMNIA model comparison' });
    const fixturePath = process.env.ANANSI_SOMNIA_LIVE_FIXTURE;
    const fixture = endpoint && fixturePath
      ? (JSON.parse(await readFile(fixturePath, 'utf8')).cases as { baseline: string; candidate: string; expected: string }[])[0]
      : { baseline: 'Replay original history.', candidate: 'Replay compact history.', expected: '' };

    const hits = session.fakeHits();
    await panel.getByLabel('Model ID').fill(model);
    await panel.getByLabel('Original replay prompt').fill(fixture.baseline);
    await panel.getByLabel('Proposed replay prompt').fill(fixture.candidate);
    await expect(panel.getByRole('button', { name: 'Run approved model comparison' })).toHaveCount(0);
    await panel.getByRole('button', { name: 'Review model comparison' }).click();
    await expect(panel.getByText('Local only', { exact: false })).toBeVisible();
    expect(session.fakeHits()).toBe(hits);
    await panel.getByRole('button', { name: 'Run approved model comparison' }).click();
    const results = panel.getByRole('region', { name: 'Model comparison results' });
    await expect(results).toBeVisible({ timeout: 100_000 });
    if (!endpoint) {
      await expect(results.getByText('12 input + 4 output = 16 total tokens', { exact: false })).toHaveCount(2);
      expect(session.fakeHits()).toBe(hits + 2);
    }
    await results.getByRole('button', { name: 'Save model comparison to project' }).click();
    await expect(results.getByText('Saved private model comparison to project artifacts.')).toBeVisible();
    await expect(results.getByRole('button', { name: 'Save model comparison to project' })).toBeDisabled();
    const list = await session.call('artifacts', 'list', { workspaceId: session.workspaceId, projectId: session.projectId });
    const matchingArtifacts = list.body.artifacts.filter((item: { title: string }) => item.title === 'SOMNIA model comparison');
    expect(matchingArtifacts.length).toBe(1);
    const artifact = matchingArtifacts[0];
    expect(artifact).toBeTruthy();
    expect(artifact.privacy).toBe('private');
    const read = await session.call('artifacts', 'read', { workspaceId: session.workspaceId, artifactId: artifact.artifactId, version: artifact.version });
    expect(read.status).toBe(200);
    const saved = JSON.parse(read.body.body);
    expect(saved.input).toMatchObject({ baseline: fixture.baseline, candidate: fixture.candidate, model, cloud: false });
    expect(saved.baseline.disclosure.local).toBe(true);
    expect(saved.candidate.disclosure.local).toBe(true);
    if (endpoint) {
      for (const arm of ['baseline', 'candidate']) {
        expect(saved[arm].response.usage.inputTokens).toBeGreaterThan(0);
        expect(saved[arm].response.usage.outputTokens).toBeGreaterThan(0);
      }
      await writeFile(testInfo.outputPath('live-somnia-comparison.json'), JSON.stringify({ expected: fixture.expected, saved }, null, 2));
      await panel.screenshot({ path: testInfo.outputPath('live-somnia-comparison.png') });
    }
  } finally {
    await session.cleanup();
  }
});

test('SOMNIA model comparison can be cancelled mid-flight without second dispatch', async ({ page }) => {
  const session = await startSession(page, false);
  let releaseDispatch!: () => void;
  const hold = new Promise<void>(resolve => { releaseDispatch = resolve; });
  try {
    await session.openProject();
    const panel = page.getByRole('region', { name: 'SOMNIA model comparison' });
    await panel.getByLabel('Model ID').fill('llama-x');
    await panel.getByLabel('Original replay prompt').fill('Prompt 1');
    await panel.getByLabel('Proposed replay prompt').fill('Prompt 2');
    await panel.getByRole('button', { name: 'Review model comparison' }).click();

    await page.route('**/v1/ai/providers-dispatch', async route => {
      await hold;
      await route.continue();
    });

    await panel.getByRole('button', { name: 'Run approved model comparison' }).click();
    await expect(panel.getByText('Running model comparison…')).toBeVisible();
    await expect(panel.getByRole('button', { name: 'Cancel model comparison' })).toBeVisible();

    await panel.getByRole('button', { name: 'Cancel model comparison' }).click();
    await expect(panel.getByText('Model comparison cancelled.')).toBeVisible();
    releaseDispatch();

    await expect(panel.getByRole('region', { name: 'Model comparison results' })).toHaveCount(0);
  } finally {
    releaseDispatch();
    await session.cleanup();
  }
});
