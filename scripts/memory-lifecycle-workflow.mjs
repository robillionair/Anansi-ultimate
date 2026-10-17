import { expect as baseExpect } from '@playwright/test';
const expect = baseExpect.configure({ timeout: 30000 });
const button=(page,name)=>page.getByRole('button',{name,exact:true});
async function saved(page,text){await expect(page.getByRole('status').filter({hasText:text})).toBeVisible({timeout:30000});await expect(button(page,'Reopen latest revision')).toBeEnabled();}
async function create(page,kind,title,body){
  await button(page,kind==='note'?'Notes':'Memory').click();
  await button(page,'New '+kind).click();
  await page.getByLabel(kind==='note'?'Note title':'Memory title',{exact:true}).fill(title);
  await page.getByLabel(kind==='note'?'Note text':'Memory text',{exact:true}).fill(body);
  await button(page,'Save '+kind).click();await saved(page,kind==='note'?'Note saved.':'Memory saved.');
}
async function action(page,state,successor){
  await page.getByLabel('Lifecycle action',{exact:true}).selectOption(state);
  await expect(button(page,'Apply lifecycle action')).toBeDisabled();
  if(successor)await page.getByLabel('Successor record ID (same workspace)').fill(successor);
  await page.getByLabel('I confirm this lifecycle action',{exact:true}).check();
  await button(page,'Apply lifecycle action').click();await saved(page,'Lifecycle saved.');
}
async function until(page,value){
  const input=page.getByLabel('Valid until (UTC)',{exact:true});
  if(!await input.isVisible())await page.getByText('Dates, confidence and sources',{exact:true}).click();
  await input.fill(value);
}
export async function prepareLifecycle(page,label){
  await create(page,'note',label+' note','First note content');
  await page.getByLabel('Note text',{exact:true}).fill('Second note content');await button(page,'Save note').click();await saved(page,'Note saved. Revision 2.');
  await action(page,'archived');await expect(button(page,label+' note')).toHaveCount(0);
  await page.getByLabel('Include archived records').check();await expect(button(page,label+' note')).toBeEnabled();
  await action(page,'restore_archive');await page.getByLabel('Include archived records').uncheck();await expect(button(page,label+' note')).toBeEnabled();
  await button(page,'View revision 1').click();await button(page,'Compare with latest').click();
  await expect(page.getByRole('region',{name:'Revision comparison'})).toContainText('Second note content');
  await button(page,'Restore selected content as new revision').click();await saved(page,'Historical content restored as revision 5.');
  await expect(page.getByLabel('Note text',{exact:true})).toHaveValue('First note content');
  await create(page,'memory',label+' successor','Newer fact');
  const id=(await page.locator('.help').filter({hasText:'Record ID:'}).innerText()).match(/[a-f0-9]{32}/)[0];
  await create(page,'memory',label+' lifecycle','Older fact');
  await action(page,'confirmed');await action(page,'durable');
  await until(page,'2099-01-01T00:00:00Z');await action(page,'temporary');
  await action(page,'superseded',id);await action(page,'archived');
  await expect(button(page,label+' lifecycle')).toHaveCount(0);await page.getByLabel('Include archived records').check();await expect(button(page,label+' lifecycle')).toBeEnabled();
  await create(page,'memory',label+' expired','Keep expired original');
  await until(page,'2099-01-01T00:00:00Z');await action(page,'temporary');
  await until(page,'2000-01-01T00:00:00Z');await button(page,'Save memory').click();await saved(page,'Memory saved. Revision 3.');
  await expect(button(page,label+' expired')).toHaveCount(0);await page.getByLabel('Include expired records').check();await expect(button(page,label+' expired')).toBeEnabled();
}
export async function checkLifecycle(page,label){
  await button(page,'Notes').click();await button(page,label+' note').click();
  await expect(page.getByLabel('Note text',{exact:true})).toHaveValue('First note content');
  await button(page,'View revision 2').click();await button(page,'Compare with latest').click();
  const comparison=page.getByRole('region',{name:'Revision comparison'});await expect(comparison).toContainText('Second note content');await expect(comparison).toContainText('First note content');
  await button(page,'Memory').click();await expect(button(page,label+' lifecycle')).toHaveCount(0);await page.getByLabel('Include archived records').check();await button(page,label+' lifecycle').click();await expect(page.getByText(/Viewing revision 6/)).toContainText('archived');await action(page,'restore_archive');await expect(page.getByText(/Viewing revision 7/)).toContainText('superseded');
  await expect(button(page,label+' expired')).toHaveCount(0);await page.getByLabel('Include expired records').check();await button(page,label+' expired').click();await expect(page.getByLabel('Memory text',{exact:true})).toHaveValue('Keep expired original');await expect(page.getByText(/Viewing revision 3/)).toContainText('temporary');await expect(button(page,'Reopen latest revision')).toBeEnabled();
}
