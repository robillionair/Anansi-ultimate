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
  path.resolve('tests/fixtures/docx', name);

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

async function importDocxViaApi(
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

test('documents DOCX UI: import, section provenance, headings render as Markdown', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    await page.getByRole('button', { name: 'Documents', exact: true }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Your documents' })).toBeVisible();
    await page.locator('#document-file').setInputFiles(fixture('normal.docx'));
    await expect(page.getByText('Ready to import normal.docx')).toBeVisible();
    await expect(
      page.getByText('DOCX — text extracted on the server during import'),
    ).toBeVisible();
    await page.getByRole('button', { name: 'Import document', exact: true }).click();
    await expect(page.getByText(/Document imported/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: 'normal.docx' })).toBeVisible();
    await page.getByRole('button', { name: 'normal.docx' }).click();
    // Headings render as Markdown (h3 for `#`, h4 for `##`, h5 for `###`).
    await expect(page.locator('h3', { hasText: 'Harbor Ledger' })).toBeVisible();
    await expect(page.locator('h4', { hasText: 'Manifest' })).toBeVisible();
    await expect(page.locator('h5', { hasText: 'Notes' })).toBeVisible();
    // Unlike a PDF, a DOCX body is Markdown, so the source toggle exists.
    await expect(page.getByRole('button', { name: 'View source' })).toBeVisible();
    await page.getByRole('button', { name: 'View source' }).click();
    await expect(page.locator('.documents pre')).toContainText('# Harbor Ledger');
    // Provenance names the sections and their character counts.
    await page.getByText('Source provenance').click();
    await expect(
      page.getByText(
        /Media type: application\/vnd\.openxmlformats-officedocument\.wordprocessingml\.document/,
      ),
    ).toBeVisible();
    await expect(page.getByText(/4 sections/)).toBeVisible();
    await expect(page.getByText(/Harbor Ledger: 15 characters/)).toBeVisible();
    await expect(page.getByText(/Customs: 62 characters/)).toBeVisible();
    await expect(page.getByText(/Manifest: 86 characters/)).toBeVisible();
    await expect(page.getByText(/Notes: 131 characters/)).toBeVisible();
    expect(await listDocuments(session, workspaceId)).toHaveLength(1);
  } finally {
    await session.cleanup();
  }
});

test('documents DOCX search: a term names the section it lives on', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const bytes = fs.readFileSync(fixture('normal.docx'));
    await importDocxViaApi(session, workspaceId, 'normal.docx', bytes);
    await page.getByRole('button', { name: 'Documents', exact: true }).click();
    const results = page.locator('section[aria-label="Search documents"]');
    // A Manifest-section term names Manifest.
    await page.locator('#document-search').fill('copper fittings');
    await page.getByRole('button', { name: 'Search documents' }).click();
    await expect(results.getByText('normal.docx')).toBeVisible();
    await expect(results.getByText('Manifest')).toBeVisible();
    // A Customs-section term names Customs; a Notes-section term names Notes.
    await page.locator('#document-search').fill('bramblewick');
    await page.getByRole('button', { name: 'Search documents' }).click();
    await expect(results.getByText('Customs')).toBeVisible();
    await page.locator('#document-search').fill('lighthouse keeper');
    await page.getByRole('button', { name: 'Search documents' }).click();
    await expect(results.getByText('Notes')).toBeVisible();
  } finally {
    await session.cleanup();
  }
});

test('documents DOCX Ask: imported content is cited as ## Manifest (line 5)', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const bytes = fs.readFileSync(fixture('normal.docx'));
    await importDocxViaApi(session, workspaceId, 'normal.docx', bytes);
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
    // Ask with terms from the Manifest section's content, so that
    // chunk ranks first (a heading-only term does not rank its chunk).
    await page.getByLabel('Your Question').fill('How many crates of copper fittings are there?');
    await page.getByRole('button', { name: 'Ask Question' }).click();
    await expect(page.locator('.status-banner.status-offline')).toBeVisible({ timeout: 60000 });
    await expect(page.locator('.citation-title').first()).toContainText('normal.docx');
    await expect(page.locator('.citation-meta').first()).toContainText('## Manifest (line 5)');
  } finally {
    await session.cleanup();
  }
});

test('documents DOCX API: backup and fresh restore preserves the import and its sections', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const bytes = fs.readFileSync(fixture('normal.docx'));
    await importDocxViaApi(session, workspaceId, 'normal.docx', bytes);
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
    const landing = await session.call('workspaces', 'create', { name: 'DOCX Landing' });
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
      name: 'DOCX Restored',
      expectedHash: sha256(tar),
      confirm: true,
    });
    expect(restored.status).toBe(200);
    const restoredId = (restored.body.workspaces as Array<{ id: string }>)[0]?.id as
      | string
      | undefined;
    expect(restoredId).toBeTruthy();
    const docs = await listDocuments(session, restoredId!);
    const found = docs.filter(d => d.title === 'normal.docx');
    expect(found).toHaveLength(1);
    expect(found[0].document.mediaType).toBe(
      'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    );
    expect(found[0].document.rawHash).toBe(sha256(bytes));
    expect(found[0].document.sections).toHaveLength(4);
    expect(
      found[0].document.sections.map((s: { heading: string }) => s.heading),
    ).toEqual(['Harbor Ledger', 'Customs', 'Manifest', 'Notes']);
    const read = await session.call('memories', 'read', { workspaceId: restoredId, id: found[0].id });
    expect(read.body.body).toContain('Harbor Ledger');
    expect(read.body.body).toContain('copper fittings');
    expect(read.body.body).toContain('lighthouse keeper');
    const search = await listDocuments(session, restoredId!, 'copper');
    expect(search.map(d => d.title)).toContain('normal.docx');
  } finally {
    await session.cleanup();
  }
});

test('documents DOCX API: macro, encrypted, bomb, oversized and malformed DOCX fail and publish nothing', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const cases: Array<[string, string]> = [
      ['macro.docx', 'does not support'],
      ['altchunk.docx', 'does not support'],
      ['encrypted-entry.docx', 'password-protected'],
      ['zip-bomb.docx', 'could not be parsed'],
      ['oversized-text.docx', 'exceeds the limit'],
      ['corrupt-bad-xml.docx', 'could not be parsed'],
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

test('documents DOCX API: cancel during upload publishes nothing and removes staging', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const bytes = fs.readFileSync(fixture('normal.docx'));
    const opened = await session.call('jobs', 'ingest-begin', {
      workspaceId,
      fileName: 'normal.docx',
      bytes: bytes.length,
      sha256: sha256(bytes),
      privacy: 'private',
    });
    const id = opened.body.id as string;
    const part = await session.call('jobs', 'ingest-part', {
      workspaceId,
      id,
      offset: 0,
      bytes: Array.from(bytes.subarray(0, 6)),
      finish: false,
    });
    expect(part.status).toBe(200);
    const list = await session.call('jobs', 'list', { workspaceId });
    const job = (list.body.jobs as Array<Record<string, any>>).find(j => j.id === id)!;
    const cancelled = await session.call('jobs', 'control', {
      workspaceId,
      id,
      expectedVersion: job.version,
      action: 'cancel',
    });
    expect(cancelled.status).toBe(200);
    expect(cancelled.body.job.state).toBe('Cancelled');
    const late = await session.call('jobs', 'ingest-part', {
      workspaceId,
      id,
      offset: 6,
      bytes: Array.from(bytes.subarray(6)),
      finish: true,
    });
    expect(late.status).not.toBe(200);
    expect(await listDocuments(session, workspaceId)).toHaveLength(0);
  } finally {
    await session.cleanup();
  }
});

test('documents DOCX API: killing the API mid-upload, restarting, and resuming imports once', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const bytes = fs.readFileSync(fixture('normal.docx'));
    const hash = sha256(bytes);
    const opened = await session.call('jobs', 'ingest-begin', {
      workspaceId,
      fileName: 'normal.docx',
      bytes: bytes.length,
      sha256: hash,
      privacy: 'private',
    });
    expect(opened.status).toBe(200);
    const id = opened.body.id as string;
    // Kill the harness-owned API child, confirm it exits, then restart
    // the platform-correct executable against the same storage root.
    await session.restartApi();
    // The interrupted upload resumes from offset 0 (nothing was sent yet).
    const resumed = await session.call('jobs', 'ingest-part', {
      workspaceId,
      id,
      offset: 0,
      bytes: Array.from(bytes),
      finish: true,
    });
    expect(resumed.status).toBe(200);
    const job = await pollJob(session, workspaceId, id, 'Completed');
    expect(job?.state).toBe('Completed');
    const docs = await listDocuments(session, workspaceId);
    const found = docs.filter(d => d.title === 'normal.docx');
    expect(found).toHaveLength(1);
    expect(found[0].document.fileName).toBe('normal.docx');
    expect(found[0].document.sections).toHaveLength(4);
  } finally {
    await session.cleanup();
  }
});
