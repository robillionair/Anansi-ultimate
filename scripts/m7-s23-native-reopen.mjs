// Resume native inspection of the retained synthetic fixture; no inference.
import fs from 'node:fs';
import path from 'node:path';
import { spawn } from 'node:child_process';
const fixture = process.argv[2];
if (!fixture || !fs.existsSync(path.join(fixture, 'restore.result.json'))) throw Error('Supply a restored fixture');
const finish = path.join(fixture, 'reopen.finish.json');
if (fs.existsSync(finish)) throw Error('Reopen already completed');
const child = spawn('E:\\Anansi\\build-cache\\preview-m7-slices2-3\\debug\\anansi-desktop.exe', [], {
  env: { ...process.env, ANANSI_STORAGE_ROOT: path.join(fixture, 'storage'),
    ANANSI_VAULT_TEST_PROVIDER: '', ANANSI_PROVIDER_TEST_CREDENTIAL: '',
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '', TAURI_CONFIG: '' }, stdio: 'ignore', windowsHide: false,
});
child.once('error', e => { throw e; });
console.log(JSON.stringify({ fixture, desktopPid: child.pid, finish }));
try {
  const deadline = Date.now() + 30 * 60 * 1000;
  while (!fs.existsSync(finish)) {
    if (Date.now() > deadline) throw Error('Reopen inspection timed out');
    if (child.exitCode !== null) throw Error('Owned desktop exited');
    await new Promise(resolve => setTimeout(resolve, 500));
  }
} finally {
  if (child.exitCode === null) await new Promise(resolve => { child.once('exit', resolve); child.kill(); });
}
