# Run on the OLD development machine. Does not commit, checkout or clean anything.
[CmdletBinding()]
param(
    [string[]]$SourcePaths = @('E:\Zonendpoint', 'E:\Zonendpoint-vic', 'E:\Zonendpoint-QA-e3ed872'),
    [string]$OutputDirectory = 'E:\Zonendpoint-recovery-export'
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Invoke-RecoveryGit([string]$Repo, [string[]]$Arguments) {
    $result = & git.exe -C $Repo @Arguments
    if ($LASTEXITCODE -ne 0) { throw "git failed in $Repo : $($Arguments -join ' ')" }
    return $result
}

$out = [IO.Path]::GetFullPath($OutputDirectory)
foreach ($source in $SourcePaths) {
    $root = [IO.Path]::GetFullPath($source).TrimEnd('\')
    if ($out.Equals($root, [StringComparison]::OrdinalIgnoreCase) -or
        $out.StartsWith($root + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'OutputDirectory must be outside every source checkout.'
    }
    if (-not (Test-Path -LiteralPath $root)) { throw "Missing source: $root" }
    $null = Invoke-RecoveryGit $root @('rev-parse', '--show-toplevel')
    if (@(Invoke-RecoveryGit $root @('ls-files', '--unmerged')).Count) { throw "Resolve merge conflicts first: $root" }
}
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$folder = Join-Path $out ("Zonendpoint-$stamp-" + [guid]::NewGuid().ToString('N').Substring(0,8))
$null = New-Item -ItemType Directory -Path $folder
$manifest = @()
foreach ($source in $SourcePaths) {
    $root = [IO.Path]::GetFullPath($source).TrimEnd('\')
    $dest = Join-Path $folder ([string]$manifest.Count + '-' + (Split-Path $root -Leaf))
    $null = New-Item -ItemType Directory -Path $dest
    $before = @(Invoke-RecoveryGit $root @('status', '--porcelain=v1', '--untracked-files=all')) -join "`n"
    $head = [string](Invoke-RecoveryGit $root @('rev-parse', 'HEAD'))
    $branch = [string](Invoke-RecoveryGit $root @('rev-parse', '--abbrev-ref', 'HEAD'))
    $null = Invoke-RecoveryGit $root @('bundle', 'create', (Join-Path $dest 'repository.bundle'), '--all', 'HEAD')
    $null = Invoke-RecoveryGit $root @('bundle', 'verify', (Join-Path $dest 'repository.bundle'))
    # Git writes patch bytes directly, preserving binary changes and avoiding shell encoding.
    $null = Invoke-RecoveryGit $root @('diff', '--binary', '--full-index', '--no-ext-diff', '--no-textconv', '--cached', "--output=$(Join-Path $dest 'staged.patch')", 'HEAD', '--')
    $null = Invoke-RecoveryGit $root @('diff', '--binary', '--full-index', '--no-ext-diff', '--no-textconv', "--output=$(Join-Path $dest 'unstaged.patch')", '--')
    $null = Invoke-RecoveryGit $root @('diff', '--binary', '--full-index', '--no-ext-diff', '--no-textconv', "--output=$(Join-Path $dest 'working-tree.patch')", 'HEAD', '--')
    $untracked = Join-Path $dest 'untracked'
    $null = New-Item -ItemType Directory -Path $untracked
    $names = ((Invoke-RecoveryGit $root @('-c', 'core.quotepath=false', 'ls-files', '--others', '--exclude-standard', '-z')) -join "`n").Split([char]0)
    foreach ($name in $names) {
        if (-not $name) { continue }
        $from = Join-Path $root $name
        $to = [IO.Path]::GetFullPath((Join-Path $untracked $name))
        if (-not $to.StartsWith($untracked + '\', [StringComparison]::OrdinalIgnoreCase)) { throw "Unsafe path: $name" }
        $item = Get-Item -LiteralPath $from -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Copy reparse point manually: $from" }
        $null = New-Item -ItemType Directory -Force -Path (Split-Path $to -Parent)
        Copy-Item -LiteralPath $from -Destination $to -Force
    }
    Invoke-RecoveryGit $root @('ls-files', '--others', '--ignored', '--exclude-standard') | Set-Content (Join-Path $dest 'ignored-files-NOT-INCLUDED.txt') -Encoding UTF8
    $after = @(Invoke-RecoveryGit $root @('status', '--porcelain=v1', '--untracked-files=all')) -join "`n"
    if ($before -cne $after -or $head -ne [string](Invoke-RecoveryGit $root @('rev-parse', 'HEAD'))) { throw "Source changed during export; stop dev/test and retry: $root" }
    $manifest += [pscustomobject]@{ Folder = Split-Path $dest -Leaf; Source = $root; Head = $head; Branch = $branch; Status = $before }
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $folder 'manifest.json') -Encoding UTF8
@'
RESTORE INTO NEW EMPTY DIRECTORIES ONLY. Keep original workspaces untouched.
For each manifest entry:
  git clone <entry>/repository.bundle <new-directory>
  git -C <new-directory> checkout --detach <Head from manifest>
If staged.patch is nonempty:
  git -C <new-directory> apply --index <absolute staged.patch path>
If unstaged.patch is nonempty:
  git -C <new-directory> apply <absolute unstaged.patch path>
Copy CONTENTS of entry/untracked into new-directory, including hidden files.
Compare git status against manifest. working-tree.patch is an alternative combined
patch; DO NOT apply it in addition to staged.patch/unstaged.patch.
Each bundle is independent of the original worktree paths. Restore submodules,
Git LFS objects and ignored files separately if present. Review ignored-files list.
Ignored files (including node_modules, target, dist, .env) are NOT included.
Export may contain private source/untracked files: transfer privately.
'@ | Set-Content (Join-Path $folder 'RESTORE.txt') -Encoding UTF8
Get-ChildItem -LiteralPath $folder -Recurse -File | ForEach-Object {
    "$((Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash)  $($_.FullName.Substring($folder.Length + 1))"
} | Set-Content (Join-Path $folder 'SHA256SUMS.txt') -Encoding UTF8
$archive = "$folder.zip"
& tar.exe -a -cf $archive -C $out (Split-Path $folder -Leaf)
if ($LASTEXITCODE -ne 0) { throw "Archive failed; exported files remain at $folder" }
Write-Host "Export complete: $archive"
Write-Host 'Keep source apps/dev/test stopped during export. Review ignored-files-NOT-INCLUDED.txt.'

