import { test, expect, type Page, type Locator } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
let child:ChildProcess,root:string,file:string,pairing:{endpoint:string;token:string};
async function start(){
  child=spawn(path.resolve('target/debug/anansi-api'+(process.platform==='win32'?'.exe':'')),['--port','0','--pairing-root',root],{windowsHide:true,env:{...process.env,ANANSI_STORAGE_ROOT:path.join(root,'storage')},stdio:['ignore','pipe','ignore']});
  await new Promise<void>((resolve,reject)=>{let output='';const timer=setTimeout(()=>reject(Error('API startup timed out')),20000);child.once('error',reject);child.stdout!.on('data',data=>{output+=data.toString();const match=output.match(/Pairing file: (.+)\r?\n/);if(match){file=match[1].trim();pairing=JSON.parse(fs.readFileSync(file,'utf8'));clearTimeout(timer);resolve();}});});
}
async function stop(){if(child?.exitCode===null){const stopped=new Promise(resolve=>child.once('exit',resolve));child.kill();await stopped;}if(file&&fs.existsSync(file)){fs.unlinkSync(file);fs.rmdirSync(path.dirname(file));}}
test.beforeAll(async()=>{root=fs.mkdtempSync(path.join(os.tmpdir(),'anansi-onboarding-'));await start();});
test.afterAll(stop);
async function pair(page:Page){await page.goto('/');await page.getByLabel('Local API address').fill(pairing.endpoint);await page.getByLabel('Session token').fill(pairing.token);await page.getByRole('button',{name:'Connect to core'}).click();await expect(page.locator('.badge')).toHaveText('Connected');await expect(page.getByRole('button',{name:'Refresh workspaces'})).toBeEnabled({timeout:15000});}
async function tabTo(page:Page,target:Locator){for(let index=0;index<45;index++){if(await target.evaluate(element=>element===document.activeElement))return;await page.keyboard.press('Tab');}throw Error('Keyboard target unreachable');}

test('offline first-run, keyboard navigation, privacy, skip AI, changeable durable intent and motion',async({page})=>{
  test.setTimeout(120000);const external:string[]=[];
  await page.route('**/*',async route=>{if(new URL(route.request().url()).hostname==='127.0.0.1')await route.continue();else{external.push(new URL(route.request().url()).hostname);await route.abort();}});
  await page.emulateMedia({reducedMotion:'no-preference'});await pair(page);
  await expect(page.getByRole('group',{name:'What would you like to do?'})).toBeVisible();await expect(page.getByRole('radio')).toHaveCount(5);await page.screenshot({path:'test-results/onboarding-welcome.png',fullPage:true});
  const explore=page.getByRole('radio',{name:/Explore ANANSI/});await tabTo(page,explore);await page.keyboard.press('ArrowDown');await page.keyboard.press('ArrowDown');await expect(page.getByRole('radio',{name:/Invent a product/})).toBeChecked();
  await tabTo(page,page.getByRole('button',{name:'Continue to workspace'}));await page.keyboard.press('Enter');await tabTo(page,page.getByLabel('New workspace name'));await page.keyboard.type('Keyboard workspace');await page.keyboard.press('Enter');
  await expect(page.getByRole('heading',{name:'Review your privacy'})).toBeFocused({timeout:15000});await expect(page.getByText(/not an encrypted vault/)).toBeVisible();await expect(page.getByRole('button',{name:'Continue to AI setup'})).toBeDisabled();await page.screenshot({path:'test-results/onboarding-privacy.png',fullPage:true});
  await tabTo(page,page.getByLabel('I have reviewed these privacy limits'));await page.keyboard.press('Space');await expect(page.getByRole('button',{name:'Continue to AI setup'})).toBeEnabled({timeout:15000});await tabTo(page,page.getByRole('button',{name:'Continue to AI setup'}));await page.keyboard.press('Enter');await expect(page.getByRole('heading',{name:'Choose AI setup'})).toBeFocused();await expect(page.getByText('Local hardware detection and model recommendations are not available yet.')).toBeVisible();await page.screenshot({path:'test-results/onboarding-ai-skip.png',fullPage:true});
  await tabTo(page,page.getByRole('button',{name:'Skip AI setup and enter ANANSI'}));await page.keyboard.press('Enter');await expect(page.getByRole('heading',{name:'Keyboard workspace',exact:true})).toBeVisible({timeout:15000});
  const id=await page.locator('.workspace-id code').innerText();
  const settings=page.getByRole('button',{name:'Settings',exact:true});await tabTo(page,settings);await expect(settings).toHaveCSS('outline-width','3px');await page.keyboard.press('Enter');await expect(page.getByRole('heading',{name:'Make it yours'})).toBeFocused();await expect(settings).toHaveAttribute('aria-current','page');
  await tabTo(page,page.getByRole('radio',{name:/Invent a product/}));await page.keyboard.press('ArrowDown');await expect(page.getByRole('radio',{name:/Build an application/})).toBeChecked();await tabTo(page,page.getByRole('button',{name:'Save starting point'}));await page.keyboard.press('Enter');await expect(page.getByRole('status').filter({hasText:'Workspace preferences saved.'})).toBeVisible({timeout:15000});
  await tabTo(page,page.getByRole('button',{name:'Turn reduced motion on'}));await page.keyboard.press('Enter');await expect(page.getByRole('button',{name:'Turn reduced motion off'})).toBeEnabled({timeout:15000});await expect.poll(()=>page.locator('.badge').evaluate(element=>getComputedStyle(element).transitionDuration)).toBe('0s');
  await page.setViewportSize({width:390,height:844});expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);await page.screenshot({path:'test-results/onboarding-settings-narrow.png',fullPage:true});
  await stop();await start();await pair(page);await page.getByLabel('Existing workspace').selectOption(id);await page.getByRole('button',{name:'Open workspace',exact:true}).click();await expect(page.getByRole('heading',{name:'Keyboard workspace',exact:true})).toBeVisible({timeout:15000});await expect(page.getByText('Build an application',{exact:true})).toBeVisible();await expect(page.getByRole('heading',{name:'Review your privacy'})).toHaveCount(0);
  await settings.click();await expect(page.getByRole('radio',{name:/Build an application/})).toBeChecked();await expect(page.getByRole('button',{name:'Turn reduced motion off'})).toBeVisible();expect(await page.evaluate(()=>localStorage.length+sessionStorage.length)).toBe(0);expect(external).toEqual([]);
  await page.setViewportSize({width:1280,height:900});await page.getByText('Explore planned tools',{exact:true}).click();await expect(page.getByRole('region',{name:'KNOWLEDGE planned tools'})).toContainText('Search');await expect(page.getByRole('button',{name:'Memory',exact:true})).toBeEnabled();await page.screenshot({path:'test-results/onboarding-settings-wide.png',fullPage:true});
  await tabTo(page,page.getByRole('link',{name:'Skip to main content'}));await page.keyboard.press('Enter');await expect(page.getByRole('main')).toBeFocused();
});

test('invalid names recover and system reduced motion is respected',async({page})=>{
  await pair(page);await page.getByRole('button',{name:'Continue to workspace'}).click();await page.getByLabel('New workspace name').fill('../invalid');await page.getByRole('button',{name:'Create workspace'}).click();await expect(page.getByRole('alert')).toContainText('valid workspace',{timeout:15000});await expect(page.getByLabel('New workspace name')).toBeVisible();await page.emulateMedia({reducedMotion:'reduce'});await expect.poll(()=>page.locator('.badge').evaluate(element=>getComputedStyle(element).transitionDuration)).toBe('0s');
});

test('unfinished onboarding reopens safely after reload',async({page})=>{
  test.setTimeout(60000);await pair(page);await page.getByRole('radio',{name:/Connect AI models/}).check();await page.getByRole('button',{name:'Continue to workspace'}).click();await page.getByLabel('New workspace name').fill('Resume workspace');await page.getByRole('button',{name:'Create workspace'}).click();await expect(page.getByRole('heading',{name:'Review your privacy'})).toBeVisible({timeout:15000});
  await expect(page.getByText(/Your starting point: Connect AI models/)).toBeVisible({timeout:15000});await pair(page);await page.getByLabel('Existing workspace').selectOption({label:'Resume workspace'});await page.getByRole('button',{name:'Open workspace',exact:true}).click();await expect(page.getByRole('heading',{name:'Review your privacy'})).toBeVisible({timeout:15000});await expect(page.getByText(/Your starting point: Connect AI models/)).toBeVisible();await expect(page.getByLabel('I have reviewed these privacy limits')).not.toBeChecked();await expect(page.getByRole('button',{name:'Continue to AI setup'})).toBeDisabled();await page.screenshot({path:'test-results/onboarding-privacy.png',fullPage:true});
});
