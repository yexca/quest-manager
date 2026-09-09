Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Initialize-QuestEnvironment {
    $script:QuestRoot = Split-Path -Parent $PSScriptRoot
    $script:QuestEnv = Join-Path $script:QuestRoot 'env'
    $script:QuestVersions = Get-Content -LiteralPath (Join-Path $script:QuestRoot 'toolchain.versions.json') -Raw | ConvertFrom-Json
    $env:CARGO_HOME = Join-Path $script:QuestEnv 'cargo'
    $env:RUSTUP_HOME = Join-Path $script:QuestEnv 'rustup'
    $env:CARGO_TARGET_DIR = Join-Path $script:QuestEnv 'target'
    $env:RUSTUP_TOOLCHAIN = $script:QuestVersions.rust
    $env:npm_config_cache = Join-Path $script:QuestEnv 'npm-cache'
    $env:PATH = (Join-Path $env:CARGO_HOME 'bin') + ';' + (Join-Path $script:QuestEnv 'platform-tools') + ';' + $env:PATH
    Set-Location -LiteralPath $script:QuestRoot
}

function Invoke-QuestCommand {
    param([Parameter(Mandatory)][string]$File, [string[]]$Arguments = @())
    & $File @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$File failed (exit $LASTEXITCODE)." }
}

function Assert-QuestInstalled {
    if (!(Test-Path -LiteralPath (Join-Path $script:QuestEnv 'installed-versions.json'))) {
        throw 'Run .\run-install.ps1 first.'
    }
    if ((& node --version).TrimStart('v') -ne $script:QuestVersions.node -or (& npm.cmd --version).Trim() -ne $script:QuestVersions.npm) {
        throw "Use system Node $($script:QuestVersions.node) / npm $($script:QuestVersions.npm) as recorded in toolchain.versions.json."
    }
    $questInstalledRecord = Get-Content -LiteralPath (Join-Path $script:QuestEnv 'installed-versions.json') -Raw | ConvertFrom-Json
    $questNpmLockHash = (Get-FileHash -LiteralPath (Join-Path $script:QuestRoot 'package-lock.json') -Algorithm SHA256).Hash
    $questCargoLockHash = (Get-FileHash -LiteralPath (Join-Path $script:QuestRoot 'src-tauri\Cargo.lock') -Algorithm SHA256).Hash
    if ($questNpmLockHash -ne $questInstalledRecord.npmLockSha256 -or $questCargoLockHash -ne $questInstalledRecord.cargoLockSha256 -or (Get-FileHash -LiteralPath (Join-Path $script:QuestRoot 'package.json')).Hash -ne (Get-FileHash -LiteralPath (Join-Path $script:QuestEnv 'package.json')).Hash) {
        throw 'Dependency manifests changed. Run .\run-install.ps1 again before building.'
    }
}

Initialize-QuestEnvironment
