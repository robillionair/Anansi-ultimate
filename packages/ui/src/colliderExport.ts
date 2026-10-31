// M8 §27 deterministic experiment export bundle.
//
// A minimal ZIP writer (STORE method, no compression)
// with fixed timestamps and fixed entry order, so the
// same inputs produce byte-identical output. No new
// dependency: CRC-32 is implemented here, SHA-256
// comes from the platform (`crypto.subtle`, injected
// so node tests can pass their own hasher).
//
// MANIFEST integrity rule: the manifest lists the six
// derived documents plus `experiment.json` with byte
// lengths and SHA-256 hashes. The manifest does NOT
// list itself (its own hash would be circular); its
// integrity inside the archive is covered by the ZIP
// CRC-32 of its entry. When the saved document
// carries stored artifact hashes, export verifies the
// fresh derivation against them and refuses on ANY
// mismatch — a corrupt derivation never ships.
import type { ColliderExperimentDoc } from '@anansi/contracts';
import { deriveAllArtifacts } from './colliderArtifacts';

export interface ExportFile {
  name: string;
  data: Uint8Array;
}

const CRC_TABLE: Uint32Array = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n += 1) {
    let c = n;
    for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

export function crc32(data: Uint8Array): number {
  let crc = 0xffffffff;
  for (let i = 0; i < data.length; i += 1) {
    crc = CRC_TABLE[(crc ^ data[i]) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function u16(value: number): [number, number] {
  return [value & 0xff, (value >>> 8) & 0xff];
}

function u32(value: number): [number, number, number, number] {
  return [value & 0xff, (value >>> 8) & 0xff, (value >>> 16) & 0xff, (value >>> 24) & 0xff];
}

export function buildZip(files: ExportFile[]): Uint8Array {
  const chunks: number[] = [];
  const central: number[] = [];
  let offset = 0;
  // Fixed DOS timestamp: 1980-01-01 00:00:00.
  const modTime = 0;
  const modDate = 0x0021;
  for (const file of files) {
    const name = new TextEncoder().encode(file.name);
    const crc = crc32(file.data);
    const header = [
      ...u32(0x04034b50),
      ...u16(20),
      ...u16(0),
      ...u16(0),
      ...u16(modTime),
      ...u16(modDate),
      ...u32(crc),
      ...u32(file.data.length),
      ...u32(file.data.length),
      ...u16(name.length),
      ...u16(0),
      ...name,
    ];
    chunks.push(...header, ...file.data);
    central.push(
      ...u32(0x02014b50),
      ...u16(20),
      ...u16(20),
      ...u16(0),
      ...u16(0),
      ...u16(modTime),
      ...u16(modDate),
      ...u32(crc),
      ...u32(file.data.length),
      ...u32(file.data.length),
      ...u16(name.length),
      ...u16(0),
      ...u16(0),
      ...u16(0),
      ...u16(0),
      ...u32(0),
      ...u32(offset),
      ...name,
    );
    offset += header.length + file.data.length;
  }
  const cdStart = offset;
  const cdSize = central.length;
  chunks.push(...central);
  chunks.push(
    ...u32(0x06054b50),
    ...u16(0),
    ...u16(0),
    ...u16(files.length),
    ...u16(files.length),
    ...u32(cdSize),
    ...u32(cdStart),
    ...u16(0),
  );
  return new Uint8Array(chunks);
}

export type HashBytes = (data: Uint8Array) => Promise<string>;

export async function subtleSha256(data: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', data as BufferSource);
  return Array.from(new Uint8Array(digest))
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

export interface ExportManifestFile {
  name: string;
  bytes: number;
  sha256: string;
}

export interface ExportManifest {
  format: string;
  version: number;
  experimentArtifactId: string;
  experimentVersion: number;
  experimentTimestamp: string;
  provider: string;
  model: string;
  files: ExportManifestFile[];
}

export interface BuiltExport {
  bytes: Uint8Array;
  filename: string;
  manifest: ExportManifest;
}

export async function buildExperimentExport(
  input: {
    doc: ColliderExperimentDoc;
    body: string;
    artifactId: string;
    version: number;
    timestamp: string;
  },
  hashBytes: HashBytes = subtleSha256,
): Promise<BuiltExport> {
  const derived = deriveAllArtifacts(input.doc);
  const encoder = new TextEncoder();
  const files: ExportFile[] = derived.map(({ file, text }) => ({
    name: file,
    data: encoder.encode(text),
  }));
  files.push({ name: 'experiment.json', data: encoder.encode(input.body) });
  // Fail closed: when the saved document carries
  // stored artifact hashes, the fresh derivation must
  // agree with every one of them before anything
  // ships.
  const stored = input.doc.artifacts;
  if (stored) {
    const byFile: Record<string, { sha256: string; bytes: number }> = {
      'CONCEPT.md': stored.concept,
      'ARCHITECTURE.md': stored.architecture,
      'FEASIBILITY.md': stored.feasibility,
      'ROADMAP.md': stored.roadmap,
      'RISKS.md': stored.risks,
      'TEST_PLAN.md': stored.testPlan,
    };
    for (const file of files) {
      if (file.name === 'experiment.json') continue;
      const hash = await hashBytes(file.data);
      const record = byFile[file.name];
      if (!record || hash !== record.sha256 || file.data.length !== record.bytes) {
        throw new Error(
          `Export refused: derived ${file.name} does not match the saved experiment's recorded hash.`,
        );
      }
    }
  }
  const manifestFiles: ExportManifestFile[] = [];
  for (const file of files) {
    manifestFiles.push({
      name: file.name,
      bytes: file.data.length,
      sha256: await hashBytes(file.data),
    });
  }
  const manifest: ExportManifest = {
    format: 'anansi-collider-experiment-export',
    version: 1,
    experimentArtifactId: input.artifactId,
    experimentVersion: input.version,
    experimentTimestamp: input.timestamp,
    provider: input.doc.provider,
    model: input.doc.model,
    files: manifestFiles,
  };
  const manifestBytes = encoder.encode(`${JSON.stringify(manifest, null, 2)}\n`);
  const allFiles = [...files, { name: 'MANIFEST.json', data: manifestBytes }];
  return {
    bytes: buildZip(allFiles),
    filename: `anansi-collider-${input.artifactId}-v${input.version}.zip`,
    manifest,
  };
}
