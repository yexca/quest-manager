Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Initialize-QuestEnvironment {
    $script:QuestRoot = Split-Path -Parent $PSScriptRoot
    $script:QuestEnv = Join-Path $script:QuestRoot 'env'
    $script:QuestVersions = Get-Content -LiteralPath (Join-Path $script:QuestRoot 'toolchain.versions.json') -Raw | ConvertFrom-Json
    $script:QuestNodeDirectory = Join-Path $script:QuestEnv "node\node-v$($script:QuestVersions.node)-win-x64"
    $script:QuestNode = Join-Path $script:QuestNodeDirectory 'node.exe'
    $script:QuestNpm = Join-Path $script:QuestNodeDirectory 'npm.cmd'
    $env:CARGO_HOME = Join-Path $script:QuestEnv 'cargo'
    $env:RUSTUP_HOME = Join-Path $script:QuestEnv 'rustup'
    $env:CARGO_TARGET_DIR = Join-Path $script:QuestEnv 'target'
    $env:RUSTUP_TOOLCHAIN = $script:QuestVersions.rust
    $env:npm_config_cache = Join-Path $script:QuestEnv 'npm-cache'
    $env:PATH = $script:QuestNodeDirectory + ';' + (Join-Path $env:CARGO_HOME 'bin') + ';' + (Join-Path $script:QuestEnv 'platform-tools') + ';' + $env:PATH
    Set-Location -LiteralPath $script:QuestRoot
}

function Invoke-QuestCommand {
    param([Parameter(Mandatory)][string]$File, [string[]]$Arguments = @())
    & $File @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$File failed (exit $LASTEXITCODE)." }
}

function Assert-QuestNode {
    if (!(Test-Path -LiteralPath $script:QuestNode) -or !(Test-Path -LiteralPath $script:QuestNpm)) {
        throw 'Project-local Node/npm are missing. Run .\run-install.ps1 first; system Node is not used.'
    }
    $questActualNode = & $script:QuestNode --version
    if ($LASTEXITCODE -ne 0 -or "$questActualNode".Trim() -ne "v$($script:QuestVersions.node)") {
        throw 'Project-local Node version mismatch. Run .\run-install.ps1 again.'
    }
    $questActualNpm = & $script:QuestNpm --version
    if ($LASTEXITCODE -ne 0 -or "$questActualNpm".Trim() -ne $script:QuestVersions.npm) {
        throw 'Project-local npm version mismatch. Run .\run-install.ps1 again.'
    }
}

function Assert-QuestInstalled {
    if (!(Test-Path -LiteralPath (Join-Path $script:QuestEnv 'installed-versions.json'))) {
        throw 'Run .\run-install.ps1 first.'
    }
    Assert-QuestNode
    $questInstalledRecord = Get-Content -LiteralPath (Join-Path $script:QuestEnv 'installed-versions.json') -Raw | ConvertFrom-Json
    if (!($questInstalledRecord.PSObject.Properties.Name -contains 'toolchainSha256') -or $questInstalledRecord.toolchainSha256 -ne (Get-FileHash -LiteralPath (Join-Path $script:QuestRoot 'toolchain.versions.json')).Hash -or !(Test-Path -LiteralPath (Join-Path $script:QuestEnv 'aapt2\aapt2.exe'))) {
        throw 'Tool versions changed. Run .\run-install.ps1 again before building.'
    }
    foreach ($questRequiredTool in @('apk-tools\apktool.jar', 'apk-tools\apksigner.jar', 'apk-tools\zipalign.exe', 'apk-tools\jre\bin\java.exe', 'apk-tools\jre\bin\keytool.exe')) {
        if (!(Test-Path -LiteralPath (Join-Path $script:QuestEnv $questRequiredTool))) { throw 'APK preparation tools are missing. Run .\run-install.ps1 again.' }
    }
    $questNpmLockHash = (Get-FileHash -LiteralPath (Join-Path $script:QuestRoot 'package-lock.json') -Algorithm SHA256).Hash
    $questCargoLockHash = (Get-FileHash -LiteralPath (Join-Path $script:QuestRoot 'src-tauri\Cargo.lock') -Algorithm SHA256).Hash
    if ($questNpmLockHash -ne $questInstalledRecord.npmLockSha256 -or $questCargoLockHash -ne $questInstalledRecord.cargoLockSha256 -or (Get-FileHash -LiteralPath (Join-Path $script:QuestRoot 'package.json')).Hash -ne (Get-FileHash -LiteralPath (Join-Path $script:QuestEnv 'package.json')).Hash) {
        throw 'Dependency manifests changed. Run .\run-install.ps1 again before building.'
    }
}

Initialize-QuestEnvironment
