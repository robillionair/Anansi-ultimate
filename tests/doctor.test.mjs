import assert from 'node:assert/strict';
import test from 'node:test';
import { evaluate, toolchainIdentity } from '../scripts/doctor.mjs';

const complete = {
  platform:'win32',node:'v24.19.0',pnpm:'11.25.0',git:{ok:true},
  rust:{ok:true,output:'rustc synthetic\nhost: x86_64-pc-windows-msvc'},
  cargo:{ok:true},native:{cpp:true,sdk:true,webview:true},
};
test('detected prerequisites never imply a verified native build',()=>{
  const actual=evaluate(complete);
  assert.equal(actual.foundationReady,true);
  assert.equal(actual.windowsPrerequisitesDetected,true);
  assert.equal(actual.buildVerified,false);
});
test('missing native tools preserve independent foundation readiness',()=>{
  const actual=evaluate({...complete,rust:{ok:false},cargo:{ok:false},native:{}});
  assert.equal(actual.foundationReady,true);
  assert.equal(actual.windowsPrerequisitesDetected,false);
  for (const name of ['Rust MSVC','Cargo','C++','SDK','WebView2']) assert.ok(actual.nativeMissing.some(m=>m.includes(name)),name);
});
test('wrong package manager and obsolete Node fail foundation readiness',()=>{
  const actual=evaluate({...complete,pnpm:'10.0.0',node:'v18.0.0',git:{ok:false}});
  assert.equal(actual.foundationReady,false);
  assert.equal(actual.missing.length,3);
});
test('GNU toolchain and other platforms do not masquerade as Windows MSVC readiness',()=>{
  const actual=evaluate({...complete,platform:'linux',rust:{ok:true,output:'host: x86_64-unknown-linux-gnu'}});
  assert.equal(actual.foundationReady,true);
  assert.equal(actual.windowsPrerequisitesDetected,false);
  assert.equal(actual.nativeMissing.length,2);
});
test('registered SDK without headers/libraries and directory-only WebView evidence stay distinct',()=>{
  const actual=evaluate({...complete,native:{cpp:true,sdk:false,webview:true}});
  assert.deepEqual(actual.nativeMissing,['Windows SDK headers and x64 libraries']);
  assert.equal(actual.buildVerified,false);
});
test('combined pnpm/Vite Node floor enforces the stricter package-manager minimum',()=>{
  assert.equal(evaluate({...complete,node:'v22.0.0'}).foundationReady,false);
  assert.equal(evaluate({...complete,node:'v22.12.0'}).foundationReady,false);
  assert.equal(evaluate({...complete,node:'v22.13.0'}).foundationReady,true);
  assert.equal(evaluate({...complete,node:'v20.18.0'}).foundationReady,false);
  assert.equal(evaluate({...complete,node:'v20.19.0'}).foundationReady,false);
});
test('toolchain identity records versions only, never paths or usernames',()=>{
  const identity=toolchainIdentity({
    platform:'win32',node:'v26.7.0',pnpm:'11.25.0',
    git:{ok:true,output:'git version 2.53.0.windows.2'},
    rust:{ok:true,output:'rustc 1.99.1 (58cf8f9c3 2026-10-13)\nbinary: rustc\ncommit-hash: 58cf8f9c35b355199727627d07ae34a4e2846d67\ncommit-date: 2026-10-13\nhost: x86_64-pc-windows-msvc\nrelease: 1.99.1',command:'C:\\Users\\robin\\.cargo\\bin\\rustc.exe'},
    cargo:{ok:true,output:'cargo 1.99.1 (5f94df478 2026-08-27)',command:'C:\\Users\\robin\\.cargo\\bin\\cargo.exe'},
    native:{cpp:true,sdk:true,webview:true,sdkEvidence:{root:'C:\\Program Files (x86)\\Windows Kits\\10\\',version:'10.0.26100.0'},webviewVersions:['157.0.4341.42','157.0.4341.46'],sdkRoots:['C:\\Program Files\\Windows Kits\\10\\']},
  });
  assert.deepEqual(identity,{node:'v26.7.0',pnpm:'11.25.0',git:'git version 2.53.0.windows.2',rustc:'rustc 1.99.1 (58cf8f9c3 2026-10-13)',rustcHost:'x86_64-pc-windows-msvc',rustcCommit:'58cf8f9c35b355199727627d07ae34a4e2846d67',cargo:'cargo 1.99.1 (5f94df478 2026-08-27)',windowsSdk:'10.0.26100.0',webviewVersions:['157.0.4341.42','157.0.4341.46']});
  const serialized=JSON.stringify(identity);
  for(const leak of ["robin","Users","Program Files","C:"])assert.ok(!serialized.includes(leak),leak);
});
test('toolchain identity degrades to nulls when probes are missing',()=>{
  assert.deepEqual(toolchainIdentity({}),{node:null,pnpm:null,git:null,rustc:null,rustcHost:null,rustcCommit:null,cargo:null,windowsSdk:null,webviewVersions:[]});
  assert.deepEqual(toolchainIdentity({node:'v24.0.0',native:{probeError:'probe failed'}}).webviewVersions,[]);
});
