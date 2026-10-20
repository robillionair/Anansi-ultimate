// Actual Tauri UI acceptance with the explicitly approved installed local model.
import {chromium,expect} from '@playwright/test';
import {spawn} from 'node:child_process';
import fs from 'node:fs';import os from 'node:os';import path from 'node:path';
import {assertPortsFree} from './verify-ports.mjs';
await assertPortsFree([19323]);
const root=fs.mkdtempSync(path.join(os.tmpdir(),'anansi-ask-native-'));
const child=spawn(path.resolve('target/debug/anansi-desktop.exe'),[],{windowsHide:true,env:{...process.env,ANANSI_STORAGE_ROOT:path.join(root,'storage')},stdio:'ignore'});
let browser;
try{
  let ready=false;
  for(let n=0;n<150;n++){try{if((await fetch('http://127.0.0.1:19323/json/version')).ok){ready=true;break;}}catch{}if(child.exitCode!==null)throw Error('Desktop exited');await new Promise(r=>setTimeout(r,200));}
  if(!ready)throw Error('Native WebView2 debug endpoint unavailable');
  browser=await chromium.connectOverCDP('http://127.0.0.1:19323');
  let page;
  for(let n=0;n<100;n++){page=browser.contexts().flatMap(context=>context.pages()).find(p=>p.url().includes('tauri.localhost'));if(page)break;await new Promise(resolve=>setTimeout(resolve,100));}
  if(!page)throw Error('Native application page missing: '+browser.contexts().flatMap(context=>context.pages().map(page=>page.url())).join(','));
  await page.waitForFunction(()=>!!globalThis.__TAURI_INTERNALS__);
  const call=(command,operation,request)=>page.evaluate(async({command,operation,request})=>globalThis.__TAURI_INTERNALS__.invoke(command,{command:operation,request:{contractVersion:1,...request}}),{command,operation,request});
  const {workspace}=await call('workspace_command','create',{name:'Real local Ask native'});
  await call('workspace_command','settings',{id:workspace.id,reducedMotion:true,onboardingComplete:true});
  await call('memory_command','create',{workspaceId:workspace.id,kind:'decision',draft:{title:'Local search architecture',body:'# Architecture\nOur local search database is SQLite with FTS5.',privacy:'private'}});
  await page.reload();
  if(await page.getByRole('button',{name:'Continue to workspace'}).isVisible())await page.getByRole('button',{name:'Continue to workspace'}).click();
  await page.getByRole('button',{name:'Refresh workspaces',exact:true}).click();
  await page.getByLabel('Existing workspace').selectOption(workspace.id);
  await page.getByRole('button',{name:'Open workspace',exact:true}).click();
  await page.getByRole('button',{name:'Ask ANANSI',exact:true}).click();
  await page.getByLabel('Installed Ollama model').fill('qwen2.5:0.5b');
  await page.getByLabel('Your Question').fill('What is our local search database?');
  await page.getByRole('button',{name:'Ask Question',exact:true}).click();
  await expect(page.getByRole('region',{name:'Streaming answer'})).toBeVisible({timeout:180000});
  await expect(page.locator('.status-banner.status-answered')).toBeVisible({timeout:210000});
  await expect(page.locator('.answer-text')).toContainText(/SQLite/i);
  await page.getByRole('button',{name:'View What ANANSI Sent (Disclosure)'}).click();
  await expect(page.locator('.manifest-metrics')).toContainText('qwen2.5:0.5b');
  await page.getByRole('button',{name:'Ask Question',exact:true}).click();
  await page.getByRole('button',{name:'Cancel generation'}).click();
  await expect(page.getByRole('button',{name:'Ask Question',exact:true})).toBeEnabled({timeout:30000});
  await expect(page.locator('.status-banner.status-answered')).toHaveCount(0);
  fs.mkdirSync('test-results',{recursive:true});await page.screenshot({path:'test-results/native-ask-streaming.png',fullPage:true});
  console.log('PASS: actual native UI real-model streaming answer, source disclosure and cancellation. Synthetic catalog: '+root);
}finally{if(browser)await browser.close();if(child.exitCode===null){const done=new Promise(r=>child.once('exit',r));child.kill();await done;}}
