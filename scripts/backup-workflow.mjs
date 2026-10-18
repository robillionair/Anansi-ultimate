import {expect as baseExpect} from '@playwright/test';
import {prepareEntities,checkEntities} from './entities-workflow.mjs';
const expect=baseExpect.configure({timeout:30000});
const button=(p,name)=>p.getByRole('button',{name,exact:true});

export async function prepareBackup(page,label,createData=true){
  await button(page,'Settings').click();
  const startingPoint=page.getByRole('radio',{name:/Build an application/});
  if(!await startingPoint.isChecked()){await startingPoint.check();await button(page,'Save starting point').click();await expect(page.getByRole('status').filter({hasText:'Workspace preferences saved.'})).toBeVisible();await expect(button(page,'Save starting point')).toBeDisabled();}
  if(createData)await prepareEntities(page,label);
  await button(page,'Backups').click();
  await expect(button(page,'Create backup')).toBeDisabled();
  await expect(page.getByText(/Saved schedule: Off/)).toBeVisible();
  await page.getByLabel('Enable scheduled workspace backups',{exact:true}).check();
  await expect(button(page,'Save backup schedule')).toBeDisabled();
  await page.getByLabel('I consent to a plaintext copy',{exact:false}).check();
  await page.getByLabel('Use private managed storage on this computer.',{exact:true}).check();
  await page.getByLabel('Keep all versions without automatic pruning.',{exact:true}).check();
  await button(page,'Save backup schedule').click();
  await expect(page.getByText(/Saved schedule: Enabled/)).toBeVisible();
  await page.getByLabel('Enable scheduled workspace backups',{exact:true}).uncheck();
  await button(page,'Save backup schedule').click();
  await expect(page.getByText(/Saved schedule: Off/)).toBeVisible();
  await button(page,'Create backup').click();
  const region=page.getByRole('region',{name:'Back up and recover'});
  await expect.poll(()=>region.getAttribute('data-created-backup-id')).toMatch(/^[a-f0-9]{32}$/);
  const id=await region.getAttribute('data-created-backup-id');
  const row=page.locator(`[data-backup-job="${id}"]`);
  await expect.poll(async()=>{await button(page,'Refresh backups').click();return row.innerText();},{timeout:180000}).toContain('Completed');
  return id;
}
export async function restoreBackup(page,jobId,name){
  await button(page,'Backups').click();
  const row=page.locator(`[data-backup-job="${jobId}"]`);
  await row.getByRole('button',{name:'Preview restore',exact:true}).click();
  const preview=page.getByRole('region',{name:'Restore preview'});
  await expect(preview).toContainText('12 records');
  await expect(preview).toContainText('9 relationships');
  await expect(preview).toContainText('Sealed and credentials excluded');
  await expect(button(page,'Restore verified backup')).toBeDisabled();
  await page.getByLabel('Restored workspace name',{exact:true}).fill(name);
  await page.getByLabel('Restore into fresh workspace IDs',{exact:false}).check();
  await button(page,'Restore verified backup').click();
  await expect(page.getByRole('status').filter({hasText:'Restore complete:'})).toContainText(name,{timeout:120000});
  await button(page,'Workspace').click();await button(page,'Close workspace').click();
  await page.getByLabel('Existing workspace',{exact:true}).selectOption({label:name});
  await button(page,'Open workspace').click();
  const fresh=await page.locator('.workspace-id code').innerText();
  return fresh;
}
export async function checkBackup(page,label){
  await checkEntities(page,label);
  await button(page,'Settings').click();
  await expect(page.getByRole('radio',{name:/Build an application/})).toBeChecked();
  await expect(button(page,'Turn reduced motion on')).toBeVisible();
}
