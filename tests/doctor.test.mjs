import assert from 'node:assert/strict';
import test from 'node:test';
import { evaluate } from '../scripts/doctor.mjs';

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
test('Vite Node floor rejects early Node 22 and accepts supported patch lines',()=>{
  assert.equal(evaluate({...complete,node:'v22.0.0'}).foundationReady,false);
  assert.equal(evaluate({...complete,node:'v22.12.0'}).foundationReady,true);
  assert.equal(evaluate({...complete,node:'v20.18.0'}).foundationReady,false);
  assert.equal(evaluate({...complete,node:'v20.19.0'}).foundationReady,true);
});
