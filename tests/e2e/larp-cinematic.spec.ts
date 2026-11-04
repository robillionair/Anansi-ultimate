import { test, expect } from '@playwright/test';
import { startSession } from './collider-three-helpers';
import fs from 'node:fs';

const evidence = process.env.ANANSI_LARP_EVIDENCE;
test('cinematic observatory renders six views, stays local, and exits cleanly', async ({ page }) => {
  test.setTimeout(300000);
  await page.setViewportSize({width:1600,height:1000});
  const errors:string[]=[];
  page.on('pageerror', e=>errors.push(e.message));
  page.on('console', message=>{if(message.type()==='error' && /shader|webgl|GL_INVALID/i.test(message.text())) errors.push(message.text());});
  const session=await startSession(page,false);
  try {
    await session.pairAndOpen(session.workspaceId);
    await page.getByLabel('Toggle LARP mode').click();
    const hud=page.getByLabel('LARP telemetry frame');
    await expect(hud).toBeVisible();
    await expect(hud.getByText('NOT REPORTED',{exact:true}).first()).toBeVisible();
    const hits=session.fakeHits();
    for (const view of [1,2,3,4,5,6]) {
      await page.getByLabel(`LARP view ${view}`,{exact:true}).click();
      await expect(hud).toHaveAttribute('data-view',String(view));
      if ([1,6].includes(view)) await expect.poll(async()=>Number(await hud.locator('.larp-view-container').getAttribute('data-frames'))).toBeGreaterThan(1);
      if(evidence) {
        fs.mkdirSync(evidence,{recursive:true});
        await page.screenshot({path:`${evidence}/scene-${view}.png`});
      }
    }
    await page.emulateMedia({reducedMotion:'reduce'});
    const field=hud.locator('.larp-view-container');
    const initial=await field.getAttribute('data-frames');
    await page.waitForTimeout(250);
    expect(await field.getAttribute('data-frames')).toBe(initial);
    if(evidence) await page.screenshot({path:`${evidence}/scene-6-reduced.png`});
    await page.setViewportSize({width:1280,height:720});
    await expect(hud).toBeVisible();
    const dock = await hud.locator('.larp-mode-dock').boundingBox();
    const footer = await hud.locator('.larp-rail-bottom').boundingBox();
    expect(dock!.y+dock!.height).toBeLessThanOrEqual(footer!.y+1);
    await page.setViewportSize({width:640,height:800});
    await expect(page.getByLabel('LARP view 6',{exact:true})).toBeInViewport();
    await expect(page.getByLabel('Exit LARP mode')).toBeInViewport();
    if(evidence) await page.screenshot({path:`${evidence}/compact-reduced.png`});
    expect(session.fakeHits()).toBe(hits);
    await page.getByLabel('Exit LARP mode').click();
    await expect(hud).toHaveCount(0);
    expect(errors).toEqual([]);
  } finally {await session.cleanup();}
});
