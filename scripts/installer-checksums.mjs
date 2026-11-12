import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { nsisPayloadHash } from './installer-artifact.mjs';
import { collect, toolchainIdentity } from './doctor.mjs';
if(process.platform!=='win32')throw Error('Windows installer checksums require a Windows build.');
const config=JSON.parse(fs.readFileSync('apps/desktop/src-tauri/tauri.conf.json','utf8'));
if(process.env.TAURI_CONFIG||config.app.windows.some(window=>window.additionalBrowserArgs))throw Error('Release artifacts must not contain the native debug test override.');
// Honor an isolated cargo target dir (absolute only) so the checksum
// manifest describes the build that was actually produced.
if(process.env.CARGO_TARGET_DIR&&!path.isAbsolute(process.env.CARGO_TARGET_DIR))throw Error('CARGO_TARGET_DIR must be absolute for release packaging.');
const targetDir=process.env.CARGO_TARGET_DIR||'target';
const directory=path.join(targetDir,'release','bundle','nsis');
const exePath=path.join(targetDir,'release','anansi-desktop.exe');
const filename=`${config.productName}_${config.version}_x64-setup.exe`;
const hash=file=>createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const installerSha256=hash(path.join(directory,filename));
// Tauri patches the payload's bundle marker and restores the source executable.
// Reproduce only that exact pinned-toolchain marker; compare every installed byte.
const applicationSha256=nsisPayloadHash(fs.readFileSync(exePath));
const manifest={schemaVersion:1,kind:'unsigned-development-nsis',architecture:'x64',version:config.version,gitCommit:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),dirtySource:!!execFileSync('git',['status','--porcelain'],{encoding:'utf8'}).trim(),toolchain:toolchainIdentity(collect()),installer:filename,installerSha256,applicationSha256,unbundledApplicationSha256:hash(exePath),bundleMarker:'__TAURI_BUNDLE_TYPE_VAR_NSS',configSha256:hash('apps/desktop/src-tauri/tauri.conf.json'),webviewRequirement:'Existing WebView2, or internet to install the runtime using the embedded Microsoft bootstrapper.'};
fs.writeFileSync(path.join(directory,'SHA256SUMS'),`${installerSha256} *${filename}\n`);
fs.writeFileSync(path.join(directory,'packaging.json'),JSON.stringify(manifest,null,2)+'\n');
console.log('PASS: release NSIS checksum manifest generated (unsigned development artifact).');
