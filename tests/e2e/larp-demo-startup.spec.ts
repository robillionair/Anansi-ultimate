import {test,expect} from '@playwright/test';
import {spawn} from 'node:child_process';
import fs from 'node:fs';import path from 'node:path';import os from 'node:os';
test('demo build automatically opens only the sample workspace and enters the populated brain',async({page})=>{
 test.skip(process.env.ANANSI_DEMO_TEST!=='1','Requires the explicitly built demo profile.');
 test.setTimeout(180000);page.setDefaultTimeout(8000);
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'anansi-larp-demo-'));
 const api=spawn(path.resolve('target/debug/anansi-api'+(process.platform==='win32'?'.exe':'')),['--port','0','--pairing-root',path.join(root,'pairing')],{windowsHide:true,env:{...process.env,ANANSI_STORAGE_ROOT:path.join(root,'storage')},stdio:['ignore','pipe','ignore']});
 try{
  const pairing=await new Promise<{endpoint:string;token:string}>((resolve,reject)=>{let out='';const timer=setTimeout(()=>reject(Error('demo API timeout')),30000);api.once('error',reject);api.stdout!.on('data',chunk=>{out+=chunk;const m=out.match(/Pairing file: (.+)\r?\n/);if(m){clearTimeout(timer);resolve(JSON.parse(fs.readFileSync(m[1].trim(),'utf8')));}});});
  const call=async(group:string,operation:string,input:Record<string,unknown>)=>{const r=await fetch(`${pairing.endpoint}/v1/${group}/${operation}`,{method:'POST',headers:{Origin:'http://127.0.0.1:1420',Authorization:`Bearer ${pairing.token}`,'Content-Type':'application/json'},body:JSON.stringify({contractVersion:1,...input})});expect(r.ok).toBe(true);return r.json();};
  const demo=(await call('workspaces','create',{name:'LARP Demo'})).workspace;
  await call('workspaces','settings',{id:demo.id,reducedMotion:false,onboardingComplete:true});
  await call('memories','create',{workspaceId:demo.id,kind:'memory',draft:{title:'Visible demo memory',body:'Clearly labelled synthetic local demo record.'}});
  const other=(await call('workspaces','create',{name:'Private work'})).workspace;
  await call('memories','create',{workspaceId:other.id,kind:'memory',draft:{title:'PRIVATE_NOT_AUTO_OPENED',body:'Must not be loaded by demo startup.'}});
  await page.goto('/');await page.getByLabel('Local API address').fill(pairing.endpoint);await page.getByLabel('Session token').fill(pairing.token);await page.getByRole('button',{name:'Connect to core'}).click();
  await expect(page.getByLabel('LARP telemetry frame')).toBeVisible({timeout:60000});await expect(page.getByText('DEMO / SAMPLE DATA',{exact:true})).toBeVisible();
  await expect(page.locator('canvas[data-interactive="true"]')).toHaveAttribute('data-camera-distance','190.000');
  await expect(page.getByText('Visible demo memory',{exact:true}).first()).toBeVisible();await expect(page.getByText('PRIVATE_NOT_AUTO_OPENED',{exact:true})).toHaveCount(0);
  await page.getByLabel('Exit LARP mode').click();await expect(page.getByRole('heading',{name:'Your memories',exact:true})).toBeVisible();
 }finally{if(api.exitCode===null)await new Promise<void>(resolve=>{api.once('exit',()=>resolve());api.kill();});}
});
