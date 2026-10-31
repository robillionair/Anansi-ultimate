import { test, expect } from '@playwright/test';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import {
  collideAndSave,
  startSession,
} from './collider-three-helpers';

/**
 * M8 §27 vertical slice in the real browser: collide
 * → save → six-artifact viewer (tabs, search, copy
 * affordance) → deterministic export bundle (zip +
 * MANIFEST hashes verified against the saved record)
 * → Build This Product (Forge concept with the
 * derived ARCHITECTURE.md as initial specification)
 * → backup/restore → viewer and already-transferred
 * Build survive with identical hashes. The live,
 * unapproved panel has none of these actions.
 */

// Minimal STORE-only ZIP reader for the spec: local
// headers are sequential, so entries parse in order.
function readZip(bytes: Buffer): Array<{ name: string; data: Buffer; crc: number }> {
  const entries: Array<{ name: string; data: Buffer; crc: number }> = [];
  let offset = 0;
  while (bytes.readUInt32LE(offset) === 0x04034b50) {
    const crc = bytes.readUInt32LE(offset + 14);
    const size = bytes.readUInt32LE(offset + 18);
    const nameLen = bytes.readUInt16LE(offset + 26);
    const extraLen = bytes.readUInt16LE(offset + 28);
    const nameStart = offset + 30 + extraLen;
    const name = bytes.subarray(nameStart, nameStart + nameLen).toString('utf8');
    const dataStart = nameStart + nameLen;
    entries.push({ name, data: bytes.subarray(dataStart, dataStart + size), crc });
    offset = dataStart + size;
  }
  expect(bytes.readUInt32LE(offset)).toBe(0x02014b50);
  return entries;
}

function sha256(data: Buffer): string {
  return createHash('sha256').update(data).digest('hex');
}

test('§27 artifacts: viewer, export bundle, Build This Product, restore survival', async ({ page }) => {
  test.setTimeout(300000);
  page.setDefaultTimeout(8000);
  const session = await startSession(page, false);
  const shotDir = process.env.VIZ_SHOT_DIR;
  const viewer = session.artifacts.locator('section[aria-label="Experiment artifacts"]');
  const build = session.artifacts.locator('section[aria-label="Build this product"]');
  try {
    // Unapproved live panel: no viewer, no export, no Build.
    await expect(session.collider.getByRole('button', { name: 'Build This Product' })).toHaveCount(0);
    await expect(session.collider.locator('section[aria-label="Experiment artifacts"]')).toHaveCount(0);

    await collideAndSave(session.collider, 'First beam about kitchens.', 'First beam about orbitals.');
    await session.artifacts.getByRole('button', { name: /Collider experiment: First beam about kitchens/ }).click();
    await expect(viewer.getByRole('heading', { name: 'Experiment artifacts' })).toBeVisible();

    // All six tabs exist; ARCHITECTURE and RISKS carry
    // the record's own text, with unstated values marked.
    for (const file of ['CONCEPT.md', 'ARCHITECTURE.md', 'FEASIBILITY.md', 'ROADMAP.md', 'RISKS.md', 'TEST_PLAN.md']) {
      await expect(viewer.getByRole('tab', { name: file })).toBeVisible();
    }
    await viewer.getByRole('tab', { name: 'ARCHITECTURE.md' }).click();
    await expect(viewer.getByText(/Signal ingest, local scoring/)).toBeVisible();
    await viewer.getByRole('tab', { name: 'RISKS.md' }).click();
    await expect(viewer.getByText(/Stale signals could route into worse fills/)).toBeVisible();
    await expect(viewer.getByText(/Severity: Not stated in the source record/)).toBeVisible();

    // Search filters to matching lines; clearing restores.
    await viewer.getByLabel('Search RISKS.md').fill('Stale');
    await expect(viewer.getByText(/matching line/)).toBeVisible();
    await viewer.getByLabel('Search RISKS.md').fill('');
    await expect(viewer.getByText(/Stale signals could route into worse fills/)).toBeVisible();
    if (shotDir) await viewer.screenshot({ path: path.join(shotDir, 'artifacts-viewer.png') });

    // Export twice: byte-identical bundles, manifest
    // hashes equal to entry hashes and to the saved
    // record's stored artifact hashes.
    async function downloadBundle(): Promise<Buffer> {
      const downloadPromise = page.waitForEvent('download');
      await viewer.getByRole('button', { name: 'Export Full Experiment' }).click();
      const download = await downloadPromise;
      expect(download.suggestedFilename()).toMatch(/^anansi-collider-.*-v1\.zip$/);
      const filePath = await download.path();
      return fs.readFileSync(filePath!);
    }
    const first = await downloadBundle();
    const second = await downloadBundle();
    expect(first.equals(second)).toBe(true);
    const entries = readZip(first);
    expect(entries.map(e => e.name)).toEqual([
      'CONCEPT.md',
      'ARCHITECTURE.md',
      'FEASIBILITY.md',
      'ROADMAP.md',
      'RISKS.md',
      'TEST_PLAN.md',
      'experiment.json',
      'MANIFEST.json',
    ]);
    const byName = new Map(entries.map(e => [e.name, e]));
    const experiment = JSON.parse(byName.get('experiment.json')!.data.toString('utf8'));
    expect(experiment.format).toBe('anansi-collider-experiment');
    expect(experiment.artifacts).toBeTruthy();
    const manifest = JSON.parse(byName.get('MANIFEST.json')!.data.toString('utf8'));
    expect(manifest.experimentVersion).toBe(1);
    expect(manifest.provider).toBe('ollama');
    for (const file of manifest.files) {
      const entry = byName.get(file.name)!;
      expect(file.sha256).toBe(sha256(entry.data));
      expect(file.bytes).toBe(entry.data.length);
    }
    const storedByFile: Record<string, { sha256: string }> = {
      'CONCEPT.md': experiment.artifacts.concept,
      'ARCHITECTURE.md': experiment.artifacts.architecture,
      'FEASIBILITY.md': experiment.artifacts.feasibility,
      'ROADMAP.md': experiment.artifacts.roadmap,
      'RISKS.md': experiment.artifacts.risks,
      'TEST_PLAN.md': experiment.artifacts.testPlan,
    };
    for (const file of manifest.files) {
      if (storedByFile[file.name]) {
        expect(file.sha256).toBe(storedByFile[file.name].sha256);
      }
    }

    // Build This Product: one concept, architecture as
    // initial specification, Forge panel, no auto-spec.
    await expect(build.getByRole('heading', { name: 'Build This Product' })).toBeVisible();
    await build.getByLabel('Combination to build').selectOption({ label: 'Mycelial order router' });
    await build.getByRole('button', { name: 'Build This Product' }).click();
    await expect(page.getByText('Built: Forge concept created from the approved outputs.')).toBeVisible();
    const concept = session.artifacts.locator('section[aria-label="Transferred concept"]');
    await expect(concept).toBeVisible();
    await expect(concept.getByText(/the record's concept brief, used verbatim by Build This Product/)).toBeVisible();
    await expect(concept.getByText('Initial technical specification (derived from the pinned experiment')).toBeVisible();
    await expect(concept.getByText(/Signal ingest, local scoring/)).toBeVisible();
    await expect(session.artifacts.getByRole('heading', { name: 'Product Forge — develop a concept (§§28–33)' })).toBeVisible();
    await expect(session.artifacts.getByRole('heading', { name: /^Product specification:/ })).toHaveCount(0);
    if (shotDir) await concept.screenshot({ path: path.join(shotDir, 'artifacts-concept.png') });

    // Backup, fresh restore, reopen: viewer tabs and
    // Build idempotency survive with identical hashes.
    const { workspaceId } = session;
    const backup = await session.call('jobs', 'backup', { workspaceId, application: false, plaintextConsent: true });
    expect(backup.status).toBe(200);
    const jobId = backup.body.job.id as string;
    let job: Record<string, any> | undefined;
    for (let i = 0; i < 60 && (!job || job.state !== 'Completed'); i++) {
      await new Promise(r => setTimeout(r, 1000));
      const list = await session.call('jobs', 'list', { workspaceId });
      job = (list.body.jobs as Array<Record<string, any>>).find(j => j.id === jobId);
    }
    expect(job!.state).toBe('Completed');
    const artifactBytes = job!.artifactBytes as number;
    const chunks: number[] = [];
    for (let offset = 0; offset < artifactBytes;) {
      const chunk = await session.call('jobs', 'artifact', { workspaceId, jobId, artifactId: job!.artifactId as string, offset });
      const bytes = chunk.body.bytes as number[];
      expect(bytes.length).toBeGreaterThan(0);
      chunks.push(...bytes);
      offset += bytes.length;
    }
    expect(chunks.length).toBe(artifactBytes);
    const tar = Buffer.from(chunks);
    const landing = await session.call('workspaces', 'create', { name: 'Artifacts Landing' });
    const landingId = landing.body.workspace.id as string;
    const begin = await session.call('jobs', 'upload-begin', { workspaceId: landingId, bytes: tar.length, sha256: sha256(tar), plaintextConsent: true });
    const uploadId = begin.body.id as string;
    let uploadArtifactId = '';
    let importJobId = '';
    for (let offset = 0; offset < tar.length; offset += 65536) {
      const end = Math.min(offset + 65536, tar.length);
      const part = await session.call('jobs', 'upload-part', { workspaceId: landingId, id: uploadId, offset, bytes: Array.from(tar.subarray(offset, end)), finish: end === tar.length });
      if (part.body.job?.artifactId) uploadArtifactId = part.body.job.artifactId as string;
      if (part.body.job?.id) importJobId = part.body.job.id as string;
    }
    expect(uploadArtifactId).not.toBe('');
    expect(importJobId).not.toBe('');
    const preview = await session.call('jobs', 'preview', { workspaceId: landingId, jobId: importJobId, artifactId: uploadArtifactId });
    expect(preview.status).toBe(200);
    const restored = await session.call('jobs', 'restore', { workspaceId: landingId, jobId: importJobId, artifactId: uploadArtifactId, name: 'Artifacts Restored', expectedHash: preview.body.sha256, confirm: true });
    expect(restored.status).toBe(200);
    const restoredWs = (restored.body.workspaces as Array<Record<string, any>>)[0].id as string;
    expect(restoredWs).not.toBe(workspaceId);

    await session.pairAndOpen(restoredWs);
    await session.artifacts.getByRole('button', { name: /Collider experiment: First beam about kitchens/ }).click();
    await expect(viewer.getByRole('heading', { name: 'Experiment artifacts' })).toBeVisible();
    for (const file of ['CONCEPT.md', 'ARCHITECTURE.md', 'FEASIBILITY.md', 'ROADMAP.md', 'RISKS.md', 'TEST_PLAN.md']) {
      await expect(viewer.getByRole('tab', { name: file })).toBeVisible();
    }
    await viewer.getByRole('tab', { name: 'ARCHITECTURE.md' }).click();
    await expect(viewer.getByText(/Signal ingest, local scoring/)).toBeVisible();
    await build.getByLabel('Combination to build').selectOption({ label: 'Mycelial order router' });
    await build.getByRole('button', { name: 'Build This Product' }).click();
    await expect(page.getByText('Already built: opened the existing Forge concept for this combination.')).toBeVisible();
    await expect(concept).toBeVisible();
    await expect(concept.getByText(/Signal ingest, local scoring/)).toBeVisible();
  } finally {
    await session.cleanup();
  }
});
