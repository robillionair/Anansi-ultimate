import fs from 'node:fs';
import { spawn } from 'node:child_process';
const fixture = 'E:/Anansi/tmp/m7-ui-acceptance-S14T39';
const desktop = spawn('E:/Anansi/build-cache/preview-m7/debug/anansi-desktop.exe', [], {
  env: { ...process.env, ANANSI_STORAGE_ROOT: `${fixture}/storage`, ANANSI_VAULT_TEST_PROVIDER: '',
    ANANSI_PROVIDER_TEST_CREDENTIAL: '', WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '', TAURI_CONFIG: '' },
  windowsHide: false, stdio: 'ignore',
});
const completion = `${fixture}/reopen-finish.json`;
console.log(JSON.stringify({ fixture, desktopPid: desktop.pid, completion }));
try {
  const deadline = Date.now() + 30 * 60 * 1000;
  while (!fs.existsSync(completion)) {
    if (Date.now() > deadline) throw Error('Reopen session timed out.');
    if (desktop.exitCode !== null) throw Error('Desktop exited unexpectedly.');
    await new Promise(resolve => setTimeout(resolve, 500));
  }
} finally {
  if (desktop.exitCode === null) await new Promise(resolve => { desktop.once('exit', resolve); desktop.kill(); });
}
