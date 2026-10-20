// Prepare synthetic application data only. A human operates all credential and
// recovery dialogs. Never inspect, fill, capture or log those dialogs.
import {chromium} from '@playwright/test';
import {spawn} from 'node:child_process';
import fs from 'node:fs';import os from 'node:os';import path from 'node:path';
import {assertPortsFree} from './verify-ports.mjs';
await assertPortsFree([19323]);
const fixture=fs.mkdtempSync(path.join(os.tmpdir(),'anansi-vault-manual-'));
const env={...process.env,ANANSI_STORAGE_ROOT:path.join(fixture,'storage')};
delete env.ANANSI_VAULT_TEST_PROVIDER;
const child=spawn(path.resolve('target/debug/anansi-desktop.exe'),[],{env,stdio:'ignore'});
let browser;
try{
  for(let n=0;n<150;n++){try{if((await fetch('http://127.0.0.1:19323/json/version')).ok)break;}catch{}if(child.exitCode!==null)throw Error('Desktop exited');await new Promise(r=>setTimeout(r,200));}
  browser=await chromium.connectOverCDP('http://127.0.0.1:19323');
  let page;
  for(let n=0;n<100;n++){page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url().includes('tauri.localhost'));if(page)break;await new Promise(r=>setTimeout(r,100));}
  if(!page)throw Error('Desktop page unavailable');
  await page.waitForFunction(()=>!!globalThis.__TAURI_INTERNALS__);
  const call=(command,operation,request)=>page.evaluate(({command,operation,request})=>globalThis.__TAURI_INTERNALS__.invoke(command,{command:operation,request:{contractVersion:1,...request}}),{command,operation,request});
  const {workspace}=await call('workspace_command','create',{name:'Manual vault acceptance'});
  await call('workspace_command','settings',{id:workspace.id,reducedMotion:true,onboardingComplete:true});
  await call('memory_command','create',{workspaceId:workspace.id,kind:'memory',draft:{title:'Synthetic vault test record',body:'Synthetic vault test body. This is not personal data.',privacy:'private'}});
  const {job}=await call('job_command','backup',{workspaceId:workspace.id,application:false,plaintextConsent:true});
  let completed;
  for(let n=0;n<240;n++){const result=await call('job_command','list',{workspaceId:workspace.id,offset:0});completed=result.jobs.find(j=>j.id===job.id&&j.state==='Completed');if(completed)break;await new Promise(r=>setTimeout(r,250));}
  if(!completed?.artifactId)throw Error('Backup did not complete');
  const preview=await call('job_command','preview',{workspaceId:workspace.id,jobId:job.id,artifactId:completed.artifactId});
  await page.reload();
  if(await page.getByRole('button',{name:'Continue to workspace'}).isVisible())await page.getByRole('button',{name:'Continue to workspace'}).click();
  await page.getByRole('button',{name:'Refresh workspaces',exact:true}).click();
  await page.getByLabel('Existing workspace').selectOption(workspace.id);
  await page.getByRole('button',{name:'Open workspace',exact:true}).click();
  await page.getByText('Seal using a verified plaintext backup',{exact:true}).click();
  await page.getByLabel('Backup job ID',{exact:true}).fill(job.id);
  await page.getByLabel('Artifact ID',{exact:true}).fill(completed.artifactId);
  await page.getByLabel('Backup SHA-256',{exact:true}).fill(preview.sha256);
  fs.writeFileSync(path.join(fixture,'fixture.json'),JSON.stringify({workspaceId:workspace.id,jobId:job.id,artifactId:completed.artifactId,sha256:preview.sha256},null,2));
  console.log('MANUAL READY. No credential/recovery dialog automation. Synthetic fixture: '+fixture);
  await browser.close();browser=null;
  await new Promise(resolve=>child.once('exit',resolve));
  console.log('Manual desktop closed. Results require human report; no automatic acceptance claim.');
}finally{if(browser)await browser.close();if(child.exitCode===null){const done=new Promise(r=>child.once('exit',r));child.kill();await done;}}
