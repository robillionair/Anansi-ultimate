import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import crypto from 'node:crypto';

import {
  CONFIGURED_PATHS,
  formatBytes,
  hashBuffer,
  hashFile,
  redactSecrets,
  getDiskSpace,
  getRamInfo,
  checkTestLease,
} from '../../scripts/windows-release/common.mjs';

import {
  evaluatePreflight,
  collectPreflight,
} from '../../scripts/windows-release/preflight.mjs';

import {
  verifyManifest,
} from '../../scripts/windows-release/build-verify.mjs';

import {
  runGuardedLaunch,
  acquireLease,
  releaseLease,
} from '../../scripts/windows-release/launch-acceptance.mjs';

import {
  collectDiagnostics,
  testCanaryRedaction,
} from '../../scripts/windows-release/support-diagnostics.mjs';

test('AC-1.1: Preflight fails closed when C: drive free space is below threshold', () => {
  const { report } = collectPreflight();
  // Artificially demand 100 TiB free space
  const excessiveRequirement = 100 * 1024 * 1024 * 1024 * 1024;
  const evaluation = evaluatePreflight(report, { minFreeCDriveBytes: excessiveRequirement });

  assert.equal(evaluation.ready, false);
  assert.ok(evaluation.failures.some(f => f.includes('Insufficient free space on C: drive')));
});

test('AC-1.1: Preflight fails closed when E: drive free space is below threshold', () => {
  const { report } = collectPreflight();
  const excessiveRequirement = 100 * 1024 * 1024 * 1024 * 1024;
  const evaluation = evaluatePreflight(report, { minFreeEDriveBytes: excessiveRequirement });

  assert.equal(evaluation.ready, false);
  assert.ok(evaluation.failures.some(f => f.includes('Insufficient free space on E: drive')));
});

test('AC-1.1: Preflight fails closed when free RAM is below threshold', () => {
  const { report } = collectPreflight();
  const excessiveRequirement = 128 * 1024 * 1024 * 1024;
  const evaluation = evaluatePreflight(report, { minRamBytes: excessiveRequirement });

  assert.equal(evaluation.ready, false);
  assert.ok(evaluation.failures.some(f => f.includes('Insufficient free RAM')));
});

test('AC-1.3: Preflight warns when application-test lease is occupied', () => {
  const { report } = collectPreflight();
  const mockReport = JSON.parse(JSON.stringify(report));
  mockReport.processSafety.applicationTestLease = {
    occupied: true,
    path: 'C:\\Users\\robin\\.codex\\anansi-application-test.lease.json',
    details: { owner: 'Parallel M8 Integrator', pid: 99999 },
  };

  const evaluation = evaluatePreflight(mockReport);
  assert.ok(evaluation.warnings.some(w => w.includes('Parallel M8 Integrator') && w.includes('99999')));
});

test('AC-1.3: Preflight fails closed when active target executable is running', () => {
  const { report } = collectPreflight();
  const mockReport = JSON.parse(JSON.stringify(report));
  mockReport.processSafety.runningTargetProcesses = [
    { imageName: 'anansi-desktop.exe', pid: 44120 },
  ];

  const evaluation = evaluatePreflight(mockReport);
  assert.equal(evaluation.ready, false);
  assert.ok(evaluation.failures.some(f => f.includes('anansi-desktop.exe (PID 44120)')));
});

test('AC-1.4: Read-only disk inventory proposal matches expected safety schema', () => {
  const proposalPath = path.resolve('docs/reviews/windows-release/cleanup-proposal.json');
  assert.ok(fs.existsSync(proposalPath), 'cleanup-proposal.json must exist');

  const proposal = JSON.parse(fs.readFileSync(proposalPath, 'utf8'));
  assert.equal(typeof proposal.totalReclaimableBytes, 'number');
  assert.equal(typeof proposal.totalPreservedBytes, 'number');
  assert.ok(Array.isArray(proposal.entries));
  assert.ok(proposal.entries.length > 0);

  for (const entry of proposal.entries) {
    assert.ok(entry.path, 'Entry must have path');
    assert.equal(typeof entry.sizeBytes, 'number');
    assert.equal(typeof entry.rebuildable, 'boolean');
    assert.equal(typeof entry.safeToPurge, 'boolean');
    assert.ok(entry.recommendedAction, 'Entry must have recommendedAction');

    // Never purge user preview or git database
    if (entry.category === 'appData' || entry.category === 'gitHistory') {
      assert.equal(entry.safeToPurge, false, `${entry.path} must not be marked safe to purge`);
    }
  }
});

test('AC-2.3: Offline artifact verifier detects missing required files', () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-verif-missing-'));
  try {
    const manifest = {
      schemaVersion: 1,
      artifacts: {
        executable: { filename: 'anansi-desktop.exe', sizeBytes: 100, sha256: 'abcdef12345' },
      },
    };
    const manifestPath = path.join(tempDir, 'release-manifest.json');
    fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));

    const result = verifyManifest(manifestPath);
    assert.equal(result.valid, false);
    assert.ok(result.errors.some(e => e.includes('Missing desktop executable')));
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('AC-2.3: Offline artifact verifier verifies API executable when specified in manifest', () => {
  const tempDir = fs.mkdtempSync('E:/Anansi/tmp/verif-api-');
  try {
    const dummyApi = path.join(tempDir, 'anansi-api.exe');
    fs.writeFileSync(dummyApi, 'dummy api content');
    const stats = fs.statSync(dummyApi);
    const hash = hashFile(dummyApi);
    const dummyDesktop = path.join(tempDir, 'anansi-desktop.exe');
    fs.writeFileSync(dummyDesktop, 'dummy desktop content');
    const dStats = fs.statSync(dummyDesktop);
    const dHash = hashFile(dummyDesktop);

    const manifest = {
      schemaVersion: 1,
      artifacts: {
        executable: { filename: 'anansi-desktop.exe', sizeBytes: dStats.size, sha256: dHash },
        api: { filename: 'anansi-api.exe', sizeBytes: stats.size, sha256: hash },
      },
    };
    const manifestPath = path.join(tempDir, 'release-manifest.json');
    fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));

    const result = verifyManifest(manifestPath);
    assert.equal(result.valid, true);
    assert.equal(result.verifiedCount, 2);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('AC-2.3: Offline artifact verifier detects altered / truncated byte length', () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-verif-size-'));
  try {
    const exePath = path.join(tempDir, 'anansi-desktop.exe');
    fs.writeFileSync(exePath, 'short-content');
    const manifest = {
      schemaVersion: 1,
      artifacts: {
        executable: { filename: 'anansi-desktop.exe', sizeBytes: 99999, sha256: hashFile(exePath) },
      },
    };
    const manifestPath = path.join(tempDir, 'release-manifest.json');
    fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));

    const result = verifyManifest(manifestPath);
    assert.equal(result.valid, false);
    assert.ok(result.errors.some(e => e.includes('Truncated or altered size')));
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('AC-2.3: Offline artifact verifier detects bit flips / SHA-256 mismatches', () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-verif-hash-'));
  try {
    const exePath = path.join(tempDir, 'anansi-desktop.exe');
    fs.writeFileSync(exePath, 'original-content');
    const manifest = {
      schemaVersion: 1,
      artifacts: {
        executable: {
          filename: 'anansi-desktop.exe',
          sizeBytes: 16,
          sha256: '0000000000000000000000000000000000000000000000000000000000000000',
        },
      },
    };
    const manifestPath = path.join(tempDir, 'release-manifest.json');
    fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));

    const result = verifyManifest(manifestPath);
    assert.equal(result.valid, false);
    assert.ok(result.errors.some(e => e.includes('Checksum mismatch')));
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('AC-2.3: Offline artifact verifier warns on unexpected extraneous files', () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-verif-extra-'));
  try {
    const exePath = path.join(tempDir, 'anansi-desktop.exe');
    fs.writeFileSync(exePath, 'real-content');
    const extraPath = path.join(tempDir, 'rogue-file.exe');
    fs.writeFileSync(extraPath, 'malicious');

    const manifest = {
      schemaVersion: 1,
      artifacts: {
        executable: {
          filename: 'anansi-desktop.exe',
          sizeBytes: fs.statSync(exePath).size,
          sha256: hashFile(exePath),
        },
      },
    };
    const manifestPath = path.join(tempDir, 'release-manifest.json');
    fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));

    const result = verifyManifest(manifestPath, { checkUnexpected: true });
    assert.equal(result.valid, true);
    assert.ok(result.warnings.some(w => w.includes('Unexpected extraneous file') && w.includes('rogue-file.exe')));
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('AC-3.1: Guarded launch acceptance fails closed when lease is occupied', async () => {
  const tempLease = path.join(os.tmpdir(), `test-lease-${crypto.randomUUID()}.json`);
  fs.writeFileSync(tempLease, JSON.stringify({ identity: 'test-holder', owner: 'Other Agent', pid: 1234 }));

  try {
    const result = await runGuardedLaunch({ customLeasePath: tempLease });
    assert.equal(result.ok, false);
    assert.equal(result.blockedReason, 'LEASE_OCCUPIED');
  } finally {
    if (fs.existsSync(tempLease)) fs.unlinkSync(tempLease);
  }
});

test('AC-3.1: Guarded launch acceptance rejects executable with mismatched SHA-256', async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-launch-hash-'));
  const dummyExe = path.join(tempDir, 'anansi-desktop.exe');
  fs.writeFileSync(dummyExe, 'dummy-binary-bytes');
  const tempLease = path.join(os.tmpdir(), `test-lease-${crypto.randomUUID()}.json`);

  try {
    const result = await runGuardedLaunch({
      exePath: dummyExe,
      expectedSha256: '1111111111111111111111111111111111111111111111111111111111111111',
      customLeasePath: tempLease,
    });

    assert.equal(result.ok, false);
    assert.equal(result.blockedReason, 'HASH_MISMATCH');
  } finally {
    if (fs.existsSync(tempLease)) fs.unlinkSync(tempLease);
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('AC-3.1: Guarded launch acceptance refuses unsafe synthetic data root on C: drive', async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-launch-badpath-'));
  const dummyExe = path.join(tempDir, 'anansi-desktop.exe');
  fs.writeFileSync(dummyExe, 'dummy-binary-bytes');
  const tempLease = path.join(os.tmpdir(), `test-lease-${crypto.randomUUID()}.json`);

  try {
    const result = await runGuardedLaunch({
      exePath: dummyExe,
      expectedSha256: hashFile(dummyExe),
      tempDir: 'C:\\Users\\robin\\AppData\\Local\\ANANSI-Preview\\test-collision',
      customLeasePath: tempLease,
    });
    assert.equal(result.ok, false);
    assert.equal(result.blockedReason, 'DRIVE_ESCAPE_REFUSED');
  } finally {
    if (fs.existsSync(tempLease)) fs.unlinkSync(tempLease);
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('AC-4.3: Synthetic secret canaries are 100% redacted in diagnostic logs', () => {
  const canaryResult = testCanaryRedaction();
  assert.equal(canaryResult.passed, true);
  assert.equal(canaryResult.failures.length, 0);
});

test('AC-4.1: Support diagnostics excludes private environment variables and databases', () => {
  const diagnostics = collectDiagnostics();
  assert.equal(diagnostics.schemaVersion, 1);
  assert.equal(diagnostics.diagnosticKind, 'anansi-windows-support-diagnostic');

  // Verify allowlisted structure
  assert.ok(diagnostics.build);
  assert.ok(diagnostics.host);
  assert.ok(diagnostics.toolchains);
  assert.ok(diagnostics.privacyNotice);

  // Assert sensitive keys are strictly absent
  const raw = JSON.stringify(diagnostics);
  assert.doesNotMatch(raw, /OPENAI_API_KEY/i);
  assert.doesNotMatch(raw, /ANTHROPIC_API_KEY/i);
  assert.doesNotMatch(raw, /OPENROUTER_API_KEY/i);
  assert.doesNotMatch(raw, /sqlite3-wal/i);
  assert.doesNotMatch(raw, /Authorization:\s*Bearer/i);
});
