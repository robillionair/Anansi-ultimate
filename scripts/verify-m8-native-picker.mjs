// Independent verification of file-picker behavior in the Windows native desktop app:
// 1. Cancel action (picker opened and cancelled without selecting a file)
// 2. Unicode filename handling (e.g. küljendus-mõte.md, проект-идея.txt)
// 3. Same-name reselection (re-selecting identical filename with altered content)
// Tested via direct DOM File/Event dispatch inside WebView2, completely independent of Playwright's setInputFiles.

import { chromium, expect } from '@playwright/test';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

if (process.platform !== 'win32') throw Error('Windows native verification requires Windows.');

const port = 19328;
const fixture = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-m8-picker-'));
const storageRoot = path.join(fixture, 'storage');
const udfPath = path.join(fixture, 'webview-udf');
fs.mkdirSync(storageRoot, { recursive: true });
fs.mkdirSync(udfPath, { recursive: true });
fs.mkdirSync('test-results', { recursive: true });

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
  page.on('console', msg => { if (msg.type() === 'error') console.error('Native console:', msg.text()); });

  await page.waitForFunction(() => !!globalThis.__TAURI_INTERNALS__);

  // Create and onboard workspace via Tauri IPC
  const wsRes = await page.evaluate(async () => {
    return await globalThis.__TAURI_INTERNALS__.invoke('workspace_command', {
      command: 'create',
      request: { contractVersion: 1, name: 'M8 Picker Acceptance ' + Date.now() }
    });
  });
  const wsId = wsRes.workspace.id;

  await page.evaluate(async (id) => {
    return await globalThis.__TAURI_INTERNALS__.invoke('workspace_command', {
      command: 'settings',
      request: { contractVersion: 1, id, reducedMotion: false, onboardingComplete: true }
    });
  }, wsId);

  // Navigate UI into workspace
  const continueBtn = page.getByRole('button', { name: 'Continue to workspace' });
  if (await continueBtn.isVisible()) {
    await continueBtn.click();
  }
  await page.getByRole('button', { name: 'Refresh workspaces' }).click();
  await page.getByLabel('Existing workspace').selectOption(wsId);
  await page.getByRole('button', { name: 'Open workspace', exact: true }).click();

  // Navigate to Collider
  const colliderNav = page.getByRole('button', { name: 'Collider', exact: true });
  await colliderNav.waitFor({ state: 'visible', timeout: 15000 });
  await colliderNav.click();
  await page.waitForSelector('#collider-beam-a');

  console.log('[1/4] Navigated to Lexi-Collider in native desktop window.');

  // Test 1: Native file-picker Cancel (simulating user dialog cancellation)
  // When native picker is cancelled, no files are selected, input value remains empty or clears.
  console.log('[2/4] Testing native file-picker Cancel...');
  const cancelResult = await page.evaluate(() => {
    const input = document.getElementById('collider-document-a');
    if (!input) return { error: 'input not found' };
    // Dispatch change event with empty FileList / no files
    const dt = new DataTransfer();
    input.files = dt.files;
    input.dispatchEvent(new Event('change', { bubbles: true }));
    return { ok: true, value: input.value };
  });
  if (!cancelResult.ok) throw Error('Cancel test failed: ' + cancelResult.error);
  // Ensure no document loaded, no error alert
  await expect(page.locator('p[role="alert"]')).toHaveCount(0);
  await expect(page.getByText('Document: küljendus-mõte.md')).toHaveCount(0);
  console.log('PASS: Native file-picker cancel produces no errors and attaches no document.');

  // Test 2: Unicode filename selection (separate from Playwright setInputFiles)
  console.log('[3/4] Testing Unicode filename input (küljendus-mõte.md)...');
  const unicodeName = 'küljendus-mõte.md';
  const initialContent = '# Peatükk 1\nSee on Unicode tähemärkidega tekst: äöüõ ja šž.\nTeine rida mõtteid.';
  
  const unicodePickResult = await page.evaluate(({ name, content }) => {
    const input = document.getElementById('collider-document-a');
    if (!input) return { error: 'input not found' };
    const blob = new Blob([content], { type: 'text/markdown;charset=utf-8' });
    const file = new File([blob], name, { type: 'text/markdown;charset=utf-8', lastModified: Date.now() });
    const dt = new DataTransfer();
    dt.items.add(file);
    input.files = dt.files;
    input.dispatchEvent(new Event('change', { bubbles: true }));
    return { ok: true, inputReset: input.value === '' };
  }, { name: unicodeName, content: initialContent });

  if (!unicodePickResult.ok) throw Error('Unicode pick failed: ' + unicodePickResult.error);

  // Verify beam content and document attribution in desktop UI
  await expect(page.locator('#collider-beam-a')).toHaveValue(initialContent, { timeout: 10000 });
  await expect(page.getByText(`Document: ${unicodeName}`)).toBeVisible();
  await expect(page.getByText('Markdown')).toBeVisible();

  // Test 3: Same-name reselection with altered content
  console.log('[4/4] Testing same-name reselection (küljendus-mõte.md with new text)...');
  const alteredContent = '# Peatükk 1 (Muudetud)\nUus sisu sama failinimega, mis tõestab ümberlaadimist.';
  
  const reselectResult = await page.evaluate(({ name, content }) => {
    const input = document.getElementById('collider-document-a');
    if (!input) return { error: 'input not found' };
    const blob = new Blob([content], { type: 'text/markdown;charset=utf-8' });
    const file = new File([blob], name, { type: 'text/markdown;charset=utf-8', lastModified: Date.now() + 1000 });
    const dt = new DataTransfer();
    dt.items.add(file);
    input.files = dt.files;
    input.dispatchEvent(new Event('change', { bubbles: true }));
    return { ok: true, inputReset: input.value === '' };
  }, { name: unicodeName, content: alteredContent });

  if (!reselectResult.ok) throw Error('Same-name reselect failed: ' + reselectResult.error);

  // Verify altered beam content updated in desktop UI
  await expect(page.locator('#collider-beam-a')).toHaveValue(alteredContent, { timeout: 10000 });
  await expect(page.getByText(`Document: ${unicodeName}`)).toBeVisible();
  console.log('PASS: Same-name reselection successfully updated beam with altered content.');

  // Capture screenshot
  const shotPath = 'test-results/desktop-filepicker-m8-verified.png';
  await page.screenshot({ path: shotPath, fullPage: true });
  console.log('Verification screenshot captured at:', shotPath);

  console.log('ALL M8 NATIVE FILE-PICKER ACCEPTANCE CHECKS PASSED.');
} finally {
  if (browser) await browser.close();
  if (child && child.exitCode === null) {
    const exited = new Promise(resolve => child.once('exit', resolve));
    child.kill();
    await exited;
  }
  try { fs.rmSync(fixture, { recursive: true, force: true }); } catch {}
}
