import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { nsisPayloadHash } from './installer-artifact.mjs';
if(process.platform!=='win32')throw Error('Windows installer checksums require a Windows build.');
const config=JSON.parse(fs.readFileSync('apps/desktop/src-tauri/tauri.conf.json','utf8'));
if(process.env.TAURI_CONFIG||config.app.windows.some(window=>window.additionalBrowserArgs))throw Error('Release artifacts must not contain the native debug test override.');
const directory='target/release/bundle/nsis';
const filename=`${config.productName}_${config.version}_x64-setup.exe`;
const hash=file=>createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const installerSha256=hash(path.join(directory,filename));
// Tauri patches the payload's bundle marker and restores the source executable.
// Reproduce only that exact pinned-toolchain marker; compare every installed byte.
const applicationSha256=nsisPayloadHash(fs.readFileSync('target/release/anansi-desktop.exe'));
const manifest={schemaVersion:1,kind:'unsigned-development-nsis',architecture:'x64',version:config.version,gitCommit:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),dirtySource:!!execFileSync('git',['status','--porcelain'],{encoding:'utf8'}).trim(),installer:filename,installerSha256,applicationSha256,unbundledApplicationSha256:hash('target/release/anansi-desktop.exe'),bundleMarker:'__TAURI_BUNDLE_TYPE_VAR_NSS',configSha256:hash('apps/desktop/src-tauri/tauri.conf.json'),webviewRequirement:'Existing WebView2, or internet to install the runtime using the embedded Microsoft bootstrapper.'};
fs.writeFileSync(path.join(directory,'SHA256SUMS'),`${installerSha256} *${filename}\n`);
fs.writeFileSync(path.join(directory,'packaging.json'),JSON.stringify(manifest,null,2)+'\n');
console.log('PASS: release NSIS checksum manifest generated (unsigned development artifact).');
