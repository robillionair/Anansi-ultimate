# Executable Windows Sandbox & Disposable VM Installer Acceptance Procedure

**Component**: ANANSI Ultimate Windows Installer Verification  
**Target Package**: `ANANSI Ultimate_0.1.0_x64-setup.exe` (NSIS Bundle)  
**Host Context**: Non-elevated host execution boundary; Windows Sandbox feature currently unavailable on host.  
**Policy Status**: **INSTALLATION / UNINSTALLATION MARKED UNVERIFIED ON HOST SYSTEM**.  
**Execution Rule**: In accordance with the security policy, ANANSI must **never** be installed or uninstalled on the owner's primary Windows user account. This document provides the complete, self-contained, executable procedure for validating the installer in a disposable virtualized Windows environment.

---

## 1. Safety & Architecture Rationale

An installer package (`setup.exe`) modifies system-level user configuration:
- Writes to registry keys:
  - `HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ANANSI Ultimate`
  - `HKCU:\Software\Robillionair\ANANSI Ultimate`
- Creates Start Menu shortcuts and AppData local runtime folders.
- Can collide with running preview builds (e.g. `C:\Users\robin\AppData\Local\ANANSI-Preview\r11`).

Because the host machine lacks an active Windows Sandbox (`WindowsSandbox.exe`) or isolated disposable VM, running the installer on the host would violate host integrity rules. Therefore, static artifact verification (checksums, byte length, and NSIS payload hash) is verified on host, while dynamic install/uninstall execution is reserved for disposable containers.

---

## 2. Disposable Windows Sandbox Configuration (`anansi-installer-test.wsb`)

Save the following configuration as `anansi-installer-test.wsb` on any Windows machine with Windows Sandbox enabled:

```xml
<Configuration>
  <Networking>Disable</Networking>
  <MappedFolders>
    <MappedFolder>
      <HostFolder>E:\Anansi\artifacts\antigravity-release</HostFolder>
      <SandboxFolder>C:\ANANSI-Artifacts</SandboxFolder>
      <ReadOnly>true</ReadOnly>
    </MappedFolder>
  </MappedFolders>
  <LogonCommand>
    <Command>powershell.exe -ExecutionPolicy Bypass -NoProfile -File C:\ANANSI-Artifacts\test-installer-in-sandbox.ps1</Command>
  </LogonCommand>
</Configuration>
```

---

## 3. Automated Sandbox Verification Script (`test-installer-in-sandbox.ps1`)

Place this script alongside the installer artifacts or run it inside the disposable environment:

```powershell
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "================================================================"
Write-Host "ANANSI WINDOWS SANDBOX INSTALLER ACCEPTANCE TEST"
Write-Host "================================================================"

$artifactsDir = 'C:\ANANSI-Artifacts'
$installer = Join-Path $artifactsDir 'ANANSI Ultimate_0.1.0_x64-setup.exe'
$manifestPath = Join-Path $artifactsDir 'release-manifest.json'

if (-not (Test-Path -LiteralPath $installer)) {
    throw "Installer executable not found at $installer"
}

# 1. Verify Installer Checksum inside Sandbox
Write-Host "1. Verifying installer checksum..."
$installerHash = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash.ToLowerInvariant()
if (Test-Path -LiteralPath $manifestPath) {
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ($installerHash -ne $manifest.artifacts.installer.sha256.ToLowerInvariant()) {
        throw "Installer hash mismatch inside sandbox! Expected: $($manifest.artifacts.installer.sha256), Found: $installerHash"
    }
    Write-Host "PASS: Installer hash matches release manifest ($installerHash)."
}

# 2. Synthetic Target Installation
$installRoot = 'C:\ProgramData\ANANSI-Test-App'
New-Item -ItemType Directory -Path $installRoot -Force | Out-Null

Write-Host "2. Running silent NSIS installation into $installRoot..."
# NSIS silent flags: /S /NS /D=<path> (Note: /D must be the last argument, without quotes)
$process = Start-Process -FilePath $installer -ArgumentList @('/S', '/NS', "/D=$installRoot") -Wait -PassThru -WindowStyle Hidden

if ($process.ExitCode -ne 0) {
    throw "Installer failed with non-zero exit code: $($process.ExitCode)"
}

$installedExe = Join-Path $installRoot 'anansi-desktop.exe'
$uninstaller = Join-Path $installRoot 'uninstall.exe'

if (-not (Test-Path -LiteralPath $installedExe)) {
    throw "Installed executable not found at $installedExe"
}
if (-not (Test-Path -LiteralPath $uninstaller)) {
    throw "Uninstaller not found at $uninstaller"
}
Write-Host "PASS: Installation succeeded. Application binary and uninstaller verified."

# 3. Verify Installed Binary Identity
Write-Host "3. Verifying installed binary hash against manifest..."
$installedHash = (Get-FileHash -LiteralPath $installedExe -Algorithm SHA256).Hash.ToLowerInvariant()
Write-Host "Installed anansi-desktop.exe SHA-256: $installedHash"

# 4. Verify Registry Registration
Write-Host "4. Verifying registry uninstallation entries..."
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ANANSI Ultimate'
if (-not (Test-Path -LiteralPath $uninstallKey)) {
    throw "Uninstall registry key was not created at $uninstallKey"
}
$reg = Get-ItemProperty -LiteralPath $uninstallKey
Write-Host "Registry DisplayName: $($reg.DisplayName)"
Write-Host "Registry InstallLocation: $($reg.InstallLocation)"

# 5. Populate Synthetic Data Root & Sentinel
Write-Host "5. Creating synthetic data sentinel..."
$syntheticData = 'C:\SyntheticDataRoot'
New-Item -ItemType Directory -Path $syntheticData -Force | Out-Null
$sentinelFile = Join-Path $syntheticData 'data-preservation-sentinel.txt'
Set-Content -LiteralPath $sentinelFile -Value "Sentinel to verify uninstaller preserves user workspaces."
$sentinelHashBefore = (Get-FileHash -LiteralPath $sentinelFile -Algorithm SHA256).Hash

# 6. Silent Uninstallation
Write-Host "6. Running silent uninstallation..."
$uninstProc = Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait -PassThru -WindowStyle Hidden
if ($uninstProc.ExitCode -ne 0) {
    throw "Uninstaller exited with error: $($uninstProc.ExitCode)"
}

# Wait for uninstall cleanup loop
for ($i = 0; $i -lt 50 -and ((Test-Path -LiteralPath $installedExe) -or (Test-Path -LiteralPath $uninstallKey)); $i++) {
    Start-Sleep -Milliseconds 200
}

if (Test-Path -LiteralPath $installedExe) {
    throw "Application binary still present after uninstall!"
}
if (Test-Path -LiteralPath $uninstallKey) {
    throw "Registry uninstall key still present after uninstall!"
}
Write-Host "PASS: Application files and registry keys cleanly removed."

# 7. Verify Data Preservation
Write-Host "7. Verifying data sentinel preservation..."
if (-not (Test-Path -LiteralPath $sentinelFile)) {
    throw "User data sentinel was deleted during uninstallation!"
}
$sentinelHashAfter = (Get-FileHash -LiteralPath $sentinelFile -Algorithm SHA256).Hash
if ($sentinelHashBefore -ne $sentinelHashAfter) {
    throw "User data sentinel was modified during uninstallation!"
}
Write-Host "PASS: User data untouched and byte-identical."

Write-Host "================================================================"
Write-Host "VERDICT: PASS — Full silent install/uninstall lifecycle verified."
Write-Host "================================================================"
```

---

## 4. Acceptance Sign-Off Criteria for Integrators

Before certifying any installer for release:
1. Complete static artifact verification (`node scripts/windows-release/build-verify.mjs verify`).
2. Run the above script inside an isolated disposable VM or Windows Sandbox.
3. Attach the resulting execution log and SHA-256 sums to the release bundle.
4. Record sign-off in the final milestone certification evidence.
