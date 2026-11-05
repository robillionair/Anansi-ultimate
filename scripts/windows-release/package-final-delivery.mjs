import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { hashFile } from './common.mjs';

const worktreeDir = process.cwd();
const deliveriesDir = 'E:\\Anansi\\deliveries';
const stageDir = 'E:\\Anansi\\tmp\\antigravity-release\\final-delivery-stage';
const baselineCommit = '240af4ef58b09e7ada405286f46f927c902f9c52';

fs.mkdirSync(deliveriesDir, { recursive: true });
if (fs.existsSync(stageDir)) {
  fs.rmSync(stageDir, { recursive: true, force: true });
}
fs.mkdirSync(stageDir, { recursive: true });

console.log('1. Creating final Git bundle...');
const bundleName = 'anansi-windows-release-engineering-final.bundle';
const bundlePath = path.join(stageDir, bundleName);

const currentBranch = spawnSync('git', ['branch', '--show-current'], { cwd: worktreeDir, encoding: 'utf8' }).stdout.trim() || 'HEAD';

const bundleRes = spawnSync('git', [
  'bundle',
  'create',
  bundlePath,
  `${baselineCommit}..${currentBranch}`,
], { cwd: worktreeDir, encoding: 'utf8', windowsHide: true });

if (bundleRes.status !== 0) {
  throw new Error(`Git bundle creation failed: ${bundleRes.stderr || bundleRes.stdout}`);
}
fs.copyFileSync(bundlePath, path.join(deliveriesDir, bundleName));
console.log(`   Git bundle created at ${bundlePath} and copied to ${deliveriesDir}`);

console.log('2. Staging documentation, scripts, and test suites...');
const filesToStage = [
  // Docs
  { src: 'docs/reviews/windows-release/acceptance-criteria.md', dest: 'docs/acceptance-criteria.md' },
  { src: 'docs/reviews/windows-release/build-acceptance-report.md', dest: 'docs/build-acceptance-report.md' },
  { src: 'docs/reviews/windows-release/cleanup-proposal.json', dest: 'docs/cleanup-proposal.json' },
  { src: 'docs/reviews/windows-release/disk-inventory.md', dest: 'docs/disk-inventory.md' },
  { src: 'docs/reviews/windows-release/independent-review-2026-10-30.md', dest: 'docs/independent-review-2026-10-30.md' },
  { src: 'docs/reviews/windows-release/independent-repair-review-2026-10-30.md', dest: 'docs/independent-repair-review-2026-10-30.md' },
  { src: 'docs/reviews/windows-release/integration-notes.md', dest: 'docs/integration-notes.md' },
  { src: 'docs/reviews/windows-release/operator-guide.md', dest: 'docs/operator-guide.md' },
  { src: 'docs/reviews/windows-release/repeatability-report.md', dest: 'docs/repeatability-report.md' },
  { src: 'docs/reviews/windows-release/windows-sandbox-installer-procedure.md', dest: 'docs/windows-sandbox-installer-procedure.md' },
  // Scripts
  { src: 'scripts/windows-release/build-verify.mjs', dest: 'scripts/build-verify.mjs' },
  { src: 'scripts/windows-release/common.mjs', dest: 'scripts/common.mjs' },
  { src: 'scripts/windows-release/launch-acceptance.mjs', dest: 'scripts/launch-acceptance.mjs' },
  { src: 'scripts/windows-release/package-final-delivery.mjs', dest: 'scripts/package-final-delivery.mjs' },
  { src: 'scripts/windows-release/preflight.mjs', dest: 'scripts/preflight.mjs' },
  { src: 'scripts/windows-release/repeatability-compare.mjs', dest: 'scripts/repeatability-compare.mjs' },
  { src: 'scripts/windows-release/support-diagnostics.mjs', dest: 'scripts/support-diagnostics.mjs' },
  { src: 'scripts/windows-release/verify-delivery.mjs', dest: 'scripts/verify-delivery.mjs' },
  // Tests
  { src: 'tests/windows-release/reviewer-probes.test.mjs', dest: 'tests/reviewer-probes.test.mjs' },
  { src: 'tests/windows-release/reviewer-followup.test.mjs', dest: 'tests/reviewer-followup.test.mjs' },
  { src: 'tests/windows-release/acceptance-criteria.test.mjs', dest: 'tests/acceptance-criteria.test.mjs' },
  { src: 'tests/windows-release/qualification-stress.test.mjs', dest: 'tests/qualification-stress.test.mjs' },
  { src: 'tests/windows-release/repair-verifications.test.mjs', dest: 'tests/repair-verifications.test.mjs' },
];

for (const f of filesToStage) {
  const srcPath = path.join(worktreeDir, f.src);
  const destPath = path.join(stageDir, f.dest);
  fs.mkdirSync(path.dirname(destPath), { recursive: true });
  fs.copyFileSync(srcPath, destPath);
}

console.log('3. Generating MANIFEST.txt...');
const allStagedFiles = [];
function collectStaged(dir, rel = '') {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const ent of entries) {
    const full = path.join(dir, ent.name);
    const subRel = rel ? `${rel}/${ent.name}` : ent.name;
    if (ent.isDirectory()) {
      collectStaged(full, subRel);
    } else if (ent.name !== 'MANIFEST.txt') {
      const stats = fs.statSync(full);
      const sha256 = hashFile(full);
      allStagedFiles.push({
        path: subRel,
        bytes: stats.size,
        sha256,
      });
    }
  }
}
collectStaged(stageDir);

allStagedFiles.sort((a, b) => a.path.localeCompare(b.path));

const manifestLines = allStagedFiles.map(f => `${f.sha256}  ${f.bytes}  ${f.path}`);
const manifestPath = path.join(stageDir, 'MANIFEST.txt');
fs.writeFileSync(manifestPath, manifestLines.join('\n') + '\n', 'utf8');
console.log(`   Manifest generated with ${allStagedFiles.length} entries.`);

console.log('4. Creating delivery ZIP archive...');
const zipName = 'anansi-windows-release-engineering-final.zip';
const zipPath = path.join(deliveriesDir, zipName);
if (fs.existsSync(zipPath)) fs.unlinkSync(zipPath);

// Create ZIP using tar or PowerShell
const tarRes = spawnSync('tar.exe', ['-cf', zipPath, '-C', stageDir, '.'], { encoding: 'utf8', windowsHide: true });
if (tarRes.status !== 0) {
  const psRes = spawnSync('powershell.exe', [
    '-NoProfile',
    '-Command',
    `Compress-Archive -Path '${stageDir}\\*' -DestinationPath '${zipPath}' -Force`,
  ], { encoding: 'utf8', windowsHide: true });
  if (psRes.status !== 0) {
    throw new Error(`Failed to create ZIP: ${psRes.stderr || psRes.stdout || tarRes.stderr}`);
  }
}

const zipSha256 = hashFile(zipPath);
const shaPath = `${zipPath}.sha256`;
fs.writeFileSync(shaPath, `${zipSha256} *${zipName}\n`, 'utf8');

console.log('================================================================');
console.log('FINAL DELIVERY PACKAGE READY');
console.log('================================================================');
console.log(`ZIP Path:   ${zipPath}`);
console.log(`SHA-256:    ${zipSha256}`);
console.log(`Checksum:   ${shaPath}`);
console.log(`Bundle Tip: ${spawnSync('git', ['rev-parse', 'HEAD'], { cwd: worktreeDir, encoding: 'utf8' }).stdout.trim()}`);
console.log('================================================================');
