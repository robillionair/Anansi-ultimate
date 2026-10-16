[CmdletBinding()]
param([switch]$SkipUi)
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
if (-not $IsWindows) { throw 'Installer verification requires Windows PowerShell 7.' }
$repository=Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repository
$config=Get-Content -LiteralPath 'apps/desktop/src-tauri/tauri.conf.json' -Raw | ConvertFrom-Json
if ($config.bundle.windows.nsis.installMode -ne 'currentUser' -or $config.productName -ne 'ANANSI Ultimate' -or $config.bundle.publisher -ne 'Robillionair') { throw 'Unexpected installer identity or privileges.' }
$identity=[Security.Principal.WindowsIdentity]::GetCurrent()
$principal=[Security.Principal.WindowsPrincipal]::new($identity)
if (-not $SkipUi -and $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Installed release UI verification needs a non-elevated host; CI must explicitly select -SkipUi and run its separate native UI checks.' }
$uninstallKeys=@('HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ANANSI Ultimate','HKCU:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\ANANSI Ultimate','HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ANANSI Ultimate','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\ANANSI Ultimate')
$productKey='HKCU:\Software\Robillionair\ANANSI Ultimate'
foreach ($key in ($uninstallKeys+@($productKey))) { if(Test-Path -LiteralPath $key){throw 'An existing ANANSI installation or remembered location is present; no installer was run.'} }
$installer=Join-Path $repository ("target/release/bundle/nsis/ANANSI Ultimate_"+$config.version+'_x64-setup.exe')
$manifest=Get-Content -LiteralPath 'target/release/bundle/nsis/packaging.json' -Raw | ConvertFrom-Json
$installerHash=(Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash.ToLowerInvariant()
if ($installerHash -ne $manifest.installerSha256) { throw 'Installer checksum mismatch.' }
if ($manifest.kind -ne 'unsigned-development-nsis' -or $manifest.installer -ne [IO.Path]::GetFileName($installer)) { throw 'Unexpected installer manifest.' }
$signature=(Get-AuthenticodeSignature -LiteralPath $installer).Status.ToString()
if($signature -ne 'NotSigned'){throw 'Expected the explicitly unsigned development installer.'}
$tempBase=[IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$testRoot=Join-Path $tempBase ('anansi-installer-'+[Guid]::NewGuid().ToString('N'))
$installRoot=Join-Path $testRoot 'installed application'
if($testRoot.StartsWith($repository+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Installer fixtures must stay outside source.'}
foreach($target in @($testRoot,$installRoot)){
  if(-not [IO.Path]::GetFullPath($target).StartsWith($tempBase,[StringComparison]::OrdinalIgnoreCase)){throw 'Test path escaped the temporary root.'}
  $cursor=$target
  while($cursor){if(Test-Path -LiteralPath (Join-Path $cursor '.git')){throw 'Installer fixtures cannot use Git ancestry.'};if(Test-Path -LiteralPath $cursor){if((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint){throw 'Linked test path refused.'}};$parent=[IO.Path]::GetDirectoryName($cursor);if($parent -eq $cursor){break};$cursor=$parent}
}
if(Test-Path -LiteralPath $testRoot){throw 'Test directory already exists.'}
New-Item -ItemType Directory -Path $testRoot | Out-Null
$installedExe=Join-Path $installRoot 'anansi-desktop.exe'
$uninstaller=Join-Path $installRoot 'uninstall.exe'
$record=[ordered]@{schemaVersion=1;installerSha256=$installerHash;signature=$signature;installPassed=$false;installedBinaryMatches=$false;uiWorkflowPassed=$false;uninstallPassed=$false;dataPreserved=$false;uiSkipped=[bool]$SkipUi}
$previousVerifierRoot=$env:ANANSI_VERIFIER_ROOT
try{
  # NSIS requires /D to be the final, unquoted remainder (including spaces).
  $process=Start-Process -FilePath $installer -ArgumentList @('/S','/NS',('/D='+$installRoot)) -Wait -PassThru -WindowStyle Hidden
  if($process.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $installedExe)){throw 'Installer failed; synthetic test location retained.'}
  $record.installPassed=$true
  if((Get-FileHash -LiteralPath $installedExe -Algorithm SHA256).Hash.ToLowerInvariant() -ne $manifest.applicationSha256){throw 'Installed application does not match the built release.'}
  $record.installedBinaryMatches=$true
  $registration=Get-ItemProperty -LiteralPath $uninstallKeys[0]
  if($registration.InstallLocation.Trim('"').TrimEnd('\') -ne $installRoot.TrimEnd('\')){throw 'Installer registration points outside its synthetic target.'}
  if(-not $SkipUi){
    $env:ANANSI_VERIFIER_ROOT=$testRoot
    & node scripts/verify-workspace.mjs $installedExe
    if($LASTEXITCODE -ne 0){throw 'Installed native/browser workflow failed.'}
    $record.uiWorkflowPassed=$true
    $workflows=@(Get-ChildItem -LiteralPath $testRoot -Directory | Where-Object Name -like 'anansi-cross-adapter-*')
    if($workflows.Count -ne 1){throw 'Unexpected synthetic workflow root.'}
    $dataRoot=Join-Path $workflows[0].FullName 'storage'
    if(-not (Test-Path -LiteralPath (Join-Path $dataRoot 'workspaces.sqlite3'))){throw 'Workflow did not produce a real workspace catalog.'}
  }else{
    $dataRoot=Join-Path $testRoot 'synthetic data'
    New-Item -ItemType Directory -Path $dataRoot | Out-Null
    Set-Content -LiteralPath (Join-Path $dataRoot 'preserved.txt') -Value 'CI data preservation sentinel; UI workflow explicitly skipped.'
  }
  $before=@(Get-ChildItem -LiteralPath $dataRoot -File -Force | Sort-Object Name | ForEach-Object { $_.Name+':'+(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash })
  # Run only the uninstaller produced in this fresh, verified installation.
  if(-not [IO.Path]::GetFullPath($uninstaller).StartsWith($testRoot+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Uninstaller escaped its generated target.'}
  $process=Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait -PassThru -WindowStyle Hidden
  if($process.ExitCode -ne 0){throw 'Synthetic uninstall failed.'}
  for($attempt=0;$attempt -lt 100 -and ((Test-Path -LiteralPath $installedExe) -or (Test-Path -LiteralPath $uninstaller) -or (Test-Path -LiteralPath $uninstallKeys[0]));$attempt++){Start-Sleep -Milliseconds 200}
  if((Test-Path -LiteralPath $installedExe) -or (Test-Path -LiteralPath $uninstaller) -or (Test-Path -LiteralPath $uninstallKeys[0])){throw 'Application, uninstaller or registration remains after uninstall.'}
  $record.uninstallPassed=$true
  $after=@(Get-ChildItem -LiteralPath $dataRoot -File -Force | Sort-Object Name | ForEach-Object { $_.Name+':'+(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash })
  if(($before -join "`n") -ne ($after -join "`n") -or $before.Count -eq 0){throw 'Uninstall modified synthetic workspace data.'}
  $record.dataPreserved=$true
  # NSIS remembers its last location. Remove only the test-owned product key,
  # originally absent, and only when it still points to our generated directory.
  if(Test-Path -LiteralPath $productKey){
    if((Get-Item -LiteralPath $productKey).GetValue('') -ne $installRoot){throw 'Remembered location changed; leave it untouched.'}
    Remove-Item -LiteralPath $productKey -Recurse
  }
}finally{
  $env:ANANSI_VERIFIER_ROOT=$previousVerifierRoot
  $record | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $testRoot 'installer-verification.json')
  $artifactRecord=Join-Path $repository 'target/release/bundle/nsis/installer-verification.json'
  $record | ConvertTo-Json | Set-Content -LiteralPath $artifactRecord
  Write-Output ('Synthetic installer evidence retained: '+$testRoot)
}
if($SkipUi){Write-Output 'PASS: release package install/binary identity/uninstall/data-sentinel preservation. Installed UI explicitly NOT tested by this CI mode.'}
else{Write-Output 'PASS: installed release onboarding/settings/restart/native-browser workflow and uninstall with byte-identical preserved catalog.'}
