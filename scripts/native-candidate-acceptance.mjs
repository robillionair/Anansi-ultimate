// Native Candidate Acceptance Suite for ANANSI Ultimate
// Candidate locations are explicit environment variables with no defaults:
// ANANSI_CANDIDATE_BINARY, ANANSI_CANDIDATE_STORAGE_ROOT,
// ANANSI_CANDIDATE_WEBVIEW_UDF, plus optional ANANSI_CANDIDATE_CDP_PORT
// (default 19323) and ANANSI_CANDIDATE_EXPECTED_SHA256. Locations are
// validated and the host is checked free before anything spawns; see
// docs/handoffs/v1-pilot-installer-checklist.md for the commands.
import { chromium } from '@playwright/test';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import { resolveCandidateLocations, assertQualificationHostFree } from './windows-release/candidate-locations.mjs';
import { assertArtifactDetails, assertReadiness } from './windows-release/artifact-acceptance.mjs';

if (process.platform !== 'win32') throw Error('Native candidate acceptance requires Windows.');
const { binaryPath, storageRoot, udfPath, port, binarySha256, binaryBytes } = resolveCandidateLocations(process.env);
await assertQualificationHostFree({ port, env: process.env });

fs.mkdirSync(storageRoot, { recursive: true });
fs.mkdirSync(udfPath, { recursive: true });
fs.mkdirSync('test-results', { recursive: true });

console.log('--- Native Candidate Acceptance Starting ---');
console.log('Candidate Binary:', binaryPath);
console.log('Candidate SHA-256:', binarySha256, `(${binaryBytes} bytes)`);
console.log('Storage Root:', storageRoot);
console.log('WebView2 UDF:', udfPath);
console.log('CDP Port:', port);

const env = {
  ...process.env,
  ANANSI_STORAGE_ROOT: storageRoot,
  // Harness-only synthetic credential provider. A pass here never evidences
  // native credential-store acceptance; that is a manual checklist item.
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
  // NQ-01 audit: the pass line below used to be unconditional. Readiness
  // is now actually invoked and asserted before it prints.
  const readiness = await invoke('get_readiness', { request: { contractVersion: 1 } });
  assertReadiness(readiness, 'readiness');
  console.log('[1/7] Startup & Readiness verified.', readiness.applicationName, readiness.applicationVersion, 'core', readiness.coreVersion, 'os', readiness.operatingSystem);

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

  const artifactTitle = 'Synthesized Idea Artifact';
  const artifactBody = 'Verified idea body content\nTags: synthetic, test';
  const artRes = await invoke('artifact_command', {
    command: 'register',
    request: {
      contractVersion: 1,
      workspaceId: wsId,
      projectId: projId,
      type: 'idea',
      title: artifactTitle,
      body: artifactBody,
      privacy: 'standard'
    }
  });
  // NQ-01: the register response carries the identifier as
  // `artifact.artifactId` (ArtifactRecord); the old code read
  // `artifact.id`, printed `undefined`, and asserted nothing.
  const expectedArtifact = {
    projectId: projId,
    type: 'idea',
    version: 1,
    title: artifactTitle,
    body: artifactBody
  };
  const artifactId = assertArtifactDetails(artRes, expectedArtifact, 'artifact registration');
  console.log('[5/7] Registered artifact ID:', artifactId);

  // Read the exact artifact back through native IPC and assert identity
  // and content match the registration.
  const readback = await invoke('artifact_command', {
    command: 'read',
    request: { contractVersion: 1, workspaceId: wsId, artifactId, version: 1 }
  });
  assertArtifactDetails(readback, { artifactId, ...expectedArtifact }, 'artifact readback');
  console.log('[5/7] Artifact readback verified via native IPC.');

  // Workspace scoping: the same artifact id must not be readable from a
  // different workspace. (ArtifactRecord carries no workspace field, so
  // the scoped read plus this negative probe are the workspace assertion.)
  const ws2Res = await invoke('workspace_command', {
    command: 'create',
    request: { contractVersion: 1, name: wsName + ' scope probe' }
  });
  const ws2Id = ws2Res.workspace.id;
  let scopeProbeFailed = false;
  try {
    await invoke('artifact_command', {
      command: 'read',
      request: { contractVersion: 1, workspaceId: ws2Id, artifactId, version: 1 }
    });
  } catch {
    scopeProbeFailed = true;
  }
  if (!scopeProbeFailed) throw Error('FAIL: Artifact was readable from a different workspace; workspace scoping violated.');
  console.log('[5/7] Artifact correctly unreadable from a different workspace.');

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

  // NQ-01: reopen the same artifact after the restart and verify its
  // identity and content persisted.
  const verifyArtifact = await invoke('artifact_command', {
    command: 'read',
    request: { contractVersion: 1, workspaceId: wsId, artifactId, version: 1 }
  });
  assertArtifactDetails(verifyArtifact, { artifactId, ...expectedArtifact }, 'artifact after restart');
  console.log('[6/7] PASS: Artifact persisted and survived desktop restart.');

  // Step 5: Screen capture & Clean Exit
  await page.screenshot({ path: 'test-results/native-candidate-verified.png', fullPage: true });
  console.log('[7/7] Captured screenshot: test-results/native-candidate-verified.png');

  console.log('--- ALL 7 NATIVE CANDIDATE ACCEPTANCE CHECKS PASSED ---');
} finally {
  await stopCandidate();
}
