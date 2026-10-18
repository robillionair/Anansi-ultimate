import {expect as baseExpect} from '@playwright/test';
import fs from 'node:fs';import path from 'node:path';import {execFileSync} from 'node:child_process';import {createHash} from 'node:crypto';
const expect=baseExpect.configure({timeout:30000});
const button=(p,name)=>p.getByRole('button',{name,exact:true});
export async function roundTripSavedBackup(page,jobId,root){
  await button(page,'Jobs and export').click();
  const row=page.locator(`[data-job-id="${jobId}"]`);await expect(row).toContainText('Completed');
  const waiting=page.waitForEvent('download');await row.getByRole('button',{name:'Download complete export',exact:true}).click();
  const download=await waiting;const destination=path.join(root,'backup-roundtrip.tar');await download.saveAs(destination);expect(await download.failure()).toBeNull();
  const extracted=fs.mkdtempSync(path.join(root,'independent-backup-'));execFileSync('tar',['-xf',destination,'-C',extracted]);
  const manifest=JSON.parse(fs.readFileSync(path.join(extracted,'manifest.json'),'utf8'));
  expect(manifest.format).toBe('anansi-workspace-backup');
  for(const member of manifest.members){const bytes=fs.readFileSync(path.join(extracted,member.path));expect(bytes.length).toBe(member.bytes);expect(createHash('sha256').update(bytes).digest('hex')).toBe(member.sha256);}
  expect(manifest.members.some(m=>m.path==='catalog.sqlite3')).toBe(true);
  await button(page,'Backups').click();await page.getByLabel('I consent to a plaintext copy',{exact:false}).check();
  await page.getByLabel('Saved backup file',{exact:true}).setInputFiles(destination);await button(page,'Import saved backup').click();
  await expect(page.getByRole('status').filter({hasText:'Saved backup verified.'})).toBeVisible({timeout:120000});
  const imported=await page.getByRole('region',{name:'Back up and recover'}).getAttribute('data-created-backup-id');expect(imported).toMatch(/^[a-f0-9]{32}$/);expect(imported).not.toBe(jobId);
  return imported;
}
