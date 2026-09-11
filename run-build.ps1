param([switch]$Installer)
. (Join-Path $PSScriptRoot 'scripts\Environment.ps1')
Assert-QuestInstalled
$questBuildArguments = @('run', 'tauri', '--', 'build')
if ($Installer) { $questBuildArguments += @('--bundles', 'nsis') } else { $questBuildArguments += '--no-bundle' }
$questBuildArguments += @('--', '--locked')
Invoke-QuestCommand -File 'npm.cmd' -Arguments $questBuildArguments
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
