[CmdletBinding()]
param([switch]$Install)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$url = 'https://aka.ms/vs/17/release/vs_BuildTools.exe'
$arguments = @('--quiet','--wait','--norestart','--nocache','--add','Microsoft.VisualStudio.Workload.VCTools','--includeRecommended')
$isWin = if (Get-Variable -Name IsWindows -ErrorAction SilentlyContinue) { $IsWindows } else { $env:OS -eq 'Windows_NT' }
$administrator = $false
if ($isWin) {
    $principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
    $administrator = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}
if (-not $Install) {
    [ordered]@{ mode='plan'; administrator=$administrator; url=$url; arguments=$arguments; next='Run this script with -Install in an already elevated PowerShell. No self-elevation or restart is performed.' } | ConvertTo-Json -Depth 3
    exit 0
}
if (-not $isWin -or -not $administrator) { Write-Error 'Installation requires an already elevated Windows PowerShell; nothing downloaded or installed'; exit 1 }
$directory = Join-Path ([IO.Path]::GetTempPath()) ('anansi-msvc-setup-' + [guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $directory
$installer = Join-Path $directory 'vs_BuildTools.exe'
Invoke-WebRequest -Uri $url -OutFile $installer
$signature = Get-AuthenticodeSignature -LiteralPath $installer
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') { throw "Microsoft publisher verification failed; bootstrapper retained at $directory" }
$record = [ordered]@{ url=$url; sha256=(Get-FileHash -LiteralPath $installer).Hash; publisher=$signature.SignerCertificate.Subject; arguments=$arguments }
$record | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath (Join-Path $directory 'installer-record.json')
$process = Start-Process -FilePath $installer -ArgumentList $arguments -Wait -PassThru -WindowStyle Hidden
if ($process.ExitCode -notin @(0,3010)) { throw "MSVC installer failed with exit $($process.ExitCode); record at $directory" }
Write-Output "MSVC setup returned $($process.ExitCode); no restart initiated. Run pnpm doctor:native and pnpm verify:native to verify. Record: $directory"
if ($process.ExitCode -eq 3010) { Write-Warning 'Installer requests a restart; do not count native readiness as verified until checks pass after user-managed restart' }
