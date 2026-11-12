import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import http from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { prepareBackup, restoreBackup, checkBackup } from '../../scripts/backup-workflow.mjs';
import { roundTripSavedBackup } from '../../scripts/backup-file-workflow.mjs';

// Pilot journey walk (v1 RC, AC-V1-04): first-time-user path over
// synthetic data only, end to end in the browser client:
// fresh start -> workspace/project -> provider setup -> document
// import -> search -> Ask with citations -> restart -> backup ->
// fresh restore -> reopen. The model is a loopback fake speaking
// the Ollama /api/generate NDJSON contract; no real model, no
// network, no credentials.

test('pilot journey: fresh start to restored reopen with synthetic data', async ({ page }) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-pilot-journey-'));
  let child: ChildProcess | undefined;
  let pairing: { endpoint: string; token: string } = { endpoint: '', token: '' };
  let pairingFile = '';

  // --- fake Ollama: 'answer' replies at once; 'hold' waits for the
  // test to release the gate, so cancellation is deterministic. ---
  let fakeMode: 'answer' | 'hold' = 'answer';
  let releaseGate: (() => void) | null = null;
  let gatePromise: Promise<void> | null = null;
  let fakeHits = 0;
  const fake = http.createServer((req, res) => {
    if (req.method !== 'POST' || req.url !== '/api/generate') {
      res.writeHead(404).end();
      return;
    }
    fakeHits += 1;
    res.on('error', () => {});
    let body = '';
    req.on('data', (c) => {
      body += c;
    });
    req.on('end', async () => {
      void body;
      if (fakeMode === 'hold') {
        gatePromise = new Promise<void>((resolve) => {
          releaseGate = resolve;
        });
        await gatePromise;
      }
      res.writeHead(200, { 'Content-Type': 'application/x-ndjson' });
      res.write(
        JSON.stringify({ model: 'fake-pilot', response: 'The quokka pilot stores records in SQLite with FTS5. ', done: false }) + '\n',
      );
      res.write(JSON.stringify({ model: 'fake-pilot', response: '', done: true, eval_count: 4 }) + '\n');
      res.end();
    });
  });
  await new Promise<void>((r) => fake.listen(0, '127.0.0.1', r));
  const fakeUrl = `http://127.0.0.1:${(fake.address() as { port: number }).port}`;

  async function startApi() {
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

  async function stopApi() {
    if (child && child.exitCode === null) {
      const ended = new Promise((resolve) => child!.once('exit', resolve));
      child.kill();
      await ended;
    }
    child = undefined;
    if (pairingFile && fs.existsSync(pairingFile)) {
      fs.unlinkSync(pairingFile);
      try {
        fs.rmdirSync(path.dirname(pairingFile));
      } catch {}
    }
  }

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
    expect(response.ok).toBe(true);
    return response.json();
  }

  async function pair() {
    await page.goto('/');
    await page.getByLabel('Local API address').fill(pairing.endpoint);
    await page.getByLabel('Session token').fill(pairing.token);
    await page.getByRole('button', { name: 'Connect to core' }).click();
    await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled();
  }

  let external = 0;
  page.on('request', (request) => {
    if (!['127.0.0.1', 'localhost', 'tauri.localhost'].includes(new URL(request.url()).hostname)) {
      external += 1;
    }
  });

  try {
    await startApi();

    // 1. Fresh start: onboarding, workspace creation, privacy review,
    // AI setup skipped. Empty states are honest.
    await pair();
    await expect(page.getByRole('group', { name: 'What would you like to do?' })).toBeVisible();
    await page.getByRole('radio', { name: /Explore ANANSI/ }).check();
    await page.getByRole('button', { name: 'Continue to workspace' }).click();
    await page.getByLabel('New workspace name').fill('Pilot Journey');
    await page.getByRole('button', { name: 'Create workspace' }).click();
    await expect(page.getByRole('heading', { name: 'Review your privacy' })).toBeVisible();
    await expect(page.getByText(/not an encrypted vault/)).toBeVisible();
    await expect(page.getByRole('button', { name: 'Continue to AI setup' })).toBeDisabled();
    await page.getByLabel('I have reviewed these privacy limits').check();
    await page.getByRole('button', { name: 'Continue to AI setup' }).click();
    await expect(page.getByRole('heading', { name: 'Choose AI setup' })).toBeVisible();
    await page.getByRole('button', { name: 'Skip AI setup and enter ANANSI' }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Dashboard' })).toBeVisible();
    const overview = page.getByRole('region', { name: 'Workspace overview' });
    await expect(overview).toContainText('Pilot Journey');
    const workspaceId = await overview.locator('code').innerText();
    await page.getByRole('button', { name: 'Memory', exact: true }).click();
    await expect(page.getByText('No saved memories on this page.')).toBeVisible();
    await page.getByRole('button', { name: 'Documents', exact: true }).click();
    await expect(page.getByText('No imported documents yet.')).toBeVisible();

    // 2. Project via the UI.
    await page.getByRole('button', { name: 'Projects', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Your projects', exact: true })).toBeVisible();
    await page.getByLabel('Project title').fill('Quokka Pilot');
    await page.getByLabel('Description & objectives').fill('Synthetic pilot project for the v1 journey walk.');
    await page.getByRole('button', { name: 'Save project' }).click();
    await expect(page.getByRole('button', { name: /Quokka Pilot/ }).first()).toBeVisible();

    // 3. Provider setup: local discovery reports the truth (no local
    // runtime here), and the browser Connect AI flow refuses key
    // entry with a desktop direction. Zero-Cost Lock stays on.
    await page.getByRole('button', { name: 'Local Models', exact: true }).click();
    await expect(page.getByRole('button', { name: 'Re-probe System' })).toBeVisible();
    await page.getByRole('button', { name: 'Re-probe System' }).click();
    await expect(page.getByRole('region', { name: 'Discovered Local Runtimes' })).toBeVisible();
    await page.getByRole('button', { name: 'AI Providers', exact: true }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'AI providers' })).toBeVisible();
    await expect(page.locator('#dispatch-lock')).toBeChecked();
    await page.getByRole('button', { name: 'OpenRouter', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Step 2 — Endpoint' })).toBeVisible();
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Credential setup' })).toBeVisible();
    await page.getByRole('button', { name: 'Generate reference' }).click();
    await expect(page.getByRole('alert', { name: 'Connect AI error' })).toContainText('desktop app');
    await page.getByRole('button', { name: 'Continue without a credential' }).click();
    // No ChatGPT sign-in control is offered outside the dev gate.
    await expect(page.getByText('Continue with ChatGPT')).toHaveCount(0);

    // 4. Document import: TXT + Markdown + PDF via the UI; unsupported
    // and oversized files are refused with understandable errors.
    const txt = path.join(root, 'field-notes.txt');
    fs.writeFileSync(txt, '# Field notes\nThe quokka pilot stores records in SQLite with FTS5 for local-first search.\n');
    const md = path.join(root, 'guide.md');
    fs.writeFileSync(md, '# Guide\nQuokka decisions are recorded as versioned project artifacts.\n');
    const exe = path.join(root, 'notes.exe');
    fs.writeFileSync(exe, Buffer.from([0x4d, 0x5a, 0x90, 0x00, 0x01, 0x02]));
    const big = path.join(root, 'oversized.txt');
    fs.writeFileSync(big, 'x'.repeat(70000));
    await page.getByRole('button', { name: 'Documents', exact: true }).click();
    await page.locator('#document-file').setInputFiles(exe);
    await expect(page.getByRole('alert')).toContainText('EXE files are not supported yet.');
    await page.locator('#document-file').setInputFiles(big);
    await expect(page.getByRole('alert')).toContainText('The file must be at most 65536 bytes.');
    await page.locator('#document-file').setInputFiles(txt);
    await page.getByRole('button', { name: 'Import document', exact: true }).click();
    await expect(page.getByRole('button', { name: 'field-notes.txt' })).toBeVisible();
    await page.locator('#document-file').setInputFiles(md);
    await page.getByRole('button', { name: 'Import document', exact: true }).click();
    await expect(page.getByRole('button', { name: 'guide.md' })).toBeVisible();
    await page.locator('#document-file').setInputFiles(path.resolve('tests/fixtures/pdf/normal-3page.pdf'));
    await page.getByRole('button', { name: 'Import document', exact: true }).click();
    await expect(page.getByRole('button', { name: 'normal-3page.pdf' })).toBeVisible();
    await page.getByRole('button', { name: 'field-notes.txt' }).click();
    await expect(page.getByText(/quokka pilot stores records/)).toBeVisible();

    // 5. Search: the imported codeword is found; a miss is an honest
    // empty state.
    await page.getByRole('button', { name: 'Search', exact: true }).click();
    await page.getByLabel('Full-body search (includes Markdown text)').check();
    await page.getByLabel('Search query').fill('quokka');
    await page.getByRole('button', { name: 'Search knowledge' }).click();
    await expect(page.getByText('field-notes.txt')).toBeVisible();
    await page.getByLabel('Search query').fill('nonexistentcodeword');
    await page.getByRole('button', { name: 'Search knowledge' }).click();
    await expect(page.getByText(/No matching records/)).toBeVisible();

    // 6. Ask: missing provider is truthful (Offline + citations +
    // disclosure); the fake provider answers with citations; a held
    // generation cancels cleanly.
    await page.getByRole('button', { name: 'Ask ANANSI', exact: true }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Ask ANANSI' })).toBeVisible();
    const dead = http.createServer();
    await new Promise<void>((resolve) => dead.listen(0, '127.0.0.1', resolve));
    const deadPort = (dead.address() as import('node:net').AddressInfo).port;
    await new Promise<void>((resolve) => dead.close(() => resolve()));
    await page.getByLabel('Local inference endpoint').fill(`http://127.0.0.1:${deadPort}`);
    await page.getByLabel('Your Question').fill('What does the quokka pilot store records in?');
    await page.getByRole('button', { name: 'Ask Question' }).click();
    await expect(page.locator('.status-banner.status-offline')).toBeVisible();
    await expect(page.locator('.citation-title').first()).toContainText('field-notes.txt');
    await page.getByRole('button', { name: 'View What ANANSI Sent (Disclosure)' }).click();
    await expect(page.locator('.manifest-details')).toBeVisible();
    await expect(page.locator('.compiled-prompt-text')).toContainText('quokka pilot');

    fakeMode = 'answer';
    await page.getByLabel('Local inference endpoint').fill(fakeUrl);
    // A colon in the model tag selects the Ollama /api/generate path
    // for a non-default loopback port (see ask.rs runtime detection).
    await page.getByLabel('Installed Ollama model').fill('fake-pilot:test');
    await page.getByLabel('Your Question').fill('What does the quokka pilot store records in?');
    await page.getByRole('button', { name: 'Ask Question' }).click();
    await expect(page.locator('.status-banner.status-answered')).toBeVisible();
    await expect(page.locator('.answer-text')).toContainText('SQLite with FTS5');
    await expect(page.locator('.citation-title').first()).toContainText('field-notes.txt');
    expect(fakeHits).toBe(1);

    fakeMode = 'hold';
    await page.getByLabel('Your Question').fill('What does the quokka pilot store records in?');
    await page.getByRole('button', { name: 'Ask Question' }).click();
    await expect(page.getByRole('button', { name: 'Cancel generation' })).toBeVisible();
    await page.getByRole('button', { name: 'Cancel generation' }).click();
    await expect(page.getByRole('button', { name: 'Ask Question', exact: true })).toBeEnabled();
    await expect(page.locator('.status-banner.status-answered')).toHaveCount(0);
    releaseGate?.();
    await page.waitForTimeout(500);
    await expect(page.locator('.status-banner.status-answered')).toHaveCount(0);

    // 7. Restart: data survives a process restart and reopen.
    await stopApi();
    await startApi();
    await pair();
    await page.getByLabel('Existing workspace').selectOption(workspaceId);
    await page.getByRole('button', { name: 'Open workspace', exact: true }).click();
    await expect(page.getByText('Pilot Journey', { exact: true }).first()).toBeVisible();
    await page.getByRole('button', { name: 'Documents', exact: true }).click();
    await expect(page.getByRole('button', { name: 'field-notes.txt' })).toBeVisible();
    await page.getByRole('button', { name: 'Projects', exact: true }).click();
    await expect(page.getByRole('button', { name: /Quokka Pilot/ }).first()).toBeVisible();

    // 8. Backup -> saved-file round trip (incl. invalid-archive
    // refusal) -> fresh restore -> reopen, reusing the shared
    // backup workflow helpers.
    const jobId = await prepareBackup(page, 'Journey');
    const imported = await roundTripSavedBackup(page, jobId, root);
    // 16 records = 12 entity-fixture records + 1 project + 3 documents;
    // 9 relationships come from the entity fixture.
    const restored = await restoreBackup(page, imported, 'Restored pilot journey', {
      records: '16 records',
      relationships: '9 relationships',
    });
    expect(restored).not.toBe(workspaceId);
    await checkBackup(page, 'Journey');
    await page.getByRole('button', { name: 'Documents', exact: true }).click();
    await expect(page.getByRole('button', { name: 'field-notes.txt' })).toBeVisible();
    await page.getByRole('button', { name: 'Search', exact: true }).click();
    await page.getByLabel('Full-body search (includes Markdown text)').check();
    await page.getByLabel('Search query').fill('quokka');
    await page.getByRole('button', { name: 'Search knowledge' }).click();
    await expect(page.getByText('field-notes.txt')).toBeVisible();

    expect(external).toBe(0);
    await page.screenshot({ path: 'test-results/browser-pilot-journey.png', fullPage: true });
  } finally {
    releaseGate?.();
    fake.close();
    await stopApi();
  }
});
