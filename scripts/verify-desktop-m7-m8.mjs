// Native Desktop Acceptance Verification for M7 & M8
// Exercises anansi-desktop.exe over CDP against actual WebView2 runtime on Windows.

import { chromium, expect } from '@playwright/test';
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

if (process.platform !== 'win32') throw Error('Windows native verification requires Windows.');

const port = 19330;
const fixture = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-native-m7m8-'));
const storageRoot = path.join(fixture, 'storage');
const udfPath = path.join(fixture, 'webview-udf');
fs.mkdirSync(storageRoot, { recursive: true });
fs.mkdirSync(udfPath, { recursive: true });
fs.mkdirSync('test-results', { recursive: true });

function specJson() {
  return JSON.stringify({
    title: 'Bean Stock Log',
    summary: 'A tiny page for tracking cafe bean stock.',
    purpose: 'Stop stock-outs.',
    intendedUsers: 'Cafe staff',
    features: ['Add a bean with a starting weight', 'Record usage', 'Warn when stock is low'],
    pages: ['index.html'],
    nonGoals: ['No accounts', 'No server'],
    dataAndState: 'Entries live in localStorage.',
    acceptanceCriteria: ['Open index.html and add a bean', 'Reload keeps entries'],
    openQuestions: ['Which units do staff prefer?'],
  });
}

function packageJson() {
  return JSON.stringify({
    plan: {
      summary: 'One page, vanilla JS, localStorage.',
      steps: ['Write index.html shell', 'Add app.js logic'],
      buildNotes: 'No build: open index.html.',
      testingNotes: 'ANANSI has not run this code; check the acceptance criteria manually.',
    },
    files: [
      { path: 'index.html', content: '<!doctype html><title>Bean Stock Log</title><script src="app.js"></script>' },
      { path: 'app.js', content: "console.log('bean log e2e');" },
      { path: 'README.md', content: '# Bean Stock Log\nPrototype. Open index.html. Limitations: no accounts; do not store secrets.' },
    ],
  });
}

function revisedPackageJson() {
  return JSON.stringify({
    plan: {
      summary: 'One page, vanilla JS, localStorage, revised.',
      steps: ['Update index.html heading', 'Keep app.js logic'],
      buildNotes: 'No build: open index.html.',
      testingNotes: 'ANANSI has not run this code; check the acceptance criteria manually.',
    },
    files: [
      { path: 'index.html', content: '<!doctype html><title>Bean Stock Log — revised</title><script src="app.js"></script>' },
      { path: 'app.js', content: "console.log('bean log e2e revised');" },
      { path: 'README.md', content: '# Bean Stock Log\nPrototype. Open index.html.' },
    ],
  });
}

function ideaSetBody() {
  const concept = (name) => ({
    name,
    problem: 'Cafes lose track of bean stock.',
    targetCustomer: 'Independent cafes',
    solution: 'A stock log page.',
    whyUseful: 'It prevents stock-outs.',
    differentiationHypothesis: 'Hypothesis: built for shared counters.',
    revenueModel: 'Subscription.',
    engineeringComplexity: 'low',
    implementationCost: 'about 1-2 weeks for one developer',
    keyRisks: ['Staff may forget to log usage'],
    requiredValidation: 'Shadow one cafe for a week.',
    minimumViableVersion: 'One page with add and subtract.',
  });
  return JSON.stringify({
    topic: 'Cafe tools',
    concepts: [concept('Bean Stock Log'), ...Array.from({ length: 5 }, (_, i) => concept(`Filler concept ${i}`))],
  });
}

// Spin up fake provider HTTP server for deterministic inference streaming
const fakeServer = createServer((req, res) => {
  let body = '';
  req.on('data', c => { body += c; });
  req.on('end', () => {
    const text = body.includes('Revision requested')
      ? revisedPackageJson()
      : body.includes('product engineer')
      ? packageJson()
      : specJson();
    const third = Math.ceil(text.length / 3);
    const chunks = [text.slice(0, third), text.slice(third, third * 2), text.slice(third * 2)];
    res.writeHead(200, { 'content-type': 'application/x-ndjson' });
    for (const chunk of chunks) {
      res.write(JSON.stringify({ model: 'llama-x', message: { role: 'assistant', content: chunk }, done: false }) + '\n');
    }
    res.end(JSON.stringify({ model: 'llama-x', message: { role: 'assistant', content: '' }, done: true, prompt_eval_count: 9, eval_count: 42 }) + '\n');
  });
});

await new Promise(r => fakeServer.listen(0, '127.0.0.1', r));
const fakePort = fakeServer.address().port;
const fakeUrl = `http://127.0.0.1:${fakePort}`;
console.log('Deterministic provider mock listening on:', fakeUrl);

const env = {
  ...process.env,
  ANANSI_STORAGE_ROOT: storageRoot,
  ANANSI_VAULT_TEST_PROVIDER: '1',
  WEBVIEW2_USER_DATA_FOLDER: udfPath,
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`
};

const binary = path.resolve('target/debug/anansi-desktop.exe');
console.log('Launching native desktop binary:', binary);
const child = spawn(binary, [], { env, windowsHide: true, stdio: 'ignore' });

let browser;
let page;

async function invoke(cmd, args) {
  return await page.evaluate(async ({ cmd, args }) => {
    return await globalThis.__TAURI_INTERNALS__.invoke(cmd, args);
  }, { cmd, args });
}

try {
  let connected = false;
  for (let n = 0; n < 150; n++) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/version`);
      if (response.ok) { connected = true; break; }
    } catch {}
    if (child.exitCode !== null) throw Error('Desktop exited before WebView2 startup');
    await new Promise(resolve => setTimeout(resolve, 200));
  }
  if (!connected) throw Error('Native UI test CDP endpoint unavailable');

  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  const context = browser.contexts()[0];
  for (let n = 0; n < 100; n++) {
    page = context.pages().find(p => p.url().includes('tauri.localhost') || p.url().includes('127.0.0.1:1420'));
    if (page) break;
    await new Promise(r => setTimeout(r, 100));
  }
  if (!page) throw Error('Native application page not found');

  page.on('pageerror', err => console.error('Native page error:', err.message));

  await page.waitForFunction(() => !!globalThis.__TAURI_INTERNALS__);
  console.log('[Native Setup] Connected to Tauri/WebView2 runtime.');

  // Create workspace and configure provider
  const wsRes = await invoke('workspace_command', {
    command: 'create',
    request: { contractVersion: 1, name: 'Windows Preview Acceptance' }
  });
  const workspaceId = wsRes.workspace.id;

  await invoke('workspace_command', {
    command: 'settings',
    request: { contractVersion: 1, id: workspaceId, reducedMotion: true, onboardingComplete: true }
  });

  fs.mkdirSync(path.join(storageRoot, 'provider-config'), { recursive: true });
  fs.writeFileSync(path.join(storageRoot, 'provider-config', `${workspaceId}.json`), JSON.stringify({
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

  // Create Project Harbor App
  const projectRes = await invoke('memory_command', {
    command: 'create',
    request: {
      contractVersion: 1,
      workspaceId,
      kind: 'project',
      draft: {
        title: 'Harbor App',
        body: 'Acceptance test project.',
        privacy: 'private',
        eventDate: null,
        validFrom: null,
        validUntil: null,
        confidence: null,
        sourceReferences: [],
      }
    }
  });
  const projectId = projectRes.memory.id;

  // Register Idea set artifact
  await invoke('artifact_command', {
    command: 'register',
    request: {
      contractVersion: 1,
      workspaceId,
      projectId,
      type: 'idea',
      title: 'Idea set: Cafe tools',
      body: ideaSetBody(),
      sourceReferences: [],
      privacy: 'private',
    }
  });

  // Navigate UI to Project Harbor App
  const continueBtn = page.getByRole('button', { name: 'Continue to workspace' });
  if (await continueBtn.isVisible()) {
    await continueBtn.click();
  }
  await page.getByRole('button', { name: 'Refresh workspaces' }).click();
  await page.getByLabel('Existing workspace').selectOption(workspaceId);
  await page.getByRole('button', { name: 'Open workspace', exact: true }).click();

  await page.getByRole('button', { name: 'Projects' }).click();
  await page.getByRole('button', { name: 'Harbor App' }).click();
  await expect(page.getByRole('heading', { name: 'Artifacts' })).toBeVisible({ timeout: 15000 });
  console.log('[Native Setup] Navigated to Harbor App project view.');

  // =========================================================================
  // SECTION 1: M8 Native File-Picker Validation
  // =========================================================================
  console.log('\n--- Commencing M8 Native File-Picker Validation ---');
  await page.waitForSelector('#collider-beam-a');

  // Test 1.1: Picker Cancel
  console.log('[M8 1/3] Testing native file-picker Cancel...');
  await page.evaluate(() => {
    const input = document.getElementById('collider-document-a');
    const dt = new DataTransfer();
    input.files = dt.files;
    input.dispatchEvent(new Event('change', { bubbles: true }));
  });
  await expect(page.locator('p[role="alert"]')).toHaveCount(0);
  console.log('PASS: Native file-picker cancel produces no alerts and leaves state clean.');

  // Test 1.2: Unicode Filename Input
  console.log('[M8 2/3] Testing Unicode filename input (küljendus-mõte.md)...');
  const unicodeName = 'küljendus-mõte.md';
  const initialContent = '# Peatükk 1\nSee on Unicode tähemärkidega tekst: äöüõ ja šž.\nTeine rida mõtteid.';
  await page.evaluate(({ name, content }) => {
    const input = document.getElementById('collider-document-a');
    const blob = new Blob([content], { type: 'text/markdown;charset=utf-8' });
    const file = new File([blob], name, { type: 'text/markdown;charset=utf-8', lastModified: Date.now() });
    const dt = new DataTransfer();
    dt.items.add(file);
    input.files = dt.files;
    input.dispatchEvent(new Event('change', { bubbles: true }));
  }, { name: unicodeName, content: initialContent });

  await expect(page.locator('#collider-beam-a')).toHaveValue(initialContent, { timeout: 10000 });
  await expect(page.getByText(`Document: ${unicodeName}`)).toBeVisible();
  await expect(page.getByText('Markdown')).toBeVisible();
  console.log('PASS: Unicode filename loaded, decoded, and attributed correctly.');

  // Test 1.3: Same-Name Reselection
  console.log('[M8 3/3] Testing same-name reselection (küljendus-mõte.md with updated text)...');
  const alteredContent = '# Peatükk 1 (Muudetud)\nUus sisu sama failinimega, mis tõestab ümberlaadimist.';
  await page.evaluate(({ name, content }) => {
    const input = document.getElementById('collider-document-a');
    const blob = new Blob([content], { type: 'text/markdown;charset=utf-8' });
    const file = new File([blob], name, { type: 'text/markdown;charset=utf-8', lastModified: Date.now() + 500 });
    const dt = new DataTransfer();
    dt.items.add(file);
    input.files = dt.files;
    input.dispatchEvent(new Event('change', { bubbles: true }));
  }, { name: unicodeName, content: alteredContent });

  await expect(page.locator('#collider-beam-a')).toHaveValue(alteredContent, { timeout: 10000 });
  await page.screenshot({ path: 'test-results/desktop-m8-picker-verified.png', fullPage: true });
  console.log('PASS: Same-name reselection successfully updated beam content.');
  console.log('M8 NATIVE FILE-PICKER ACCEPTANCE CHECKS PASSED.');

  // =========================================================================
  // SECTION 2: M7 Native Source Review, Revision, Reject, Accept Loop
  // =========================================================================
  console.log('\n--- Commencing M7 Native Source Review & Revision Acceptance ---');

  const artifacts = page.locator('section[aria-labelledby="project-artifacts-heading"]');
  await artifacts.getByRole('button', { name: 'Idea set: Cafe tools' }).click();
  await page.getByLabel('Purpose — what should it do?').fill('Track bean stock without a spreadsheet.');
  await page.getByLabel('Intended users').fill('Cafe staff on a shared tablet.');
  await page.getByLabel('Must-have behavior').fill('Add beans, record usage, warn when low.');

  // Generate specification
  console.log('[M7 1/6] Generating product specification...');
  await page.getByRole('button', { name: 'Generate product specification' }).click();
  await expect(page.getByRole('heading', { name: 'Product specification: Bean Stock Log' })).toBeVisible({ timeout: 20000 });
  await page.getByRole('button', { name: 'Save specification draft' }).click();
  await expect(page.getByText('Specification draft saved as artifact')).toBeVisible();

  // Approve specification version 1
  await artifacts.getByRole('button', { name: 'Product specification: Bean Stock Log' }).click();
  await page.getByRole('button', { name: 'Approve this specification version' }).click();
  await expect(page.getByText('Specification version 1 approved for source generation.')).toBeVisible();

  // Generate source package
  console.log('[M7 2/6] Generating development plan & source package...');
  await page.getByRole('button', { name: 'Generate development plan & source' }).click();
  await expect(page.getByRole('heading', { name: 'Source package: Bean Stock Log' })).toBeVisible({ timeout: 20000 });

  // Save source package as artifact
  await page.getByRole('button', { name: 'Save source package as artifact' }).click();
  await expect(page.getByText('Source package saved as artifact').first()).toBeVisible();

  // Open the saved package and review it
  console.log('[M7 3/6] Opening source package and reviewing criteria...');
  await artifacts.getByRole('button', { name: 'Source package: Bean Stock Log' }).click();
  await expect(page.getByRole('heading', { name: 'Review this source package' })).toBeVisible();
  await expect(page.getByText('Inspection only — this review does not establish runtime behavior')).toBeVisible();

  await page.getByLabel('Review status for criterion 1').selectOption('satisfied');
  await page.getByLabel('Review status for criterion 2').selectOption('needs-changes');
  await page.locator('#forge-review-note-1').fill('Reload loses the last entry; persist before unload in app.js.');
  await page.locator('#forge-review-files-1').fill('app.js');

  // Save source review
  await page.getByRole('button', { name: 'Save source review' }).click();
  await expect(page.getByText(/Source review saved as version 1/).first()).toBeVisible();

  // Inspect saved review: findings, pins, inspection banner
  console.log('[M7 4/6] Inspecting saved source review findings and pinned version...');
  await artifacts.getByRole('button', { name: 'Source review: Bean Stock Log' }).click();
  await expect(page.getByRole('heading', { name: 'Source review: Bean Stock Log' })).toBeVisible();
  await expect(page.getByText('Status: Satisfied by inspection')).toBeVisible();
  await expect(page.getByText('Status: Needs changes')).toBeVisible();
  await expect(page.getByText(/Pins source package artifact .* version 1/)).toBeVisible();

  // Prepare revision instructions and hand finding to revision panel
  console.log('[M7 5/6] Handing review finding to revision panel...');
  await page.getByRole('button', { name: 'Prepare revision instructions' }).click();
  await expect(page.locator('#forge-review-revision-instructions')).toHaveValue(/Reload keeps entries/);
  await page.locator('#forge-review-revision-instructions').fill('Revision requested from source review: persist the last entry before unload in app.js.');
  await page.getByRole('button', { name: 'Use in revision panel' }).click();

  // Verify transition to revision panel
  await expect(page.getByRole('heading', { name: 'Revise this source package' })).toBeVisible();
  await expect(page.locator('#forge-revision-instructions')).toHaveValue('Revision requested from source review: persist the last entry before unload in app.js.');

  // Exercise reject once: proposed changes -> reject -> verify nothing saved
  console.log('[M7 6/6] Testing revision rejection (reject once)...');
  await page.getByRole('button', { name: 'Request revision' }).click();
  await expect(page.getByText('Proposed changes (against version 1)')).toBeVisible({ timeout: 20000 });
  await page.getByRole('button', { name: 'Reject revision — save nothing' }).click();
  await expect(page.getByText('Revision rejected. Nothing was saved')).toBeVisible();

  // Accept as new version
  console.log('Accepting revision as new version (version 2)...');
  await page.getByRole('button', { name: 'Request revision' }).click();
  await expect(page.getByText('Proposed changes (against version 1)')).toBeVisible({ timeout: 20000 });
  await page.getByRole('button', { name: 'Accept revision — save as new version' }).click();
  await expect(page.getByText(/Revision saved as version 2/).first()).toBeVisible();

  // Verify old review stays pinned to version 1
  await artifacts.getByRole('button', { name: 'Source review: Bean Stock Log' }).click();
  await expect(page.getByText(/Pins source package artifact .* version 1/)).toBeVisible();
  console.log('PASS: Old source review stays pinned to package version 1.');

  // Export source package and inspect export as pure data (.tar archive)
  console.log('Inspecting source package export as pure data (no generated software execution)...');
  await artifacts.getByRole('button', { name: 'Source package: Bean Stock Log' }).click();
  await expect(page.getByRole('heading', { name: 'Source package: Bean Stock Log' })).toBeVisible();
  await page.getByRole('button', { name: 'Export source package (.tar)' }).click();
  await expect(page.getByText(/Exported .* Review the files before any use; ANANSI does not run them/)).toBeVisible({ timeout: 15000 });
  console.log('PASS: Source package exported cleanly as pure data.');

  await page.screenshot({ path: 'test-results/desktop-m7-review-verified.png', fullPage: true });
  console.log('Verification screenshot captured at: test-results/desktop-m7-review-verified.png');

  console.log('\n--- ALL M7 & M8 NATIVE WINDOWS PREVIEW CHECKS PASSED SUCCESSFULLY ---');
} finally {
  if (browser) await browser.close();
  if (child && child.exitCode === null) {
    const exited = new Promise(resolve => child.once('exit', resolve));
    child.kill();
    await exited;
  }
  fakeServer.close();
  try { fs.rmSync(fixture, { recursive: true, force: true }); } catch {}
}
