// Actual cross-adapter workflow using synthetic external storage only.
import { prepareExport, downloadExport, inspectExport } from './export-workflow.mjs';
import { chromium, expect } from '@playwright/test';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { assertPortsFree } from './verify-ports.mjs';

if (process.platform !== 'win32') throw Error('Cross-adapter verification requires Windows.');
await assertPortsFree([1420, 19323]);

const root = fs.mkdtempSync(path.join(process.env.ANANSI_VERIFIER_ROOT || os.tmpdir(), 'anansi-export-adapter-'));
const env = { ...process.env, ANANSI_STORAGE_ROOT: path.join(root, 'storage') };
console.log('Verification evidence: '+root);

if (process.argv.includes('--configured-cdp')) delete env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS;
else env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=19323';

const binary = process.argv.slice(2).find(argument => !argument.startsWith('--')) || 'target/debug/anansi-desktop.exe';

let api, desktop, preview, nativeBrowser, webBrowser, nativePage, webPage, pairing, pairingFile, dialog;

async function kill(child) {
  if (child && child.exitCode === null) {
    const stopped = new Promise(resolve => child.once('exit', resolve));
    child.kill();
    await stopped;
  }
}

async function waitFor(url, child) {
  for (let n = 0; n < 150; n++) {
    try {
      if ((await fetch(url)).ok) return;
    } catch {}
    if (child.exitCode !== null) throw Error('Test service exited');
    await new Promise(r => setTimeout(r, 200));
  }
  throw Error('Test service startup timed out');
}

async function startApi() {
  api = spawn(
    path.resolve('target/debug/anansi-api.exe'),
    ['--port', '0', '--pairing-root', path.join(root, 'pairing')],
    { env, windowsHide: true, stdio: ['ignore', 'pipe', 'ignore'] }
  );
  await new Promise((resolve, reject) => {
    let output = '';
    const timer = setTimeout(() => reject(Error('API pairing timed out')), 20000);
    api.once('error', reject);
    api.stdout.on('data', data => {
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
  await kill(api);
  if (pairingFile && fs.existsSync(pairingFile)) {
    fs.unlinkSync(pairingFile);
    fs.rmdirSync(path.dirname(pairingFile));
  }
}

async function ownedSaveDialogPid() {
  const query=spawn('powershell.exe',['-NoProfile','-NonInteractive','-Command',"$ErrorActionPreference='Stop'; Get-CimInstance Win32_Process -Filter ('ParentProcessId='+[int]$env:ANANSI_EXPORT_DESKTOP_PID) | Where-Object { $_.Name -eq 'powershell.exe' -and $_.CommandLine -like '*SaveFileDialog*' -and $_.CommandLine -like '*Save complete knowledge export*' } | ForEach-Object { $dialogProcess=[Diagnostics.Process]::GetProcessById($_.ProcessId); if($dialogProcess.MainWindowTitle -like 'Save complete knowledge export*'){Write-Output $_.ProcessId} }"],{windowsHide:true,env:{...process.env,ANANSI_EXPORT_DESKTOP_PID:String(desktop.pid)},stdio:['ignore','pipe','ignore']});
  let output='';query.stdout.on('data',data=>output+=data);
  await new Promise((resolve,reject)=>{query.once('error',reject);query.once('exit',code=>code===0?resolve():reject(Error('Owned Save dialog query failed')));});
  const pid=Number(output.trim());return Number.isSafeInteger(pid)&&pid>0?pid:undefined;
}

async function startDesktop() {
  desktop = spawn(path.resolve(binary), [], { env, windowsHide: true, stdio: 'ignore' });
  await waitFor('http://127.0.0.1:19323/json/version', desktop);
  nativeBrowser = await chromium.connectOverCDP('http://127.0.0.1:19323');
  for (let n = 0; n < 100; n++) {
    nativePage = nativeBrowser.contexts()[0].pages().find(page => page.url().includes('tauri.localhost'));
    if (nativePage) break;
    await new Promise(r => setTimeout(r, 100));
  }
  if (!nativePage) throw Error('Native page missing');
  await expect(nativePage.locator('.badge')).toHaveText('Connected', { timeout: 20000 });
  await expect(nativePage.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled({ timeout: 15000 });
}

async function stopDesktop() {
  // The trusted Save dialog runs in an owned PowerShell child. Stop only that
  // known child's dialog source before ending its desktop parent on any failure.
  let cleanupFailure;
  if(desktop?.pid){
    try {
      const cleanup=spawn('powershell.exe',['-NoProfile','-NonInteractive','-Command',"$ErrorActionPreference='Stop'; Get-CimInstance Win32_Process -Filter ('ParentProcessId='+[int]$env:ANANSI_EXPORT_DESKTOP_PID) | Where-Object { $_.Name -eq 'powershell.exe' -and $_.CommandLine -like '*SaveFileDialog*' -and $_.CommandLine -like '*Save complete knowledge export*' } | ForEach-Object { Stop-Process -Id $_.ProcessId -ErrorAction Stop }"],{windowsHide:true,env:{...process.env,ANANSI_EXPORT_DESKTOP_PID:String(desktop.pid)},stdio:'ignore'});
      await new Promise((resolve,reject)=>{cleanup.once('error',reject);cleanup.once('exit',code=>code===0?resolve():reject(Error('Owned export dialog cleanup failed')));});
    }catch(error){cleanupFailure=error;}
  }
  if (nativeBrowser) {
    await nativeBrowser.close();
    nativeBrowser = null;
  }
  await kill(desktop);
  if(cleanupFailure)throw cleanupFailure;
}

async function pairBrowser() {
  await webPage.goto('http://127.0.0.1:1420');
  await webPage.getByLabel('Local API address').fill(pairing.endpoint);
  await webPage.getByLabel('Session token').fill(pairing.token);
  await webPage.getByRole('button', { name: 'Connect to core' }).click();
  await expect(webPage.locator('.badge')).toHaveText('Connected');
  await expect(webPage.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled({ timeout: 15000 });
}

async function create(page, name) {
  if (await page.getByRole('button', { name: 'Continue to workspace' }).isVisible()) {
    await page.getByRole('radio', { name: /Build an application/ }).check();
    await page.getByRole('button', { name: 'Continue to workspace' }).click();
  }
  await page.getByLabel('New workspace name').fill(name);
  await page.getByRole('button', { name: 'Create workspace' }).click();
  await expect(page.getByRole('heading', { name: 'Review your privacy' })).toBeFocused({ timeout: 15000 });
  await expect(page.getByRole('button', { name: 'Continue to AI setup' })).toBeDisabled();
  await page.getByLabel('I have reviewed these privacy limits').focus();
  await page.keyboard.press('Space');
  await expect(page.getByRole('button', { name: 'Continue to AI setup' })).toBeEnabled({ timeout: 15000 });
  await page.keyboard.press('Tab');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { name: 'Choose AI setup' })).toBeFocused();
  await page.getByRole('button', { name: 'Skip AI setup and enter ANANSI' }).click();
  await expect(page.getByRole('heading', { name, exact: true })).toBeVisible({ timeout: 15000 });
  return page.locator('.workspace-id code').innerText();
}

async function open(page, id) {
  await page.getByRole('button', { name: 'Refresh workspaces' }).click();
  await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled({ timeout: 15000 });
  await page.getByLabel('Existing workspace').selectOption(id);
  await page.getByRole('button', { name: 'Open workspace', exact: true }).click();
  await expect(page.locator('.workspace-id code')).toHaveText(id, { timeout: 15000 });
}

try {
  preview = spawn(
    process.execPath,
    [path.resolve('apps/web/node_modules/vite/bin/vite.js'), 'preview', '--host', '127.0.0.1', '--port', '1420', '--strictPort'],
    { cwd: path.resolve('apps/web'), windowsHide: true, stdio: 'ignore' }
  );
  await waitFor('http://127.0.0.1:1420', preview);
  await startApi();
  await startDesktop();

  webBrowser = await chromium.launch();
  webPage = await webBrowser.newPage();
  let external = 0;
  const destinations = new Set();

  webPage.on('console', msg => console.log('WEB LOG:', msg.type(), msg.text()));
  webPage.on('pageerror', err => console.log('WEB PAGE ERROR:', err));
  nativePage.on('console', msg => console.log('NATIVE LOG:', msg.type(), msg.text()));
  nativePage.on('pageerror', err => console.log('NATIVE PAGE ERROR:', err));

  function observe(page, native = false) {
    page.on('request', request => {
      const url = new URL(request.url());
      const localIpc = native && url.origin === 'http://ipc.localhost';
      if (!localIpc && !['127.0.0.1', 'tauri.localhost'].includes(url.hostname)) {
        external++;
        destinations.add(url.protocol + '//' + url.hostname);
      }
    });
  }

  observe(nativePage, true);
  observe(webPage);

  const workspaceId = await create(nativePage, 'Export verification workspace');
  await pairBrowser();
  await open(webPage, workspaceId);

  const nativeJob = await prepareExport(nativePage, 'Native');
  const browserJob = await prepareExport(webPage, 'Browser');
  const nativeDestination = path.join(root, 'native.tar');
  dialog = spawn('powershell.exe', ['-NoProfile', '-NonInteractive', '-Mta', '-Command', '& ([scriptblock]::Create([IO.File]::ReadAllText($env:ANANSI_DIALOG_SCRIPT))) -DesktopProcessId $env:ANANSI_DIALOG_PID -Destination $env:ANANSI_DIALOG_DESTINATION'], {windowsHide:true,env:{...process.env,ANANSI_DIALOG_SCRIPT:path.resolve('scripts/complete-export-dialog.ps1'),ANANSI_DIALOG_PID:String(desktop.pid),ANANSI_DIALOG_DESTINATION:nativeDestination},stdio:['ignore','pipe','pipe']});
  const dialogResult = new Promise(resolve=>{let stderr='';dialog.stderr.on('data',data=>stderr+=data);dialog.once('error',()=>resolve({code:-1,stderr:'Dialog automation could not start'}));dialog.once('exit',code=>resolve({code,stderr}));});
  await nativePage.locator(`[data-job-id="${nativeJob}"]`).getByRole('button',{name:'Save complete export',exact:true}).click();
  const selectedDialog = await dialogResult;
  if(selectedDialog.code!==0)throw Error('Owned native dialog verification failed: '+selectedDialog.stderr);
  await expect(nativePage.getByRole('status').filter({hasText:'Complete export saved to your selected file.'})).toBeVisible({timeout:60000});
  const nativeInspection = inspectExport(nativeDestination, 'Native');
  const browserDestination = path.join(root, 'browser.tar');
  await downloadExport(webPage, browserJob, browserDestination);
  const browserInspection = inspectExport(browserDestination, 'Browser');

  // Interrupt an actual second Save dialog without choosing a file. Its real
  // parent monitor must terminate it; test cleanup must not hide this assertion.
  await nativePage.locator(`[data-job-id="${nativeJob}"]`).getByRole('button',{name:'Save complete export',exact:true}).click();
  let pendingDialog;
  await expect.poll(async()=>{pendingDialog=await ownedSaveDialogPid();return Boolean(pendingDialog);},{timeout:30000}).toBe(true);
  await kill(desktop);
  await expect.poll(()=>{try{process.kill(pendingDialog,0);return true;}catch{return false;}},{timeout:10000}).toBe(false);
  await stopDesktop();
  await stopApi();
  await startApi();
  await startDesktop();
  observe(nativePage, true);
  await pairBrowser();

  for (const page of [nativePage, webPage]) {
    await open(page, workspaceId);
  }

  for (const page of [nativePage,webPage]) await page.getByRole('button',{name:'Jobs and export',exact:true}).click();
  await expect(nativePage.locator(`[data-job-id="${nativeJob}"]`)).toContainText('Completed');
  await expect(webPage.locator(`[data-job-id="${browserJob}"]`)).toContainText('Completed');
  const restartedDestination=path.join(root,'browser-after-restart.tar');
  await downloadExport(webPage,browserJob,restartedDestination);
  expect(inspectExport(restartedDestination,'Browser').sha256).toBe(browserInspection.sha256);

  if (external) throw Error('Unexpected request destination: ' + [...destinations].join(', '));
  await nativePage.setViewportSize({ width: 390, height: 844 });
  if (!(await nativePage.evaluate(() => document.documentElement.scrollWidth <= innerWidth))) {
    throw Error('Native narrow overflow');
  }

  const screenshots=path.join(root,'screenshots');
  fs.mkdirSync(screenshots, { recursive: true });
  await nativePage.screenshot({ path: path.join(screenshots,'native-export-narrow.png'), fullPage: true });
  await nativePage.setViewportSize({ width: 1280, height: 1000 });
  await nativePage.screenshot({ path: path.join(screenshots,'native-export.png'), fullPage: true });
  await webPage.screenshot({ path: path.join(screenshots,'cross-browser-export.png'), fullPage: true });

  fs.writeFileSync(
    path.join(root, 'verification.json'),
    JSON.stringify(
      {
        nativeBrowser: true,
        portableMarkdownAndJson: true,
        nativeTrustedSaveDialog: true,
        browserObservedDownload: true,
        dialogParentTermination: true,
        nativeInspection,
        browserInspection,
        bothProcessRestarts: true,
        externalRequests: external,
      },
      null,
      2
    )
  );
  console.log(
    'PASS: actual native export save, browser download, independently extracted Markdown/JSON revisions, both process restarts, no external requests and narrow layout.'
  );
} finally {
  await kill(dialog);
  if (webBrowser) await webBrowser.close();
  await stopDesktop();
  await stopApi();
  await kill(preview);
}
