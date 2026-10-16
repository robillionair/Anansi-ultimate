import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
let child:ChildProcess,root:string,file:string,pairing:{endpoint:string,token:string};
async function start(){
  child=spawn(path.resolve('target/debug/anansi-api'+(process.platform==='win32'?'.exe':'')),['--port','0','--pairing-root',root],{windowsHide:true,env:{...process.env,ANANSI_STORAGE_ROOT:path.join(root,'storage')},stdio:['ignore','pipe','pipe']});
  await new Promise<void>((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('API startup timed out')),15000);let output='';child.stdout!.on('data',data=>{output+=data.toString();const match=output.match(/Pairing file: (.+)\r?\n/);if(match){file=match[1].trim();pairing=JSON.parse(fs.readFileSync(file,'utf8'));clearTimeout(timer);resolve();}});child.once('error',reject);});
}
async function stop(){
  if(child?.exitCode===null){const stopped=new Promise(resolve=>child.once('exit',resolve));child.kill();await stopped;}
  if(file&&fs.existsSync(file)){fs.unlinkSync(file);fs.rmdirSync(path.dirname(file));}
}
test.beforeAll(async()=>{root=fs.mkdtempSync(path.join(os.tmpdir(),'anansi-workspace-browser-'));await start();});
test.afterAll(async()=>{await stop();/* Preserve synthetic catalog outside Git for recovery inspection. */});
async function connect(page:import('@playwright/test').Page){
  await page.goto('/');await page.getByLabel('Local API address').fill(pairing.endpoint);await page.getByLabel('Session token').fill(pairing.token);await page.getByRole('button',{name:'Connect to core'}).click();
  await expect(page.locator('.badge')).toHaveText('Connected');await expect(page.getByRole('button',{name:'Refresh workspaces'})).toBeEnabled({timeout:15000});
}
test('workspace creation, duplicate recovery, durable motion setting and API restart',async({page})=>{
  test.setTimeout(90000);await page.emulateMedia({reducedMotion:'no-preference'});await connect(page);
  await page.getByRole('button',{name:'Continue to workspace'}).click();
  await page.getByLabel('New workspace name').fill('Synthetic browser workspace');await page.getByRole('button',{name:'Create workspace'}).click();
  await expect(page.getByRole('heading',{name:'Review your privacy'})).toBeVisible({timeout:15000});
  await page.getByLabel('I have reviewed these privacy limits').check();await page.getByRole('button',{name:'Continue to AI setup'}).click();await page.getByRole('button',{name:'Skip AI setup and enter ANANSI'}).click();
  const id=(await page.locator('.workspace-id code').innerText());expect(id).toMatch(/^[a-f0-9]{32}$/);
  await page.getByRole('button',{name:'Settings',exact:true}).click();
  await page.getByRole('button',{name:'Turn reduced motion on'}).click();await expect(page.getByRole('button',{name:'Turn reduced motion off'})).toBeEnabled({timeout:15000});
  await expect.poll(()=>page.locator('.badge').evaluate(element=>getComputedStyle(element).transitionDuration)).toBe('0s');
  await page.getByRole('button',{name:'Workspace',exact:true}).click();await page.getByRole('button',{name:'Close workspace'}).click();await expect(page.getByRole('button',{name:'Refresh workspaces'})).toBeEnabled({timeout:15000});
  await page.getByLabel('New workspace name').fill('synthetic browser workspace');await page.getByRole('button',{name:'Create workspace'}).click();await expect(page.getByRole('alert')).toContainText('already exists',{timeout:15000});
  await stop();await start();await connect(page);
  await page.getByLabel('Existing workspace').selectOption(id);await page.getByRole('button',{name:'Open workspace',exact:true}).click();
  await expect(page.locator('.workspace-id code')).toHaveText(id,{timeout:15000});await page.getByRole('button',{name:'Settings',exact:true}).click();await expect(page.getByRole('button',{name:'Turn reduced motion off'})).toBeVisible();
  await expect.poll(()=>page.evaluate(()=>document.documentElement.dataset.reducedMotion)).toBe('true');
  expect(await page.evaluate(()=>({local:localStorage.length,session:sessionStorage.length}))).toEqual({local:0,session:0});
  await page.setViewportSize({width:390,height:844});expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  await page.screenshot({path:'test-results/browser-workspace.png',fullPage:true});
});
