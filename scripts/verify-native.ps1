Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
if (-not $IsWindows) { Write-Error 'This verifier requires a Windows MSVC host'; exit 1 }
$command = Get-Command rustc -ErrorAction SilentlyContinue
$rustc = if ($command) { $command.Source } else { $null }
if (-not $rustc) {
    $cargoRoot = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
    $rustc = Join-Path $cargoRoot 'bin/rustc.exe'
}
if (-not (Test-Path -LiteralPath $rustc)) { Write-Error 'Rust compiler not found; install the pinned Rust toolchain first'; exit 1 }
$version = & $rustc -vV
if ($LASTEXITCODE -ne 0 -or ($version -join "`n") -notmatch 'host: .*windows-msvc') { Write-Error 'MSVC Rust host required'; exit 1 }
$fixture = Join-Path ([IO.Path]::GetTempPath()) ('anansi-native-smoke-' + [guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $fixture
$source = Join-Path $fixture 'main.rs'
$binary = Join-Path $fixture 'anansi-native-smoke.exe'
Set-Content -LiteralPath $source -Value 'fn main() { println!("ANANSI_NATIVE_SMOKE_OK"); }' -Encoding utf8
$version | Set-Content -LiteralPath (Join-Path $fixture 'rust-version.log')
& $rustc $source --crate-name anansi_native_smoke --emit=metadata -o (Join-Path $fixture 'smoke.rmeta') *> (Join-Path $fixture 'compile-only.log')
if ($LASTEXITCODE -ne 0) { Write-Output "Compile failed; fixture/logs retained at $fixture"; exit 1 }
& $rustc $source --crate-name anansi_native_smoke -o $binary *> (Join-Path $fixture 'link.log')
if ($LASTEXITCODE -ne 0) {
    Get-Content -LiteralPath (Join-Path $fixture 'link.log')
    Write-Output "MSVC compile/link failed; fixture/logs retained at $fixture"
    exit 1
}
$output = & $binary
if ($LASTEXITCODE -ne 0 -or $output -ne 'ANANSI_NATIVE_SMOKE_OK') { Write-Error "Smoke executable failed; fixture retained at $fixture"; exit 1 }
[ordered]@{ compiled=$true; linked=$true; executed=$true; output=$output; fixture=$fixture; compiler=$rustc; version=$version } | ConvertTo-Json -Depth 3
