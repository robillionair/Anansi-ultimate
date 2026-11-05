import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {runGuardedLaunch} from '../../scripts/windows-release/launch-acceptance.mjs';
import {collectDiagnostics} from '../../scripts/windows-release/support-diagnostics.mjs';
const base = 'E:/Anansi/artifacts/release-review';

test('review: nonzero early exit and absent persistence cannot qualify launch', async () => {
  const artifactsDir = path.join(base, 'launch');
  fs.mkdirSync(artifactsDir, {recursive:true});
  // Existing Windows utility, no generated executable or application data.
  const result = await runGuardedLaunch({
    exePath: path.join(process.env.SystemRoot, 'System32', 'where.exe'),
    artifactsDir, tempDir:'E:/Anansi/tmp/release-review',
    smokeDurationMs:100, restart:true,
  });
  assert.equal(result.evidence?.phase1.exitCode, 2);
  assert.equal(result.evidence?.createdFilesCount, 0);
  assert.equal(result.evidence?.persistenceVerified, false);
  assert.equal(result.ok, false, 'nonzero early exits cannot pass launch qualification');
});

test('review: diagnostic allowlist excludes arbitrary free-form log text', () => {
  const artifactsDir = path.join(base, 'privacy');
  fs.mkdirSync(artifactsDir,{recursive:true});
  const canary='SYNTHETIC_PRIVATE_SENTENCE_DO_NOT_EXPORT';
  fs.writeFileSync(path.join(artifactsDir,'build-execution.log'), `Compiler note: ${canary}\n`);
  const result=collectDiagnostics({artifactsDir});
  assert.equal(JSON.stringify(result).includes(canary), false,
    'privacy cannot depend on recognizing secret syntax or knowing a canary in advance');
});
