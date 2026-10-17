import { spawnSync } from 'node:child_process';
import path from 'node:path';
import os from 'node:os';
import fs from 'node:fs';
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
export function findWindowsSdk(roots, io=fs) {
  for (const root of new Set(roots.filter(r=>typeof r==='string' && r))) {
    let versions;
    try { versions=io.readdirSync(path.join(root,'Include'),{withFileTypes:true}); } catch { continue; }
    for (const version of versions.filter(v=>v.isDirectory())) {
      const files=[['Include',version.name,'um','Windows.h'],['Include',version.name,'ucrt','stdio.h'],
        ['Lib',version.name,'um','x64','kernel32.lib'],['Lib',version.name,'ucrt','x64','ucrt.lib']];
      if (files.every(parts=>io.existsSync(path.join(root,...parts)))) return {root,version:version.name};
    }
  }
  return null;
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
$sdkRoot = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots' -ErrorAction SilentlyContinue).KitsRoot10
$sdkRoot32 = (Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows Kits\Installed Roots' -ErrorAction SilentlyContinue).KitsRoot10
$sdkRoots = @($sdkRoot, $sdkRoot32)
foreach ($base in @([Environment]::GetFolderPath('ProgramFilesX86'), [Environment]::GetFolderPath('ProgramFiles'))) {
  if ($base) { $sdkRoots += Join-Path $base 'Windows Kits/10' }
}
$versions = @()
foreach ($base in @([Environment]::GetFolderPath('ProgramFilesX86'), [Environment]::GetFolderPath('ProgramFiles'), $env:LOCALAPPDATA)) {
  if ($base) { $versions += @(Get-ChildItem -LiteralPath (Join-Path $base 'Microsoft/EdgeWebView/Application') -Directory -ErrorAction SilentlyContinue | Where-Object Name -Match '^\d+\.\d+\.\d+\.\d+$' | Select-Object -ExpandProperty Name) }
}
@{cpp=$cpp; registeredSdkRoot=$sdkRoot; sdkRoots=@($sdkRoots | Where-Object {$_} | Select-Object -Unique); webview=($versions.Count -gt 0); webviewVersions=@($versions | Sort-Object -Unique)} | ConvertTo-Json -Compress
`;
    let result=run('pwsh',['-NoProfile','-NonInteractive','-Command',script]);
    if (!result.ok) result=run('powershell.exe',['-NoProfile','-NonInteractive','-Command',script]);
    try {
      native=JSON.parse(result.output);
      native.sdkEvidence=findWindowsSdk(native.sdkRoots ?? []);
      native.sdk=!!native.sdkEvidence;
    } catch { native={probeError:'Windows prerequisite discovery failed; inspect PowerShell availability and paths'}; }
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
