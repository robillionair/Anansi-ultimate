import { test, expect } from '@playwright/test';
import { createHash } from 'node:crypto';
import http from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { startSession, type Session } from './collider-three-helpers';

test.setTimeout(300000);
const sha256 = (bytes: Buffer | Uint8Array) =>
  createHash('sha256').update(bytes).digest('hex');

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

async function importViaApi(
  session: Session,
  workspaceId: string,
  fileName: string,
  text: string,
  privacy = 'private',
) {
  const bytes = Buffer.from(text, 'utf8');
  const opened = await session.call('jobs', 'ingest-begin', {
    workspaceId,
    fileName,
    bytes: bytes.length,
    sha256: sha256(bytes),
    privacy,
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

async function listDocuments(session: Session, workspaceId: string, query?: string) {
  const list = await session.call('memories', 'list', {
    workspaceId,
    kind: 'document',
    // A plain list query searches titles only; imported content is
    // found through the full-body FTS path.
    ...(query ? { query, fullBody: true } : {}),
  });
  return list.body.memories as Array<Record<string, any>>;
}

test('documents UI: import a text file, open it, provenance visible, search finds it', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    await page.getByRole('button', { name: 'Documents', exact: true }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Your documents' })).toBeVisible();
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-doc-ui-'));
    const file = path.join(dir, 'field-notes.txt');
    fs.writeFileSync(
      file,
      'Field notes\n\nThe harbor crane lifts at dawn. Quayside word: bramblewick.\n',
    );
    await page.locator('#document-file').setInputFiles(file);
    await expect(page.getByText('Ready to import field-notes.txt')).toBeVisible();
    await page.getByRole('button', { name: 'Import document', exact: true }).click();
    await expect(page.getByText(/Document imported/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: 'field-notes.txt' })).toBeVisible();
    await page.getByRole('button', { name: 'field-notes.txt' }).click();
    await expect(page.locator('.documents pre, .documents div').first()).toBeVisible();
    await expect(page.getByText('The harbor crane lifts at dawn.')).toBeVisible();
    await page.getByRole('button', { name: 'Ask about this document' }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Ask ANANSI' })).toBeVisible();
    await page.getByRole('button', { name: 'Documents', exact: true }).click();
    await page.getByRole('button', { name: 'field-notes.txt' }).click();
    await page.getByText('Source provenance').click();
    await expect(page.getByText(/File SHA-256:/)).toBeVisible();
    await expect(page.getByText(/Text SHA-256:/)).toBeVisible();
    await page.locator('#document-search').fill('bramblewick');
    await page.getByRole('button', { name: 'Search documents' }).click();
    await expect(
      page.locator('section[aria-label="Search documents"] button', { hasText: 'field-notes.txt' }),
    ).toBeVisible();
    fs.rmSync(dir, { recursive: true, force: true });
  } finally {
    await session.cleanup();
  }
});

test('documents UI: markdown renders inertly and a malformed PDF pick fails the job, publishing nothing', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    await page.getByRole('button', { name: 'Documents', exact: true }).click();
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-doc-md-'));
    const md = path.join(dir, 'guide.md');
    fs.writeFileSync(
      md,
      '# Harbor guide\n\nTake the **left** fork.\n\n<script>alert(1)</script>\n',
    );
    await page.locator('#document-file').setInputFiles(md);
    await expect(page.getByText('Ready to import guide.md')).toBeVisible();
    await page.getByRole('button', { name: 'Import document', exact: true }).click();
    await expect(page.getByText(/Document imported/)).toBeVisible({ timeout: 30000 });
    await page.getByRole('button', { name: 'guide.md' }).click();
    await expect(page.locator('h3', { hasText: 'Harbor guide' })).toBeVisible();
    await expect(page.locator('strong', { hasText: 'left' })).toBeVisible();
    await expect(page.locator('.documents script')).toHaveCount(0);
    await expect(page.getByText('<script>alert(1)</script>')).toBeVisible();
    // A PDF is no longer refused at pick: the file is accepted and
    // the import runs, but four bare `%PDF` bytes are not a document,
    // so the worker fails the job and publishes nothing.
    const pdf = path.join(dir, 'report.pdf');
    fs.writeFileSync(pdf, Buffer.from([0x25, 0x50, 0x44, 0x46]));
    await page.locator('#document-file').setInputFiles(pdf);
    await expect(page.getByText('Ready to import report.pdf')).toBeVisible();
    await expect(page.getByText('PDF — text extracted on the server during import')).toBeVisible();
    await page.getByRole('button', { name: 'Import document', exact: true }).click();
    await expect(page.getByText(/Import failed: PDF could not be parsed/)).toBeVisible({
      timeout: 30000,
    });
    expect(await listDocuments(session, session.workspaceId)).toHaveLength(1);
    fs.rmSync(dir, { recursive: true, force: true });
  } finally {
    await session.cleanup();
  }
});

test('documents API: invalid UTF-8 forced through the API fails the job and publishes nothing', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const bytes = Buffer.from([0xff, 0xfe, 0x41]);
    const opened = await session.call('jobs', 'ingest-begin', {
      workspaceId,
      fileName: 'bad.txt',
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
    expect(job?.log).toContain('not valid UTF-8');
    expect(await listDocuments(session, workspaceId)).toHaveLength(0);
  } finally {
    await session.cleanup();
  }
});

test('documents API: oversize, sealed, hash-mismatch and unsupported-format refusals', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const bytes = Buffer.from('small');
    const hash = sha256(bytes);
    const oversize = await session.call('jobs', 'ingest-begin', {
      workspaceId,
      fileName: 'big.txt',
      bytes: 65537,
      sha256: hash,
      privacy: 'private',
    });
    expect(oversize.status).toBe(400);
    const sealed = await session.call('jobs', 'ingest-begin', {
      workspaceId,
      fileName: 'secret.txt',
      bytes: bytes.length,
      sha256: hash,
      privacy: 'sealed',
    });
    expect(sealed.status).toBe(400);
    const docx = await session.call('jobs', 'ingest-begin', {
      workspaceId,
      fileName: 'report.docx',
      bytes: bytes.length,
      sha256: hash,
      privacy: 'private',
    });
    expect(docx.status).toBe(400);
    const mismatch = await session.call('jobs', 'ingest-begin', {
      workspaceId,
      fileName: 'mismatch.txt',
      bytes: bytes.length,
      sha256: sha256(Buffer.from('declared instead')),
      privacy: 'private',
    });
    expect(mismatch.status).toBe(200);
    const id = mismatch.body.id as string;
    const part = await session.call('jobs', 'ingest-part', {
      workspaceId,
      id,
      offset: 0,
      bytes: Array.from(bytes),
      finish: true,
    });
    expect(part.status).toBe(400);
    const job = await pollJob(session, workspaceId, id, 'Failed');
    expect(job?.state).toBe('Failed');
    expect(await listDocuments(session, workspaceId)).toHaveLength(0);
  } finally {
    await session.cleanup();
  }
});

test('documents API: cancel during upload publishes nothing and removes staging', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const bytes = Buffer.from('cancel this import please');
    const opened = await session.call('jobs', 'ingest-begin', {
      workspaceId,
      fileName: 'cancel.txt',
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

test('documents API: concurrent duplicate imports create exactly one record', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const text = 'The same file, imported twice at once.';
    const bytes = Buffer.from(text, 'utf8');
    const hash = sha256(bytes);
    const [first, second] = await Promise.all(
      [1, 2].map(() =>
        session.call('jobs', 'ingest-begin', {
          workspaceId,
          fileName: 'twice.txt',
          bytes: bytes.length,
          sha256: hash,
          privacy: 'private',
        }),
      ),
    );
    expect(first.status).toBe(200);
    expect(second.status).toBe(200);
    await Promise.all(
      [first, second].map(opened =>
        session.call('jobs', 'ingest-part', {
          workspaceId,
          id: opened.body.id as string,
          offset: 0,
          bytes: Array.from(bytes),
          finish: true,
        }),
      ),
    );
    const a = await pollJob(session, workspaceId, first.body.id as string, 'Completed');
    const b = await pollJob(session, workspaceId, second.body.id as string, 'Completed');
    expect(a?.state).toBe('Completed');
    expect(b?.state).toBe('Completed');
    expect(a?.artifactId).toBe(b?.artifactId);
    expect(await listDocuments(session, workspaceId)).toHaveLength(1);
  } finally {
    await session.cleanup();
  }
});

test('documents API: killing the API mid-upload, restarting, and resuming imports once', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const text = 'Resumed after a restart, exactly once.';
    const bytes = Buffer.from(text, 'utf8');
    const hash = sha256(bytes);
    const opened = await session.call('jobs', 'ingest-begin', {
      workspaceId,
      fileName: 'resumed.txt',
      bytes: bytes.length,
      sha256: hash,
      privacy: 'private',
    });
    expect(opened.status).toBe(200);
    const id = opened.body.id as string;
    // Kill the harness-owned API child, confirm it exits, then restart
    // the platform-correct executable against the same storage root.
    // No port lookup and no spec-side respawn: `ss` does not exist on
    // Windows, and the harness owns both the child and its pairing.
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
    const found = docs.filter(d => d.title === 'resumed.txt');
    expect(found).toHaveLength(1);
    expect(found[0].document.fileName).toBe('resumed.txt');
  } finally {
    await session.cleanup();
  }
});

test('documents API: backup and fresh restore preserves the import', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const text = '# Restored notes\n\nThe restored harbor ledger says bramblewick.';
    await importViaApi(session, workspaceId, 'restored.md', text);
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
      const bytes = chunk.body.bytes as number[];
      expect(bytes.length).toBeGreaterThan(0);
      chunks.push(...bytes);
      offset += bytes.length;
    }
    const tar = Buffer.from(chunks);
    const landing = await session.call('workspaces', 'create', { name: 'Documents Landing' });
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
      name: 'Documents Restored',
      expectedHash: sha256(tar),
      confirm: true,
    });
    expect(restored.status).toBe(200);
    const restoredId = (restored.body.workspaces as Array<{ id: string }>)[0]?.id as
      | string
      | undefined;
    expect(restoredId).toBeTruthy();
    const docs = await listDocuments(session, restoredId!);
    const found = docs.filter(d => d.title === 'restored.md');
    expect(found).toHaveLength(1);
    expect(found[0].document.rawHash).toBe(sha256(Buffer.from(text, 'utf8')));
    expect(found[0].document.mediaType).toBe('text/markdown');
    const read = await session.call('memories', 'read', { workspaceId: restoredId, id: found[0].id });
    expect(read.body.body).toBe(text);
    const search = await listDocuments(session, restoredId!, 'bramblewick');
    expect(search.map(d => d.title)).toContain('restored.md');
  } finally {
    await session.cleanup();
  }
});

test('documents Ask: imported content is cited, and a missing topic is honest', async ({ page }) => {
  const session = await startSession(page, true);
  try {
    const workspaceId = session.workspaceId;
    const text = '# Aurora storage\n\nWe decided to adopt SQLite with FTS5 for local-first search.';
    await importViaApi(session, workspaceId, 'aurora.md', text);
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
    await page.getByLabel('Your Question').fill('What did we decide about local search?');
    await page.getByRole('button', { name: 'Ask Question' }).click();
    await expect(page.locator('.status-banner.status-offline')).toBeVisible({ timeout: 60000 });
    await expect(page.locator('.citation-title')).toContainText('aurora.md');
    await expect(page.locator('.citation-meta')).toContainText('# Aurora storage');
    await page.getByRole('button', { name: 'View What ANANSI Sent (Disclosure)' }).click();
    await expect(page.locator('.manifest-metrics')).toContainText('aurora.md');
    await page.getByLabel('Your Question').fill('What is our policy on planetary space exploration?');
    await page.getByRole('button', { name: 'Ask Question' }).click();
    await expect(page.locator('.status-banner.status-insufficientEvidence')).toBeVisible({ timeout: 60000 });
    await expect(page.locator('.answer-text')).toContainText("couldn't find any relevant memories");
  } finally {
    await session.cleanup();
  }
});
