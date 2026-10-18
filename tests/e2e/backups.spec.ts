// @ts-expect-error shared executable acceptance workflow
import {prepareBackup,restoreBackup,checkBackup} from '../../scripts/backup-workflow.mjs';
// @ts-expect-error shared executable acceptance workflow
import {roundTripSavedBackup} from '../../scripts/backup-file-workflow.mjs';
import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

let child: ChildProcess, root: string, file: string, pairing: { endpoint: string; token: string };

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
  if (child?.exitCode === null && child.signalCode === null) {
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
  root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-backup-browser-'));
  await start();
});

test.afterAll(stop);

async function pair(page: import('@playwright/test').Page) {
  await page.goto('/');
  await page.getByLabel('Local API address').fill(pairing.endpoint);
  await page.getByLabel('Session token').fill(pairing.token);
  await page.getByRole('button', { name: 'Connect to core' }).click();
  await expect(page.getByRole('button', { name: 'Refresh workspaces' })).toBeEnabled({ timeout: 20000 });
}


test('backup captures real catalog and restores all entity/history data after restart',async({page})=>{
  test.setTimeout(1200000);let external=0;page.on('request',r=>{if(!['127.0.0.1','tauri.localhost'].includes(new URL(r.url()).hostname))external++;});
  await pair(page);await page.getByRole('button',{name:'Continue to workspace'}).click();
  await page.getByLabel('New workspace name').fill('Backup browser fixture');await page.getByRole('button',{name:'Create workspace'}).click();
  await page.getByLabel('I have reviewed these privacy limits').check();await page.getByRole('button',{name:'Continue to AI setup'}).click();await page.getByRole('button',{name:'Skip AI setup and enter ANANSI'}).click();
  const original=await page.locator('.workspace-id code').innerText();const job=await prepareBackup(page,'Browser');
  const imported=await roundTripSavedBackup(page,job,root);
  const restored=await restoreBackup(page,imported,'Restored browser fixture');expect(restored).not.toBe(original);await checkBackup(page,'Browser');
  await stop();await start();await pair(page);await page.getByLabel('Existing workspace').selectOption(restored);await page.getByRole('button',{name:'Open workspace',exact:true}).click();
  await checkBackup(page,'Browser');await page.getByRole('button',{name:'Backups',exact:true}).click();
  await page.setViewportSize({width:390,height:844});expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  expect(external).toBe(0);await page.screenshot({path:'test-results/browser-backup.png',fullPage:true});
});

