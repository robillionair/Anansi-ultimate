// Test-only CDP access; never enabled by the production launcher or configuration.
import { chromium, expect } from '@playwright/test';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
if(process.platform!=='win32')throw Error('Windows desktop verification requires Windows.');
const port=19323;
const child=spawn(path.resolve(process.argv[2]||'target/debug/anansi-desktop.exe'),[],{env:{...process.env,WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:`--remote-debugging-port=${port}`},windowsHide:true,stdio:'ignore'});
let browser;
try{
  for(let n=0;n<150;n++){
    try{const response=await fetch(`http://127.0.0.1:${port}/json/version`);if(response.ok)break;}catch{}
    if(child.exitCode!==null)throw Error('Desktop exited before WebView2 startup');
    await new Promise(resolve=>setTimeout(resolve,200));
  }
  browser=await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  console.log('Native WebView2 version:',browser.version());
  const context=browser.contexts()[0];
  let page;
  for(let n=0;n<100;n++){page=context.pages().find(p=>p.url().includes('tauri.localhost')||p.url().includes('127.0.0.1:1420'));if(page)break;await new Promise(r=>setTimeout(r,100));}
  if(!page)throw Error('Native application page not found');
  page.on('pageerror',error=>console.error('Native page error:',error.message));
  page.on('console',message=>{if(message.type()==='error')console.error('Native console:',message.text());});
  page.on('requestfailed',request=>{if(request.failure()?.errorText!=='net::ERR_ABORTED')console.error('Native resource failed:',request.url(),request.failure()?.errorText);});
  await page.reload();
  await expect(page.getByRole('status')).toHaveText('Connected',{timeout:20000});
  await expect(page.getByText('Windows desktop',{exact:false})).toBeVisible();
  await expect(page.locator('dd').filter({hasText:'windows'})).toBeVisible();
  const first=Number((await page.locator('dd').nth(3).innerText()).replace(' ms',''));
  await page.getByRole('button',{name:'Check again'}).click();
  await expect.poll(async()=>Number((await page.locator('dd').nth(3).innerText()).replace(' ms',''))).toBeGreaterThan(first);
  const rejected=await page.evaluate(async()=>{
    try{await window.__TAURI_INTERNALS__.invoke('get_readiness',{request:{contractVersion:2}});return null;}catch(e){return e;}
  });
  if(rejected?.code!=='UNSUPPORTED_CONTRACT')throw Error('Native contract error parity failed');
  const denied=await page.evaluate(async()=>{
    try{await window.__TAURI_INTERNALS__.invoke('plugin:shell|execute',{program:'invalid-test-command'});return '';}catch(error){return typeof error==='string'?error:JSON.stringify(error);}
  });
  if(!/plugin shell not found|shell\.execute not allowed|shell:execute.*not allowed|Command plugin:shell\|execute not allowed by ACL/i.test(denied))throw Error('Shell was not explicitly rejected by the capability boundary: '+denied);
  fs.mkdirSync('test-results',{recursive:true});await page.screenshot({path:'test-results/desktop-connected.png',fullPage:true});
  console.log('PASS: actual Windows Tauri/WebView2 readiness, live refresh, contract rejection and shell denial.');
}finally{
  if(browser)await browser.close();
  if(child.exitCode===null){const stopped=new Promise(resolve=>child.once('exit',resolve));child.kill();await stopped;}
}
