import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, type Server } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M6 Slice 4 in the real browser application: project-scoped
 * Opportunity Discovery (§21) over the M5 substrate with a
 * synthetic loopback provider.
 *
 * Open project -> supply a problem + a document excerpt,
 * explicitly approve a memory source -> review exactly what
 * will be sent -> discover -> inspect opportunities with
 * their citations and assumptions -> save as an immutable
 * artifact -> evaluate with the existing rubric panel ->
 * backup -> fresh restore -> restart -> the restored set
 * reopens with its sources resolving to the RESTORED memory,
 * and the restored evaluation resolves to the restored set.
 */
function opportunityJson(count: number, memorySourceId: string | null): string {
  const refs = memorySourceId ? ['supplied-1', memorySourceId] : ['supplied-1'];
  const concepts = Array.from({ length: count }, (_, i) => ({
    name: `Opportunity ${i}`,
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
    sourceReferences: refs,
    assumptions: ['Quotes arrive mostly by email.'],
  }));
  return JSON.stringify({ concepts });
}

test('opportunity discovery: supplied sources -> discover -> save -> evaluate -> backup -> fresh restore -> restored sources resolve', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-opportunities-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };

  const sentBodies: string[] = [];
  const fake: Server = createServer((req, res) => {
    let body = '';
    req.on('data', c => { body += c; });
    req.on('end', () => {
      sentBodies.push(body);
      // Cite the memory source id the request actually used,
      // exactly as a grounding model would.
      const memoryMatch = body.match(/Supplied source ([a-f0-9]{32}) \(memory/);
      const text = opportunityJson(6, memoryMatch ? memoryMatch[1] : null);
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
  }

  async function artifactByType(workspaceId: string, projectId: string, type: string) {
    const list = await call('artifacts', 'list', { workspaceId, projectId });
    const found = (list.body.artifacts as Record<string, any>[]).filter(a => a.type === type);
    if (found.length !== 1) throw new Error(`expected one ${type} artifact, got ${found.length}`);
    return found[0];
  }

  const problemText = 'Dock offices lose supplier quotes in shared inboxes.';
  try {
    await startApi();
    const created = await call('workspaces', 'create', { name: 'Opportunities Workspace' });
    const workspaceId = created.body.workspace.id as string;
    await call('workspaces', 'settings', { id: workspaceId, reducedMotion: true, onboardingComplete: true });
    const notes = await call('memories', 'create', {
      workspaceId,
      draft: {
        title: 'Field notes', body: 'The approved e2e source phrase.',
        privacy: 'private', eventDate: null, validFrom: null, validUntil: null,
        confidence: null, sourceReferences: [],
      },
    });
    const originalMemoryId = notes.body.memory.id as string;
    await call('memories', 'create', {
      workspaceId,
      draft: {
        title: 'Private diary', body: 'The never-send-this e2e phrase.',
        privacy: 'private', eventDate: null, validFrom: null, validUntil: null,
        confidence: null, sourceReferences: [],
      },
    });
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

    await pairAndOpen(workspaceId);
    await page.getByRole('button', { name: 'New project' }).click();
    await page.getByLabel('Project title').fill('Harbor App');
    await page.getByLabel('Description & objectives').fill('A test project.');
    await page.getByRole('button', { name: 'Save project' }).click();
    await expect(page.getByRole('heading', { name: 'Opportunity discovery' })).toBeVisible();

    const opp = page.locator('section[aria-labelledby="project-opportunities-heading"]');
    await opp.getByLabel('Source text').fill(problemText);
    await opp.getByRole('button', { name: 'Add another source' }).click();
    await opp.getByLabel('Source kind').nth(1).selectOption('excerpt');
    await opp.getByLabel('Label (optional)').nth(1).fill('Harbour report');
    await opp.getByLabel('Source text').nth(1).fill('The e2e excerpt phrase about seasonal demand.');
    await opp.getByLabel(/Field notes/).check();

    // Review: exactly these sources will be sent.
    const review = opp.getByRole('list', { name: 'Sources that will be sent' });
    await expect(review).toContainText('Problem statement');
    await expect(review).toContainText('Harbour report');
    await expect(review).toContainText('Memory: Field notes');

    await opp.getByRole('button', { name: 'Discover opportunities' }).click();
    await expect(page.getByText('1. Opportunity 0')).toBeVisible();
    await expect(page.locator('article').first()).toContainText('Derived from:');
    await expect(page.locator('article').first()).toContainText('Problem statement');
    await expect(page.locator('article').first()).toContainText('Field notes');
    await page.getByRole('button', { name: 'Explore' }).first().click();
    await expect(page.getByText(/Assumptions \(unverified\):/)).toBeVisible();
    await expect(page.getByText('Quotes arrive mostly by email.')).toBeVisible();
    await expect(page.getByText(/Required validation:/)).toBeVisible();
    // Wire discipline: supplied + selected sent; unselected not.
    const wire = sentBodies.join('');
    expect(wire).toContain('approved e2e source phrase');
    expect(wire).toContain('e2e excerpt phrase');
    expect(wire).not.toContain('never-send-this');
    await expect(page.getByRole('heading', { name: 'View what ANANSI sent' })).toBeVisible();

    await opp.getByRole('button', { name: 'Save opportunity set as artifact' }).click();
    await expect(opp.getByText(/Saved as artifact/)).toBeVisible();
    await expect(page.getByRole('button', { name: /Opportunity set: Dock offices/ })).toBeVisible();

    const projectsList = await call('memories', 'list', { workspaceId, includeArchived: true });
    const projectId = (projectsList.body.memories as Record<string, any>[]).find(m => m.title === 'Harbor App')!.id as string;
    const originalSet = await artifactByType(workspaceId, projectId, 'idea');

    // Evaluate the saved set with the existing rubric panel.
    await page.getByRole('button', { name: 'Opportunity set: Dock offices lose supplier quotes in shared inboxes.', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Supplied sources' })).toBeVisible();
    await page.getByLabel('Opportunity 0', { exact: true }).check();
    await page.getByLabel('Problem clarity score').first().selectOption('5');
    await page.getByRole('button', { name: 'Save evaluation' }).click();
    await expect(page.getByText(/Evaluation saved as artifact/)).toBeVisible();
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
    const backupFile = path.join(root, 'opportunities-backup.tar');
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
    await expect(preview).toContainText('Opportunities Workspace');
    await page.getByLabel('Restored workspace name', { exact: true }).fill('Restored Opportunities');
    await page.getByLabel(/Restore into fresh workspace IDs/).check();
    await page.getByRole('button', { name: 'Restore verified backup' }).click();
    await expect(page.getByRole('status').filter({ hasText: 'Restore complete:' })).toContainText('Restored Opportunities', { timeout: 120000 });

    // ---- Restart; restored sources resolve ----------------
    await stopApi();
    await startApi();
    const workspaces = await call('workspaces', 'list', {});
    const restored = (workspaces.body.workspaces as Record<string, any>[]).find(w => w.name === 'Restored Opportunities');
    if (!restored) throw new Error('restored workspace missing after restart');
    const restoredWorkspaceId = restored.id as string;
    expect(restoredWorkspaceId).not.toBe(workspaceId);

    const restoredMemories = await call('memories', 'list', { workspaceId: restoredWorkspaceId, includeArchived: true });
    const restoredNotes = (restoredMemories.body.memories as Record<string, any>[]).find(m => m.title === 'Field notes');
    if (!restoredNotes) throw new Error('restored Field notes missing');
    const restoredMemoryId = restoredNotes.id as string;
    expect(restoredMemoryId).not.toBe(originalMemoryId);
    const restoredProjectId = (restoredMemories.body.memories as Record<string, any>[]).find(m => m.title === 'Harbor App')!.id as string;

    const restoredSet = await artifactByType(restoredWorkspaceId, restoredProjectId, 'idea');
    expect(restoredSet.artifactId).not.toBe(originalSet.artifactId);
    const setRead = await call('artifacts', 'read', {
      workspaceId: restoredWorkspaceId, artifactId: restoredSet.artifactId, version: 1,
    });
    const setDoc = JSON.parse(setRead.body.body as string);
    const memorySource = (setDoc.sources as Record<string, any>[]).find(s => s.kind === 'memory');
    expect(memorySource.memoryId).toBe(restoredMemoryId);
    expect(memorySource.sourceId).toBe(restoredMemoryId);
    const suppliedSource = (setDoc.sources as Record<string, any>[]).find(s => s.kind === 'problem');
    expect(suppliedSource.text).toBe(problemText);
    expect(setDoc.concepts[0].sourceReferences).toContain(restoredMemoryId);
    expect(setDoc.concepts[0].sourceReferences).toContain('supplied-1');

    const restoredEvaluation = await artifactByType(restoredWorkspaceId, restoredProjectId, 'evaluation');
    const evaluationRead = await call('artifacts', 'read', {
      workspaceId: restoredWorkspaceId, artifactId: restoredEvaluation.artifactId, version: 1,
    });
    const evaluationDoc = JSON.parse(evaluationRead.body.body as string);
    expect(evaluationDoc.sourceArtifactId).toBe(restoredSet.artifactId);
    const originalRead = await call('artifacts', 'read', {
      workspaceId, artifactId: originalEvaluation.artifactId, version: 1,
    });
    expect(JSON.parse(originalRead.body.body as string).sourceArtifactId).toBe(originalSet.artifactId);

    // The restored set renders in the UI, citing the restored
    // memory as its source.
    await pairAndOpen(restoredWorkspaceId);
    await page.getByRole('button', { name: /Harbor App/ }).first().click();
    await page.getByRole('button', { name: 'Opportunity set: Dock offices lose supplier quotes in shared inboxes.', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Supplied sources' })).toBeVisible();
    await expect(page.getByText(restoredMemoryId).first()).toBeVisible();
    await expect(page.getByText('1. Opportunity 0')).toBeVisible();
  } finally {
    await stopApi();
    fake.close();
  }
});
