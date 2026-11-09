// Desktop WebView2 File-Picker Regression Suite
// Target Binary: target/debug/anansi-desktop.exe / target/release/anansi-desktop.exe
// Commit: 3f41281a90aac65afc7065dcba77a1f5ab281a90
//
// SCOPE & BOUNDARY:
// Exercises anansi-desktop.exe via Playwright CDP connection.
// Playwright CDP attaches with file chooser interception enabled (Page.setInterceptFileChooserDialog).
// This suite verifies the desktop application's internal document handling pipeline:
// 1. HTML input accept attribute (.txt,.md,text/plain,text/markdown)
// 2. Cancellation of file chooser event (no files passed) -> Beam A unchanged, zero error alerts
// 3. Unicode filename handling via setInputFiles (proov-küljendus-2026.md) -> text decoding, attribution, byte/char count, SHA-256
// 4. Same-name reselection via setInputFiles after altering disk contents -> beam text and SHA-256 update
// 5. Screenshot capture: test-results/desktop-os-picker-verified.png
//
// NOTE: This test exercises the WebView2 event interception path. It does NOT observe the native
// Win32 Common Item Dialog (#32770) or Windows filter dropdown UI. Visual confirmation of the real
// OS dialog is retained as a manual gate for the interactive operator (see docs/reviews/owner-manual-security-checklist.md).

import { chromium, expect } from '@playwright/test';
import { spawn } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

if (process.platform !== 'win32') throw Error('Windows native verification requires Windows.');

const port = 19340;
const testDir = path.resolve('E:/Anansi/tmp/interactive-picker-test');
const storageRoot = path.join(testDir, 'storage');
const udfPath = path.join(testDir, 'webview-udf');
const fixturesDir = path.join(testDir, 'fixtures');
fs.mkdirSync(storageRoot, { recursive: true });
fs.mkdirSync(udfPath, { recursive: true });
fs.mkdirSync(fixturesDir, { recursive: true });
fs.mkdirSync('test-results', { recursive: true });

// Prepare initial Unicode fixture file
const unicodeFileName = 'proov-küljendus-2026.md';
const unicodeFilePath = path.join(fixturesDir, unicodeFileName);
const initialText = '# Peatükk 1: Sissejuhatus\nSee on Unicode tekst eesti keeles (ä ö õ ü š ž) failisüsteemis.\nTeine rida märkmetega.\n';
fs.writeFileSync(unicodeFilePath, initialText, 'utf8');

const initialBytes = Buffer.byteLength(initialText, 'utf8');
const initialChars = initialText.length;
const initialHash = crypto.createHash('sha256').update(initialText, 'utf8').digest('hex');

// Modified text for same-name reselection
const modifiedText = '# Peatükk 1: Sissejuhatus (Muudetud)\nUus sisu sama failinimega proovimiseks (õ ä ö ü).\nKolmas rida muudatustega täiendatud.\n';
const modifiedBytes = Buffer.byteLength(modifiedText, 'utf8');
const modifiedChars = modifiedText.length;
const modifiedHash = crypto.createHash('sha256').update(modifiedText, 'utf8').digest('hex');

console.log('--- Desktop WebView2 File-Picker Regression Suite Starting ---');
console.log('Fixture file:   ', unicodeFilePath);
console.log('Initial Hash:   ', initialHash);
console.log('Modified Hash:  ', modifiedHash);

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

  await page.waitForFunction(() => !!globalThis.__TAURI_INTERNALS__);
  console.log('[Native Setup] Connected to Tauri/WebView2 runtime.');

  // Create test workspace via IPC and complete onboarding
  const wsRes = await page.evaluate(async () => {
    return await globalThis.__TAURI_INTERNALS__.invoke('workspace_command', {
      command: 'create',
      request: { contractVersion: 1, name: 'Interactive Picker WS' }
    });
  });
  const wsId = wsRes.workspace.id;

  await page.evaluate(async (id) => {
    return await globalThis.__TAURI_INTERNALS__.invoke('workspace_command', {
      command: 'settings',
      request: { contractVersion: 1, id, reducedMotion: true, onboardingComplete: true }
    });
  }, wsId);

  await page.evaluate(async (wsId) => {
    return await globalThis.__TAURI_INTERNALS__.invoke('memory_command', {
      command: 'create',
      request: {
        contractVersion: 1,
        workspaceId: wsId,
        kind: 'project',
        draft: {
          title: 'Harbor App',
          body: 'Project for interactive file picker qualification.',
          privacy: 'private',
          eventDate: null,
          validFrom: null,
          validUntil: null,
          confidence: null,
          sourceReferences: []
        }
      }
    });
  }, wsId);
  console.log('[Native Setup] Test workspace & project memory seeded:', wsId);

  // Navigate UI into workspace -> Projects -> Harbor App (Lexi-Collider)
  const continueBtn = page.getByRole('button', { name: 'Continue to workspace' });
  if (await continueBtn.isVisible()) await continueBtn.click();
  await page.getByRole('button', { name: 'Refresh workspaces' }).click();
  await page.getByLabel('Existing workspace').selectOption(wsId);
  await page.getByRole('button', { name: 'Open workspace', exact: true }).click();
  await page.getByRole('button', { name: 'Projects' }).click();
  await page.getByRole('button', { name: 'Harbor App' }).click();

  const colliderInput = page.locator('#collider-document-a');
  await colliderInput.waitFor({ state: 'attached', timeout: 15000 });
  await colliderInput.scrollIntoViewIfNeeded();
  console.log('[Native Setup] Navigated to Harbor App Collider.');

  // =========================================================================
  // STEP 1: Supported File Filters (DOM accept attribute)
  // =========================================================================
  console.log('\n[1/5] Verifying supported file filters on Collider input...');
  const acceptAttr = await colliderInput.getAttribute('accept');
  console.log('App Observation: accept attribute =', acceptAttr);
  expect(acceptAttr).toBe('.txt,.md,text/plain,text/markdown');
  console.log('PASS: HTML input accept attribute matches expected file filters.');
  console.log('NOTE: Native Win32 filter dropdown UI inspection remains a manual operator check.');

  // =========================================================================
  // STEP 2: Cancel Action via Intercepted FileChooser Event
  // =========================================================================
  console.log('\n[2/5] Testing file picker Cancel action via filechooser event...');
  const beamABefore = await page.locator('#collider-beam-a').inputValue();

  // Trigger file chooser event and dismiss without passing files
  const fileChooserPromise = page.waitForEvent('filechooser');
  await colliderInput.click();
  const fileChooser = await fileChooserPromise;
  console.log('WebView2 Event Observation: Intercepted filechooser event caught. isMultiple =', fileChooser.isMultiple());
  console.log('WebView2 Event Action: Filechooser dismissed without selecting files (simulating cancel).');

  // Verify app reaction
  const beamAAfter = await page.locator('#collider-beam-a').inputValue();
  expect(beamAAfter).toBe(beamABefore);
  const alerts = await page.locator('[role="alert"]').count();
  expect(alerts).toBe(0);
  console.log('App Observation (Cancel):');
  console.log('  - Beam A remains unmodified (empty)');
  console.log('  - Zero error alerts rendered');
  console.log('PASS: Cancel handled cleanly without state corruption in WebView2.');

  // =========================================================================
  // STEP 3: Select Unicode Filename via setInputFiles
  // =========================================================================
  console.log(`\n[3/5] Testing selection of Unicode filename via setInputFiles: ${unicodeFileName}...`);
  console.log('WebView2 Event Action: setInputFiles dispatch with path:', unicodeFilePath);

  await colliderInput.setInputFiles(unicodeFilePath);

  // Verify app state
  await expect(page.locator('#collider-beam-a')).toHaveValue(initialText, { timeout: 10000 });
  await expect(page.getByText(`Document: ${unicodeFileName}`)).toBeVisible();
  await expect(page.getByText('Markdown')).toBeVisible();
  await expect(page.getByText(`${initialBytes} bytes`)).toBeVisible();
  await expect(page.getByText(`${initialChars} characters`)).toBeVisible();
  await expect(page.getByText(initialHash.slice(0, 16))).toBeVisible();

  console.log('App Observation (Select Unicode):');
  console.log(`  - Beam text matches initial fixture (${initialChars} chars)`);
  console.log(`  - Attribution label: Document: ${unicodeFileName} · Markdown`);
  console.log(`  - Byte count: ${initialBytes} bytes`);
  console.log(`  - SHA-256 match: ${initialHash.slice(0, 16)}...`);
  console.log('PASS: Unicode filename decoded, attributed, and hashed correctly in application.');

  // =========================================================================
  // STEP 4: Same-Name Reselection After Changing File Contents on Disk
  // =========================================================================
  console.log('\n[4/5] Modifying file contents on disk and reselecting same filename...');
  fs.writeFileSync(unicodeFilePath, modifiedText, 'utf8');
  console.log(`File ${unicodeFilePath} updated on disk with new content.`);
  console.log('New Modified Hash:', modifiedHash);

  // Reselect same file path via setInputFiles
  await colliderInput.setInputFiles(unicodeFilePath);

  // Verify app state updated to modified text and modified hash
  await expect(page.locator('#collider-beam-a')).toHaveValue(modifiedText, { timeout: 10000 });
  await expect(page.getByText(`Document: ${unicodeFileName}`)).toBeVisible();
  await expect(page.getByText(`${modifiedBytes} bytes`)).toBeVisible();
  await expect(page.getByText(`${modifiedChars} characters`)).toBeVisible();
  await expect(page.getByText(modifiedHash.slice(0, 16))).toBeVisible();

  console.log('App Observation (Same-Name Reselect):');
  console.log(`  - Beam text updated to modified content (${modifiedChars} chars)`);
  console.log(`  - Attribution label: Document: ${unicodeFileName} · Markdown`);
  console.log(`  - Byte count: ${modifiedBytes} bytes`);
  console.log(`  - SHA-256 match: ${modifiedHash.slice(0, 16)}...`);
  console.log('PASS: Same-name reselection after content alteration successfully verified.');

  // =========================================================================
  // STEP 5: Screenshot & Evidence
  // =========================================================================
  const shotPath = 'test-results/desktop-os-picker-verified.png';
  await page.screenshot({ path: shotPath, fullPage: true });
  console.log(`\n[5/5] Verification screenshot captured: ${shotPath}`);

  console.log('\n--- ALL WEBVIEW2 FILE-PICKER REGRESSION CHECKS PASSED (Win32 #32770 OS Dialog remains pending manual operator execution) ---');
} finally {
  if (browser) await browser.close();
  if (child && child.exitCode === null) {
    const exited = new Promise(resolve => child.once('exit', resolve));
    child.kill();
    await exited;
  }
  try { fs.rmSync(testDir, { recursive: true, force: true }); } catch {}
}
