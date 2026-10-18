// @ts-expect-error export-workflow is an mjs script
import { prepareExport, createSelectionExport, downloadExport, inspectExport } from '../../scripts/export-workflow.mjs';
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
  if (child?.exitCode === null) {
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
  root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-export-browser-'));
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

test('project export downloads open Markdown and JSON and survives service restart',async({page})=>{
  test.setTimeout(300000);let external=0;page.on('request',r=>{if(!['127.0.0.1','tauri.localhost'].includes(new URL(r.url()).hostname))external++;});
  await pair(page);await page.getByRole('button',{name:'Continue to workspace'}).click();
  await page.getByLabel('New workspace name').fill('Export browser fixture');await page.getByRole('button',{name:'Create workspace'}).click();
  await page.getByLabel('I have reviewed these privacy limits').check();await page.getByRole('button',{name:'Continue to AI setup'}).click();await page.getByRole('button',{name:'Skip AI setup and enter ANANSI'}).click();
  const workspaceId=await page.locator('.workspace-id code').innerText();const jobId=await prepareExport(page,'Browser');
  const before=path.join(root,'before.tar');await downloadExport(page,jobId,before);const inspected=inspectExport(before,'Browser');
  for(const scope of ['records','workspace']){
    const selectedJob=await createSelectionExport(page,scope,'Browser');const destination=path.join(root,scope+'.tar');
    await downloadExport(page,selectedJob,destination);inspectExport(destination,'Browser',scope);
  }
  await stop();await start();await pair(page);await page.getByLabel('Existing workspace').selectOption(workspaceId);await page.getByRole('button',{name:'Open workspace',exact:true}).click();
  await page.getByRole('button',{name:'Jobs and export',exact:true}).click();await expect(page.locator(`[data-job-id="${jobId}"]`)).toContainText('Completed');
  const after=path.join(root,'after.tar');await downloadExport(page,jobId,after);expect(inspectExport(after,'Browser').sha256).toBe(inspected.sha256);
  await page.setViewportSize({width:390,height:844});expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  expect(await page.evaluate(()=>Reflect.get(globalThis,'exportExecuted')??false)).toBe(false);expect(external).toBe(0);
  await page.screenshot({path:'test-results/browser-export.png',fullPage:true});
});
