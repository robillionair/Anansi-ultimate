// Actual cross-adapter workflow using synthetic external storage only.
import { chromium, expect } from '@playwright/test';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { assertPortsFree } from './verify-ports.mjs';
if(process.platform!=='win32')throw Error('Cross-adapter verification requires Windows.');
await assertPortsFree([1420,19323]);
const root=fs.mkdtempSync(path.join(process.env.ANANSI_VERIFIER_ROOT||os.tmpdir(),'anansi-cross-adapter-'));
const env={...process.env,ANANSI_STORAGE_ROOT:path.join(root,'storage')};
if(process.argv.includes('--configured-cdp'))delete env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS;
else env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=19323';
const binary=process.argv.slice(2).find(argument=>!argument.startsWith('--'))||'target/debug/anansi-desktop.exe';
let api,desktop,preview,nativeBrowser,webBrowser,nativePage,webPage,pairing,pairingFile;
async function kill(child){if(child&&child.exitCode===null){const stopped=new Promise(resolve=>child.once('exit',resolve));child.kill();await stopped;}}
async function waitFor(url,child){for(let n=0;n<150;n++){try{if((await fetch(url)).ok)return;}catch{}if(child.exitCode!==null)throw Error('Test service exited');await new Promise(r=>setTimeout(r,200));}throw Error('Test service startup timed out');}
async function startApi(){
  api=spawn(path.resolve('target/debug/anansi-api.exe'),['--port','0','--pairing-root',path.join(root,'pairing')],{env,windowsHide:true,stdio:['ignore','pipe','ignore']});
  await new Promise((resolve,reject)=>{let output='';const timer=setTimeout(()=>reject(Error('API pairing timed out')),20000);api.once('error',reject);api.stdout.on('data',data=>{output+=data.toString();const match=output.match(/Pairing file: (.+)\r?\n/);if(match){pairingFile=match[1].trim();pairing=JSON.parse(fs.readFileSync(pairingFile,'utf8'));clearTimeout(timer);resolve();}});});
}
async function stopApi(){await kill(api);if(pairingFile&&fs.existsSync(pairingFile)){fs.unlinkSync(pairingFile);fs.rmdirSync(path.dirname(pairingFile));}}
async function startDesktop(){
  desktop=spawn(path.resolve(binary),[],{env,windowsHide:true,stdio:'ignore'});
  await waitFor('http://127.0.0.1:19323/json/version',desktop);
  nativeBrowser=await chromium.connectOverCDP('http://127.0.0.1:19323');
  for(let n=0;n<100;n++){nativePage=nativeBrowser.contexts()[0].pages().find(page=>page.url().includes('tauri.localhost'));if(nativePage)break;await new Promise(r=>setTimeout(r,100));}
  if(!nativePage)throw Error('Native page missing');
  await expect(nativePage.locator('.badge')).toHaveText('Connected',{timeout:20000});
  await expect(nativePage.getByRole('button',{name:'Refresh workspaces'})).toBeEnabled({timeout:15000});
}
async function stopDesktop(){if(nativeBrowser){await nativeBrowser.close();nativeBrowser=null;}await kill(desktop);}
async function pairBrowser(){
  await webPage.goto('http://127.0.0.1:1420');
  await webPage.getByLabel('Local API address').fill(pairing.endpoint);await webPage.getByLabel('Session token').fill(pairing.token);
  await webPage.getByRole('button',{name:'Connect to core'}).click();await expect(webPage.locator('.badge')).toHaveText('Connected');
  await expect(webPage.getByRole('button',{name:'Refresh workspaces'})).toBeEnabled({timeout:15000});
}
async function create(page,name){
  if(await page.getByRole('button',{name:'Continue to workspace'}).isVisible()){
    await page.getByRole('radio',{name:/Build an application/}).check();await page.getByRole('button',{name:'Continue to workspace'}).click();
  }
  await page.getByLabel('New workspace name').fill(name);await page.getByRole('button',{name:'Create workspace'}).click();
  await expect(page.getByRole('heading',{name:'Review your privacy'})).toBeFocused({timeout:15000});
  await expect(page.getByRole('button',{name:'Continue to AI setup'})).toBeDisabled();
  await page.getByLabel('I have reviewed these privacy limits').focus();await page.keyboard.press('Space');await expect(page.getByRole('button',{name:'Continue to AI setup'})).toBeEnabled({timeout:15000});await page.keyboard.press('Tab');await page.keyboard.press('Enter');
  await expect(page.getByRole('heading',{name:'Choose AI setup'})).toBeFocused();
  await page.getByRole('button',{name:'Skip AI setup and enter ANANSI'}).click();
  await expect(page.getByRole('heading',{name,exact:true})).toBeVisible({timeout:15000});
  return page.locator('.workspace-id code').innerText();
}
async function open(page,id){await page.getByRole('button',{name:'Refresh workspaces'}).click();await expect(page.getByRole('button',{name:'Refresh workspaces'})).toBeEnabled({timeout:15000});await page.getByLabel('Existing workspace').selectOption(id);await page.getByRole('button',{name:'Open workspace',exact:true}).click();await expect(page.locator('.workspace-id code')).toHaveText(id,{timeout:15000});}
async function close(page){await page.getByRole('button',{name:'Workspace',exact:true}).click();await page.getByRole('button',{name:'Close workspace'}).click();await expect(page.getByRole('button',{name:'Refresh workspaces'})).toBeEnabled({timeout:15000});}
async function motion(page,on){await page.getByRole('button',{name:'Settings',exact:true}).click();await page.getByRole('button',{name:on?'Turn reduced motion on':'Turn reduced motion off'}).click();await expect(page.getByRole('button',{name:on?'Turn reduced motion off':'Turn reduced motion on'})).toBeEnabled({timeout:15000});}
try{
  preview=spawn(process.execPath,[path.resolve('apps/web/node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','1420','--strictPort'],{cwd:path.resolve('apps/web'),windowsHide:true,stdio:'ignore'});
  await waitFor('http://127.0.0.1:1420',preview);await startApi();await startDesktop();
  webBrowser=await chromium.launch();webPage=await webBrowser.newPage({reducedMotion:'no-preference'});
  const nativeId=await create(nativePage,'Synthetic native workspace');await motion(nativePage,true);
  await nativePage.getByRole('radio',{name:/Invent a product/}).check();await nativePage.getByRole('button',{name:'Save starting point'}).click();await expect(nativePage.getByRole('status').filter({hasText:'Workspace preferences saved.'})).toBeVisible({timeout:15000});
  await pairBrowser();await open(webPage,nativeId);await webPage.getByRole('button',{name:'Settings',exact:true}).click();await expect(webPage.getByRole('radio',{name:/Invent a product/})).toBeChecked();await expect(webPage.getByRole('button',{name:'Turn reduced motion off'})).toBeVisible();await motion(webPage,false);
  await close(nativePage);await open(nativePage,nativeId);await nativePage.getByRole('button',{name:'Settings',exact:true}).click();await expect(nativePage.getByRole('button',{name:'Turn reduced motion on'})).toBeVisible();
  await close(webPage);const browserId=await create(webPage,'Synthetic browser workspace');await motion(webPage,true);
  await close(nativePage);await open(nativePage,browserId);await nativePage.getByRole('button',{name:'Settings',exact:true}).click();await expect(nativePage.getByRole('button',{name:'Turn reduced motion off'})).toBeVisible();
  await stopDesktop();await stopApi();await startApi();await startDesktop();await pairBrowser();
  await open(nativePage,browserId);await nativePage.getByRole('button',{name:'Settings',exact:true}).click();await expect(nativePage.getByRole('button',{name:'Turn reduced motion off'})).toBeVisible();
  await open(webPage,nativeId);await webPage.getByRole('button',{name:'Settings',exact:true}).click();await expect(webPage.getByRole('button',{name:'Turn reduced motion on'})).toBeVisible();
  await expect.poll(()=>nativePage.evaluate(()=>document.documentElement.dataset.reducedMotion)).toBe('true');
  await expect.poll(()=>nativePage.locator('.badge').evaluate(element=>getComputedStyle(element).transitionDuration)).toBe('0s');
  const stored=await webPage.evaluate(()=>localStorage.length+sessionStorage.length);if(stored!==0)throw Error('Unexpected browser persistence');
  await expect(webPage.getByRole('radio',{name:/Invent a product/})).toBeChecked();await nativePage.setViewportSize({width:390,height:844});if(!await nativePage.evaluate(()=>document.documentElement.scrollWidth<=innerWidth))throw Error('Native narrow overflow');await nativePage.screenshot({path:'test-results/native-onboarding-narrow.png',fullPage:true});await nativePage.setViewportSize({width:1280,height:900});fs.mkdirSync('test-results',{recursive:true});await nativePage.screenshot({path:'test-results/native-workspace.png',fullPage:true});await webPage.screenshot({path:'test-results/cross-browser-workspace.png',fullPage:true});
  console.log('PASS: actual desktop/browser onboarding, privacy review, skip AI, intent/motion settings parity, process restarts, narrow layout and live reduced-motion CSS.');
}finally{
  if(webBrowser)await webBrowser.close();await stopDesktop();await stopApi();await kill(preview);
  // Retain only private synthetic catalog data outside source for recovery inspection.
}
