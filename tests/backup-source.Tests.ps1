Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$scriptPath = Join-Path (Split-Path -Parent $PSScriptRoot) 'scripts/backup-source.ps1'
$fixture = Join-Path ([IO.Path]::GetTempPath()) ('anansi-backup-tests-' + [guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $fixture
$repo = Join-Path $fixture 'source with spaces'
$null = New-Item -ItemType Directory -Path $repo
$passed = 0
function Invoke-FixtureGit([string[]]$Arguments) {
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $output = & git -C $repo -c core.autocrlf=false @Arguments 2>&1
    $ErrorActionPreference = $prev
    if ($LASTEXITCODE -ne 0) { throw "Fixture git failed: $($output -join ' ')" }
    return ($output -join "`n")
}
function Assert([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}
$isWin = if (Get-Variable -Name IsWindows -ErrorAction SilentlyContinue) { $IsWindows } else { $env:OS -eq 'Windows_NT' }
$shellExe = if (Test-Path (Join-Path $PSHOME 'pwsh.exe')) { Join-Path $PSHOME 'pwsh' } else { Join-Path $PSHOME 'powershell' }
function Expect-Failure([string]$Name, [string]$Target, [string]$Source = $repo) {
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $output = & $shellExe -NoProfile -ExecutionPolicy Bypass -File $scriptPath -Repository $Source -Destination $Target 2>$null
    $ErrorActionPreference = $prev
    Assert ($LASTEXITCODE -ne 0) "$Name unexpectedly succeeded"
    $script:passed++
    Write-Output "PASS: $Name refused"
}
try {
    $null = Invoke-FixtureGit -Arguments @('init','-b','master')
    $null = Invoke-FixtureGit -Arguments @('config','user.name','Synthetic Backup Test')
    $null = Invoke-FixtureGit -Arguments @('config','user.email','synthetic@example.invalid')
    Expect-Failure 'empty history' (Join-Path $fixture 'empty.bundle')
    Set-Content -LiteralPath (Join-Path $repo 'source.txt') -Value 'synthetic source'
    Set-Content -LiteralPath (Join-Path $repo '.gitignore') -Value ".env`n*.db"
    $null = Invoke-FixtureGit -Arguments @('add','source.txt','.gitignore')
    $null = Invoke-FixtureGit -Arguments @('commit','-m','synthetic source fixture')
    $null = Invoke-FixtureGit -Arguments @('branch','other')
    $null = Invoke-FixtureGit -Arguments @('tag','-a','fixture-v1','-m','synthetic tag')
    $null = Invoke-FixtureGit -Arguments @('update-ref','refs/remotes/example/other','HEAD')
    $null = Invoke-FixtureGit -Arguments @('update-ref','refs/codex/private-test','HEAD')
    Set-Content -LiteralPath (Join-Path $repo '.env') -Value 'SYNTHETIC_SECRET=never-archive'
    Set-Content -LiteralPath (Join-Path $repo 'memory.db') -Value 'synthetic private data'
    Expect-Failure 'relative destination' 'relative.bundle'
    Expect-Failure 'in-repository destination' (Join-Path $repo 'bad.bundle')
    Expect-Failure 'Git directory destination' (Join-Path $repo '.git/bad.bundle')
    Expect-Failure 'missing parent' (Join-Path $fixture 'missing/bad.bundle')
    Expect-Failure 'non-repository source' (Join-Path $fixture 'no-repo.bundle') $fixture
    Expect-Failure 'wrong extension' (Join-Path $fixture 'bad.zip')
    $dirty = Join-Path $repo 'untracked.txt'
    Set-Content -LiteralPath $dirty -Value 'uncommitted synthetic file'
    Expect-Failure 'untracked source' (Join-Path $fixture 'dirty.bundle')
    Assert (-not (Test-Path -LiteralPath (Join-Path $fixture 'dirty.bundle'))) 'Dirty failure created a backup'
    Remove-Item -LiteralPath $dirty
    Add-Content -LiteralPath (Join-Path $repo 'source.txt') -Value 'uncommitted change'
    Expect-Failure 'modified tracked source' (Join-Path $fixture 'dirty-tracked.bundle')
    $null = Invoke-FixtureGit -Arguments @('restore','source.txt') # Only the known synthetic fixture is restored.
    $bundle = Join-Path $fixture 'source backup.bundle'
    $output = & $shellExe -NoProfile -ExecutionPolicy Bypass -File $scriptPath -Repository $repo -Destination $bundle 2>&1
    Assert ($LASTEXITCODE -eq 0) "Valid backup failed: $output"
    $record = ($output -join "`n") | ConvertFrom-Json
    Assert ($record.head -eq (Invoke-FixtureGit -Arguments @('rev-parse','HEAD'))) 'Backup checkpoint differs'
    $null = Invoke-FixtureGit -Arguments @('bundle','verify',$bundle)
    $refs = Invoke-FixtureGit -Arguments @('bundle','list-heads',$bundle)
    foreach ($ref in @('refs/heads/master','refs/heads/other','refs/tags/fixture-v1','refs/remotes/example/other')) { Assert ($refs.Contains($ref)) "Missing ref $ref" }
    Assert (-not $refs.Contains('refs/codex')) 'Private tool refs included'
    $restore = Join-Path $fixture 'restored source'
    $null = Invoke-FixtureGit -Arguments @('clone',$bundle,$restore)
    $restoredHead = & git -C $restore rev-parse HEAD
    Assert ($restoredHead -eq $record.head) 'Restored HEAD differs'
    $restoredTree = & git -C $restore rev-parse 'HEAD^{tree}'
    Assert ($restoredTree -eq $record.tree) 'Restored tree differs'
    Assert (-not (Test-Path -LiteralPath (Join-Path $restore '.env'))) 'Ignored secret was copied'
    Assert (-not (Test-Path -LiteralPath (Join-Path $restore 'memory.db'))) 'Ignored database was copied'
    $passed++; Write-Output 'PASS: independent bundle verify, source refs, clone HEAD/tree and ignored data exclusion'
    $before = (Get-FileHash -LiteralPath $bundle).Hash
    Expect-Failure 'overwrite' $bundle
    Assert ($before -eq (Get-FileHash -LiteralPath $bundle).Hash) 'Existing backup changed'
    $link = Join-Path $fixture 'linked destination'
    $null = New-Item -ItemType $(if ($isWin) {'Junction'} else {'SymbolicLink'}) -Path $link -Target $repo
    Expect-Failure 'junction/symlink escape' (Join-Path $link 'escape.bundle')
    $corrupt = Join-Path $fixture 'corrupt.bundle'
    Set-Content -LiteralPath $corrupt -Value 'not a bundle'
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    & git -C $repo bundle verify $corrupt 2>$null | Out-Null
    Assert ($LASTEXITCODE -ne 0) 'Corrupt bundle was accepted'
    & git -C $repo bundle verify (Join-Path $fixture 'absent.bundle') 2>$null | Out-Null
    Assert ($LASTEXITCODE -ne 0) 'Missing bundle was accepted'
    $ErrorActionPreference = $prev
    $passed++; Write-Output 'PASS: Git rejects corrupt and missing bundles'
    Write-Output "PASS: $passed backup/recovery checks. Synthetic fixture retained at $fixture"
} catch {
    Write-Error $_ -ErrorAction Continue
    Write-Output "Fixture retained for diagnosis: $fixture"
    exit 1
}
