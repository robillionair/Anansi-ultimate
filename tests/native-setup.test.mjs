import assert from 'node:assert/strict';
import path from 'node:path';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
import { root } from '../scripts/validate-docs.mjs';
import { rustCandidates, findWindowsSdk } from '../scripts/doctor.mjs';

test('Rust discovery respects CARGO_HOME without modifying PATH',()=>{
  assert.deepEqual(rustCandidates('cargo','win32','custom-cargo','unused'),['cargo',path.join('custom-cargo','bin','cargo.exe')]);
  assert.deepEqual(rustCandidates('rustc','linux',undefined,'user'),['rustc',path.join('user','.cargo','bin','rustc')]);
});
test('MSVC installer default mode is read-only and explicitly scoped',()=>{
  const shell = spawnSync('pwsh', ['--version']).error ? 'powershell' : 'pwsh';
  const r=spawnSync(shell,['-NoProfile','-ExecutionPolicy','Bypass','-File','scripts/install-msvc.ps1'],{cwd:root,encoding:'utf8'});
  assert.equal(r.status,0,r.stderr || r.error?.message);
  const p=JSON.parse(r.stdout);
  assert.equal(p.mode,'plan');
  assert.equal(p.url,'https://aka.ms/vs/17/release/vs_BuildTools.exe');
  assert.ok(p.arguments.includes('--norestart'));
  assert.ok(p.arguments.includes('Microsoft.VisualStudio.Workload.VCTools'));
  assert.match(p.next,/already elevated/);
});
test('SDK probe falls back from stale registry root to actual headers and libraries',()=>{
  const valid='standard-sdk';
  const names=[['Include','10.0.26100.0','um','Windows.h'],['Include','10.0.26100.0','ucrt','stdio.h'],
    ['Lib','10.0.26100.0','um','x64','kernel32.lib'],['Lib','10.0.26100.0','ucrt','x64','ucrt.lib']];
  const present=new Set(names.map(p=>path.join(valid,...p)));
  const io={readdirSync:p=>{if(p===path.join('stale-sdk','Include')) throw Error('absent');return [{name:'10.0.26100.0',isDirectory:()=>true}];},existsSync:p=>present.has(p)};
  assert.deepEqual(findWindowsSdk(['stale-sdk',valid],io),{root:valid,version:'10.0.26100.0'});
  present.delete(path.join(valid,...names[3]));
  assert.equal(findWindowsSdk([valid],io),null,'SDK without UCRT library must not pass');
});
