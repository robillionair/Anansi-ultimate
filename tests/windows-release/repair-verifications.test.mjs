import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import crypto from 'node:crypto';
import { spawn } from 'node:child_process';
import { DatabaseSync } from 'node:sqlite';

import {
  runGuardedLaunch,
  verifyApplicationReadiness,
  verifyApplicationPersistence,
  ensureProcessTreeTerminated,
  setProcessTreeEnumeratorForTesting,
  enumerateProcessTree,
  checkSqliteCatalogIntegrity,
  verifyAnansiCatalogContract,
  initSyntheticAnansiCatalog,
  ANANSI_STORAGE_CONTRACT,
  killProcessTree,
  isProcessRunning,
} from '../../scripts/windows-release/launch-acceptance.mjs';

import {
  collectDiagnostics,
  validateString,
  validateNumber,
  validateBoolean,
  validateUuid,
  validateIsoDate,
} from '../../scripts/windows-release/support-diagnostics.mjs';

function safeCleanDir(dirPath) {
  try {
    fs.rmSync(dirPath, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  } catch {}
}

test('WR-01b failure-path: truncated or invalid SQLite header is rejected during readiness', () => {
  const root = fs.mkdtempSync('E:/Anansi/tmp/release-review/trunc-sqlite-');
  try {
    const dbPath = path.join(root, 'workspaces.sqlite3');
    fs.writeFileSync(dbPath, 'CorruptSQLiteHdr'); // 16 bytes, but not 'SQLite format 3'
    const result = verifyApplicationReadiness(root, []);
    assert.equal(result.ready, false);
    assert.ok(result.reason.includes('invalid SQLite header'));
  } finally {
    safeCleanDir(root);
  }
});

test('WR-01b failure-path: Phase 1 artifact truncated across restart is rejected during persistence', () => {
  const root = fs.mkdtempSync('E:/Anansi/tmp/release-review/trunc-restart-');
  try {
    const dbPath = path.join(root, 'workspaces.sqlite3');
    fs.writeFileSync(dbPath, ''); // 0 bytes
    const result = verifyApplicationPersistence(root, ['workspaces.sqlite3']);
    assert.equal(result.persistent, false);
    assert.ok(result.reason.includes('truncated to 0 bytes'));
  } finally {
    safeCleanDir(root);
  }
});

test('WR-01b failure-path: missing phase 1 artifact across restart fails persistence', () => {
  const root = fs.mkdtempSync('E:/Anansi/tmp/release-review/missing-artifact-');
  try {
    const result = verifyApplicationPersistence(root, ['.anansi-root']);
    assert.equal(result.persistent, false);
    assert.ok(result.reason.includes('missing after application restart'));
  } finally {
    safeCleanDir(root);
  }
});

test('WR-02b validator unit tests: type and bound constraints', () => {
  // Strings
  assert.equal(validateString(12345), null);
  assert.equal(validateString({ leaked: 'secret' }), null);
  assert.equal(validateString(' valid ', { maxLength: 10 }), 'valid');
  assert.equal(validateString('a'.repeat(300), { maxLength: 256 }), null);

  // Numbers
  assert.equal(validateNumber('not a number'), null);
  assert.equal(validateNumber(NaN), null);
  assert.equal(validateNumber(Infinity), null);
  assert.equal(validateNumber(-5, { min: 0 }), null);
  assert.equal(validateNumber(1000000, { max: 500 }), null);
  assert.equal(validateNumber(42, { min: 0, max: 100 }), 42);
  assert.equal(validateNumber(3.14, { integerOnly: true }), null);
  assert.equal(validateNumber(3.14, { integerOnly: false }), 3.14);

  // UUIDs
  assert.equal(validateUuid('not-a-uuid'), null);
  assert.equal(validateUuid({ text: 'uuid' }), null);
  assert.equal(validateUuid('12345678-1234-1234-1234-123456789abc'), '12345678-1234-1234-1234-123456789abc');

  // ISO Dates
  assert.equal(validateIsoDate('invalid-date'), null);
  assert.equal(validateIsoDate('2026-10-30T19:00:00.000Z'), '2026-10-30T19:00:00.000Z');
  assert.equal(validateIsoDate(123456789), null);
});

test('WR-02b failure-path: diagnostic collection rejects hostile deeply nested objects in all fields', () => {
  const artifactsDir = fs.mkdtempSync('E:/Anansi/artifacts/release-review/hostile-diag-');
  try {
    const maliciousEvidence = {
      launchId: { evil: 'nested_token_1' },
      timestamp: [1, 2, 3],
      smokeDurationMs: { overflow: 99999999999999 },
      restartTested: 'should_be_bool',
      createdFilesCount: 'five',
      phase1: { exitCode: { nested: 'string' } },
      phase2: { exitCode: 'crash' },
    };
    fs.writeFileSync(path.join(artifactsDir, 'launch-evidence.json'), JSON.stringify(maliciousEvidence));

    const diag = collectDiagnostics({ artifactsDir });
    assert.equal(diag.launchEvidence.launchId, null);
    assert.equal(diag.launchEvidence.timestamp, null);
    assert.equal(diag.launchEvidence.smokeDurationMs, null);
    assert.equal(diag.launchEvidence.restartTested, null);
    assert.equal(diag.launchEvidence.createdFilesCount, null);
    assert.equal(diag.launchEvidence.phase1ExitCode, null);
    assert.equal(diag.launchEvidence.phase2ExitCode, null);
  } finally {
    fs.rmSync(artifactsDir, { recursive: true, force: true });
  }
});

test('WR-01c: padded malformed SQLite file fails integrity check and readiness', () => {
  const root = fs.mkdtempSync('E:/Anansi/tmp/release-review/padded-malformed-');
  try {
    const dbPath = path.join(root, 'workspaces.sqlite3');
    // 1024 bytes: starts with SQLite header but body is non-database garbage
    const garbage = Buffer.concat([Buffer.from('SQLite format 3\0'), Buffer.alloc(1008, 0x58)]);
    fs.writeFileSync(dbPath, garbage);
    const integrity = checkSqliteCatalogIntegrity(dbPath);
    assert.equal(integrity.ok, false);
    const readiness = verifyApplicationReadiness(root, []);
    assert.equal(readiness.ready, false);
  } finally {
    safeCleanDir(root);
  }
});

test('WR-01c: SQLite database with zero schema/tables is rejected', () => {
  const root = fs.mkdtempSync('E:/Anansi/tmp/release-review/empty-schema-');
  try {
    const dbPath = path.join(root, 'workspaces.sqlite3');
    const db = new DatabaseSync(dbPath);
    db.exec('PRAGMA user_version = 0;');
    db.close();
    const integrity = checkSqliteCatalogIntegrity(dbPath);
    assert.equal(integrity.ok, false);
    assert.ok(integrity.reason.includes('contains no application schema') || integrity.reason.includes('truncated'));
  } finally {
    safeCleanDir(root);
  }
});

test('WR-01c: valid SQLite database with schema passes readiness and persistence', () => {
  const root = fs.mkdtempSync('E:/Anansi/tmp/release-review/valid-schema-');
  try {
    const dbPath = path.join(root, 'workspaces.sqlite3');
    initSyntheticAnansiCatalog(dbPath, {
      userVersion: 10,
      workspaces: [{ id: 'ws-1', name: 'Test Workspace', name_key: 'test-workspace' }],
    });

    const integrity = checkSqliteCatalogIntegrity(dbPath);
    assert.equal(integrity.ok, true);
    assert.ok(integrity.tables.includes('workspaces'));

    const readiness = verifyApplicationReadiness(root, []);
    assert.equal(readiness.ready, true);

    const persistence = verifyApplicationPersistence(root, ['workspaces.sqlite3']);
    assert.equal(persistence.persistent, true);
  } finally {
    safeCleanDir(root);
  }
});

test('WR-01c: lone pairing.json without core catalog or root is rejected', () => {
  const root = fs.mkdtempSync('E:/Anansi/tmp/release-review/lone-pairing-');
  try {
    fs.writeFileSync(path.join(root, 'pairing.json'), JSON.stringify({ token: 'abc' }));
    const readiness = verifyApplicationReadiness(root, []);
    assert.equal(readiness.ready, false);
    assert.ok(readiness.reason.includes('No recognized ANANSI storage artifacts') || readiness.reason.includes('Storage contract not established'));
  } finally {
    safeCleanDir(root);
  }
});

test('WR-01d: marker alone with valid content refuses application readiness with initializationOnly', () => {
  const root = fs.mkdtempSync('E:/Anansi/tmp/release-review/marker-content-only-');
  try {
    fs.writeFileSync(path.join(root, '.anansi-root'), 'ANANSI workspace root v1\n');
    const readiness = verifyApplicationReadiness(root, []);
    assert.equal(readiness.ready, false);
    assert.equal(readiness.initializationOnly, true);
    assert.ok(readiness.reason.includes('root marker alone is initialization-only'));
  } finally {
    safeCleanDir(root);
  }
});

test('WR-01d: unrelated valid SQLite database refuses readiness and persistence', () => {
  const root = fs.mkdtempSync('E:/Anansi/tmp/release-review/unrelated-db-');
  try {
    const dbPath = path.join(root, 'workspaces.sqlite3');
    const db = new DatabaseSync(dbPath);
    db.exec("CREATE TABLE custom_notes (id INTEGER PRIMARY KEY, note TEXT); INSERT INTO custom_notes VALUES (1, 'Hello');");
    db.close();

    const readiness = verifyApplicationReadiness(root, []);
    assert.equal(readiness.ready, false);
    assert.ok(readiness.reason.includes('ANANSI storage contract violated') || readiness.reason.includes('missing required ANANSI tables'));

    const persistence = verifyApplicationPersistence(root, ['workspaces.sqlite3']);
    assert.equal(persistence.persistent, false);
    assert.ok(persistence.reason.includes('ANANSI storage contract failed') || persistence.reason.includes('missing required ANANSI tables'));
  } finally {
    safeCleanDir(root);
  }
});

test('WR-01d: valid ANANSI catalog with supported schema and tables passes readiness and persistence', () => {
  const root = fs.mkdtempSync('E:/Anansi/tmp/release-review/contract-valid-');
  try {
    const dbPath = path.join(root, 'workspaces.sqlite3');
    initSyntheticAnansiCatalog(dbPath, {
      userVersion: 10,
      workspaces: [{ id: 'ws-synth-1', name: 'Synthetic Workspace 1', name_key: 'synth-1' }],
    });

    const readiness = verifyApplicationReadiness(root, []);
    assert.equal(readiness.ready, true);
    assert.equal(readiness.catalogSnapshot.recordCount, 1);
    assert.equal(readiness.catalogSnapshot.workspaces[0].id, 'ws-synth-1');

    const persistence = verifyApplicationPersistence(root, ['workspaces.sqlite3'], {
      baselineSnapshot: readiness.catalogSnapshot,
      expectedWorkspace: { id: 'ws-synth-1', name: 'Synthetic Workspace 1' },
    });
    assert.equal(persistence.persistent, true);
  } finally {
    safeCleanDir(root);
  }
});

test('WR-01d: deleted workspace record in structurally valid SQLite database fails persistence across restart', () => {
  const root = fs.mkdtempSync('E:/Anansi/tmp/release-review/contract-deleted-');
  try {
    const dbPath = path.join(root, 'workspaces.sqlite3');
    initSyntheticAnansiCatalog(dbPath, {
      userVersion: 10,
      workspaces: [
        { id: 'ws-alpha', name: 'Alpha Workspace', name_key: 'alpha' },
        { id: 'ws-beta', name: 'Beta Workspace', name_key: 'beta' },
      ],
    });

    const readiness = verifyApplicationReadiness(root, []);
    assert.equal(readiness.ready, true);
    assert.equal(readiness.catalogSnapshot.recordCount, 2);

    // Delete ws-beta while preserving valid SQLite structure
    const db = new DatabaseSync(dbPath);
    db.exec("DELETE FROM workspace_settings WHERE workspace_id = 'ws-beta'; DELETE FROM workspaces WHERE id = 'ws-beta';");
    db.close();

    // Check integrity - structurally valid SQLite!
    const integrity = checkSqliteCatalogIntegrity(dbPath);
    assert.equal(integrity.ok, true, 'database remains structurally valid SQLite');

    // Persistence across restart must detect the deleted record
    const persistence = verifyApplicationPersistence(root, ['workspaces.sqlite3'], {
      baselineSnapshot: readiness.catalogSnapshot,
    });
    assert.equal(persistence.persistent, false);
    assert.ok(persistence.reason.includes("Workspace record deleted across restart: 'ws-beta'"));
  } finally {
    safeCleanDir(root);
  }
});

test('WR-01d: modified/tampered workspace record in structurally valid SQLite database fails persistence across restart', () => {
  const root = fs.mkdtempSync('E:/Anansi/tmp/release-review/contract-modified-');
  try {
    const dbPath = path.join(root, 'workspaces.sqlite3');
    initSyntheticAnansiCatalog(dbPath, {
      userVersion: 10,
      workspaces: [{ id: 'ws-alpha', name: 'Original Name', name_key: 'alpha' }],
    });

    const readiness = verifyApplicationReadiness(root, []);
    assert.equal(readiness.ready, true);

    // Tamper with record value while preserving valid SQLite structure
    const db = new DatabaseSync(dbPath);
    db.exec("UPDATE workspaces SET name = 'Tampered Name' WHERE id = 'ws-alpha';");
    db.close();

    const integrity = checkSqliteCatalogIntegrity(dbPath);
    assert.equal(integrity.ok, true, 'database remains structurally valid SQLite');

    const persistence = verifyApplicationPersistence(root, ['workspaces.sqlite3'], {
      baselineSnapshot: readiness.catalogSnapshot,
    });
    assert.equal(persistence.persistent, false);
    assert.ok(persistence.reason.includes("Workspace record 'ws-alpha' name changed across restart"));
  } finally {
    safeCleanDir(root);
  }
});

test('WR-01d: replaced workspace record in structurally valid SQLite database fails persistence across restart', () => {
  const root = fs.mkdtempSync('E:/Anansi/tmp/release-review/contract-replaced-');
  try {
    const dbPath = path.join(root, 'workspaces.sqlite3');
    initSyntheticAnansiCatalog(dbPath, {
      userVersion: 10,
      workspaces: [{ id: 'ws-alpha', name: 'Alpha', name_key: 'alpha' }],
    });

    const readiness = verifyApplicationReadiness(root, []);
    assert.equal(readiness.ready, true);

    // Replace record with different ID
    const db = new DatabaseSync(dbPath);
    db.exec("DELETE FROM workspace_settings; DELETE FROM workspaces; INSERT INTO workspaces (id, name, name_key) VALUES ('ws-impostor', 'Alpha', 'alpha'); INSERT INTO workspace_settings (workspace_id, reduced_motion) VALUES ('ws-impostor', 0);");
    db.close();

    const persistence = verifyApplicationPersistence(root, ['workspaces.sqlite3'], {
      baselineSnapshot: readiness.catalogSnapshot,
    });
    assert.equal(persistence.persistent, false);
    assert.ok(persistence.reason.includes("Workspace record deleted across restart: 'ws-alpha'"));
  } finally {
    safeCleanDir(root);
  }
});

test('WR-01d: expectedWorkspace identity and value verification matches or refuses', () => {
  const root = fs.mkdtempSync('E:/Anansi/tmp/release-review/contract-expected-');
  try {
    const dbPath = path.join(root, 'workspaces.sqlite3');
    initSyntheticAnansiCatalog(dbPath, {
      userVersion: 10,
      workspaces: [{ id: 'ws-verified', name: 'Verified Workspace', name_key: 'verified' }],
    });

    // 1. Matching expected workspace passes
    const passResult = verifyApplicationPersistence(root, ['workspaces.sqlite3'], {
      expectedWorkspace: { id: 'ws-verified', name: 'Verified Workspace' },
    });
    assert.equal(passResult.persistent, true);

    // 2. Missing expected workspace ID fails
    const failIdResult = verifyApplicationPersistence(root, ['workspaces.sqlite3'], {
      expectedWorkspace: { id: 'ws-nonexistent', name: 'Verified Workspace' },
    });
    assert.equal(failIdResult.persistent, false);
    assert.ok(failIdResult.reason.includes("Expected synthetic workspace 'ws-nonexistent' not found"));

    // 3. Mismatched expected workspace value fails
    const failValResult = verifyApplicationPersistence(root, ['workspaces.sqlite3'], {
      expectedWorkspace: { id: 'ws-verified', name: 'Wrong Value' },
    });
    assert.equal(failValResult.persistent, false);
    assert.ok(failValResult.reason.includes("Workspace 'ws-verified' name mismatch"));
  } finally {
    safeCleanDir(root);
  }
});

test('Cleanup: deterministic process enumeration failure causes unconfirmed teardown and lease retention', async () => {
  const tempLease = path.join(os.tmpdir(), `test-enum-fail-lease-${crypto.randomUUID()}.json`);
  const artifactsDir = fs.mkdtempSync('E:/Anansi/artifacts/release-review/enum-fail-');

  // Inject deterministic enumeration failure
  setProcessTreeEnumeratorForTesting(() => ({
    ok: false,
    error: 'Simulated deterministic WMI enumeration timeout',
    pids: [],
  }));

  try {
    const termRes = await ensureProcessTreeTerminated(99999999, 500);
    assert.equal(termRes.confirmed, false);
    assert.ok(termRes.reason.includes('Process enumeration failed'));

    // Verify guarded launch retains the lease
    const launchRes = await runGuardedLaunch({
      exePath: path.join(process.env.SystemRoot, 'System32', 'whoami.exe'),
      customLeasePath: tempLease,
      artifactsDir,
      tempDir: 'E:/Anansi/tmp/release-review',
      smokeDurationMs: 100,
    });

    assert.equal(launchRes.ok, false);
    // Lease MUST be retained on disk because cleanup could not be confirmed!
    assert.ok(fs.existsSync(tempLease), 'Lease file must be retained on disk when cleanup is unconfirmed');
    const leaseData = JSON.parse(fs.readFileSync(tempLease, 'utf8'));
    assert.equal(leaseData.blockedReason, 'TEARDOWN_CONFIRMATION_FAILED');
  } finally {
    setProcessTreeEnumeratorForTesting(null);
    if (fs.existsSync(tempLease)) fs.unlinkSync(tempLease);
    fs.rmSync(artifactsDir, { recursive: true, force: true });
  }
});

test('Cleanup: grandchild process tree coverage and confirmed termination', async () => {
  const grandchildScript = 'setInterval(() => {}, 1000);';
  const childScript = `const { spawn } = require('node:child_process'); spawn(process.execPath, ['-e', ${JSON.stringify(grandchildScript)}]); setInterval(() => {}, 1000);`;
  const parent = spawn(process.execPath, ['-e', childScript], { windowsHide: true });
  const parentPid = parent.pid;

  try {
    // Wait for process tree to establish (parent -> child node -> grandchild node)
    await new Promise(r => setTimeout(r, 1500));

    const enumRes = enumerateProcessTree(parentPid);
    assert.equal(enumRes.ok, true);
    assert.ok(enumRes.pids.length >= 2, `Should find at least child and grandchild, found: ${enumRes.pids.join(', ')}`);

    // Terminate the entire process tree
    const termRes = await ensureProcessTreeTerminated(parentPid, 5000);
    assert.equal(termRes.confirmed, true);

    // Assert that neither parent nor any descendant is running
    assert.equal(isProcessRunning(parentPid), false, 'Parent must be terminated');
    for (const dPid of enumRes.pids) {
      assert.equal(isProcessRunning(dPid), false, `Descendant PID ${dPid} must be terminated`);
    }
  } finally {
    killProcessTree(parentPid);
  }
});

test('Cleanup: parent-first-exit coverage terminates detached descendants', async () => {
  const parent = spawn('cmd.exe', [
    '/c',
    'start /b powershell.exe -NoProfile -Command "Start-Sleep -Seconds 30"',
  ], { windowsHide: true });
  const parentPid = parent.pid;

  let childPid = null;
  const start = Date.now();
  while (Date.now() - start < 1500) {
    const enumRes = enumerateProcessTree(parentPid);
    if (enumRes.ok && enumRes.pids.length > 0) {
      childPid = enumRes.pids[0];
      break;
    }
    await new Promise(r => setTimeout(r, 100));
  }

  try {
    await new Promise(r => setTimeout(r, 500));
    assert.equal(isProcessRunning(parentPid), false, 'Parent should have exited');

    if (childPid) {
      assert.equal(isProcessRunning(childPid), true, 'Child should still be running after parent exit');
      const termRes = await ensureProcessTreeTerminated(parentPid, 5000, {
        knownPids: [childPid],
      });
      assert.equal(termRes.confirmed, true);
      assert.equal(isProcessRunning(childPid), false, 'Child must be terminated');
    }
  } finally {
    if (childPid) killProcessTree(childPid);
  }
});
