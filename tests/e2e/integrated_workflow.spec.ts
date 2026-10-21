import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * The integrated M6 application workflow in the real browser
 * app, against a synthetic loopback provider (application
 * integration evidence, NOT real-model behavior):
 *
 *   project -> generate ideas -> save idea set -> compare &
 *   evaluate under the standard rubric -> workspace backup ->
 *   download the backup file -> import it -> restore into a
 *   fresh workspace -> restart -> reopen the restored
 *   workspace -> the restored evaluation renders AND resolves
 *   to the RESTORED idea set and rubric (its references were
 *   remapped at restore, never left pointing at originals).
 */
function ideaJson(names: string[]): string {
  const concepts = names.map((name) => ({
    name,
    problem: 'A problem worth solving.',
    targetCustomer: 'Independent cafes',
    solution: 'A focused tool.',
    whyUseful: 'It saves time weekly.',
    differentiationHypothesis: 'Hypothesis: incumbents ignore this niche.',
    revenueModel: 'Subscription.',
    engineeringComplexity: 'medium',
    implementationCost: 'about 3-5 weeks for one developer',
    keyRisks: ['Adoption may be slow'],
    requiredValidation: 'Interview five customers.',
    minimumViableVersion: 'One workflow end to end.',
  }));
  return JSON.stringify({ concepts });
}

const NAMES = ['Alpha', 'Beta', 'Gamma', 'Delta', 'Epsilon', 'Zeta'];

test('integrated workflow: project -> ideas -> evaluation -> backup -> fresh restore -> restart -> restored evaluation resolves', async ({ page }) => {
  test.setTimeout(600000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-integrated-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };

  const text = ideaJson(NAMES);
  const third = Math.ceil(text.length / 3);
  const chunks = [text.slice(0, third), text.slice(third, third * 2), text.slice(third * 2)];
  const fake: Server = createServer((req, res) => {
    let body = '';
    req.on('data', (c) => { body += c; });
    req.on('end', () => {
      res.writeHead(200, { 'content-type': 'application/x-ndjson' });
      for (const chunk of chunks) {
        res.write(JSON.stringify({ model: 'llama-x', message: { role: 'assistant', content: chunk }, done: false }) + '\n');
      }
      res.end(JSON.stringify({ model: 'llama-x', message: { role: 'assistant', content: '' }, done: true, prompt_eval_count: 9, eval_count: 42 }) + '\n');
    });
  });
  await new Promise<void>((r) => fake.listen(0, '127.0.0.1', r));
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

  async function pair() {
    await page.goto('/');
    await page.getByLabel('Local API address').fill(pairing.endpoint);
    await page.getByLabel('Session token').fill(pairing.token);
    await page.getByRole('button', { name: 'Connect to core' }).click();
    await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled();
  }

  async function openWorkspace(label: string) {
    await page.getByLabel('Existing workspace').selectOption({ label });
    await page.getByRole('button', { name: 'Open workspace', exact: true }).click();
    await page.getByRole('button', { name: 'Projects' }).click();
  }

  async function artifactByType(workspaceId: string, projectId: string, type: string) {
    const list = await call('artifacts', 'list', { workspaceId, projectId });
    const found = (list.body.artifacts as Record<string, any>[]).find((a) => a.type === type);
    if (!found) throw new Error(`no ${type} artifact in ${workspaceId}/${projectId}`);
    return found;
  }

  try {
    await startApi();
    const created = await call('workspaces', 'create', { name: 'Workflow Workspace' });
    const workspaceId = created.body.workspace.id as string;
    await call('workspaces', 'settings', { id: workspaceId, reducedMotion: true, onboardingComplete: true });
    await call('memories', 'create', {
      workspaceId,
      draft: {
        title: 'Field notes', body: 'The approved integrated context phrase.',
        privacy: 'private', eventDate: null, validFrom: null, validUntil: null,
        confidence: null, sourceReferences: [],
      },
    });
    // Owner-side provider configuration via the persisted file
    // (the desktop-equivalent step; see ideas.spec.ts).
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

    // ---- Project -> ideas -> saved artifact ----------------
    await pair();
    await openWorkspace('Workflow Workspace');
    await page.getByRole('button', { name: 'New project' }).click();
    await page.getByLabel('Project title').fill('Harbor App');
    await page.getByLabel('Description & objectives').fill('A test project.');
    await page.getByRole('button', { name: 'Save project' }).click();
    await expect(page.getByRole('heading', { name: 'Idea generation' })).toBeVisible();
    await page.getByLabel('Industry or topic').fill('harbor logistics');
    await page.getByLabel('Intended customer').fill('marina operators');
    await page.getByLabel('Available budget').fill('small');
    await page.getByLabel('Preferred technology').fill('rust');
    await page.getByLabel('Business or personal objective').fill('find a product');
    await page.getByLabel(/Field notes/).check();
    await page.getByRole('button', { name: 'Generate product ideas' }).click();
    await expect(page.getByText('1. Alpha')).toBeVisible();
    await page.getByRole('button', { name: 'Save idea set as artifact' }).click();
    await expect(page.getByText(/Saved as artifact/)).toBeVisible();

    const projectList = await call('memories', 'list', { workspaceId, includeArchived: true });
    const projectMemory = (projectList.body.memories as Record<string, any>[]).find((m) => m.title === 'Harbor App');
    if (!projectMemory) throw new Error('project memory missing');
    const projectId = projectMemory.id as string;
    const originalIdea = await artifactByType(workspaceId, projectId, 'idea');

    // ---- Comparison / evaluation ---------------------------
    await page.getByRole('button', { name: /Idea set: harbor logistics/ }).click();
    await expect(page.getByRole('heading', { name: 'Compare & evaluate' })).toBeVisible();
    await page.getByLabel('Alpha', { exact: true }).check();
    await page.getByLabel('Gamma', { exact: true }).check();
    await page.getByLabel('Problem clarity score').first().selectOption('5');
    await page.getByLabel('Rationale').first().fill('Very sharp problem statement.');
    await expect(page.getByText(/Preview summary: 5 \(assessed 1 of 5 dimensions\)/)).toBeVisible();
    await page.getByRole('button', { name: 'Save evaluation' }).click();
    await expect(page.getByText(/Evaluation saved as artifact/)).toBeVisible();
    await expect(page.getByRole('button', { name: /Standard evaluation rubric/ })).toBeVisible();
    const originalRubric = await artifactByType(workspaceId, projectId, 'rubric');
    const originalEvaluation = await artifactByType(workspaceId, projectId, 'evaluation');

    // ---- Backup: create, download, import, restore -------
    await page.getByRole('button', { name: 'Backups' }).click();
    await page.getByLabel(/I consent to a plaintext copy/).check();
    await page.getByRole('button', { name: 'Create backup' }).click();
    const backupsRegion = page.getByRole('region', { name: 'Back up and recover' });
    await expect.poll(() => backupsRegion.getAttribute('data-created-backup-id')).toMatch(/^[a-f0-9]{32}$/);
    const backupJobId = (await backupsRegion.getAttribute('data-created-backup-id'))!;
    const backupRow = page.locator(`[data-backup-job="${backupJobId}"]`);
    await expect.poll(async () => {
      await page.getByRole('button', { name: 'Refresh backups' }).click();
      return backupRow.innerText();
    }, { timeout: 120000 }).toContain('Completed');

    await page.getByRole('button', { name: 'Jobs and export' }).click();
    const jobRow = page.locator(`[data-job-id="${backupJobId}"]`);
    await expect(jobRow).toContainText('Completed');
    const downloadWait = page.waitForEvent('download');
    await jobRow.getByRole('button', { name: 'Download complete export', exact: true }).click();
    const download = await downloadWait;
    const backupFile = path.join(root, 'integrated-backup.tar');
    await download.saveAs(backupFile);
    expect(await download.failure()).toBeNull();

    await page.getByRole('button', { name: 'Backups' }).click();
    await page.getByLabel(/I consent to a plaintext copy/).check();
    await page.getByLabel('Saved backup file', { exact: true }).setInputFiles(backupFile);
    await page.getByRole('button', { name: 'Import saved backup' }).click();
    await expect(page.getByRole('status').filter({ hasText: 'Saved backup verified.' })).toBeVisible();
    await expect.poll(() => backupsRegion.getAttribute('data-created-backup-id')).not.toBe(backupJobId);
    const importedJobId = (await backupsRegion.getAttribute('data-created-backup-id'))!;
    const importedRow = page.locator(`[data-backup-job="${importedJobId}"]`);
    await importedRow.getByRole('button', { name: 'Preview restore', exact: true }).click();
    const preview = page.getByRole('region', { name: 'Restore preview' });
    await expect(preview).toContainText('Workflow Workspace');
    await page.getByLabel('Restored workspace name', { exact: true }).fill('Restored Workflow');
    await page.getByLabel(/Restore into fresh workspace IDs/).check();
    await page.getByRole('button', { name: 'Restore verified backup' }).click();
    await expect(page.getByRole('status').filter({ hasText: 'Restore complete:' })).toContainText('Restored Workflow', { timeout: 120000 });

    // ---- Restart, reopen the restored workspace ------------
    await stopApi();
    await startApi();
    const workspaces = await call('workspaces', 'list', {});
    const restored = (workspaces.body.workspaces as Record<string, any>[]).find((w) => w.name === 'Restored Workflow');
    if (!restored) throw new Error('restored workspace missing after restart');
    const restoredWorkspaceId = restored.id as string;
    expect(restoredWorkspaceId).not.toBe(workspaceId);

    const restoredProjects = await call('memories', 'list', { workspaceId: restoredWorkspaceId, includeArchived: true });
    const restoredProjectMemory = (restoredProjects.body.memories as Record<string, any>[]).find((m) => m.title === 'Harbor App');
    if (!restoredProjectMemory) throw new Error('restored project memory missing');
    const restoredProjectId = restoredProjectMemory.id as string;
    expect(restoredProjectId).not.toBe(projectId);

    // The restored evaluation resolves to the RESTORED sources:
    // its embedded references and its artifact source
    // references name the restored idea set and rubric, which
    // exist and read back in the restored workspace.
    const restoredIdea = await artifactByType(restoredWorkspaceId, restoredProjectId, 'idea');
    const restoredRubric = await artifactByType(restoredWorkspaceId, restoredProjectId, 'rubric');
    const restoredEvaluation = await artifactByType(restoredWorkspaceId, restoredProjectId, 'evaluation');
    expect(restoredIdea.artifactId).not.toBe(originalIdea.artifactId);
    expect(restoredRubric.artifactId).not.toBe(originalRubric.artifactId);
    expect(restoredEvaluation.artifactId).not.toBe(originalEvaluation.artifactId);

    const evaluationRead = await call('artifacts', 'read', {
      workspaceId: restoredWorkspaceId, artifactId: restoredEvaluation.artifactId, version: 1,
    });
    const evaluationDoc = JSON.parse(evaluationRead.body.body as string);
    expect(evaluationDoc.sourceArtifactId).toBe(restoredIdea.artifactId);
    expect(evaluationDoc.rubricArtifactId).toBe(restoredRubric.artifactId);
    expect(restoredEvaluation.sourceReferences).toContain(`artifact://${restoredIdea.artifactId}@v1`);
    expect(restoredEvaluation.sourceReferences).toContain(`artifact://${restoredRubric.artifactId}@v1`);

    const ideaRead = await call('artifacts', 'read', {
      workspaceId: restoredWorkspaceId, artifactId: restoredIdea.artifactId, version: 1,
    });
    const ideaDoc = JSON.parse(ideaRead.body.body as string);
    expect(ideaDoc.concepts.map((c: { name: string }) => c.name)).toEqual(NAMES);

    // The original workspace is untouched: its evaluation
    // still resolves to the ORIGINAL idea set.
    const originalRead = await call('artifacts', 'read', {
      workspaceId, artifactId: originalEvaluation.artifactId, version: 1,
    });
    const originalDoc = JSON.parse(originalRead.body.body as string);
    expect(originalDoc.sourceArtifactId).toBe(originalIdea.artifactId);

    // ---- The restored evaluation renders in the UI ---------
    await pair();
    await openWorkspace('Restored Workflow');
    await page.getByRole('button', { name: /Harbor App/ }).first().click();
    await page.getByRole('button', { name: /Evaluation: Idea set/ }).click();
    await expect(page.getByText(/Evaluation of idea artifact/)).toBeVisible();
    await expect(page.getByText(restoredIdea.artifactId).first()).toBeVisible();
    await expect(page.getByText(/Weighted summary:/).first()).toBeVisible();
    await expect(page.getByText(/not market evidence/).first()).toBeVisible();
  } finally {
    await stopApi();
    fake.close();
  }
});
