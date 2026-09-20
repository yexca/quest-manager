param([switch]$RefreshLocks, [switch]$CheckOnly, [switch]$NonInteractive)

# Project-local tools and caches. No permanent environment or PATH changes.
. (Join-Path $PSScriptRoot 'scripts\Environment.ps1')
. (Join-Path $PSScriptRoot 'scripts\Downloads.ps1')
. (Join-Path $PSScriptRoot 'scripts\SystemPrerequisites.ps1')
if ($env:OS -ne 'Windows_NT' -or ![Environment]::Is64BitProcess -or $env:PROCESSOR_ARCHITECTURE -ne 'AMD64') { throw 'Run this bootstrap in 64-bit PowerShell on Windows x64.' }
if ($CheckOnly -and $RefreshLocks) { throw '-CheckOnly cannot be combined with -RefreshLocks.' }
$questSystem = Ensure-QuestSystemPrerequisites -CheckOnly:$CheckOnly -NonInteractive:$NonInteractive
if ($CheckOnly) { Write-Host 'System prerequisites are compatible. No tools were installed.'; return }

foreach ($questDirectory in @($QuestEnv, (Join-Path $QuestEnv 'downloads'), $env:CARGO_HOME, $env:RUSTUP_HOME, $env:npm_config_cache)) {
    New-Item -ItemType Directory -Path $questDirectory -Force | Out-Null
}

Install-QuestNode
$questNodeVersion = (& $QuestNode --version).TrimStart('v')
$questNpmVersion = (& $QuestNpm --version).Trim()

$questRustupInit = Get-QuestDownload -Url $QuestVersions.rustup.url -Name "rustup-init-$($QuestVersions.rustup.version).exe" -Sha256 $QuestVersions.rustup.sha256
$questRustup = Join-Path $env:CARGO_HOME 'bin\rustup.exe'
if (!(Test-Path -LiteralPath $questRustup)) {
    Invoke-QuestCommand -File $questRustupInit -Arguments @('-y', '--no-modify-path', '--default-toolchain', 'none', '--profile', 'minimal')
}
$questInstalledRustup = (& $questRustup --version 2>$null | Select-Object -First 1)
if ($questInstalledRustup -notlike "rustup $($QuestVersions.rustup.version) *") { throw 'Local rustup version does not match toolchain.versions.json.' }
Write-Host "Installing local Rust $($QuestVersions.rust)..."
Invoke-QuestCommand -File $questRustup -Arguments @('toolchain', 'install', $QuestVersions.rust, '--profile', 'minimal', '--component', 'rustfmt', '--component', 'clippy', '--target', $QuestVersions.rustTarget, '--no-self-update')
$questCargo = Join-Path $env:CARGO_HOME 'bin\cargo.exe'
# Verify the component payload, not just rustup's installed-component metadata.
& $questCargo clippy --version 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) {
    Invoke-QuestCommand -File $questRustup -Arguments @('component', 'remove', 'clippy')
    Invoke-QuestCommand -File $questRustup -Arguments @('component', 'add', 'clippy')
}
Invoke-QuestCommand -File $questCargo -Arguments @('clippy', '--version')
Invoke-QuestCommand -File $questCargo -Arguments @('fmt', '--version')

$questAdbArchive = Get-QuestDownload -Url $QuestVersions.adb.url -Name "platform-tools_r$($QuestVersions.adb.version)-win.zip" -Sha256 $QuestVersions.adb.sha256
$questAdb = Join-Path $QuestEnv 'platform-tools\adb.exe'
$questAdbProperties = Join-Path $QuestEnv 'platform-tools\source.properties'
if (!(Test-Path -LiteralPath $questAdbProperties) -or (Get-Content -LiteralPath $questAdbProperties -Raw) -notmatch ('Pkg.Revision=' + [regex]::Escape($QuestVersions.adb.version))) {
    Expand-Archive -LiteralPath $questAdbArchive -DestinationPath $QuestEnv -Force
}

$questAaptArchive = Get-QuestDownload -Url $QuestVersions.aapt2.url -Name "aapt2-$($QuestVersions.aapt2.version)-windows.zip" -Sha256 $QuestVersions.aapt2.sha256
Expand-Archive -LiteralPath $questAaptArchive -DestinationPath (Join-Path $QuestEnv 'aapt2') -Force
$questAapt = Join-Path $QuestEnv 'aapt2\aapt2.exe'
Invoke-QuestCommand -File $questAapt -Arguments @('version')

# APK preparation tools stay private to this project, including Java.
$questApkTools = Join-Path $QuestEnv 'apk-tools'
New-Item -ItemType Directory -Path $questApkTools -Force | Out-Null
$questApktoolArchive = Get-QuestDownload -Url $QuestVersions.apktool.url -Name "apktool-$($QuestVersions.apktool.version).jar" -Sha256 $QuestVersions.apktool.sha256
Copy-Item -LiteralPath $questApktoolArchive -Destination (Join-Path $questApkTools 'apktool.jar') -Force
$questJavaArchive = Get-QuestDownload -Url $QuestVersions.jre.url -Name "jre-$($QuestVersions.jre.version).zip" -Sha256 $QuestVersions.jre.sha256
Expand-Archive -LiteralPath $questJavaArchive -DestinationPath (Join-Path $QuestEnv 'java') -Force
$questJavaRoot = Join-Path (Join-Path $QuestEnv 'java') $QuestVersions.jre.directory
New-Item -ItemType Directory -Path (Join-Path $questApkTools 'jre') -Force | Out-Null
Get-ChildItem -LiteralPath $questJavaRoot | Copy-Item -Destination (Join-Path $questApkTools 'jre') -Recurse -Force
$questBuildToolsArchive = Get-QuestDownload -Url $QuestVersions.buildTools.url -Name 'build-tools_r37_windows.zip' -Sha256 $QuestVersions.buildTools.sha256
Expand-Archive -LiteralPath $questBuildToolsArchive -DestinationPath (Join-Path $QuestEnv 'android-build-tools') -Force
$questAndroidTools = Join-Path $QuestEnv 'android-build-tools\android-37.0'
foreach ($questToolFile in @('lib\apksigner.jar', 'zipalign.exe', 'libwinpthread-1.dll', 'NOTICE.txt')) {
    Copy-Item -LiteralPath (Join-Path $questAndroidTools $questToolFile) -Destination $questApkTools -Force
}
Invoke-QuestCommand -File (Join-Path $questApkTools 'jre\bin\java.exe') -Arguments @('-jar', (Join-Path $questApkTools 'apktool.jar'), '--version')

if ($RefreshLocks) {
    Write-Host 'Explicitly refreshing dependency lockfiles...'
    Invoke-QuestCommand -File $QuestNpm -Arguments @('install', '--package-lock-only', '--ignore-scripts')
    Invoke-QuestCommand -File (Join-Path $env:CARGO_HOME 'bin\cargo.exe') -Arguments @('generate-lockfile', '--manifest-path', 'src-tauri/Cargo.toml')
}
foreach ($questLock in @('package-lock.json', 'src-tauri\Cargo.lock')) {
    if (!(Test-Path -LiteralPath (Join-Path $QuestRoot $questLock))) { throw "Missing $questLock. Maintainers can explicitly use -RefreshLocks." }
}
foreach ($questManifest in @('package.json', 'package-lock.json', '.npmrc')) {
    Copy-Item -LiteralPath (Join-Path $QuestRoot $questManifest) -Destination (Join-Path $QuestEnv $questManifest) -Force
}
Write-Host 'Installing locked npm dependencies into env/node_modules...'
Invoke-QuestCommand -File $QuestNpm -Arguments @('ci', '--prefix', $QuestEnv, '--no-audit', '--no-fund')
$questModulesLink = Join-Path $QuestRoot 'node_modules'
$questModulesTarget = Join-Path $QuestEnv 'node_modules'
if (Test-Path -LiteralPath $questModulesLink) {
    $questExistingLink = Get-Item -LiteralPath $questModulesLink
    $questExistingTarget = @($questExistingLink.Target)[0]
    if ($questExistingLink.LinkType -ne 'Junction' -or [IO.Path]::GetFullPath([string]$questExistingTarget) -ne [IO.Path]::GetFullPath($questModulesTarget)) {
        throw 'node_modules already exists and is not the expected env/node_modules junction. Move that directory before retrying.'
    }
} else {
    New-Item -ItemType Junction -Path $questModulesLink -Target $questModulesTarget | Out-Null
}
Write-Host 'Fetching locked Rust crates into env/cargo...'
Invoke-QuestCommand -File (Join-Path $env:CARGO_HOME 'bin\cargo.exe') -Arguments @('fetch', '--locked', '--target', $QuestVersions.rustTarget, '--manifest-path', 'src-tauri/Cargo.toml')

$questRecord = [ordered]@{
    installedAt = [DateTime]::UtcNow.ToString('o')
    node = $questNodeVersion
    npm = $questNpmVersion
    rust = (& (Join-Path $env:CARGO_HOME 'bin\rustc.exe') --version)
    cargo = (& (Join-Path $env:CARGO_HOME 'bin\cargo.exe') --version)
    clippy = (& $questCargo clippy --version)
    rustfmt = (& $questCargo fmt --version)
    rustup = $questInstalledRustup
    adb = @(& $questAdb version)
    aapt2 = @(& $questAapt version)
    apktool = $QuestVersions.apktool.version
    jre = $QuestVersions.jre.version
    buildTools = $QuestVersions.buildTools.version
    toolchainSha256 = (Get-FileHash -LiteralPath (Join-Path $QuestRoot 'toolchain.versions.json')).Hash
    visualStudio = @($questSystem.visualStudio | Select-Object displayName,installationVersion)
    msvcToolsets = @($questSystem.visualStudio | ForEach-Object { $_.msvcVersion })
    windowsSdks = @($questSystem.windowsSdks | ForEach-Object { $_.version })
    webview2 = @($questSystem.webview2 | Select-Object name,pv)
    windows = [Environment]::OSVersion.Version.ToString()
    npmLockSha256 = (Get-FileHash -LiteralPath (Join-Path $QuestRoot 'package-lock.json')).Hash
    cargoLockSha256 = (Get-FileHash -LiteralPath (Join-Path $QuestRoot 'src-tauri\Cargo.lock')).Hash
}
$questRecord | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $QuestEnv 'installed-versions.json') -Encoding UTF8
Write-Host 'Ready. Start with .\run-dev.ps1 or build with .\run-build.ps1.' -ForegroundColor Green
