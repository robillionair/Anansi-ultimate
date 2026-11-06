import { test, expect } from '@playwright/test';
import { createHash } from 'node:crypto';
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { startSession, type Session } from './collider-three-helpers';

test.setTimeout(300000);
const sha256 = (bytes: Buffer | Uint8Array) =>
  createHash('sha256').update(bytes).digest('hex');
const fixture = (name: string) =>
  path.resolve('tests/fixtures/pdf', name);

async function pollJob(
  session: Session,
  workspaceId: string,
  jobId: string,
  want: string,
  tries = 60,
) {
  let job: Record<string, any> | undefined;
  for (let i = 0; i < tries && (!job || job.state !== want); i++) {
    await new Promise(r => setTimeout(r, 1000));
    const list = await session.call('jobs', 'list', { workspaceId });
    job = (list.body.jobs as Array<Record<string, any>>).find(j => j.id === jobId);
  }
  return job;
}

async function listDocuments(session: Session, workspaceId: string, query?: string) {
  const list = await session.call('memories', 'list', {
    workspaceId,
    kind: 'document',
    ...(query ? { query, fullBody: true } : {}),
  });
  return list.body.memories as Array<Record<string, any>>;
}

async function importPdfViaApi(
  session: Session,
  workspaceId: string,
  fileName: string,
  bytes: Buffer,
) {
  const opened = await session.call('jobs', 'ingest-begin', {
    workspaceId,
    fileName,
    bytes: bytes.length,
    sha256: sha256(bytes),
    privacy: 'private',
  });
  expect(opened.status).toBe(200);
  const id = opened.body.id as string;
  const part = await session.call('jobs', 'ingest-part', {
    workspaceId,
    id,
    offset: 0,
    bytes: Array.from(bytes),
    finish: true,
  });
  expect(part.status).toBe(200);
  const job = await pollJob(session, workspaceId, id, 'Completed');
  expect(job?.state).toBe('Completed');
  return job!;
}

test('documents PDF UI: import, page navigation, provenance, plain-text only', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    await page.getByRole('button', { name: 'Documents', exact: true }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Your documents' })).toBeVisible();
    await page.locator('#document-file').setInputFiles(fixture('normal-3page.pdf'));
    await expect(page.getByText('Ready to import normal-3page.pdf')).toBeVisible();
    await expect(
      page.getByText('PDF — text extracted on the server during import'),
    ).toBeVisible();
    await page.getByRole('button', { name: 'Import document', exact: true }).click();
    await expect(page.getByText(/Document imported/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: 'normal-3page.pdf' })).toBeVisible();
    await page.getByRole('button', { name: 'normal-3page.pdf' }).click();
    // Page 1: plain text in a <pre>, never the Markdown renderer.
    await expect(page.getByText('Page 1 of 3')).toBeVisible();
    await expect(page.locator('.documents pre')).toContainText('Harbor Ledger');
    await expect(page.locator('.documents pre')).toContainText('bramblewick shipment');
    // No Markdown source toggle for a PDF: the text is plain, not rendered.
    await expect(page.getByRole('button', { name: 'View source' })).toHaveCount(0);
    await page.getByRole('button', { name: 'Next page' }).click();
    await expect(page.getByText('Page 2 of 3')).toBeVisible();
    await expect(page.locator('.documents pre')).toContainText('aurora storage annex');
    await page.getByRole('button', { name: 'Next page' }).click();
    await expect(page.getByText('Page 3 of 3')).toBeVisible();
    await expect(page.locator('.documents pre')).toContainText('lighthouse keeper');
    await expect(page.getByRole('button', { name: 'Next page' })).toBeDisabled();
    await page.getByRole('button', { name: 'Previous page' }).click();
    await expect(page.getByText('Page 2 of 3')).toBeVisible();
    // Provenance names the pages and their character counts.
    await page.getByText('Source provenance').click();
    await expect(page.getByText(/Media type: application\/pdf/)).toBeVisible();
    await expect(page.getByText(/3 pages/)).toBeVisible();
    await expect(page.getByText(/Page 1: \d+ characters/)).toBeVisible();
    await expect(page.getByText(/Page 2: \d+ characters/)).toBeVisible();
    await expect(page.getByText(/Page 3: \d+ characters/)).toBeVisible();
    expect(await listDocuments(session, workspaceId)).toHaveLength(1);
  } finally {
    await session.cleanup();
  }
});

test('documents PDF search: a term names the page it lives on', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const bytes = fs.readFileSync(fixture('normal-3page.pdf'));
    await importPdfViaApi(session, workspaceId, 'normal-3page.pdf', bytes);
    await page.getByRole('button', { name: 'Documents', exact: true }).click();
    // A page-2 term names Page 2.
    await page.locator('#document-search').fill('aurora');
    await page.getByRole('button', { name: 'Search documents' }).click();
    const results = page.locator('section[aria-label="Search documents"]');
    await expect(results.getByText('normal-3page.pdf')).toBeVisible();
    await expect(results.getByText(/Page 2/)).toBeVisible();
    // A page-3 term names Page 3; a page-1 term names Page 1.
    await page.locator('#document-search').fill('lighthouse');
    await page.getByRole('button', { name: 'Search documents' }).click();
    await expect(results.getByText(/Page 3/)).toBeVisible();
    await page.locator('#document-search').fill('bramblewick');
    await page.getByRole('button', { name: 'Search documents' }).click();
    await expect(results.getByText(/Page 1/)).toBeVisible();
  } finally {
    await session.cleanup();
  }
});

test('documents PDF Ask: imported content is cited as Page N (line M)', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const bytes = fs.readFileSync(fixture('normal-3page.pdf'));
    await importPdfViaApi(session, workspaceId, 'normal-3page.pdf', bytes);
    await page.getByRole('button', { name: 'Ask ANANSI', exact: true }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Ask ANANSI' })).toBeVisible();
    // Point Ask at a closed port: the offline path still shows citations.
    const closed = await new Promise<number>(resolve => {
      const server = http.createServer();
      server.listen(0, '127.0.0.1', () => {
        const port = (server.address() as { port: number }).port;
        server.close(() => resolve(port));
      });
    });
    await page.getByLabel('Local inference endpoint').fill(`http://127.0.0.1:${closed}`);
    await page.getByLabel('Your Question').fill('What is in the aurora storage annex?');
    await page.getByRole('button', { name: 'Ask Question' }).click();
    await expect(page.locator('.status-banner.status-offline')).toBeVisible({ timeout: 60000 });
    await expect(page.locator('.citation-title').first()).toContainText('normal-3page.pdf');
    await expect(page.locator('.citation-meta').first()).toContainText('Page 2 (line 1)');
  } finally {
    await session.cleanup();
  }
});

test('documents PDF API: backup and fresh restore preserves the import and its pages', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const bytes = fs.readFileSync(fixture('normal-3page.pdf'));
    await importPdfViaApi(session, workspaceId, 'normal-3page.pdf', bytes);
    const backup = await session.call('jobs', 'backup', {
      workspaceId,
      application: false,
      plaintextConsent: true,
    });
    expect(backup.status).toBe(200);
    const job = await pollJob(session, workspaceId, backup.body.job.id as string, 'Completed');
    expect(job?.state).toBe('Completed');
    const chunks: number[] = [];
    for (let offset = 0; offset < (job!.artifactBytes as number); ) {
      const chunk = await session.call('jobs', 'artifact', {
        workspaceId,
        jobId: job!.id,
        artifactId: job!.artifactId as string,
        offset,
      });
      const chunkBytes = chunk.body.bytes as number[];
      expect(chunkBytes.length).toBeGreaterThan(0);
      chunks.push(...chunkBytes);
      offset += chunkBytes.length;
    }
    const tar = Buffer.from(chunks);
    const landing = await session.call('workspaces', 'create', { name: 'PDF Landing' });
    const landingId = landing.body.workspace.id as string;
    const begin = await session.call('jobs', 'upload-begin', {
      workspaceId: landingId,
      bytes: tar.length,
      sha256: sha256(tar),
      plaintextConsent: true,
    });
    const uploadId = begin.body.id as string;
    let uploadArtifactId = '';
    let importJobId = '';
    for (let offset = 0; offset < tar.length; offset += 65536) {
      const end = Math.min(offset + 65536, tar.length);
      const part = await session.call('jobs', 'upload-part', {
        workspaceId: landingId,
        id: uploadId,
        offset,
        bytes: Array.from(tar.subarray(offset, end)),
        finish: end === tar.length,
      });
      if (part.body.job?.artifactId) uploadArtifactId = part.body.job.artifactId as string;
      if (part.body.job?.id) importJobId = part.body.job.id as string;
    }
    expect(uploadArtifactId).not.toBe('');
    const preview = await session.call('jobs', 'preview', {
      workspaceId: landingId,
      jobId: importJobId,
      artifactId: uploadArtifactId,
    });
    expect(preview.status).toBe(200);
    const restored = await session.call('jobs', 'restore', {
      workspaceId: landingId,
      jobId: importJobId,
      artifactId: uploadArtifactId,
      name: 'PDF Restored',
      expectedHash: sha256(tar),
      confirm: true,
    });
    expect(restored.status).toBe(200);
    const restoredId = (restored.body.workspaces as Array<{ id: string }>)[0]?.id as
      | string
      | undefined;
    expect(restoredId).toBeTruthy();
    const docs = await listDocuments(session, restoredId!);
    const found = docs.filter(d => d.title === 'normal-3page.pdf');
    expect(found).toHaveLength(1);
    expect(found[0].document.mediaType).toBe('application/pdf');
    expect(found[0].document.rawHash).toBe(sha256(bytes));
    expect(found[0].document.pages).toHaveLength(3);
    expect(found[0].document.pages.map((p: { page: number }) => p.page)).toEqual([1, 2, 3]);
    const read = await session.call('memories', 'read', { workspaceId: restoredId, id: found[0].id });
    expect(read.body.body).toContain('Harbor Ledger');
    expect(read.body.body).toContain('aurora storage annex');
    expect(read.body.body).toContain('lighthouse keeper');
    const search = await listDocuments(session, restoredId!, 'aurora');
    expect(search.map(d => d.title)).toContain('normal-3page.pdf');
  } finally {
    await session.cleanup();
  }
});

test('documents PDF API: encrypted, scanned, bomb and malformed PDFs fail and publish nothing', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const cases: Array<[string, string]> = [
      ['encrypted-user.pdf', 'password-protected'],
      ['scanned-image.pdf', 'no readable text'],
      ['decompression-bomb.pdf', 'decode budget'],
      ['malformed-truncated.pdf', 'could not be parsed'],
    ];
    for (const [name, expected] of cases) {
      const bytes = fs.readFileSync(fixture(name));
      const opened = await session.call('jobs', 'ingest-begin', {
        workspaceId,
        fileName: name,
        bytes: bytes.length,
        sha256: sha256(bytes),
        privacy: 'private',
      });
      expect(opened.status).toBe(200);
      const id = opened.body.id as string;
      const part = await session.call('jobs', 'ingest-part', {
        workspaceId,
        id,
        offset: 0,
        bytes: Array.from(bytes),
        finish: true,
      });
      expect(part.status).toBe(200);
      const job = await pollJob(session, workspaceId, id, 'Failed');
      expect(job?.state).toBe('Failed');
      expect(job?.log).toContain(expected);
    }
    expect(await listDocuments(session, workspaceId)).toHaveLength(0);
  } finally {
    await session.cleanup();
  }
});
