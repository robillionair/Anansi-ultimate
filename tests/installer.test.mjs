import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { createHash } from 'node:crypto';
import { nsisPayloadHash } from '../scripts/installer-artifact.mjs';
import { assertPortsFree } from '../scripts/verify-ports.mjs';
import { createServer } from 'node:net';
test('NSIS identity check accounts for exactly the pinned Tauri bundle marker without mutating source',()=>{
  const source=Buffer.from('prefix__TAURI_BUNDLE_TYPE_VAR_UNKsuffix');
  const expected=createHash('sha256').update('prefix__TAURI_BUNDLE_TYPE_VAR_NSSsuffix').digest('hex');
  assert.equal(nsisPayloadHash(source),expected);assert.equal(source.toString(),'prefix__TAURI_BUNDLE_TYPE_VAR_UNKsuffix');
});
test('unknown or ambiguous bundle markers fail closed',()=>{
  for(const value of ['unknown','__TAURI_BUNDLE_TYPE_VAR_NSS','__TAURI_BUNDLE_TYPE_VAR_UNK__TAURI_BUNDLE_TYPE_VAR_UNK'])assert.throws(()=>nsisPayloadHash(Buffer.from(value)),/Unexpected bundle marker/);
});
test('development packaging stays per-user, unsigned, NSIS-only and retains runtime prerequisites',()=>{
  const config=JSON.parse(fs.readFileSync('apps/desktop/src-tauri/tauri.conf.json','utf8'));
  assert.equal(config.bundle.active,true);assert.deepEqual(config.bundle.targets,['nsis']);assert.equal(config.bundle.windows.nsis.installMode,'currentUser');assert.equal(config.bundle.windows.allowDowngrades,false);assert.equal(config.bundle.windows.webviewInstallMode.type,'embedBootstrapper');
  assert.equal(config.bundle.windows.certificateThumbprint,undefined);assert.equal(config.bundle.createUpdaterArtifacts,undefined);
  const app=JSON.parse(fs.readFileSync('apps/desktop/package.json','utf8'));assert.match(app.scripts.installer,/--bundles nsis.*--no-sign/);assert.match(app.scripts.build,/--no-bundle/);
});
test('UI verifier refuses an occupied port and accepts it after release',async()=>{
  const server=createServer();await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  const port=server.address().port;
  try{await assert.rejects(assertPortsFree([port]),/already in use/);}finally{await new Promise(resolve=>server.close(resolve));}
  await assertPortsFree([port]);
});
