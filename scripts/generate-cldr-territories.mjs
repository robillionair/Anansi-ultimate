// Generate the canonical CLDR territory list for the SwarmTank market
// selector.
//
// Source: Unicode CLDR territory names (English), pinned to CLDR 49.0.0:
//   https://raw.githubusercontent.com/unicode-org/cldr-json/49.0.0/cldr-json/cldr-localenames-full/main/en/territories.json
// The pinned source file's SHA-256 is recorded in the output metadata.
// The generated file is committed; the build never needs network access.
//
// Usage:
//   node scripts/generate-cldr-territories.mjs [path-to-cldr-territories.json]
// With no argument the script fetches the pinned URL.
//
// Included: the 2-letter ISO 3166-1 alpha-2 codes (countries and
// territories). Excluded:
//   - 3-digit UN M49 codes (macro-regions such as 001 World, 002 Africa,
//     150 Europe — not countries or territories),
//   - `*-alt-short` / `*-alt-variant` entries (alternate names, not base
//     territory names),
//   - `ZZ` (Unknown Region) and `EU` (European Union — a political union,
//     not a country or territory).
// The output is sorted by territory code.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const CLDR_VERSION = '49';
const SOURCE_URL = 'https://raw.githubusercontent.com/unicode-org/cldr-json/49.0.0/cldr-json/cldr-localenames-full/main/en/territories.json';
const EXCLUDED = new Set(['ZZ', 'EU']);

async function loadSource() {
  const arg = process.argv[2];
  if (arg) return readFileSync(arg);
  const response = await fetch(SOURCE_URL);
  if (!response.ok) throw new Error(`fetch ${SOURCE_URL} -> ${response.status}`);
  return Buffer.from(await response.arrayBuffer());
}

const bytes = await loadSource();
const sourceSha256 = createHash('sha256').update(bytes).digest('hex');
const data = JSON.parse(bytes.toString('utf8'));
const territories = data.main.en.localeDisplayNames.territories;
const entries = Object.entries(territories)
  .filter(([code]) => /^[A-Z]{2}$/.test(code) && !EXCLUDED.has(code))
  .map(([code, name]) => ({ code, name }))
  .sort((a, b) => (a.code < b.code ? -1 : 1));

const out = {
  format: 'anansi-cldr-territories',
  version: 1,
  cldrVersion: CLDR_VERSION,
  sourceUrl: SOURCE_URL,
  sourceSha256,
  generated: new Date().toISOString().slice(0, 10),
  territories: entries,
};
const outDir = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'crates', 'core', 'src', 'markets');
mkdirSync(outDir, { recursive: true });
const outPath = path.join(outDir, 'territories.json');
writeFileSync(outPath, JSON.stringify(out, null, 2) + '\n');
console.log(`wrote ${outPath}: ${entries.length} territories, CLDR ${CLDR_VERSION}, source ${sourceSha256.slice(0, 12)}…`);
