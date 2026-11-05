// Native Candidate Acceptance Suite for ANANSI Ultimate
// Target Binary: E:/Anansi/previews/native-candidate-7e24859e6/ANANSI-Candidate-7e24859e6.exe
// Storage Root: E:/Anansi/previews/native-candidate-7e24859e6/data (Isolated synthetic test root)

import { chromium } from '@playwright/test';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const port = 19323;
const binaryPath = path.resolve('E:/Anansi/previews/native-candidate-7e24859e6/ANANSI-Candidate-7e24859e6.exe');
const storageRoot = path.resolve('E:/Anansi/previews/native-candidate-7e24859e6/data');
const udfPath = path.resolve('E:/Anansi/previews/native-candidate-7e24859e6/webview-udf');

fs.mkdirSync(storageRoot, { recursive: true });
fs.mkdirSync(udfPath, { recursive: true });
fs.mkdirSync('test-results', { recursive: true });

console.log('--- Native Candidate Acceptance Starting ---');
console.log('Candidate Binary:', binaryPath);
console.log('Storage Root:', storageRoot);
console.log('WebView2 UDF:', udfPath);

const env = {
  ...process.env,
  ANANSI_STORAGE_ROOT: storageRoot,
  ANANSI_VAULT_TEST_PROVIDER: '1',
  WEBVIEW2_USER_DATA_FOLDER: udfPath,
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`
};

let child = null;
let browser = null;
let page = null;

async function startCandidate() {
  child = spawn(binaryPath, [], { env, windowsHide: true, stdio: 'ignore' });
  console.log('Started candidate process PID:', child.pid);

  let connected = false;
  for (let n = 0; n < 150; n++) {
    try {
      const res = await fetch(`http://127.0.0.1:${port}/json/version`);
      if (res.ok) { connected = true; break; }
    } catch {}
    if (child.exitCode !== null) throw Error('Candidate exited before WebView2 CDP ready');
    await new Promise(r => setTimeout(r, 200));
  }
  if (!connected) throw Error('CDP endpoint timed out');

  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  console.log('Connected to WebView2 CDP. Version:', browser.version());

  const context = browser.contexts()[0];
  for (let n = 0; n < 100; n++) {
    for (const p of context.pages()) {
      if (p.url().includes('tauri.localhost') || p.url().includes('127.0.0.1:1420')) {
        page = p;
        break;
      }
    }
    if (page) break;
    await new Promise(r => setTimeout(r, 100));
  }
  if (!page) throw Error('Native app page not found');
  await page.waitForFunction(() => !!globalThis.__TAURI_INTERNALS__);
}

async function stopCandidate() {
  page = null;
  if (browser) {
    await browser.close().catch(() => {});
    browser = null;
  }
  if (child && child.exitCode === null) {
    const exited = new Promise(r => child.once('exit', r));
    child.kill();
    await exited;
  }
  child = null;
}

async function invoke(command, args) {
  const result = await page.evaluate(async ({ command, args }) => {
    try {
      return { ok: true, value: await globalThis.__TAURI_INTERNALS__.invoke(command, args) };
    } catch (error) {
      return { ok: false, error: typeof error === 'string' ? error : JSON.stringify(error) };
    }
  }, { command, args });
  if (!result.ok) throw Error(`IPC command ${command} failed: ${result.error}`);
  return result.value;
}

try {
  // Step 1: Launch Candidate & Verify Capability Boundary
  await startCandidate();
  console.log('[1/7] Startup & Readiness verified.');

  const denied = await page.evaluate(async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('plugin:shell|execute', { program: 'cmd.exe' });
      return 'ALLOWED';
    } catch (e) {
      return typeof e === 'string' ? e : JSON.stringify(e);
    }
  });
  if (denied === 'ALLOWED') throw Error('FAIL: Shell execution was allowed! Capability boundary violated.');
  console.log('[2/7] Shell execution denied by capability boundary. Output:', denied);

  // Step 2: Workspace Creation & Memory Lifecycle
  const wsName = 'Candidate Validation ' + Date.now();
  const wsRes = await invoke('workspace_command', {
    command: 'create',
    request: { contractVersion: 1, name: wsName }
  });
  const wsId = wsRes.workspace.id;
  console.log('[3/7] Created test workspace ID:', wsId, 'Name:', wsName);

  const memCreate = await invoke('memory_command', {
    command: 'create',
    request: {
      contractVersion: 1,
      workspaceId: wsId,
      kind: 'memory',
      draft: {
        title: 'Alpha Candidate Memory Record',
        body: 'Synthetic memory record verifying native candidate data persistence.',
        privacy: 'standard'
      }
    }
  });
  const memId = memCreate.memory.id;
  console.log('[4/7] Created memory record ID:', memId);

  // Search memory record
  const searchRes = await invoke('memory_command', {
    command: 'list',
    request: { contractVersion: 1, workspaceId: wsId, query: 'Candidate Memory', limit: 10 }
  });
  if (!searchRes.memories || !searchRes.memories.some(r => r.id === memId)) {
    throw Error('FAIL: Created memory record was not retrievable via memory_command list/search');
  }
  console.log('[4/7] Memory record verified via search.');

  // Step 3: Project & Artifact Registration
  const projRes = await invoke('memory_command', {
    command: 'create',
    request: {
      contractVersion: 1,
      workspaceId: wsId,
      kind: 'project',
      draft: {
        title: 'Project Candidate Alpha',
        body: 'Candidate validation project description',
        privacy: 'standard'
      }
    }
  });
  const projId = projRes.memory.id;
  console.log('[5/7] Created project record ID:', projId);

  const artRes = await invoke('artifact_command', {
    command: 'register',
    request: {
      contractVersion: 1,
      workspaceId: wsId,
      projectId: projId,
      type: 'idea',
      title: 'Synthesized Idea Artifact',
      body: 'Verified idea body content\nTags: synthetic, test',
      privacy: 'standard'
    }
  });
  console.log('[5/7] Registered artifact ID:', artRes.artifact.id);

  // Step 4: Backup Job & Restart Persistence
  const backupRes = await invoke('job_command', {
    command: 'backup',
    request: { contractVersion: 1, workspaceId: wsId, application: false, plaintextConsent: true }
  });
  const backupJobId = backupRes.job.id;
  console.log('[6/7] Queued backup job ID:', backupJobId);

  let backupComplete = false;
  for (let n = 0; n < 60; n++) {
    const jobList = await invoke('job_command', {
      command: 'list',
      request: { contractVersion: 1, workspaceId: wsId, offset: 0 }
    });
    const found = jobList.jobs.find(j => j.id === backupJobId && j.state === 'Completed');
    if (found) { backupComplete = true; break; }
    await new Promise(r => setTimeout(r, 250));
  }
  if (!backupComplete) throw Error('Backup job did not complete within deadline');
  console.log('[6/7] Backup job completed.');

  // Restart process and verify survival
  console.log('Restarting candidate process to verify data survival...');
  await stopCandidate();
  await startCandidate();

  const verifyMem = await invoke('memory_command', {
    command: 'list',
    request: { contractVersion: 1, workspaceId: wsId, limit: 10 }
  });
  if (!verifyMem.memories.some(r => r.id === memId && r.title === 'Alpha Candidate Memory Record')) {
    throw Error('FAIL: Memory record did not survive desktop process restart!');
  }
  console.log('[6/7] PASS: Record persisted and survived desktop restart.');

  // Step 5: Screen capture & Clean Exit
  await page.screenshot({ path: 'test-results/native-candidate-verified.png', fullPage: true });
  console.log('[7/7] Captured screenshot: test-results/native-candidate-verified.png');

  console.log('--- ALL 7 NATIVE CANDIDATE ACCEPTANCE CHECKS PASSED ---');
} finally {
  await stopCandidate();
}
