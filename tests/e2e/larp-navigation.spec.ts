import {test,expect} from '@playwright/test';
import {startSession} from './collider-three-helpers';
test('3D scenes support bounded zoom, orbit, pan, keyboard and reset without click-through',async({page})=>{
 test.setTimeout(300000);page.setDefaultTimeout(8000);await page.setViewportSize({width:1600,height:1000});
 const errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));
 const session=await startSession(page,false);
 try{
  await session.call('memories','create',{workspaceId:session.workspaceId,kind:'memory',draft:{title:'Interactive brain sample',body:'Local sample for camera navigation.'}});
  await session.pairAndOpen(session.workspaceId);await page.getByRole('button',{name:'Memory',exact:true}).click();await expect(page.getByRole('button',{name:'Interactive brain sample',exact:true})).toBeVisible();
  await page.getByLabel('Toggle LARP mode').click();const hits=session.fakeHits();
  for(const n of [1,3,6]){
   await page.getByLabel(`LARP view ${n}`,{exact:true}).click();
   const canvas=page.locator('canvas[data-interactive="true"]');await expect(canvas).toHaveAttribute('data-camera-distance','190.000');
   const box=await canvas.boundingBox();if(!box)throw Error('missing interactive canvas');
   const x=box.x+box.width*.5,y=box.y+box.height*.55;
   const scroll=await page.evaluate(()=>window.scrollY);await page.mouse.move(x,y);await page.mouse.wheel(0,350);
   await expect.poll(async()=>Number(await canvas.getAttribute('data-camera-distance'))).toBeGreaterThan(190);
   expect(await page.evaluate(()=>window.scrollY)).toBe(scroll);
   const before=await canvas.getAttribute('data-camera-position');
   await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x+120,y+60,{steps:8});await page.mouse.up();
   await expect(canvas).not.toHaveAttribute('data-camera-position',before!);
   await page.mouse.move(x,y);await page.mouse.down({button:'right'});await page.mouse.move(x+70,y+30,{steps:5});await page.mouse.up({button:'right'});
   await expect(canvas).not.toHaveAttribute('data-camera-target','0.000,0.000,0.000');
   await page.getByLabel('Reset scene camera').click();await expect(canvas).toHaveAttribute('data-camera-distance','190.000');await expect(canvas).toHaveAttribute('data-camera-target','0.000,0.000,0.000');
   await canvas.focus();await page.keyboard.press('ArrowRight');await expect(canvas).not.toHaveAttribute('data-camera-target','0.000,0.000,0.000');
   await page.keyboard.press('Home');await expect(canvas).toHaveAttribute('data-camera-target','0.000,0.000,0.000');
   await page.emulateMedia({reducedMotion:'reduce'});await page.mouse.move(x,y);await page.mouse.wheel(0,-250);
   await expect.poll(async()=>Number(await canvas.getAttribute('data-camera-distance'))).toBeLessThan(190);
   const distance=Number(await canvas.getAttribute('data-camera-distance'));expect(distance).toBeGreaterThanOrEqual(35);expect(distance).toBeLessThanOrEqual(520);
   await page.getByLabel('Reset scene camera').click();await page.emulateMedia({reducedMotion:'no-preference'});
  }
  expect(session.fakeHits()).toBe(hits);await page.getByLabel('Exit LARP mode').click();await expect(page.getByLabel('LARP telemetry frame')).toHaveCount(0);expect(errors).toEqual([]);
 }finally{await session.cleanup();}
});
