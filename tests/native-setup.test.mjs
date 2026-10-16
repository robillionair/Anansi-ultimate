import assert from 'node:assert/strict';
import path from 'node:path';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
import { root } from '../scripts/validate-docs.mjs';
import { rustCandidates } from '../scripts/doctor.mjs';

test('Rust discovery respects CARGO_HOME without modifying PATH',()=>{
  assert.deepEqual(rustCandidates('cargo','win32','custom-cargo','unused'),['cargo',path.join('custom-cargo','bin','cargo.exe')]);
  assert.deepEqual(rustCandidates('rustc','linux',undefined,'user'),['rustc',path.join('user','.cargo','bin','rustc')]);
});
test('MSVC installer default mode is read-only and explicitly scoped',()=>{
  const r=spawnSync('pwsh',['-NoProfile','-File','scripts/install-msvc.ps1'],{cwd:root,encoding:'utf8'});
  assert.equal(r.status,0,r.stderr);
  const p=JSON.parse(r.stdout);
  assert.equal(p.mode,'plan');
  assert.equal(p.url,'https://aka.ms/vs/17/release/vs_BuildTools.exe');
  assert.ok(p.arguments.includes('--norestart'));
  assert.ok(p.arguments.includes('Microsoft.VisualStudio.Workload.VCTools'));
  assert.match(p.next,/already elevated/);
});
