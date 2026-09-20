param([switch]$Installer, [ValidateSet('Online', 'Offline', 'Both')][string]$Portable)
. (Join-Path $PSScriptRoot 'scripts\Environment.ps1')
Assert-QuestInstalled
if ($Portable -and $Installer) { throw 'Choose -Portable or -Installer, not both.' }
$questBuildArguments = @('run', 'tauri', '--', 'build')
if ($Installer) { $questBuildArguments += @('--bundles', 'nsis') } else { $questBuildArguments += '--no-bundle' }
$questBuildArguments += @('--', '--locked')
$questBuildEnvironment = @{}
try {
    if ($Portable) {
        # Remap Rust diagnostics and C __FILE__ strings before compiling, not in the binary.
        foreach ($questName in @('CARGO_ENCODED_RUSTFLAGS', 'CFLAGS', 'CXXFLAGS', 'CC_SHELL_ESCAPED_FLAGS')) {
            $questBuildEnvironment[$questName] = [Environment]::GetEnvironmentVariable($questName, 'Process')
        }
        if ($env:RUSTFLAGS -or $env:CARGO_ENCODED_RUSTFLAGS) { throw 'Build portable packages from a shell without custom Rust flags.' }
        $env:CARGO_ENCODED_RUSTFLAGS = "--remap-path-prefix=$env:USERPROFILE=build-user" + [char]31 + "--remap-path-prefix=$QuestRoot=quest-manager"
        $questMapping = '/experimental:deterministic "/pathmap:' + $env:USERPROFILE + '=build-user" "/pathmap:' + $QuestRoot + '=quest-manager"'
        $env:CFLAGS = "$env:CFLAGS $questMapping".Trim()
        $env:CXXFLAGS = "$env:CXXFLAGS $questMapping".Trim()
        $env:CC_SHELL_ESCAPED_FLAGS = '1'
    }
    Invoke-QuestCommand -File $QuestNpm -Arguments $questBuildArguments
} finally {
    foreach ($questName in $questBuildEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($questName, $questBuildEnvironment[$questName], 'Process')
    }
}
$questRelease = Join-Path $QuestRoot 'release'
New-Item -ItemType Directory -Path $questRelease -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $QuestRoot 'LICENSE') -Destination $questRelease -Force
Copy-Item -LiteralPath (Join-Path $env:CARGO_TARGET_DIR 'release\quest-manager.exe') -Destination $questRelease -Force
$questReleaseTools = Join-Path $questRelease 'platform-tools'
New-Item -ItemType Directory -Path $questReleaseTools -Force | Out-Null
Get-ChildItem -LiteralPath (Join-Path $QuestEnv 'platform-tools') | Copy-Item -Destination $questReleaseTools -Recurse -Force
$questReleaseAapt = Join-Path $questRelease 'aapt2'
New-Item -ItemType Directory -Path $questReleaseAapt -Force | Out-Null
Get-ChildItem -LiteralPath (Join-Path $QuestEnv 'aapt2') | Copy-Item -Destination $questReleaseAapt -Recurse -Force
$questReleaseApkTools = Join-Path $questRelease 'apk-tools'
New-Item -ItemType Directory -Path $questReleaseApkTools -Force | Out-Null
Get-ChildItem -LiteralPath (Join-Path $QuestEnv 'apk-tools') | Copy-Item -Destination $questReleaseApkTools -Recurse -Force
Copy-Item -LiteralPath (Join-Path $QuestEnv 'installed-versions.json') -Destination (Join-Path $questRelease 'build-environment.json') -Force
Write-Host "Portable app: $questRelease\quest-manager.exe" -ForegroundColor Green
if ($Portable) {
    & (Join-Path $PSScriptRoot 'scripts\Package-Portable.ps1') -Variant $Portable
}
