import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/**
 * M6 Slice 1 in the real browser application: the §9 artifact
 * registry on the existing project identity.
 *
 * New project -> register artifact v1 -> revise to v2 -> view
 * both immutable versions -> archive the project (existing
 * memory lifecycle; writes refused while archived) -> restore
 * -> restart the API -> reopen and find everything intact.
 * No model is involved anywhere in this slice.
 */
test('artifact registry: project -> v1 -> v2 -> inspect both -> archive/restore -> restart persistence', async ({ page }) => {
  test.setTimeout(240000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-artifacts-e2e-'));
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
    return { status: response.status, body: await response.json() };
  }

  async function pairAndOpen(workspaceId: string) {
    await page.goto('/');
    await page.getByLabel('Local API address').fill(pairing.endpoint);
    await page.getByLabel('Session token').fill(pairing.token);
    await page.getByRole('button', { name: 'Connect to core' }).click();
    await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled();
    await page.getByLabel('Existing workspace').selectOption(workspaceId);
    await page.getByRole('button', { name: 'Open workspace', exact: true }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Registry Workspace' })).toBeVisible();
  }

  try {
    await startApi();
    const workspace = (await call('workspaces', 'create', { name: 'Registry Workspace' })).body.workspace;
    await call('workspaces', 'settings', { id: workspace.id, reducedMotion: true, onboardingComplete: true });
    await pairAndOpen(workspace.id);

    // --- Project + artifact workflow, entirely in the UI ----
    await page.getByRole('button', { name: 'Projects', exact: true }).click();
    await page.getByRole('button', { name: 'New project' }).click();
    await page.getByLabel('Project title').fill('Harbor App');
    await page.getByLabel('Description & objectives').fill('A product project for the registry checkpoint.');
    await page.getByRole('button', { name: 'Save project' }).click();
    await expect(page.getByRole('heading', { name: 'Artifacts' })).toBeVisible();

    await page.getByLabel('Artifact title').fill('Concept note');
    await page.getByLabel('Artifact text').fill('First draft of the concept.');
    await page.getByRole('button', { name: 'Register artifact' }).click();
    await expect(page.getByText('Artifact registered. Version 1.')).toBeVisible();
    await expect(page.locator('pre.provider-output')).toHaveText('First draft of the concept.');

    await page.getByLabel('Text', { exact: true }).fill('Second draft, expanded.');
    await page.getByRole('button', { name: 'Save new version' }).click();
    await expect(page.getByText(/Artifact revised\. Version 2/)).toBeVisible();
    await expect(page.locator('pre.provider-output')).toHaveText('Second draft, expanded.');

    // Both immutable versions, independently inspectable.
    await page.getByRole('button', { name: 'View version 1' }).click();
    await expect(page.locator('pre.provider-output')).toHaveText('First draft of the concept.');
    await expect(page.getByText('an immutable earlier version (read-only)')).toBeVisible();
    await page.getByRole('button', { name: 'View version 2' }).click();
    await expect(page.locator('pre.provider-output')).toHaveText('Second draft, expanded.');

    // --- Archive via the existing lifecycle; writes refused --
    const listed = (await call('memories', 'list', { workspaceId: workspace.id, kind: 'project' })).body;
    const projectId = listed.memories[0].id as string;
    const read = (await call('memories', 'read', { workspaceId: workspace.id, id: projectId })).body;
    const draft = {
      title: 'Harbor App', body: 'A product project for the registry checkpoint.', privacy: 'private',
      eventDate: null, validFrom: null, validUntil: null, confidence: null, sourceReferences: [],
    };
    const archived = await call('memories', 'revise', {
      workspaceId: workspace.id, id: projectId, expectedRevision: read.memory.revisionId, draft,
      lifecycle: { state: 'archived', confirmed: true, successorId: null },
    });
    expect(archived.status).toBe(200);
    await page.getByLabel('Artifact title').fill('While archived');
    await page.getByLabel('Artifact text').fill('must be refused');
    await page.getByRole('button', { name: 'Register artifact' }).click();
    await expect(page.getByRole('alert')).toContainText('archived');

    // --- Restore, restart, reopen: everything persists ------
    const readArchived = (await call('memories', 'read', { workspaceId: workspace.id, id: projectId })).body;
    const restored = await call('memories', 'revise', {
      workspaceId: workspace.id, id: projectId, expectedRevision: readArchived.memory.revisionId, draft,
      restoreArchive: true,
    });
    expect(restored.status).toBe(200);

    await stopApi();
    await startApi();
    await pairAndOpen(workspace.id);
    await page.getByRole('button', { name: 'Projects', exact: true }).click();
    await page.getByRole('button', { name: 'Harbor App' }).click();
    await expect(page.getByText('Latest version 2')).toBeVisible();
    await page.getByRole('button', { name: 'Concept note' }).click();
    await page.getByRole('button', { name: 'View version 1' }).click();
    await expect(page.locator('pre.provider-output')).toHaveText('First draft of the concept.');
  } finally {
    await stopApi();
    fs.rmSync(root, { recursive: true, force: true });
  }
});
