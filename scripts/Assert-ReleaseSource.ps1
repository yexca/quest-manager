param([string]$Tag, [switch]$CheckClean)
$ErrorActionPreference = 'Stop'

function Get-QuestReleaseVersion {
    param([string]$Root)
    $package = Get-Content -LiteralPath (Join-Path $Root 'package.json') -Raw | ConvertFrom-Json
    # Windows PowerShell 5.1 cannot deserialize npm's empty root-package key.
    $lockText = Get-Content -LiteralPath (Join-Path $Root 'package-lock.json') -Raw
    $lock = ($lockText -replace '(?m)^(\s*)""\s*:', '$1"__quest_root_package__":') | ConvertFrom-Json
    $tauri = Get-Content -LiteralPath (Join-Path $Root 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
    $cargo = Get-Content -LiteralPath (Join-Path $Root 'src-tauri/Cargo.toml') -Raw
    $cargoLock = Get-Content -LiteralPath (Join-Path $Root 'src-tauri/Cargo.lock') -Raw
    $packageSection = [regex]::Match($cargo, '(?ms)^\[package\]\s*\r?\n(?<body>.*?)(?=^\[|\z)').Groups['body'].Value
    $cargoVersion = [regex]::Match($packageSection, '(?m)^version\s*=\s*"([^"]+)"\s*$').Groups[1].Value
    $lockedVersion = [regex]::Match($cargoLock, '(?m)^name = "quest-manager"\r?\nversion = "([^"]+)"\s*$').Groups[1].Value
    $version = [string]$package.version
    if ($version -cnotmatch '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') { throw 'Application version must be a stable major.minor.patch version.' }
    foreach ($value in @($lock.version, $lock.packages.__quest_root_package__.version, $tauri.version, $cargoVersion, $lockedVersion)) {
        if ($value -cne $version) { throw 'Application versions disagree across manifests and lockfiles.' }
    }
    return $version
}

function Assert-QuestCleanSource {
    param([string]$Root)
    $changes = & git -C $Root status --porcelain --untracked-files=all
    if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect source working-tree state.' }
    if (($changes | Out-String).Trim()) {
        foreach ($change in $changes) { Write-Host "Source change: $change" }
        throw 'Release source must have a clean working tree.'
    }
}

function Assert-QuestReleaseTag {
    param([string]$Root, [string]$Tag, [string]$Version)
    if ($Tag -cne "v$Version") { throw 'Release tag must exactly match the application version (vMAJOR.MINOR.PATCH).' }
    $head = & git -C $Root rev-parse HEAD
    if ($LASTEXITCODE -ne 0) { throw 'Cannot read source commit.' }
    $tagCommit = & git -C $Root rev-parse --verify "refs/tags/$Tag^{commit}"
    if ($LASTEXITCODE -ne 0 -or "$tagCommit".Trim() -cne "$head".Trim()) { throw 'Release tag does not resolve to the checked-out source commit.' }
    Assert-QuestCleanSource $Root
}

if ($MyInvocation.InvocationName -ne '.') {
    $root = Split-Path -Parent $PSScriptRoot
    $version = Get-QuestReleaseVersion $root
    if ($Tag) { Assert-QuestReleaseTag -Root $root -Tag $Tag -Version $version }
    elseif ($CheckClean) { Assert-QuestCleanSource $root }
    Write-Host "Application version verified: $version"
}
