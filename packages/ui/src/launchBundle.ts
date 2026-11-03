// M12 Slice 2: deterministic client-side ZIP builder for launch bundles.
//
// Stored entries only (method 0, no compression), a fixed DOS timestamp
// (1980-01-01 00:00:00) on every entry, UTF-8 filenames, and entries sorted
// by path: the same file set always yields the same bytes. No dependency.
//
// This mirrors the Forge export's guarantees (deterministic bytes, a
// version/hash manifest, portable relative paths, zero unstated files) in a
// different container: the Forge export is a server-side USTAR archive; this
// is a client-side stored ZIP, because every bundle input is already
// readable by the client and no new server op is wanted.

const CRC_TABLE: Uint32Array = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    table[n] = c >>> 0;
  }
  return table;
})();

/** IEEE CRC32. Known answer: crc32("123456789") === 0xcbf43926. */
export function crc32(data: Uint8Array): number {
  let crc = 0xffffffff;
  for (let i = 0; i < data.length; i++) {
    crc = CRC_TABLE[(crc ^ data[i]) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

export interface BundleFile {
  path: string;
  content: Uint8Array;
}

const DOS_TIME = 0;
const DOS_DATE = 0x21; // 1980-01-01: ((1980 - 1980) << 9) | (1 << 5) | 1
const VERSION = 20;
const FLAG_UTF8 = 0x0800;

function u16(view: DataView, offset: number, value: number): void {
  view.setUint16(offset, value, true);
}

function u32(view: DataView, offset: number, value: number): void {
  view.setUint32(offset, value, true);
}

/**
 * Build a stored ZIP archive. Entry order is sorted by path, so the output
 * depends only on the file set, never on selection order.
 */
export function buildLaunchBundle(files: BundleFile[]): Uint8Array {
  const encoder = new TextEncoder();
  const sorted = [...files].sort((a, b) =>
    a.path < b.path ? -1 : a.path > b.path ? 1 : 0,
  );
  const names = sorted.map(f => encoder.encode(f.path));
  let localSize = 0;
  for (let i = 0; i < sorted.length; i++) {
    localSize += 30 + names[i].length + sorted[i].content.length;
  }
  let centralSize = 0;
  for (let i = 0; i < sorted.length; i++) centralSize += 46 + names[i].length;
  const total = localSize + centralSize + 22;
  const bytes = new Uint8Array(total);
  const view = new DataView(bytes.buffer);
  let offset = 0;
  const offsets: number[] = [];
  for (let i = 0; i < sorted.length; i++) {
    const name = names[i];
    const content = sorted[i].content;
    const crc = crc32(content);
    offsets.push(offset);
    u32(view, offset, 0x04034b50);
    u16(view, offset + 4, VERSION);
    u16(view, offset + 6, FLAG_UTF8);
    u16(view, offset + 8, 0);
    u16(view, offset + 10, DOS_TIME);
    u16(view, offset + 12, DOS_DATE);
    u32(view, offset + 14, crc);
    u32(view, offset + 18, content.length);
    u32(view, offset + 22, content.length);
    u16(view, offset + 26, name.length);
    u16(view, offset + 28, 0);
    offset += 30;
    bytes.set(name, offset);
    offset += name.length;
    bytes.set(content, offset);
    offset += content.length;
  }
  const centralOffset = offset;
  for (let i = 0; i < sorted.length; i++) {
    const name = names[i];
    const content = sorted[i].content;
    const crc = crc32(content);
    u32(view, offset, 0x02014b50);
    u16(view, offset + 4, VERSION);
    u16(view, offset + 6, VERSION);
    u16(view, offset + 8, FLAG_UTF8);
    u16(view, offset + 10, 0);
    u16(view, offset + 12, DOS_TIME);
    u16(view, offset + 14, DOS_DATE);
    u32(view, offset + 16, crc);
    u32(view, offset + 20, content.length);
    u32(view, offset + 24, content.length);
    u16(view, offset + 28, name.length);
    u16(view, offset + 30, 0);
    u16(view, offset + 32, 0);
    u16(view, offset + 34, 0);
    u16(view, offset + 36, 0);
    u32(view, offset + 38, 0);
    u32(view, offset + 42, offsets[i]);
    offset += 46;
    bytes.set(name, offset);
    offset += name.length;
  }
  u32(view, offset, 0x06054b50);
  u16(view, offset + 4, 0);
  u16(view, offset + 6, 0);
  u16(view, offset + 8, sorted.length);
  u16(view, offset + 10, sorted.length);
  u32(view, offset + 12, centralSize);
  u32(view, offset + 16, centralOffset);
  u16(view, offset + 20, 0);
  return bytes;
}

export interface ManifestArtifact {
  artifactId: string;
  type: string;
  version: number;
  contentHashPrefix: string;
}

/** The bundle's own manifest: what is inside, and where it came from. */
export function buildLaunchManifest(
  artifacts: ManifestArtifact[],
  options: { providerDraft: boolean; providerLabel: string | null },
): string {
  return JSON.stringify(
    {
      format: 'anansi-launch-bundle',
      version: 1,
      generator: options.providerDraft
        ? 'deterministic-rule-based + provider draft'
        : 'deterministic-rule-based',
      providerDraft: options.providerDraft,
      providerLabel: options.providerLabel,
      channels: ['show-hn', 'product-hunt', 'x-thread', 'release-notes'],
      artifacts,
    },
    null,
    2,
  );
}
