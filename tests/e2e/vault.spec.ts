import { test, expect } from '@playwright/test';
import { spawn, spawnSync, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

let child: ChildProcess, root: string, file: string, pairing: { endpoint: string; token: string };
const apiBin = path.resolve('target/debug/anansi-api' + (process.platform === 'win32' ? '.exe' : ''));

async function start() {
  child = spawn(apiBin, ['--port', '0', '--pairing-root', path.join(root, 'pairing')], {
    windowsHide: true,
    env: {
      ...process.env,
      ANANSI_STORAGE_ROOT: path.join(root, 'storage'),
      ANANSI_VAULT_TEST_PROVIDER: '1',
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  await new Promise<void>((resolve, reject) => {
    let output = '';
    const timer = setTimeout(() => reject(Error('API startup timed out')), 20000);
    child.once('error', reject);
    child.stdout!.on('data', data => {
      output += data.toString();
      const match = output.match(/Pairing file: (.+)\r?\n/);
      if (match) {
        file = match[1].trim();
        pairing = JSON.parse(fs.readFileSync(file, 'utf8'));
        clearTimeout(timer);
        resolve();
      }
    });
  });
}

async function stop() {
  if (child?.exitCode === null) {
    const stopped = new Promise(resolve => child.once('exit', resolve));
    child.kill();
    await stopped;
  }
  if (file && fs.existsSync(file)) {
    fs.unlinkSync(file);
    fs.rmdirSync(path.dirname(file));
  }
}

test.beforeAll(async () => {
  root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-vault-browser-'));
  await start();
});

test.afterAll(async () => {
  await stop();
});

async function pair(page: import('@playwright/test').Page) {
  await page.goto('/');
  await page.getByLabel('Local API address').fill(pairing.endpoint);
  await page.getByLabel('Session token').fill(pairing.token);
  await page.getByRole('button', { name: 'Connect to core' }).click();
  await expect(page.locator('.badge')).toHaveText('Connected');
  await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled({ timeout: 20000 });
}

test('paired browser vault authorization, encrypted backup/export/preview/restore, lock and restart revocation', async ({
  page,
}) => {
  test.setTimeout(240000);
  await pair(page);
  await page.getByRole('button', { name: 'Continue to workspace' }).click();
  await page.getByLabel('New workspace name').fill('Browser vault source');
  await page.getByRole('button', { name: 'Create workspace' }).click();
  await page.getByLabel('I have reviewed these privacy limits').check();
  await page.getByRole('button', { name: 'Continue to AI setup' }).click();
  await page.getByRole('button', { name: 'Skip AI setup and enter ANANSI' }).click();

  const workspaceId = await page.locator('.workspace-id code').innerText();
  expect(workspaceId).toMatch(/^[a-f0-9]{32}$/);

  const post = async (routePath: string, body: Record<string, unknown>) => {
    const res = await fetch(`${pairing.endpoint}/v1/${routePath}`, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${pairing.token}`,
        'Content-Type': 'application/json',
        Origin: 'http://127.0.0.1:1420',
      },
      body: JSON.stringify({ contractVersion: 1, ...body }),
    });
    return { status: res.status, json: (await res.json()) as Record<string, any> };
  };

  await post('memories/create', {
    workspaceId,
    kind: 'note',
    draft: {
      title: 'Browser encrypted vault record',
      body: 'Browser encrypted vault body verified across backup, export, and fresh restore.',
      privacy: 'private',
    },
  });

  // Browser setup is forbidden by design; initial sealing is desktop-owned
  const browserSetup = await post('vault/setup', { workspaceId });
  expect(browserSetup.status).toBe(403);

  const backupJob = await post('jobs/backup', {
    workspaceId,
    application: false,
    plaintextConsent: true,
  });
  const jobId = backupJob.json.job.id as string;
  let artifactId = '';
  for (let i = 0; i < 120; i++) {
    const jobs = await post('jobs/list', { workspaceId, offset: 0 });
    const found = (jobs.json.jobs as any[]).find(j => j.id === jobId && j.state === 'Completed');
    if (found?.artifactId) {
      artifactId = found.artifactId;
      break;
    }
    await new Promise(r => setTimeout(r, 200));
  }
  expect(artifactId).toMatch(/^[a-f0-9]{32}$/);
  const preview = await post('jobs/preview', { workspaceId, jobId, artifactId });
  expect(preview.json.sha256).toMatch(/^[a-f0-9]{64}$/);

  // Seal the workspace via desktop transport helper against the shared storage root
  const sealed = spawnSync(
    apiBin,
    ['--seal-test-workspace', workspaceId, jobId, artifactId, preview.json.sha256 as string],
    {
      env: {
        ...process.env,
        ANANSI_STORAGE_ROOT: path.join(root, 'storage'),
        ANANSI_VAULT_TEST_PROVIDER: '1',
      },
      encoding: 'utf8',
    }
  );
  expect(sealed.status).toBe(0);

  // Exercise the real paired-browser Vault UI!
  const vaultRegion = page.getByRole('region', { name: 'Sealed vault' });
  await expect(page.getByRole('heading', { name: 'Sealed vault' })).toBeVisible();
  // Verify locked read fails closed first
  await vaultRegion.getByRole('button', { name: 'Verify authorized read' }).click();
  await expect(vaultRegion.getByRole('alert')).toContainText('locked', { timeout: 15000 });

  // Unlock via paired browser UI
  await vaultRegion.getByRole('button', { name: 'Unlock', exact: true }).click();
  await expect(
    vaultRegion.getByRole('status').filter({ hasText: 'Vault unlocked for this paired browser session.' })
  ).toBeVisible({
    timeout: 30000,
  });

  // Verify authorized read succeeds
  await vaultRegion.getByRole('button', { name: 'Verify authorized read' }).click();
  await expect(
    vaultRegion.getByRole('status').filter({ hasText: 'Authenticated vault read completed.' })
  ).toBeVisible({
    timeout: 15000,
  });

  // Open Encrypted vault backup, export, and fresh restore details
  await vaultRegion.getByText('Encrypted vault backup, export, and fresh restore', { exact: true }).click();

  // Create ciphertext backup & preview it
  await vaultRegion.getByRole('button', { name: 'Create ciphertext backup' }).click();
  await expect(vaultRegion.getByRole('status').filter({ hasText: 'Ciphertext vault backup created.' })).toBeVisible({
    timeout: 15000,
  });
  await expect(page.locator('#vault-archive-id')).toHaveValue(/^[a-f0-9]{32}$/);

  await vaultRegion.getByRole('button', { name: 'Preview ciphertext backup' }).click();
  await expect(page.locator('.vault-preview-summary')).toContainText('authenticated=false', { timeout: 15000 });

  // Create encrypted export and restore to a fresh sealed workspace
  await vaultRegion.getByRole('button', { name: 'Create encrypted export' }).click();
  await expect(vaultRegion.getByRole('status').filter({ hasText: 'Encrypted vault export created.' })).toBeVisible({
    timeout: 30000,
  });
  await page.locator('#vault-restore-name').fill('Restored browser vault workspace');
  await vaultRegion.getByRole('button', { name: 'Restore to new sealed workspace' }).click();
  await expect(
    vaultRegion.getByRole('status').filter({ hasText: 'Encrypted archive restored into a fresh sealed workspace.' })
  ).toBeVisible({ timeout: 60000 });
  const restoredId = await page.locator('.vault-restored-workspace code').innerText();
  expect(restoredId).toMatch(/^[a-f0-9]{32}$/);
  expect(restoredId).not.toBe(workspaceId);

  // Lock revokes the session
  await vaultRegion.getByRole('button', { name: 'Lock', exact: true }).click();
  await expect(
    vaultRegion.getByRole('status').filter({ hasText: 'Vault locked and all active sessions were revoked.' })
  ).toBeVisible({
    timeout: 15000,
  });
  await vaultRegion.getByRole('button', { name: 'Verify authorized read' }).click();
  await expect(vaultRegion.getByRole('alert')).toContainText('locked', { timeout: 15000 });

  // Re-unlock, then restart API and verify session is revoked across restart
  await vaultRegion.getByRole('button', { name: 'Unlock', exact: true }).click();
  await expect(
    vaultRegion.getByRole('status').filter({ hasText: 'Vault unlocked for this paired browser session.' })
  ).toBeVisible({
    timeout: 30000,
  });
  await stop();
  await start();
  const afterRestart = await post('vault/read', { workspaceId });
  expect(afterRestart.status).toBe(401);

  expect(await page.evaluate(() => localStorage.length + sessionStorage.length)).toBe(0);
});
