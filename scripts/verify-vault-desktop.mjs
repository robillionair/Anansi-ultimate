// Debug-only acceptance of the real Tauri command path using a compile-time
// synthetic credential provider. Production builds cannot enable this path.
import { chromium } from '@playwright/test';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { assertPortsFree } from './verify-ports.mjs';

if(process.platform!=='win32')throw Error('Windows vault acceptance requires Windows.');
await assertPortsFree([1420,19323]);
const fixture=fs.mkdtempSync(path.join(process.env.ANANSI_VERIFIER_ROOT||os.tmpdir(),'anansi-vault-desktop-'));
const env={...process.env,ANANSI_STORAGE_ROOT:path.join(fixture,'storage'),ANANSI_VAULT_TEST_PROVIDER:'1',WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=19323'};
const binary=path.resolve(process.argv[2]||'target/debug/anansi-desktop.exe');
let child,preview,browser,page;
async function start(){if(!preview){let previewError='';preview=spawn(process.execPath,[path.resolve('apps/web/node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','1420','--strictPort'],{cwd:path.resolve('apps/web'),windowsHide:true,stdio:['ignore','ignore','pipe']});preview.stderr.on('data',data=>previewError+=data);let ready=false;for(let n=0;n<100;n++){try{if((await fetch('http://127.0.0.1:1420')).ok){ready=true;break;}}catch{}if(preview.exitCode!==null)throw Error('Web preview exited: '+previewError);await new Promise(r=>setTimeout(r,100));}if(!ready)throw Error('Web preview unavailable');}child=spawn(binary,[],{env,windowsHide:true,stdio:'ignore'});for(let n=0;n<150;n++){try{if((await fetch('http://127.0.0.1:19323/json/version')).ok)break;}catch{}if(child.exitCode!==null)throw Error('Desktop exited before startup');await new Promise(r=>setTimeout(r,200));}browser=await chromium.connectOverCDP('http://127.0.0.1:19323');for(let n=0;n<100;n++){for(const candidate of browser.contexts().flatMap(c=>c.pages())){if(await candidate.evaluate(()=>!!globalThis.__TAURI_INTERNALS__).catch(()=>false)){page=candidate;break;}}if(page)break;await new Promise(r=>setTimeout(r,100));}if(!page)throw Error('Native page missing: '+browser.contexts().flatMap(c=>c.pages().map(p=>p.url())).join(','));await page.goto('http://127.0.0.1:1420');await page.waitForFunction(()=>!!globalThis.__TAURI_INTERNALS__);}
async function stop(){page=null;if(browser){await browser.close();browser=null;}if(child?.exitCode===null){const done=new Promise(r=>child.once('exit',r));child.kill();await done;}}
async function invoke(command,args){const result=await page.evaluate(async({command,args})=>{try{return {ok:true,value:await globalThis.__TAURI_INTERNALS__.invoke(command,args)}}catch(error){return {ok:false,error}}},{command,args});if(!result.ok){const failure=Error(JSON.stringify(result.error));Object.assign(failure,result.error);throw failure;}return result.value;}
try{
  await start();
  console.log('vault acceptance: desktop started');
  const workspace=(await invoke('workspace_command',{command:'create',request:{contractVersion:1,name:'Vault acceptance'}})).workspace.id;
  console.log('vault acceptance: workspace created');
  await invoke('memory_command',{command:'create',request:{contractVersion:1,workspaceId:workspace,kind:'memory',draft:{title:'Sealed synthetic title',body:'Sealed synthetic body and history',privacy:'private',confidence:null,eventDate:null,validFrom:null,validUntil:null,sourceReferences:[]}}});
  const backup=await invoke('job_command',{command:'backup',request:{contractVersion:1,workspaceId:workspace,application:false,plaintextConsent:true}});
  console.log('vault acceptance: backup queued');
  const jobId=backup.job.id;
  let completed;
  for(let n=0;n<240;n++){const jobs=await invoke('job_command',{command:'list',request:{contractVersion:1,workspaceId:workspace,offset:0}});completed=jobs.jobs.find(j=>j.id===jobId&&j.state==='Completed');if(completed)break;await new Promise(r=>setTimeout(r,250));}
  if(!completed?.artifactId)throw Error('Verified backup did not complete');
  console.log('vault acceptance: backup completed');
  const preview=await invoke('job_command',{command:'preview',request:{contractVersion:1,workspaceId:workspace,jobId,artifactId:completed.artifactId}});
  await invoke('vault_command',{command:'setup',request:{contractVersion:1,workspaceId:workspace}});
  console.log('vault acceptance: setup acknowledged');
  await invoke('vault_command',{command:'seal',request:{contractVersion:1,workspaceId:workspace,jobId,artifactId:completed.artifactId,sha256:preview.sha256}});
  console.log('vault acceptance: workspace sealed');
  await invoke('vault_command',{command:'unlock',request:{contractVersion:1,workspaceId:workspace}});
  console.log('vault acceptance: unlocked');
  const opened=await invoke('vault_command',{command:'read',request:{contractVersion:1,workspaceId:workspace}});
  if(!JSON.stringify(opened).includes('Sealed synthetic title')||!JSON.stringify(opened).includes('Sealed synthetic body and history'))throw Error('Authorized read omitted encrypted content');
  await invoke('vault_command',{command:'lock',request:{contractVersion:1,workspaceId:workspace}});
  let denied=false;try{await invoke('vault_command',{command:'read',request:{contractVersion:1,workspaceId:workspace}});}catch(error){denied=error?.code==='UNAUTHORIZED';}if(!denied)throw Error('Locked read did not fail closed');
  await stop();await start();
  let restartDenied=false;try{await invoke('vault_command',{command:'read',request:{contractVersion:1,workspaceId:workspace}});}catch(error){restartDenied=error?.code==='UNAUTHORIZED';}if(!restartDenied)throw Error('Restart retained an authorized session');
  await invoke('vault_command',{command:'unlock',request:{contractVersion:1,workspaceId:workspace}});
  const reopened=await invoke('vault_command',{command:'read',request:{contractVersion:1,workspaceId:workspace}});
  if(!JSON.stringify(reopened).includes('Sealed synthetic title'))throw Error('Restart unlock/read failed');
  fs.mkdirSync('test-results',{recursive:true});await page.screenshot({path:'test-results/vault-desktop.png',fullPage:true});
  console.log('PASS: actual Tauri setup, recovery acknowledgement, verified-backup seal, authorized read, lock denial, restart denial and unlock/read.');
  console.log('Verification evidence: '+fixture);
}finally{await stop();if(preview?.exitCode===null){const done=new Promise(r=>preview.once('exit',r));preview.kill();await done;}}
