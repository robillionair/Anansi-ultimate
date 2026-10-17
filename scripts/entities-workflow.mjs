import { expect } from '@playwright/test';

const button = (page, name) => page.getByRole('button', { name, exact: true });

async function saved(page, text) {
  await expect(page.getByRole('status').filter({ hasText: text })).toBeVisible({ timeout: 30000 });
}

export async function prepareEntities(page, label) {
  // 1. Create a Project
  await button(page, 'Projects').click();
  await button(page, 'New project').click();
  await page.getByLabel('Project title', { exact: true }).fill(label + ' Alpha Project');
  await page.getByLabel('Description & objectives', { exact: true }).fill('Core initiative description for ' + label);
  await button(page, 'Save project').click();
  await saved(page, 'Project saved.');
  await expect(button(page, label + ' Alpha Project')).toBeVisible({ timeout: 15000 });

  // 2. Create a Person
  await button(page, 'People').click();
  await button(page, 'New person').click();
  await page.getByLabel('Full name', { exact: true }).fill(label + ' Jane Doe');
  await page.getByLabel('Role, notes & bio', { exact: true }).fill('Lead Engineer on ' + label);
  await button(page, 'Save person').click();
  await saved(page, 'Person saved.');
  await expect(button(page, label + ' Jane Doe')).toBeVisible({ timeout: 15000 });

  // 3. Create a Task associated with the project
  await button(page, 'Tasks').click();
  await button(page, 'New task').click();
  await page.getByLabel('Task title', { exact: true }).fill(label + ' Primary Task');
  await page.getByLabel('Associated project (optional)', { exact: true }).selectOption({ label: label + ' Alpha Project' });
  await page.getByLabel('Due date (UTC ISO string)', { exact: true }).fill('2026-11-15T18:00:00Z');
  await page.getByLabel('Details & notes', { exact: true }).fill('Implement core telemetry harness');
  await button(page, 'Save task').click();
  await saved(page, 'Task saved.');
  await expect(page.getByRole('button', { name: label + ' Primary Task', exact: true })).toBeVisible({ timeout: 15000 });

  // Mark task completed
  const checkbox = page.getByRole('checkbox', { name: new RegExp(`Mark "${label} Primary Task" as complete`) });
  await checkbox.click();
  await saved(page, 'Task marked completed.');

  // 4. Create a Decision in Knowledge Records
  await button(page, 'Records').click();
  await button(page, 'New record').click();
  await page.getByLabel('Record kind', { exact: true }).selectOption('decision');
  await page.getByLabel('Title', { exact: true }).fill(label + ' Architecture Decision');
  await page.getByLabel('Event date (UTC ISO timestamp)', { exact: true }).fill('2026-10-17T12:00:00Z');
  await page.getByLabel('Content & description', { exact: true }).fill('Adopted typed directed relationships schema 5');
  await button(page, 'Save record').click();
  await saved(page, 'Record saved (decision).');
  await expect(button(page, label + ' Architecture Decision')).toBeVisible({ timeout: 15000 });

  // 5. Connect Relationship: Project -> documents -> Decision
  await button(page, label + ' Architecture Decision').click();
  const decisionId = (await page.locator('.help code').innerText()).trim();

  await button(page, 'Projects').click();
  await button(page, label + ' Alpha Project').click();
  await expect(page.getByText('Core initiative description for ' + label)).toBeVisible();
  await expect(page.getByText(label + ' Primary Task (Completed)')).toBeVisible();

  await page.getByLabel('Target record ID').fill(decisionId);
  await page.getByLabel('Predicate').selectOption('documents');
  await button(page, 'Add relationship').click();
  await saved(page, 'Relationship created.');
  await expect(page.getByRole('list', { name: 'Relationships for this project' }).getByText('documents')).toBeVisible();
}

export async function checkEntities(page, label) {
  // 1. Check Project
  await button(page, 'Projects').click();
  await button(page, label + ' Alpha Project').click();
  await expect(page.getByLabel('Project title', { exact: true })).toHaveValue(label + ' Alpha Project');
  await expect(page.getByLabel('Description & objectives', { exact: true })).toHaveValue('Core initiative description for ' + label);
  await expect(page.getByText(label + ' Primary Task (Completed)')).toBeVisible();
  await expect(page.getByRole('list', { name: 'Relationships for this project' }).getByText('documents')).toBeVisible();

  // 2. Check Task
  await button(page, 'Tasks').click();
  await expect(page.getByRole('button', { name: label + ' Primary Task', exact: true })).toBeVisible();
  const checkbox = page.getByRole('checkbox', { name: new RegExp(`Mark "${label} Primary Task" as incomplete`) });
  await expect(checkbox).toBeChecked();
  await button(page, label + ' Primary Task').click();
  await expect(page.getByLabel('Due date (UTC ISO string)', { exact: true })).toHaveValue('2026-11-15T18:00:00Z');

  // 3. Check Person
  await button(page, 'People').click();
  await button(page, label + ' Jane Doe').click();
  await expect(page.getByLabel('Full name', { exact: true })).toHaveValue(label + ' Jane Doe');
  await expect(page.getByLabel('Role, notes & bio', { exact: true })).toHaveValue('Lead Engineer on ' + label);

  // 4. Check Decision in Records
  await button(page, 'Records').click();
  await page.getByLabel('Filter by kind:', { exact: true }).selectOption('decision');
  await expect(button(page, label + ' Architecture Decision')).toBeVisible();
  await button(page, label + ' Architecture Decision').click();
  await expect(page.getByLabel('Event date (UTC ISO timestamp)', { exact: true })).toHaveValue('2026-10-17T12:00:00Z');
  await expect(page.getByLabel('Content & description', { exact: true })).toHaveValue('Adopted typed directed relationships schema 5');
}
