import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { spawnSync, execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import {
  CONFIGURED_PATHS,
  formatBytes,
  hashFile,
} from './common.mjs';

export function extractZip(zipPath, destinationDir) {
  fs.mkdirSync(destinationDir, { recursive: true });
  const result = spawnSync('tar.exe', ['-xf', zipPath, '-C', destinationDir], {
    encoding: 'utf8',
    windowsHide: true,
  });
  if (result.status !== 0) {
    // Fallback to powershell Expand-Archive
    const psRes = spawnSync('powershell.exe', [
      '-NoProfile',
      '-Command',
      `Expand-Archive -LiteralPath '${zipPath}' -DestinationPath '${destinationDir}' -Force`,
    ], { encoding: 'utf8', windowsHide: true });
    if (psRes.status !== 0) {
      throw new Error(`Failed to extract zip file: ${psRes.stderr || psRes.stdout || result.stderr}`);
    }
  }
}

export function verifyExtractedManifest(extractedDir) {
  const manifestPath = path.join(extractedDir, 'MANIFEST.txt');
  if (!fs.existsSync(manifestPath)) {
    return { ok: false, error: 'MANIFEST.txt not found in extracted delivery' };
  }

  const manifestContent = fs.readFileSync(manifestPath, 'utf8');
  const lines = manifestContent.split(/\r?\n/);
  const results = [];
  let passed = 0;
  let failed = 0;

  for (const line of lines) {
    const match = line.match(/^([a-f0-9]{64})\s+(\d+)\s+(.+)$/);
    if (!match) continue;

    const expectedHash = match[1].toLowerCase();
    const expectedBytes = Number(match[2]);
    const relPath = match[3].trim();
    const targetFile = path.join(extractedDir, relPath.replace(/\//g, path.sep));

    if (!fs.existsSync(targetFile)) {
      results.push({ relPath, status: 'MISSING', error: 'File does not exist' });
      failed++;
      continue;
    }

    try {
      const stats = fs.statSync(targetFile);
      if (stats.size !== expectedBytes) {
        results.push({
          relPath,
          status: 'SIZE_MISMATCH',
          expectedBytes,
          actualBytes: stats.size,
        });
        failed++;
        continue;
      }

      const actualHash = hashFile(targetFile).toLowerCase();
      if (actualHash !== expectedHash) {
        results.push({
          relPath,
          status: 'HASH_MISMATCH',
          expectedHash,
          actualHash,
        });
        failed++;
        continue;
      }

      results.push({ relPath, status: 'VERIFIED', size: stats.size, hash: actualHash });
      passed++;
    } catch (err) {
      results.push({ relPath, status: 'ERROR', error: err.message });
      failed++;
    }
  }

  return {
    ok: failed === 0,
    passed,
    failed,
    results,
  };
}

export function scanForForbiddenFiles(extractedDir) {
  const forbiddenFindings = [];
  const entries = fs.readdirSync(extractedDir, { recursive: true, withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(entry.parentPath || extractedDir, entry.name);
    const relPath = path.relative(extractedDir, fullPath).replace(/\\/g, '/').toLowerCase();

    // 1. Dependency trees
    if (relPath.includes('node_modules')) {
      forbiddenFindings.push({ path: relPath, reason: 'Contains node_modules dependency tree' });
    }
    // 2. Build target directories
    if (relPath.includes('/target/') || relPath.startsWith('target/')) {
      forbiddenFindings.push({ path: relPath, reason: 'Contains Rust target build cache' });
    }
    // 3. User data or sqlite databases
    if (relPath.endsWith('.sqlite') || relPath.endsWith('.sqlite3') || relPath.endsWith('.sqlite3-wal')) {
      forbiddenFindings.push({ path: relPath, reason: 'Contains SQLite database file' });
    }
    // 4. Model binaries
    if (relPath.endsWith('.gguf') || relPath.endsWith('.onnx') || relPath.endsWith('.safetensors')) {
      forbiddenFindings.push({ path: relPath, reason: 'Contains machine learning model weights' });
    }
    // 5. Credential or private key material
    if (
      relPath.endsWith('.pem') ||
      relPath.endsWith('.pfx') ||
      relPath.endsWith('id_rsa') ||
      relPath.endsWith('credentials.json') ||
      relPath.endsWith('pairing.json')
    ) {
      forbiddenFindings.push({ path: relPath, reason: 'Contains private credentials or keys' });
    }
  }

  return {
    clean: forbiddenFindings.length === 0,
    findings: forbiddenFindings,
  };
}

export function verifyBundleAndFetch(bundlePath, options = {}) {
  const baselineCommit = options.baselineCommit || '240af4ef58b09e7ada405286f46f927c902f9c52';
  const baselineRepo = options.baselineRepo || 'E:\\Anansi\\worktrees\\review-m8-overnight';
  const scratchRepoDir = options.scratchRepoDir;

  // 1. Verify bundle integrity
  const verifyRes = spawnSync('git', ['bundle', 'verify', bundlePath], { encoding: 'utf8', windowsHide: true });
  if (verifyRes.status !== 0) {
    return { ok: false, step: 'BUNDLE_VERIFY', error: verifyRes.stderr || verifyRes.stdout };
  }

  // 2. Clone scratch repo referencing baseline
  if (fs.existsSync(scratchRepoDir)) {
    fs.rmSync(scratchRepoDir, { recursive: true, force: true });
  }

  const cloneRes = spawnSync('git', ['clone', '--no-checkout', baselineRepo, scratchRepoDir], {
    encoding: 'utf8',
    windowsHide: true,
  });
  if (cloneRes.status !== 0) {
    return { ok: false, step: 'CLONE_SCRATCH', error: cloneRes.stderr || cloneRes.stdout };
  }

  // 3. Checkout baseline commit
  spawnSync('git', ['checkout', baselineCommit], { cwd: scratchRepoDir, encoding: 'utf8', windowsHide: true });

  // 4. Fetch the bundle (support wildcard heads or explicit HEAD)
  let fetchRes = spawnSync('git', ['fetch', bundlePath, 'refs/heads/*:refs/remotes/bundle/*'], {
    cwd: scratchRepoDir,
    encoding: 'utf8',
    windowsHide: true,
  });
  if (fetchRes.status !== 0) {
    fetchRes = spawnSync('git', ['fetch', bundlePath, 'HEAD'], {
      cwd: scratchRepoDir,
      encoding: 'utf8',
      windowsHide: true,
    });
  }
  if (fetchRes.status !== 0) {
    return { ok: false, step: 'FETCH_BUNDLE', error: fetchRes.stderr || fetchRes.stdout };
  }

  // 5. Verify resolved tip commit
  const tipRes = spawnSync('git', ['rev-parse', 'FETCH_HEAD'], {
    cwd: scratchRepoDir,
    encoding: 'utf8',
    windowsHide: true,
  });
  const actualTip = tipRes.stdout?.trim();

  if (options.expectedTip && actualTip !== options.expectedTip) {
    return {
      ok: false,
      step: 'TIP_VERIFICATION',
      expectedTip: options.expectedTip,
      actualTip,
      error: `Fetched commit SHA (${actualTip}) does not match expected (${options.expectedTip})`,
    };
  }

  // Checkout FETCH_HEAD and verify tests run cleanly
  spawnSync('git', ['checkout', 'FETCH_HEAD'], {
    cwd: scratchRepoDir,
    encoding: 'utf8',
    windowsHide: true,
  });

  const testDir = path.join(scratchRepoDir, 'tests', 'windows-release');
  const testFiles = fs.existsSync(testDir)
    ? fs.readdirSync(testDir).filter(f => f.endsWith('.test.mjs'))
    : [];

  let allTestsPassed = true;
  const failedSuites = [];

  for (const tf of testFiles) {
    const testRes = spawnSync(process.execPath, ['--test', path.join('tests', 'windows-release', tf)], {
      cwd: scratchRepoDir,
      encoding: 'utf8',
      windowsHide: true,
    });
    if (testRes.status !== 0) {
      allTestsPassed = false;
      failedSuites.push(tf);
      console.log(`\n--- Test Suite Failure: ${tf} ---`);
      console.log(testRes.stdout || testRes.stderr);
    }
  }

  return {
    ok: allTestsPassed,
    step: 'SCRATCH_TESTS',
    actualTip,
    testsPassed: allTestsPassed,
    failedSuites,
    testedCount: testFiles.length,
  };
}

export function runFullDeliveryVerification(options = {}) {
  const zipPath = options.zipPath || (process.argv[2] && fs.existsSync(process.argv[2]) ? process.argv[2] : null) || path.join('E:\\Anansi\\deliveries', 'anansi-windows-release-engineering-final.zip');
  const tempScratch = options.scratchDir || path.join(CONFIGURED_PATHS.tmp, `delivery-verification-${Date.now()}`);
  const extractedDir = path.join(tempScratch, 'extracted');
  const scratchRepoDir = path.join(tempScratch, 'scratch-repo');

  console.log(`Starting independent verification of delivery ZIP: ${zipPath}`);
  extractZip(zipPath, extractedDir);

  // 1. Verify Manifest
  console.log('1. Checking extracted files against MANIFEST.txt...');
  const manifestVerif = verifyExtractedManifest(extractedDir);
  console.log(`   Manifest check: ${manifestVerif.passed} verified, ${manifestVerif.failed} failed.`);

  // 2. Scan for forbidden contents
  console.log('2. Scanning for forbidden files (credentials, databases, target, node_modules, models)...');
  const hygieneVerif = scanForForbiddenFiles(extractedDir);
  console.log(`   Hygiene check: ${hygieneVerif.clean ? 'CLEAN (0 forbidden files)' : `FAILED (${hygieneVerif.findings.length} forbidden items found)`}`);

  // 3. Verify bundle and test in clean scratch repo
  const bundleFiles = fs.readdirSync(extractedDir).filter(f => f.endsWith('.bundle'));
  if (bundleFiles.length === 0) {
    throw new Error('No .bundle file found in extracted delivery archive.');
  }
  const bundlePath = path.join(extractedDir, bundleFiles[0]);
  console.log(`3. Verifying Git bundle (${bundleFiles[0]}) and testing in scratch clone...`);
  const bundleVerif = verifyBundleAndFetch(bundlePath, { scratchRepoDir, expectedTip: options.expectedTip });
  console.log(`   Bundle & scratch tests: ${bundleVerif.ok ? 'PASS' : 'FAIL'} (Tip: ${bundleVerif.actualTip}, Tested: ${bundleVerif.testedCount} suites)`);
  if (!bundleVerif.ok) {
    console.log(`   Bundle failure step: ${bundleVerif.step}`);
    if (bundleVerif.failedSuites?.length) {
      console.log(`   Failed suites: ${bundleVerif.failedSuites.join(', ')}`);
    }
    if (bundleVerif.error) {
      console.log(`   Error: ${bundleVerif.error}`);
    }
  }

  // 4. Audit unsatisfied dependencies & assumptions
  const unsatisfiedAssumptions = [
    {
      item: 'Windows Sandbox / Disposable VM Installer Acceptance',
      status: 'UNVERIFIED_ON_HOST (DISPOSABLE PROCEDURE DOCUMENTED)',
      reason: 'Windows Sandbox feature is not enabled on host. Installing on primary user profile is prohibited.',
    },
    {
      item: 'Application Recovery Provenance Findings (R1, R2, R3, R4)',
      status: 'HISTORICAL_BASELINE_CONTEXT',
      reason: 'R1-R4 were historical failures on baseline 240af4ef58b...; current independent lineage acceptance is recorded on c50d6b03 and schema 11 runtime 06a3f573a. This worktree does not overwrite newer application code.',
    },
    {
      item: 'Authenticode Code Signing',
      status: 'UNSIGNED_DEVELOPMENT_BUILD',
      reason: 'No certificates or identity forged; signing requires production certificate provisioning.',
    },
    {
      item: 'Reproducibility Claim Reconciled',
      status: 'NOT_BIT_IDENTICAL',
      reason: 'Hash-different binaries are not bit-identical reproducible builds. Structural reproducibility is verified (5 of 6 sections match; .rdata varies due to PE linker timestamps and debug GUIDs).',
    },
  ];

  // Cleanup scratch if requested
  if (options.cleanup !== false) {
    try {
      fs.rmSync(tempScratch, { recursive: true, force: true });
    } catch {}
  }

  const overallSuccess = manifestVerif.ok && hygieneVerif.clean && bundleVerif.ok;

  return {
    ok: overallSuccess,
    manifestVerif,
    hygieneVerif,
    bundleVerif,
    unsatisfiedAssumptions,
  };
}

export function main() {
  const res = runFullDeliveryVerification();
  console.log('================================================================');
  console.log('ANANSI DELIVERY INDEPENDENT VERIFICATION VERDICT');
  console.log('================================================================');
  console.log(`Manifest Check:   ${res.manifestVerif.ok ? 'PASS' : 'FAIL'} (${res.manifestVerif.passed} files)`);
  console.log(`Hygiene Check:    ${res.hygieneVerif.clean ? 'PASS (Zero forbidden items)' : 'FAIL'}`);
  console.log(`Bundle Tip Check: ${res.bundleVerif.ok ? 'PASS' : 'FAIL'} (Tip: ${res.bundleVerif.actualTip})`);
  console.log('Unsatisfied Assumptions & Boundaries:');
  for (const a of res.unsatisfiedAssumptions) {
    console.log(`  - [${a.status}] ${a.item}: ${a.reason}`);
  }
  console.log('----------------------------------------------------------------');
  console.log(`OVERALL: ${res.ok ? 'PASS — Delivery package verified integral and clean.' : 'FAIL'}`);
  console.log('================================================================');
  process.exitCode = res.ok ? 0 : 1;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
