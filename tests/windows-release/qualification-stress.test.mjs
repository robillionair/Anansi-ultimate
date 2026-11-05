import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import crypto from 'node:crypto';
import { spawnSync } from 'node:child_process';

import {
  CONFIGURED_PATHS,
  formatBytes,
  hashBuffer,
  hashFile,
  redactSecrets,
  isReparsePoint,
  isPathInside,
  writeJsonAtomic,
} from '../../scripts/windows-release/common.mjs';

import {
  evaluatePreflight,
  collectPreflight,
} from '../../scripts/windows-release/preflight.mjs';

import {
  runGuardedLaunch,
  acquireLease,
  releaseLease,
  preserveFailedRoot,
} from '../../scripts/windows-release/launch-acceptance.mjs';

import {
  collectDiagnostics,
  testCanaryRedaction,
} from '../../scripts/windows-release/support-diagnostics.mjs';

test('AC-7.1: Preflight fails closed on unsupported Node version (< 22.13)', () => {
  const { report } = collectPreflight();
  const mockReport = JSON.parse(JSON.stringify(report));
  mockReport.toolchain.node = {
    version: 'v20.10.0',
    major: 20,
    minor: 10,
    supported: false,
    path: 'C:\\node\\node.exe',
  };

  const evaluation = evaluatePreflight(mockReport);
  assert.equal(evaluation.ready, false);
  assert.ok(evaluation.failures.some(f => f.includes('Node.js version v20.10.0 is unsupported')));
});

test('AC-7.1: Preflight fails closed on unpinned / mismatched pnpm version', () => {
  const { report } = collectPreflight();
  const mockReport = JSON.parse(JSON.stringify(report));
  mockReport.toolchain.pnpm = {
    version: '10.5.0',
    pinned: false,
    path: 'C:\\pnpm\\pnpm.cmd',
  };

  const evaluation = evaluatePreflight(mockReport);
  assert.equal(evaluation.ready, false);
  assert.ok(evaluation.failures.some(f => f.includes('pnpm version mismatch: expected 11.25.0, found 10.5.0')));
});

test('AC-7.1: Preflight fails closed when rustc or cargo toolchain is missing', () => {
  const { report } = collectPreflight();
  const mockReport = JSON.parse(JSON.stringify(report));
  mockReport.toolchain.rustc = { ok: false, output: 'not found' };

  const evaluation = evaluatePreflight(mockReport);
  assert.equal(evaluation.ready, false);
  assert.ok(evaluation.failures.some(f => f.includes('rustc compiler is not available')));
});

test('AC-7.2: Synthetic data root rejects paths inside repository root', async () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-stress-overlap-'));
  const dummyExe = path.join(tempDir, 'dummy.exe');
  fs.writeFileSync(dummyExe, 'binary-bytes');
  const tempLease = path.join(os.tmpdir(), `test-lease-${crypto.randomUUID()}.json`);

  const mockRepo = 'E:\\Anansi\\worktrees\\test-repo';
  const nestedSynthetic = path.join(mockRepo, 'nested-data');

  try {
    const result = await runGuardedLaunch({
      exePath: dummyExe,
      expectedSha256: hashFile(dummyExe),
      dataRoot: nestedSynthetic,
      repoRoot: mockRepo,
      customLeasePath: tempLease,
    });

    assert.equal(result.ok, false);
    assert.equal(result.blockedReason, 'REPO_OVERLAP_REFUSED');
  } finally {
    if (fs.existsSync(tempLease)) fs.unlinkSync(tempLease);
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('AC-7.2: Synthetic data root rejects NTFS directory junctions and symlinks', async () => {
  const baseTestDir = path.join('E:\\Anansi\\tmp\\antigravity-release', `junction-test-${crypto.randomUUID()}`);
  fs.mkdirSync(baseTestDir, { recursive: true });

  const targetDir = path.join(baseTestDir, 'real-target');
  fs.mkdirSync(targetDir, { recursive: true });
  const junctionDir = path.join(baseTestDir, 'junction-link');

  // Create an NTFS directory junction using cmd.exe mklink /J
  const mklinkRes = spawnSync('cmd.exe', ['/d', '/c', 'mklink', '/J', junctionDir, targetDir], { windowsHide: true });
  assert.equal(mklinkRes.status, 0, 'Must create junction for test');

  assert.equal(isReparsePoint(junctionDir), true, 'isReparsePoint must detect junction');

  const dummyExe = path.join(baseTestDir, 'dummy.exe');
  fs.writeFileSync(dummyExe, 'binary-bytes');
  const tempLease = path.join(os.tmpdir(), `test-lease-${crypto.randomUUID()}.json`);

  try {
    const result = await runGuardedLaunch({
      exePath: dummyExe,
      expectedSha256: hashFile(dummyExe),
      dataRoot: junctionDir,
      repoRoot: 'E:\\Anansi\\other-repo',
      customLeasePath: tempLease,
    });

    assert.equal(result.ok, false);
    assert.equal(result.blockedReason, 'REPARSE_POINT_REFUSED');
  } finally {
    if (fs.existsSync(tempLease)) fs.unlinkSync(tempLease);
    // Remove junction via rmdir
    spawnSync('cmd.exe', ['/d', '/c', 'rmdir', junctionDir], { windowsHide: true });
    fs.rmSync(baseTestDir, { recursive: true, force: true });
  }
});

test('AC-7.2: Synthetic paths with spaces and Unicode are handled correctly', () => {
  const baseDir = path.join('E:\\Anansi\\tmp\\antigravity-release', `path space & ⚡ unicode-${crypto.randomUUID()}`);
  try {
    fs.mkdirSync(baseDir, { recursive: true });
    const testFile = path.join(baseDir, 'test-file.json');
    writeJsonAtomic(testFile, { test: 'value', unicode: '⚡✓🔑' });

    assert.ok(fs.existsSync(testFile));
    const content = JSON.parse(fs.readFileSync(testFile, 'utf8'));
    assert.equal(content.test, 'value');
    assert.equal(content.unicode, '⚡✓🔑');
  } finally {
    fs.rmSync(baseDir, { recursive: true, force: true });
  }
});

test('AC-7.2: Read-only destination errors are caught and surfaced cleanly', () => {
  const tempFile = path.join(os.tmpdir(), `ro-test-${crypto.randomUUID()}.json`);
  fs.writeFileSync(tempFile, JSON.stringify({ original: true }), { mode: 0o444 });

  try {
    assert.throws(
      () => {
        // Attempting to overwrite a read-only file atomically
        // On Windows renameSync will fail with EPERM/EACCES if dest is read-only
        writeJsonAtomic(tempFile, { modified: true });
      },
      /EPERM|EACCES/,
    );
  } finally {
    try {
      fs.chmodSync(tempFile, 0o666);
      fs.unlinkSync(tempFile);
    } catch {}
  }
});

test('AC-7.3: Exhaustive redaction across all canary classes in compound text', () => {
  const userProfile = os.homedir();
  const rawLog = `
2026-10-30T16:00:00.123Z [INFO] Initializing service on host
2026-10-30T16:00:00.125Z [DEBUG] Connecting with Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.t-IDcSemACt8x4iTMCda8Yhe3iZaWbvV5XKSTbuAn0M
2026-10-30T16:00:00.126Z [DEBUG] Active OpenAI client with sk-live-test1234567890abcdef1234567890
2026-10-30T16:00:00.127Z [DEBUG] Github push token: ghp_999999999999999999999999999999999999
2026-10-30T16:00:00.128Z [DEBUG] AWS credentials AKIAIOSFODNN7EXAMPLE and aws_secret_access_key="secretKey1234567890"
2026-10-30T16:00:00.129Z [DEBUG] Remote pairing code: pairing_code: "PAIR-54321-LINK"
2026-10-30T16:00:00.130Z [DEBUG] User session: session_token: "sess-9988-7766-5544"
2026-10-30T16:00:00.131Z [DEBUG] DB password: password = "AdminSuperSecretPassword99!"
2026-10-30T16:00:00.132Z [DEBUG] Local cache path: ${path.join(userProfile, '.anansi', 'cache.db')}
2026-10-30T16:00:00.133Z [DEBUG] User prompt: prompt: "My secret financial investment strategy details"
2026-10-30T16:00:00.134Z [DEBUG] Query run: SELECT * FROM sensitive_vault_table WHERE id = 1
2026-10-30T16:00:00.135Z [DEBUG] Custom canary canary_special_injection_xyz
`;

  const redacted = redactSecrets(rawLog, ['canary_special_injection_xyz']);

  // Assert no secrets remain
  assert.doesNotMatch(redacted, /eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9/);
  assert.doesNotMatch(redacted, /sk-live-test1234567890abcdef1234567890/);
  assert.doesNotMatch(redacted, /ghp_999999999999999999999999999999999999/);
  assert.doesNotMatch(redacted, /AKIAIOSFODNN7EXAMPLE/);
  assert.doesNotMatch(redacted, /secretKey1234567890/);
  assert.doesNotMatch(redacted, /PAIR-54321-LINK/);
  assert.doesNotMatch(redacted, /sess-9988-7766-5544/);
  assert.doesNotMatch(redacted, /AdminSuperSecretPassword99!/);
  assert.doesNotMatch(redacted, new RegExp(userProfile.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'i'));
  assert.doesNotMatch(redacted, /My secret financial investment strategy/);
  assert.doesNotMatch(redacted, /canary_special_injection_xyz/);

  // Assert expected placeholders
  assert.ok(redacted.includes('[REDACTED_TOKEN]'));
  assert.ok(redacted.includes('[REDACTED_API_KEY]'));
  assert.ok(redacted.includes('[REDACTED_GITHUB_TOKEN]'));
  assert.ok(redacted.includes('[REDACTED_AWS_KEY]'));
  assert.ok(redacted.includes('[REDACTED_SESSION_TOKEN]'));
  assert.ok(redacted.includes('[REDACTED_CREDENTIAL]'));
  assert.ok(redacted.includes('<USERPROFILE>'));
  assert.ok(redacted.includes('[REDACTED_PROMPT_BODY]'));
  assert.ok(redacted.includes('[REDACTED_SECRET_CANARY]'));
});

test('AC-7.4: Failure root preservation isolates crashed or failed runs', () => {
  const tempBase = 'E:\\Anansi\\tmp\\antigravity-release';
  const testLaunchId = crypto.randomUUID();
  const testDataRoot = path.join(tempBase, `failed-sim-${testLaunchId}`);

  try {
    fs.mkdirSync(testDataRoot, { recursive: true });
    fs.writeFileSync(path.join(testDataRoot, 'crash-dump.log'), 'simulated crash state');

    const preserved = preserveFailedRoot(testDataRoot, tempBase, testLaunchId);
    assert.ok(preserved);
    assert.ok(fs.existsSync(preserved));
    assert.ok(fs.existsSync(path.join(preserved, 'crash-dump.log')));
  } finally {
    fs.rmSync(testDataRoot, { recursive: true, force: true });
    const preservedDir = path.join(tempBase, 'failed-runs', `failed-${testLaunchId}`);
    if (fs.existsSync(preservedDir)) {
      fs.rmSync(preservedDir, { recursive: true, force: true });
    }
  }
});

test('AC-7.6: Launch qualification fails closed on early non-zero exit and readiness failure', async () => {
  const artifactsDir = path.join('E:\\Anansi\\artifacts\\release-review', `fail-launch-${crypto.randomUUID()}`);
  fs.mkdirSync(artifactsDir, { recursive: true });
  const tempLease = path.join(os.tmpdir(), `test-lease-${crypto.randomUUID()}.json`);

  try {
    const result = await runGuardedLaunch({
      exePath: path.join(process.env.SystemRoot, 'System32', 'find.exe'), // exits 2 when no args
      artifactsDir,
      tempDir: 'E:\\Anansi\\tmp\\antigravity-release',
      smokeDurationMs: 150,
      customLeasePath: tempLease,
    });

    assert.equal(result.ok, false);
    assert.equal(result.blockedReason, 'QUALIFICATION_FAILED');
    assert.equal(result.evidence?.phase1.exitCode, 2);
    assert.equal(result.evidence?.readinessVerified, false);
    assert.ok(result.message.includes('exited early with non-zero exit code: 2'));
    assert.ok(result.message.includes('Application readiness failed'));
    assert.ok(fs.existsSync(result.evidencePath));
  } finally {
    if (fs.existsSync(tempLease)) fs.unlinkSync(tempLease);
    fs.rmSync(artifactsDir, { recursive: true, force: true });
  }
});

test('AC-7.7: Diagnostic export allowlist strictly omits arbitrary log content and free-form errors', () => {
  const artifactsDir = path.join('E:\\Anansi\\artifacts\\release-review', `allowlist-test-${crypto.randomUUID()}`);
  fs.mkdirSync(artifactsDir, { recursive: true });

  const randomSecret = 'RANDOM_ARBITRARY_UNFILTERED_TEXT_12345';
  fs.writeFileSync(path.join(artifactsDir, 'build-execution.log'), `Build step error: ${randomSecret}\n`);

  const manifest = {
    schemaVersion: 1,
    artifacts: {
      executable: { filename: 'anansi-desktop.exe', sizeBytes: 100, sha256: 'deadbeef' },
    },
    unallowlistedCustomLeak: randomSecret,
  };
  fs.writeFileSync(path.join(artifactsDir, 'release-manifest.json'), JSON.stringify(manifest, null, 2));

  try {
    const diag = collectDiagnostics({ artifactsDir });
    const serialized = JSON.stringify(diag);

    assert.equal(serialized.includes(randomSecret), false, 'Allowlist must never leak arbitrary text from files');
    assert.equal(diag.buildLog?.present, true);
    assert.equal(typeof diag.buildLog?.sizeBytes, 'number');
    assert.equal(diag.sanitizedBuildLogExcerpt, undefined);
    assert.equal(typeof diag.verification?.valid, 'boolean');
    assert.equal(typeof diag.verification?.errorCount, 'number');
    assert.equal(diag.verification?.errors, undefined);
  } finally {
    fs.rmSync(artifactsDir, { recursive: true, force: true });
  }
});
