// Actual cross-adapter workflow using synthetic external storage only.
import { prepareBackup,restoreBackup,checkBackup } from './backup-workflow.mjs';
import {roundTripSavedBackup} from './backup-file-workflow.mjs';
import { chromium, expect } from '@playwright/test';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { assertPortsFree } from './verify-ports.mjs';

if (process.platform !== 'win32') throw Error('Cross-adapter verification requires Windows.');
await assertPortsFree([1420, 19323]);

const root = fs.mkdtempSync(path.join(process.env.ANANSI_VERIFIER_ROOT || os.tmpdir(), 'anansi-backup-adapter-'));
const env = { ...process.env, ANANSI_STORAGE_ROOT: path.join(root, 'storage') };
console.log('Verification evidence: '+root);

if (process.argv.includes('--configured-cdp')) delete env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS;
else env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=19323';

const binary = process.argv.slice(2).find(argument => !argument.startsWith('--')) || 'target/debug/anansi-desktop.exe';

let api, desktop, preview, nativeBrowser, webBrowser, nativePage, webPage, pairing, pairingFile, dialog;

async function kill(child) {
  if (child && child.exitCode === null && child.signalCode === null) {
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
  const query=spawn('powershell.exe',['-NoProfile','-NonInteractive','-Command',"$ErrorActionPreference='Stop'; Add-Type -AssemblyName UIAutomationClient; Add-Type -AssemblyName UIAutomationTypes; Get-CimInstance Win32_Process -Filter ('ParentProcessId='+[int]$env:ANANSI_EXPORT_DESKTOP_PID) | Where-Object { $_.Name -eq 'powershell.exe' -and $_.CommandLine -like '*SaveFileDialog*' -and $_.CommandLine -like '*Save complete knowledge export*' } | ForEach-Object { $ownedId=[int]$_.ProcessId; $condition=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ProcessIdProperty,$ownedId); $windows=[System.Windows.Automation.AutomationElement]::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children,$condition); foreach($window in $windows){if($window.Current.Name -like 'Save complete knowledge export*' -and -not $window.Current.IsOffscreen){Write-Output $ownedId}} }"],{windowsHide:true,env:{...process.env,ANANSI_EXPORT_DESKTOP_PID:String(desktop.pid)},stdio:['ignore','pipe','ignore']});
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
  preview=spawn(process.execPath,[path.resolve('apps/web/node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','1420','--strictPort'],{cwd:path.resolve('apps/web'),windowsHide:true,stdio:'ignore'});
  await waitFor('http://127.0.0.1:1420',preview); await startApi(); await startDesktop();
  webBrowser=await chromium.launch();webPage=await webBrowser.newPage();
  let external=0;
  function observe(page,native=false){page.on('request',request=>{const u=new URL(request.url());if(!(native&&u.origin==='http://ipc.localhost')&&!['127.0.0.1','tauri.localhost'].includes(u.hostname))external++;});}
  observe(nativePage,true);observe(webPage);
  const original=await create(nativePage,'Backup verification workspace');
  await pairBrowser();await open(webPage,original);
  const nativeJob=await prepareBackup(nativePage,'Native');
  const nativeRestored=await restoreBackup(nativePage,nativeJob,'Native recovered');
  await checkBackup(nativePage,'Native');
  const browserJob=await prepareBackup(webPage,'Native',false);
  const imported=await roundTripSavedBackup(webPage,browserJob,root);
  const browserRestored=await restoreBackup(webPage,imported,'Browser recovered');
  await checkBackup(webPage,'Native');
  await nativePage.getByRole('button',{name:'Backups',exact:true}).click();
  await nativePage.getByLabel('I consent to a plaintext copy',{exact:false}).check();
  await nativePage.getByLabel('Saved backup file',{exact:true}).setInputFiles(path.join(root,'backup-roundtrip.tar'));
  await nativePage.getByRole('button',{name:'Import saved backup',exact:true}).click();
  await expect(nativePage.getByRole('status').filter({hasText:'Saved backup verified.'})).toBeVisible({timeout:120000});
  if(nativeRestored===original||browserRestored===original||nativeRestored===browserRestored)throw Error('Restore reused workspace IDs');
  await stopDesktop();await stopApi();await startApi();await startDesktop();observe(nativePage,true);await pairBrowser();
  await open(nativePage,nativeRestored);await open(webPage,browserRestored);
  await checkBackup(nativePage,'Native');await checkBackup(webPage,'Native');
  await nativePage.getByRole('button',{name:'Backups',exact:true}).click();
  await webPage.getByRole('button',{name:'Backups',exact:true}).click();
  await nativePage.setViewportSize({width:390,height:844});
  await webPage.setViewportSize({width:390,height:844});
  for(const p of [nativePage,webPage])if(!await p.evaluate(()=>document.documentElement.scrollWidth<=innerWidth))throw Error('Backup narrow layout overflow');
  fs.mkdirSync(path.join(root,'screenshots'),{recursive:true});
  await nativePage.screenshot({path:path.join(root,'screenshots','native-backup-narrow.png'),fullPage:true});
  await webPage.screenshot({path:path.join(root,'screenshots','browser-backup-narrow.png'),fullPage:true});
  if(external)throw Error('External requests observed');
  fs.writeFileSync(path.join(root,'verification.json'),JSON.stringify({nativeBrowser:true,realCatalogRestore:true,completeEntityHistory:true,freshWorkspaceIds:true,bothProcessRestarts:true,scheduleConsent:true,externalRequests:external,original,nativeRestored,browserRestored},null,2));
  console.log('PASS: actual native/browser backup, preview, fresh restore, history/entities/settings, schedule consent, process restarts and narrow layout.');
} finally {
  if(webBrowser)await webBrowser.close();await stopDesktop();await stopApi();await kill(preview);
}

