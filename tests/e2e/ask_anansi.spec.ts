import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import http from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

test('integrated ask anansi workflow: grounded citations, missing evidence, offline resilience, and disclosure manifest', async ({ page }) => {
  test.setTimeout(process.env.ANANSI_REAL_MODEL==='1'?600000:180000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-ask-e2e-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string };
  let pairingFile = '';

  async function start() {
    child = spawn(
      path.resolve('target/debug/anansi-api' + (process.platform === 'win32' ? '.exe' : '')),
      ['--port', '0', '--pairing-root', path.join(root, 'pairing')],
      {
        windowsHide: true,
        env: { ...process.env, ANANSI_STORAGE_ROOT: path.join(root, 'storage') },
        stdio: ['ignore', 'pipe', 'ignore'],
      }
    );
    await new Promise<void>((resolve, reject) => {
      let output = '';
      const timer = setTimeout(() => reject(Error('Ask API startup timeout')), 20000);
      child!.once('error', reject);
      child!.stdout!.on('data', (data) => {
        output += data.toString();
        const match = output.match(/Pairing file: (.+)\r?\n/);
        if (match) {
          pairingFile = match[1].trim();
          pairing = JSON.parse(fs.readFileSync(pairingFile, 'utf8'));
          clearTimeout(timer);
          resolve();
        }
      });
    });
  }

  async function stop() {
    if (child?.exitCode === null) {
      const ended = new Promise((resolve) => child!.once('exit', resolve));
      child.kill();
      await ended;
    }
    if (pairingFile && fs.existsSync(pairingFile)) {
      fs.unlinkSync(pairingFile);
      try {
        fs.rmdirSync(path.dirname(pairingFile));
      } catch {}
    }
  }

  async function pair() {
    await page.goto('/');
    await page.getByLabel('Local API address').fill(pairing.endpoint);
    await page.getByLabel('Session token').fill(pairing.token);
    await page.getByRole('button', { name: 'Connect to core' }).click();
    await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled();
  }

  async function call(group: string, operation: string, input: Record<string, unknown>) {
    const response = await fetch(pairing.endpoint + '/v1/' + group + '/' + operation, {
      method: 'POST',
      headers: {
        Origin: 'http://127.0.0.1:1420',
        Authorization: 'Bearer ' + pairing.token,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ contractVersion: 1, ...input }),
    });
    expect(response.ok).toBe(true);
    return response.json();
  }

  let external = 0;
  page.on('request', (request) => {
    if (!['127.0.0.1', 'localhost', 'tauri.localhost'].includes(new URL(request.url()).hostname)) {
      external++;
    }
  });

  try {
    await start();
    const workspace = (await call('workspaces', 'create', { name: 'Ask ANANSI Integration Workspace' })).workspace;
    await call('workspaces', 'settings', { id: workspace.id, reducedMotion: true, onboardingComplete: true });

    // Seed notes and records
    await call('memories', 'create', {
      workspaceId: workspace.id,
      kind: 'decision',
      draft: {
        title: 'Project Aurora Storage Architecture',
        body: '# Architecture Plan\nLine 2 details\nWe decided to adopt SQLite with FTS5 for local-first search.',
      },
    });

    await pair();
    await page.getByLabel('Existing workspace').selectOption(workspace.id);
    await page.getByRole('button', { name: 'Open workspace', exact: true }).click();

    // 1. Navigate to Ask ANANSI
    await page.getByRole('button', { name: 'Ask ANANSI', exact: true }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Ask ANANSI' })).toBeVisible();

    // 2. Missing evidence query
    await page.getByLabel('Your Question').fill('What is our policy on planetary space exploration?');
    await page.getByRole('button', { name: 'Ask Question' }).click();
    await expect(page.locator('.status-banner.status-insufficientEvidence')).toBeVisible();
    await expect(page.locator('.answer-text')).toContainText("couldn't find any relevant memories");

    // 3. Offline / Unavailable Local Server query
    const unavailable=http.createServer();
    await new Promise<void>(resolve=>unavailable.listen(0,'127.0.0.1',resolve));
    const unavailablePort=(unavailable.address() as import('node:net').AddressInfo).port;
    await new Promise<void>(resolve=>unavailable.close(()=>resolve()));
    await page.getByLabel('Local inference endpoint').fill(`http://127.0.0.1:${unavailablePort}`);
    await page.getByLabel('Your Question').fill('What did we decide about local search?');
    await page.getByRole('button', { name: 'Ask Question' }).click();
    await expect(page.locator('.status-banner.status-offline')).toBeVisible();
    await expect(page.locator('.citation-title')).toContainText('Project Aurora Storage Architecture');
    await expect(page.locator('.citation-meta')).toContainText('# Architecture Plan');

    // 4. Inspect Data Disclosure Manifest ("View What ANANSI Sent")
    await page.getByRole('button', { name: 'View What ANANSI Sent (Disclosure)' }).click();
    await expect(page.locator('.manifest-details')).toBeVisible();
    await expect(page.locator('.manifest-metrics')).toContainText('Project Aurora Storage Architecture');
    await expect(page.locator('.compiled-prompt-text')).toContainText('Architecture Plan');

    expect(external).toBe(0);
    if(process.env.ANANSI_REAL_MODEL==='1'){
      await page.getByLabel('Local inference endpoint').fill('http://127.0.0.1:11434');
      await page.getByLabel('Installed Ollama model').fill('qwen2.5:0.5b');
      await page.getByLabel('Your Question').fill('What did we decide about local search?');
      await page.getByRole('button',{name:'Ask Question',exact:true}).click();
      await expect(page.getByRole('region',{name:'Streaming answer'})).toBeVisible({timeout:180000});
      await expect(page.locator('.status-banner.status-answered')).toBeVisible({timeout:210000});
      await expect(page.locator('.answer-text')).toContainText(/SQLite/i);
      await expect(page.locator('.citation-title')).toContainText('Project Aurora Storage Architecture');
      await page.getByRole('button',{name:'View What ANANSI Sent (Disclosure)'}).click();
      await expect(page.locator('.manifest-metrics')).toContainText('qwen2.5:0.5b');
      await page.getByRole('button',{name:'Ask Question',exact:true}).click();
      await expect(page.getByRole('button',{name:'Cancel generation'})).toBeVisible();
      await page.getByRole('button',{name:'Cancel generation'}).click();
      await expect(page.getByRole('button',{name:'Ask Question',exact:true})).toBeEnabled({timeout:30000});
      await expect(page.locator('.status-banner.status-answered')).toHaveCount(0);
    }
    await page.screenshot({ path: 'test-results/browser-ask-anansi-integration.png', fullPage: true });
  } finally {
    await stop();
  }
});
