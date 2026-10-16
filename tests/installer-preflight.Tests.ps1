Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
if(-not $IsWindows){throw 'Installer preflight tests require Windows PowerShell 7.'}
$repository=Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repository
$manifestPath=Join-Path $repository 'target/release/bundle/nsis/packaging.json'
$original=[IO.File]::ReadAllBytes($manifestPath)
$key='HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ANANSI Ultimate'
if(Test-Path -LiteralPath $key){throw 'A real or pending installation exists; preflight tests refused without modification.'}
$shell=(Get-Process -Id $PID).Path
function Assert-Refusal([string]$Expected){
  $output=& $shell -NoProfile -File scripts/verify-installer.ps1 -SkipUi 2>&1 | Out-String
  if($LASTEXITCODE -eq 0 -or $output -notmatch $Expected){throw 'Expected installer preflight refusal was not observed.'}
}
try{
  $manifest=[Text.Encoding]::UTF8.GetString($original) | ConvertFrom-Json
  $manifest.installerSha256='0'*64
  [IO.File]::WriteAllText($manifestPath,($manifest | ConvertTo-Json))
  Assert-Refusal 'Installer checksum mismatch'
}finally{[IO.File]::WriteAllBytes($manifestPath,$original)}
Write-Output 'PASS: altered installer checksum refused before installation.'
$sentinel=[Guid]::NewGuid().ToString('N')
try{
  New-Item -Path $key | Out-Null
  New-ItemProperty -LiteralPath $key -Name 'AnansiSyntheticTest' -Value $sentinel | Out-Null
  Assert-Refusal 'existing ANANSI installation'
}finally{
  if(Test-Path -LiteralPath $key){
    $properties=Get-ItemProperty -LiteralPath $key
    if($properties.AnansiSyntheticTest -ne $sentinel){throw 'Synthetic registration changed; preserve it for inspection.'}
    Remove-Item -LiteralPath $key
  }
}
Write-Output 'PASS: existing installation refused without replacement.'
