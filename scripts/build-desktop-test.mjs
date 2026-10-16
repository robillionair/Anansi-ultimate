// Build a debug-only UI test artifact. Never used by build:desktop or release packaging.
// Elevated WebView2 hosts ignore user environment flags; pass this test setting via
// Tauri's WebView2 API configuration instead, without changing machine policies.
import fs from 'node:fs';
import { cargo } from './cargo.mjs';
if (process.platform !== 'win32') throw Error('Native UI test build requires Windows.');
const config = JSON.parse(fs.readFileSync('apps/desktop/src-tauri/tauri.conf.json', 'utf8'));
const windows = config.app.windows.map(window => ({
  ...window, additionalBrowserArgs: '--remote-debugging-port=19323 --remote-debugging-address=127.0.0.1',
}));
const result = cargo(['build', '--locked', '-p', 'anansi-desktop', '--features', 'custom-protocol'], {
  stdio: 'inherit', env: { ...process.env, TAURI_CONFIG: JSON.stringify({ app: { windows } }) },
});
process.exitCode = result.status ?? 1;
