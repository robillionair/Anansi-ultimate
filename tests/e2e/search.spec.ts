import { test, expect } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

test('local title search filters, exact revision reader, literal content, update and restart',async({page})=>{
  test.setTimeout(180000);
  const root=fs.mkdtempSync(path.join(os.tmpdir(),'anansi-title-search-'));
  let child:ChildProcess|undefined,pairing:{endpoint:string,token:string},pairingFile='';
  async function start(){
    child=spawn(path.resolve('target/debug/anansi-api'+(process.platform==='win32'?'.exe':'')),['--port','0','--pairing-root',path.join(root,'pairing')],{windowsHide:true,env:{...process.env,ANANSI_STORAGE_ROOT:path.join(root,'storage')},stdio:['ignore','pipe','ignore']});
    await new Promise<void>((resolve,reject)=>{let output='';const timer=setTimeout(()=>reject(Error('Search API startup timeout')),20000);child!.once('error',reject);child!.stdout!.on('data',data=>{output+=data.toString();const match=output.match(/Pairing file: (.+)\r?\n/);if(match){pairingFile=match[1].trim();pairing=JSON.parse(fs.readFileSync(pairingFile,'utf8'));clearTimeout(timer);resolve();}});});
  }
  async function stop(){if(child?.exitCode===null){const ended=new Promise(resolve=>child!.once('exit',resolve));child.kill();await ended;}if(pairingFile&&fs.existsSync(pairingFile)){fs.unlinkSync(pairingFile);fs.rmdirSync(path.dirname(pairingFile));}}
  async function pair(){await page.goto('/');await page.getByLabel('Local API address').fill(pairing.endpoint);await page.getByLabel('Session token').fill(pairing.token);await page.getByRole('button',{name:'Connect to core'}).click();await expect(page.getByRole('button',{name:'Refresh workspaces'})).toBeEnabled();}
  async function call(group:string,operation:string,input:Record<string,unknown>){const response=await fetch(pairing.endpoint+'/v1/'+group+'/'+operation,{method:'POST',headers:{Origin:'http://127.0.0.1:1420',Authorization:'Bearer '+pairing.token,'Content-Type':'application/json'},body:JSON.stringify({contractVersion:1,...input})});expect(response.ok).toBe(true);return response.json();}
  async function search(query:string){await page.getByLabel('Title keywords').fill(query);await page.getByRole('button',{name:'Search titles',exact:true}).click();await expect(page.locator('section[aria-labelledby="search-title"]')).toHaveAttribute('aria-busy','false');}
  let external=0;page.on('request',request=>{if(!['127.0.0.1','tauri.localhost'].includes(new URL(request.url()).hostname))external++;});
  try{
    await start();
    const workspace=(await call('workspaces','create',{name:'Title search browser fixture'})).workspace;
    await call('workspaces','settings',{id:workspace.id,reducedMotion:true,onboardingComplete:true});
    const other=(await call('workspaces','create',{name:'Isolated search fixture'})).workspace;
    const draft={title:'Café 世界 research',body:'<script>globalThis.searchExecuted=true</script>\n![remote](https://example.invalid/private)',sourceReferences:['Synthetic source reference']};
    const original=await call('memories','create',{workspaceId:workspace.id,draft});
    await call('memories','create',{workspaceId:workspace.id,kind:'note',draft:{title:'Café notes',body:'note'}});
    await call('memories','create',{workspaceId:other.id,draft:{title:'Café otherworkspace',body:'other'}});
    await pair();await page.getByLabel('Existing workspace').selectOption(workspace.id);await page.getByRole('button',{name:'Open workspace',exact:true}).click();await page.getByRole('button',{name:'Search',exact:true}).click();
    await search('cafe');const results=page.getByRole('list',{name:'Title search results'});await expect(results.getByRole('button')).toHaveCount(2);await expect(results).not.toContainText('otherworkspace');
    await results.getByRole('button',{name:draft.title,exact:true}).click();const reader=page.getByRole('region',{name:'Search result revision'});await expect(reader).toContainText(draft.body);await expect(reader).toContainText(original.memory.revisionId);await expect(reader).toContainText('Synthetic source reference');
    expect(await page.evaluate(()=>Reflect.get(globalThis,'searchExecuted'))).toBeUndefined();
    await page.getByLabel('Record type',{exact:true}).selectOption('note');await search('cafe');await expect(results.getByRole('button')).toHaveCount(1);await expect(results).toContainText('Café notes');
    await page.getByLabel('Record type',{exact:true}).selectOption('');await search('cafe OR absent');await expect(page.getByRole('status').filter({hasText:'No matching titles'})).toBeVisible();
    await search('***');await expect(page.getByRole('alert')).toContainText('one to eight title words');await expect(reader).toHaveCount(0);
    await call('memories','revise',{workspaceId:workspace.id,id:original.memory.id,expectedRevision:original.memory.revisionId,draft:{...draft,title:'Updated research'}});
    await stop();await start();await pair();await page.getByLabel('Existing workspace').selectOption(workspace.id);await page.getByRole('button',{name:'Open workspace',exact:true}).click();await page.getByRole('button',{name:'Search',exact:true}).click();
    await search('世界');await expect(page.getByRole('status').filter({hasText:'No matching titles'})).toBeVisible();await search('Updated');await results.getByRole('button',{name:'Updated research',exact:true}).click();await expect(reader).toContainText('Revision 2');
    await page.setViewportSize({width:390,height:844});expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);expect(external).toBe(0);expect(await page.evaluate(()=>localStorage.length+sessionStorage.length)).toBe(0);
    await page.screenshot({path:'test-results/browser-title-search-reader.png',fullPage:true});
  }finally{await stop();}
});
