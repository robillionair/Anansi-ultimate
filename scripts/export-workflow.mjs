import {expect as baseExpect} from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
const expect=baseExpect.configure({timeout:30000});
const button=(page,name)=>page.getByRole('button',{name,exact:true});
export const originalExportBody='# Portable knowledge\nCafé 世界\n<script>globalThis.exportExecuted=true</script>\n![untrusted](https://export.invalid/private)\n';

export async function prepareExport(page,label){
  await button(page,'Projects').click();await button(page,'New project').click();
  await page.getByLabel('Project title',{exact:true}).fill(label+' export project');
  await page.getByLabel('Description & objectives',{exact:true}).fill(originalExportBody);
  await button(page,'Save project').click();await expect(page.getByRole('status').filter({hasText:'Project saved.'})).toBeVisible();
  await expect(button(page,'Save project')).toBeEnabled();
  await page.getByLabel('Description & objectives',{exact:true}).fill('Portable revision two');
  await button(page,'Save project').click();await expect(button(page,'Save project')).toBeEnabled();
  await button(page,'Jobs and export').click();await expect(button(page,'Create knowledge export')).toBeEnabled();
  await button(page,'Refresh jobs').click();
  await page.getByLabel('Export selection',{exact:true}).selectOption('project');
  await expect(page.getByLabel('Project to export').getByRole('option',{name:label+' export project',exact:true})).toBeAttached();
  await page.getByLabel('Project to export').selectOption({label:label+' export project'});
  const previous=await page.locator('.jobs').getAttribute('data-created-job-id');
  await button(page,'Create knowledge export').click();
  await expect.poll(()=>page.locator('.jobs').getAttribute('data-created-job-id')).not.toBe(previous);
  const created=await page.locator('.jobs').getAttribute('data-created-job-id');
  expect(created).toMatch(/^[a-f0-9]{32}$/);
  const row=page.locator(`[data-job-id="${created}"]`);
  await expect(row).toContainText('Completed',{timeout:180000});
  return row.getAttribute('data-job-id');
}

export async function downloadExport(page,jobId,destination){
  const row=page.locator(`[data-job-id="${jobId}"]`);
  const waiting=page.waitForEvent('download');
  await row.getByRole('button',{name:'Download complete export',exact:true}).click();
  const download=await waiting;
  await download.saveAs(destination);
  expect(await download.failure()).toBeNull();
}

export async function createSelectionExport(page,scope,label){
  await page.getByLabel('Export selection',{exact:true}).selectOption(scope);
  if(scope==='records')await page.getByRole('checkbox',{name:label+' export project (project)',exact:true}).check();
  const previous=await page.locator('.jobs').getAttribute('data-created-job-id');
  await button(page,'Create knowledge export').click();
  await expect.poll(()=>page.locator('.jobs').getAttribute('data-created-job-id')).not.toBe(previous);
  const created=await page.locator('.jobs').getAttribute('data-created-job-id');
  expect(created).toMatch(/^[a-f0-9]{32}$/);
  const row=page.locator(`[data-job-id="${created}"]`);await expect(row).toContainText('Completed',{timeout:180000});
  return row.getAttribute('data-job-id');
}

export function inspectExport(file,label,selection='project'){
  const directory=file+'.inspection';fs.mkdirSync(directory);
  // An independent system TAR implementation reads the exported open format.
  execFileSync('tar',['-xf',path.resolve(file),'-C',path.resolve(directory)],{windowsHide:true});
  const manifest=JSON.parse(fs.readFileSync(path.join(directory,'manifest.json'),'utf8'));
  expect(manifest.format).toBe('anansi-knowledge-export');expect(manifest.formatVersion).toBe(1);
  expect(manifest.selection.kind).toBe(selection);
  expect(manifest.records).toHaveLength(1);
  const record=manifest.records[0];expect(record.revisions).toHaveLength(2);
  expect(record.revisions[0].metadata.title).toBe(label+' export project');
  expect(record.currentRevision).toBe(record.revisions[1].metadata.revisionId);
  const bodies=record.revisions.map(revision=>{
    const bytes=fs.readFileSync(path.join(directory,revision.bodyPath));
    expect(createHash('sha256').update(bytes).digest('hex')).toBe(revision.sha256);
    expect(revision.metadata.privacy).toBe('private');expect(revision.metadata.provenance).toBe('user_authored');
    return bytes.toString('utf8');
  });
  expect(bodies).toEqual([originalExportBody,'Portable revision two']);
  return {formatVersion:manifest.formatVersion,records:manifest.records.length,revisions:record.revisions.length,sha256:createHash('sha256').update(fs.readFileSync(file)).digest('hex')};
}
