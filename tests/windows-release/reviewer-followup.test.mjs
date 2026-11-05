import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {runGuardedLaunch} from '../../scripts/windows-release/launch-acceptance.mjs';
import {collectDiagnostics} from '../../scripts/windows-release/support-diagnostics.mjs';

test('review follow-up: successful early exit plus unrelated existing file is not application readiness',async()=>{
  const root=fs.mkdtempSync('E:/Anansi/tmp/release-review/early-zero-');
  fs.writeFileSync(path.join(root,'unrelated.txt'),'synthetic unrelated sentinel');
  const artifactsDir=fs.mkdtempSync('E:/Anansi/artifacts/release-review/zero-');
  const result=await runGuardedLaunch({exePath:path.join(process.env.SystemRoot,'System32','whoami.exe'),dataRoot:root,tempDir:'E:/Anansi/tmp/release-review',artifactsDir,smokeDurationMs:500,restart:true});
  assert.equal(result.evidence?.phase1.exitCode,0);
  assert.equal(result.evidence?.phase1.childExitedEarly,true);
  assert.equal(result.ok,false,'early successful exit with unrelated files must not qualify a desktop application');
});

test('review follow-up: diagnostic metrics enforce types and bounds before export',()=>{
  const artifactsDir=fs.mkdtempSync('E:/Anansi/artifacts/release-review/typed-');
  const canary='SYNTHETIC_UNTYPED_PRIVATE_VALUE';
  fs.writeFileSync(path.join(artifactsDir,'launch-evidence.json'),JSON.stringify({launchId:{privateText:canary},timestamp:canary,smokeDurationMs:canary,phase1:{exitCode:canary}}));
  const result=collectDiagnostics({artifactsDir});
  assert.equal(JSON.stringify(result).includes(canary),false,'field names alone do not constrain exported values');
});
