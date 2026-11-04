import {test,expect} from '@playwright/test';
import {startSession} from './collider-three-helpers';
import fs from 'node:fs';
import {createServer} from 'node:http';
import {matrixLayout} from '../../packages/ui/src/larp-mode/views/instrumentScene';
test('all cinematic views retain real data, hover, live snapshots and resize behavior',async({page})=>{
  test.setTimeout(300000);page.setDefaultTimeout(8000);
  await page.setViewportSize({width:1600,height:1000});
  const errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));
  page.on('console',m=>{if(m.type()==='error'&&/shader|GL_INVALID/i.test(m.text()))errors.push(m.text());});
  let askHits=0;
  const fakeModel=createServer((req,res)=>{
    req.resume();req.on('end',()=>{askHits++;
      const body=JSON.stringify({response:'SQLite with FTS5 powers private local-first search and revision history [1].',done:true})+'\n';
      res.writeHead(200,{'content-type':'application/x-ndjson','content-length':Buffer.byteLength(body),'connection':'close'});res.end(body);
    });
  });
  await new Promise<void>(r=>fakeModel.listen(0,'127.0.0.1',r));
  const endpoint=`http://127.0.0.1:${(fakeModel.address() as {port:number}).port}`;
  let session:Awaited<ReturnType<typeof startSession>>|undefined;
  const evidence=process.env.ANANSI_LARP_EVIDENCE;
  const shot=async(n:number)=>{if(evidence){fs.mkdirSync(evidence,{recursive:true});await page.screenshot({path:`${evidence}/populated-${n}.png`});}};
  try{
    session=await startSession(page,false);
    const ids:string[]=[];
    for(let n=0;n<8;n++){
      const result=await session.call('memories','create',{workspaceId:session.workspaceId,kind:'memory',draft:{title:`Aurora knowledge ${n}`,body:`Aurora architecture decision ${n}: SQLite FTS5 powers private local-first search, with revision history and explicit context.`,confidence:(n+1)/10}});
      ids.push(result.body.memory.id);
    }
    for(let n=0;n<7;n++)await session.call('relationships','create',{workspaceId:session.workspaceId,draft:{sourceId:ids[n],targetId:ids[n+1],predicate:'relates-to'}});
    await session.pairAndOpen(session.workspaceId);
    await page.getByRole('button',{name:'Memory',exact:true}).click();
    await expect(page.getByRole('button',{name:'Aurora knowledge 0',exact:true})).toBeVisible();
    await page.getByLabel('Toggle LARP mode').click();
    const hud=page.getByLabel('LARP telemetry frame');
    await expect.poll(async()=>Number(await hud.locator('.larp-view-container').getAttribute('data-frames'))).toBeGreaterThan(1);await shot(1);
    await page.getByLabel('LARP view 3',{exact:true}).click();
    const graph=page.getByLabel('ATTENTION SPIDER // GRAPH DRIFT ENGINE');
    await expect(graph).toHaveAttribute('data-nodes','8');await shot(3);
    // Current record refresh while the graph stays mounted must update the scene.
    await session.call('memories','create',{workspaceId:session.workspaceId,kind:'memory',draft:{title:'Aurora live update',body:'SQLite live snapshot update.'}});
    await page.getByRole('button',{name:'Refresh memories',exact:true}).click();
    await expect(graph).toHaveAttribute('data-nodes','9');
    await page.getByLabel('Exit LARP mode').click();
    await page.getByRole('button',{name:'Ask ANANSI',exact:true}).click();
    await page.getByLabel('Local inference endpoint').fill(endpoint);
    await page.getByLabel('Embedding model for semantic grounding').fill('');
    await page.getByLabel('Installed Ollama model').fill('synthetic-observatory-fixture');
    for(const question of ['Which database powers Aurora search?','Describe Aurora SQLite revision history.','What powers Aurora private local search and explicit context? '+ 'Review Aurora SQLite architecture and its private revision history. '.repeat(5)]){
      await page.getByLabel('Your Question').fill(question);
      await page.getByRole('button',{name:'Ask Question',exact:true}).click();
      await expect(page.locator('.status-banner.status-answered')).toBeVisible({timeout:60000});
      await expect(page.locator('.answer-text')).toContainText('SQLite');
      await expect(page.getByRole('button',{name:'Ask Question',exact:true})).toBeEnabled();
    }
    await page.getByLabel('Toggle LARP mode').click();
    const hits=session.fakeHits();expect(askHits).toBe(3);
    await page.getByLabel('LARP view 2',{exact:true}).click();await shot(2);
    await page.getByLabel('LARP view 4',{exact:true}).click();
    const spectrum=page.getByLabel('QUANT DRIFT & COLLIDER SPECTRUM');
    await expect.poll(async()=>Number(await spectrum.getAttribute('data-markers'))).toBeGreaterThanOrEqual(3);
    await expect(spectrum.getByText('Compression 3',{exact:true})).toBeVisible();
    await expect(spectrum.locator('canvas')).toHaveAttribute('data-renderer','webgl');await shot(4);
    await page.getByLabel('LARP view 5',{exact:true}).click();
    const matrix=page.getByLabel('ATTENTION TENSOR HEATMAP // TOKEN ATTENTION MATRIX');
    await expect(matrix.locator('canvas')).toHaveAttribute('data-renderer','webgl');
    for(const [width,height] of [[1600,1000],[1100,800]]){
      const before=Number(await matrix.locator('canvas').getAttribute('data-paints'));
      await page.setViewportSize({width,height});
      if(width!==1600)await expect.poll(async()=>Number(await matrix.locator('canvas').getAttribute('data-paints'))).toBeGreaterThan(before);
      const box=await matrix.locator('canvas').boundingBox();if(!box)throw Error('missing matrix');
      const rows=Number(await matrix.getAttribute('data-raw-tokens')),cols=Number(await matrix.getAttribute('data-compiled-tokens'));
      const layout=matrixLayout(box.width,box.height,rows,cols);
      await page.mouse.move(box.x+layout.left+layout.cellW*.5,box.y+layout.top+layout.cellH*.5);
      await expect(matrix.locator('.larp-spectrum-readout')).toContainText('raw "what"');
      await page.mouse.move(box.x+1,box.y+1);await expect(matrix.locator('.larp-spectrum-readout')).toHaveText('No cell under cursor.');
    }
    await page.setViewportSize({width:1600,height:1000});await shot(5);
    await page.getByLabel('LARP view 6',{exact:true}).click();await shot(6);
    await page.emulateMedia({reducedMotion:'reduce'});
    for(const n of [3,4,5]){await page.getByLabel(`LARP view ${n}`,{exact:true}).click();await expect(hud).toHaveAttribute('data-view',String(n));}
    expect(session.fakeHits()).toBe(hits);expect(askHits).toBe(3);
    await page.getByLabel('Exit LARP mode').click();await expect(hud).toHaveCount(0);expect(errors).toEqual([]);
  }finally{try{await session?.cleanup();}finally{await new Promise<void>(r=>fakeModel.close(()=>r()));}}
});

test('cinematic instruments retain functional 2D fallbacks when WebGL is unavailable',async({page})=>{
  test.setTimeout(180000);page.setDefaultTimeout(8000);
  await page.addInitScript(()=>{
    const original=HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext=function(this:HTMLCanvasElement,kind:any,...args:any[]){
      if(['webgl','webgl2','experimental-webgl'].includes(kind))return null;
      return (original as any).call(this,kind,...args);
    } as any;
  });
  const session=await startSession(page,true);
  try{
    await session.call('memories','create',{workspaceId:session.workspaceId,kind:'memory',draft:{title:'SQLite fallback evidence',body:'SQLite powers local search.'}});
    await session.pairAndOpen(session.workspaceId);
    await page.getByRole('button',{name:'Ask ANANSI',exact:true}).click();
    await page.getByLabel('Local inference endpoint').fill('http://127.0.0.1:1');
    await page.getByLabel('Embedding model for semantic grounding').fill('');
    await page.getByLabel('Your Question').fill('What powers SQLite local search?');
    await page.getByRole('button',{name:'Ask Question',exact:true}).click();
    await expect(page.locator('.status-banner.status-offline')).toBeVisible({timeout:60000});
    await page.getByLabel('Toggle LARP mode').click();
    await page.getByLabel('LARP view 4',{exact:true}).click();
    const chart=page.getByLabel('QUANT DRIFT & COLLIDER SPECTRUM');
    await expect(chart.locator('canvas')).toHaveAttribute('data-renderer','2d');
    await expect(chart).toHaveAttribute('data-markers','1');
    await page.getByLabel('LARP view 5',{exact:true}).click();
    const matrix=page.getByLabel('ATTENTION TENSOR HEATMAP // TOKEN ATTENTION MATRIX');
    await expect(matrix.locator('canvas')).toHaveAttribute('data-renderer','2d');
    const box=await matrix.locator('canvas').boundingBox();if(!box)throw Error('missing fallback matrix');
    const layout=matrixLayout(box.width,box.height,Number(await matrix.getAttribute('data-raw-tokens')),Number(await matrix.getAttribute('data-compiled-tokens')));
    await page.mouse.move(box.x+layout.left+layout.cellW*.5,box.y+layout.top+layout.cellH*.5);
    await expect(matrix.locator('.larp-spectrum-readout')).toContainText('raw "what"');
    expect(session.fakeHits()).toBe(0);
    if(process.env.ANANSI_LARP_EVIDENCE)await page.screenshot({path:process.env.ANANSI_LARP_EVIDENCE+'/fallback-matrix.png'});
    await page.getByLabel('Exit LARP mode').click();await expect(page.getByLabel('LARP telemetry frame')).toHaveCount(0);
  }finally{await session.cleanup();}
});

