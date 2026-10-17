[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Destination,
    [string]$Repository = (Split-Path -Parent $PSScriptRoot)
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false

function Invoke-SourceGit([string[]]$Arguments) {
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $output = & git -C $Repository @Arguments 2>&1
    $ErrorActionPreference = $prev
    if ($LASTEXITCODE -ne 0) { throw "Git $($Arguments[0]) failed: $($output -join ' ')" }
    return ($output -join "`n")
}
function Assert-NoLinks([string]$ExistingPath) {
    $cursor = Get-Item -LiteralPath $ExistingPath -Force
    while ($null -ne $cursor) {
        if (($cursor.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "Symlink/junction paths are refused: $($cursor.FullName)"
        }
        if ($cursor.Name -eq '.git') { throw 'Destination/repository ancestry must not be inside .git' }
        $cursor = $cursor.Parent
    }
}
function Test-Inside([string]$Candidate, [string]$Directory) {
    $isWin = if (Get-Variable -Name IsWindows -ErrorAction SilentlyContinue) { $IsWindows } else { $env:OS -eq 'Windows_NT' }
    $comparison = if ($isWin) { [StringComparison]::OrdinalIgnoreCase } else { [StringComparison]::Ordinal }
    $base = $Directory.TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar)
    return $Candidate.Equals($base, $comparison) -or $Candidate.StartsWith($base + [IO.Path]::DirectorySeparatorChar, $comparison)
}

$temporary = $null
try {
    $method = [IO.Path].GetMethod('IsPathFullyQualified', [Type[]]@([string]))
    $isFullyQualified = if ($null -ne $method) {
        $method.Invoke($null, @($Destination))
    } else {
        [IO.Path]::IsPathRooted($Destination) -and [IO.Path]::GetPathRoot($Destination).Length -ge 2
    }
    if (-not $isFullyQualified) { throw 'Destination must be an explicit absolute path' }
    $target = [IO.Path]::GetFullPath($Destination)
    if ([IO.Path]::GetExtension($target) -ne '.bundle') { throw 'Destination must end in .bundle' }
    if (Get-Item -LiteralPath $target -Force -ErrorAction SilentlyContinue) { throw 'Destination already exists; overwrite refused' }
    $parent = [IO.Path]::GetDirectoryName($target)
    if (-not [IO.Directory]::Exists($parent)) { throw 'Destination parent must be an existing directory' }
    Assert-NoLinks $parent
    $Repository = [IO.Path]::GetFullPath($Repository)
    Assert-NoLinks $Repository
    $sourceRoot = Invoke-SourceGit -Arguments @('rev-parse', '--show-toplevel')
    $gitDir = Invoke-SourceGit -Arguments @('rev-parse', '--absolute-git-dir')
    $commonDir = Invoke-SourceGit -Arguments @('rev-parse', '--path-format=absolute', '--git-common-dir')
    foreach ($protected in @($sourceRoot, $gitDir, $commonDir)) {
        if (Test-Inside $target ([IO.Path]::GetFullPath($protected))) { throw 'Destination must be outside the checkout and Git directories' }
    }
    if ((Invoke-SourceGit -Arguments @('rev-parse','--is-shallow-repository')) -eq 'true') { throw 'Shallow history cannot produce a full source backup' }
    $head = Invoke-SourceGit -Arguments @('rev-parse', '--verify', 'HEAD')
    if (Invoke-SourceGit -Arguments @('status','--porcelain','--untracked-files=all')) {
        throw 'Dirty worktree refused. Commit reviewed source or export tracked changes and separately preserve untracked files; then retry. No backup created.'
    }
    $refs = Invoke-SourceGit -Arguments @('for-each-ref','--format=%(objectname) %(refname)','refs/heads','refs/tags','refs/remotes')
    $temporary = Join-Path $parent ('.anansi-' + [guid]::NewGuid().ToString('N') + '.bundle')
    # Include only published/local source refs and HEAD; exclude tool-private refs, stash and Git config/credentials.
    $null = Invoke-SourceGit -Arguments @('bundle','create',$temporary,'--branches','--tags','--remotes','HEAD')
    $null = Invoke-SourceGit -Arguments @('bundle','verify',$temporary)
    if ($head -ne (Invoke-SourceGit -Arguments @('rev-parse','HEAD')) -or
        $refs -ne (Invoke-SourceGit -Arguments @('for-each-ref','--format=%(objectname) %(refname)','refs/heads','refs/tags','refs/remotes')) -or
        (Invoke-SourceGit -Arguments @('status','--porcelain','--untracked-files=all'))) { throw 'Source changed during backup; retry from a stable clean checkpoint' }
    Assert-NoLinks $parent
    $tree = Invoke-SourceGit -Arguments @('rev-parse', ($head + '^{tree}'))
    $digest = (Get-FileHash -LiteralPath $temporary -Algorithm SHA256).Hash
    [IO.File]::Move($temporary, $target) # Two-argument Move refuses overwrite, including a competing destination.
    $temporary = $null
    [ordered]@{
        destination = $target
        head = $head
        tree = $tree
        sha256 = $digest
        sourceRefs = ($refs -split "`n")
        scope = 'Committed branch/tag/remote history and HEAD only; no working files, credentials, Git configuration or tool-private refs'
    } | ConvertTo-Json -Depth 3
} catch {
    Write-Error $_ -ErrorAction Continue
    exit 1
} finally {
    if ($temporary -and [IO.File]::Exists($temporary)) { Remove-Item -LiteralPath $temporary }
}
