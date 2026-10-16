import { spawnSync } from 'node:child_process';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';

export function evaluate(report) {
  const missing = [];
  if (!report.git?.ok) missing.push('Git');
  const [major, minor] = (report.node ?? '').replace(/^v/,'').split('.').map(Number);
  if (!(major > 22 || major === 22 && minor >= 13)) missing.push('Node >=22.13 for pinned pnpm and Vite');
  if (report.pnpm !== '11.25.0') missing.push('pnpm 11.25.0');
  const nativeMissing = [];
  if (report.platform !== 'win32') nativeMissing.push('Windows build host (this probe does not assess Linux/macOS Tauri prerequisites)');
  if (!report.rust?.ok || !/host: .*windows-msvc/.test(report.rust.output ?? '')) nativeMissing.push('Rust MSVC host toolchain');
  if (!report.cargo?.ok) nativeMissing.push('Cargo');
  if (!report.native?.cpp) nativeMissing.push('Visual Studio C++ build tools');
  if (!report.native?.sdk) nativeMissing.push('Windows SDK headers and x64 libraries');
  if (!report.native?.webview) nativeMissing.push('WebView2 installation evidence');
  return {
    foundationReady: missing.length === 0,
    windowsPrerequisitesDetected: missing.length === 0 && nativeMissing.length === 0,
    missing, nativeMissing,
    buildVerified: false,
    note: 'Read-only prerequisite discovery; compiling, linking and launching the application must still be verified. No installation was attempted.',
  };
}
function run(command, args) {
  const result = spawnSync(command,args,{encoding:'utf8',timeout:20000,windowsHide:true});
  return {ok:result.status===0,output:result.stdout?.trim() || null};
}
export function rustCandidates(tool, platform, cargoHome, userHome) {
  return [tool, path.join(cargoHome || path.join(userHome,'.cargo'),'bin',tool + (platform==='win32' ? '.exe' : ''))];
}
function runRust(tool,args) {
  for (const candidate of rustCandidates(tool,process.platform,process.env.CARGO_HOME,os.homedir())) {
    const result=run(candidate,args);
    if (result.ok) return {...result,command:candidate};
  }
  return {ok:false,output:null};
}
export function collect() {
  let pnpm = process.env.npm_config_user_agent?.match(/pnpm\/([^\s]+)/)?.[1];
  if (!pnpm) pnpm = (process.platform === 'win32'
    ? run('cmd.exe',['/d','/c','pnpm --version']) : run('pnpm',['--version'])).output;
  let native = null;
  if (process.platform === 'win32') {
    const script = String.raw`
$cpp = $false
$vswhere = Join-Path ([Environment]::GetFolderPath('ProgramFilesX86')) 'Microsoft Visual Studio/Installer/vswhere.exe'
if (Test-Path -LiteralPath $vswhere) {
  $instances = & $vswhere -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
  $cpp = $LASTEXITCODE -eq 0 -and [bool]($instances | Select-Object -First 1)
}
$sdk = $false
$sdkRoot = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots' -ErrorAction SilentlyContinue).KitsRoot10
if ($sdkRoot) {
  foreach ($version in @(Get-ChildItem -LiteralPath (Join-Path $sdkRoot 'Include') -Directory -ErrorAction SilentlyContinue)) {
    if ((Test-Path -LiteralPath (Join-Path $version.FullName 'um/Windows.h')) -and
        (Test-Path -LiteralPath (Join-Path $sdkRoot ('Lib/' + $version.Name + '/um/x64/kernel32.lib')))) { $sdk = $true }
  }
}
$versions = @()
foreach ($base in @([Environment]::GetFolderPath('ProgramFilesX86'), [Environment]::GetFolderPath('ProgramFiles'), $env:LOCALAPPDATA)) {
  if ($base) { $versions += @(Get-ChildItem -LiteralPath (Join-Path $base 'Microsoft/EdgeWebView/Application') -Directory -ErrorAction SilentlyContinue | Where-Object Name -Match '^\d+\.\d+\.\d+\.\d+$' | Select-Object -ExpandProperty Name) }
}
@{cpp=$cpp; sdk=$sdk; sdkRoot=$sdkRoot; webview=($versions.Count -gt 0); webviewVersions=@($versions | Sort-Object -Unique)} | ConvertTo-Json -Compress
`;
    const result=run('pwsh',['-NoProfile','-NonInteractive','-Command',script]);
    try { native=JSON.parse(result.output); } catch { native={probeError:'Windows prerequisite discovery failed; inspect PowerShell availability and paths'}; }
  }
  return {platform:process.platform,node:process.version,pnpm,git:run('git',['--version']),rust:runRust('rustc',['-vV']),cargo:runRust('cargo',['--version']),native};
}
export function main() {
  const report=collect();
  const readiness=evaluate(report);
  console.log(JSON.stringify({report,readiness},null,2));
  process.exitCode=(process.argv.includes('--require-native') ? readiness.windowsPrerequisitesDetected : readiness.foundationReady) ? 0 : 1;
}
if (process.argv[1] && path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) main();
