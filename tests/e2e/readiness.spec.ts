import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
let child:ChildProcess, root:string, file:string, pairing:{endpoint:string,token:string};
test.beforeAll(async()=>{
  root=fs.mkdtempSync(path.join(os.tmpdir(),'anansi-browser-test-'));
  child=spawn(path.resolve('target/debug/anansi-api'+(process.platform==='win32'?'.exe':'')),['--port','0','--pairing-root',root],{windowsHide:true,env:{...process.env,ANANSI_STORAGE_ROOT:path.join(root,'storage')},stdio:['ignore','pipe','pipe']});
  await new Promise<void>((resolve,reject)=>{
    const timer=setTimeout(()=>reject(Error('API startup timed out')),15000);let output='';
    child.stdout!.on('data',data=>{output+=data.toString();const match=output.match(/Pairing file: (.+)\r?\n/);if(match){file=match[1].trim();pairing=JSON.parse(fs.readFileSync(file,'utf8'));clearTimeout(timer);resolve();}});
    child.once('error',reject);child.once('exit',code=>{if(code)reject(Error('API exited before pairing'));});
  });
});
test.afterAll(async()=>{
  if(child&&child.exitCode===null){const stopped=new Promise(resolve=>child.once('exit',resolve));child.kill();await stopped;}
  if(file&&fs.existsSync(file)){fs.unlinkSync(file);fs.rmdirSync(path.dirname(file));}
  if(root&&fs.existsSync(root))fs.rmdirSync(root);
});
async function connect(page:import('@playwright/test').Page){
  await page.goto('/');await page.getByLabel('Local API address').fill(pairing.endpoint);
  await page.getByLabel('Session token').fill(pairing.token);await page.getByRole('button',{name:'Connect to core'}).click();
  await expect(page.getByRole('status')).toHaveText('Connected');
  await page.getByRole('button',{name:'Connection',exact:true}).click();
}
test('real browser pairs, refreshes live core, forgets and stores no token',async({page})=>{
  await connect(page);await expect(page.getByText('ANANSI Ultimate',{exact:true})).toBeVisible();
  await expect(page.locator('dd').filter({hasText:process.platform==='win32'?'windows':process.platform})).toBeVisible();
  const uptime=Number((await page.locator('dd').nth(3).innerText()).replace(' ms',''));
  await page.getByRole('button',{name:'Check again'}).click();await expect.poll(async()=>Number((await page.locator('dd').nth(3).innerText()).replace(' ms',''))).toBeGreaterThan(uptime);
  expect(await page.evaluate(()=>({local:localStorage.length,session:sessionStorage.length,cookies:document.cookie,search:location.search,hash:location.hash}))).toEqual({local:0,session:0,cookies:'',search:'',hash:''});
  await page.getByRole('button',{name:'Forget session'}).click();await expect(page.getByLabel('Session token')).toHaveValue('');await expect(page.getByRole('status')).toHaveText('Not connected');
  await connect(page);await page.reload();await expect(page.getByRole('status')).toHaveText('Not connected');await expect(page.getByLabel('Session token')).toHaveValue('');
});
test('wrong token and unavailable API have recoverable states',async({page})=>{
  await page.goto('/');await page.getByLabel('Local API address').fill(pairing.endpoint);await page.getByLabel('Session token').fill('0'.repeat(64));await page.getByRole('button',{name:'Connect to core'}).click();
  await expect(page.getByRole('alert')).toContainText('Pairing was rejected');await expect(page.getByLabel('Session token')).toHaveValue('');
  await page.getByLabel('Local API address').fill('http://127.0.0.1:1');await page.getByLabel('Session token').fill(pairing.token);await page.getByRole('button',{name:'Connect to core'}).click();await expect(page.getByRole('alert')).toContainText('Local core unavailable');
  await connect(page);
});
test('narrow browser layout and keyboard form remain usable',async({page})=>{
  await page.setViewportSize({width:390,height:844});await page.goto('/');
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  await page.getByLabel('Local API address').focus();await page.keyboard.press('Tab');await expect(page.getByLabel('Session token')).toBeFocused();
  await connect(page);await page.screenshot({path:'test-results/browser-connected.png',fullPage:true});
});
