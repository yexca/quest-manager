param([switch]$RefreshLocks)

# Project-local tools and caches. No permanent environment or PATH changes.
. (Join-Path $PSScriptRoot 'scripts\Environment.ps1')
if ($env:OS -ne 'Windows_NT') { throw 'This bootstrap script currently supports Windows x64.' }

$questNodeVersion = (& node --version).TrimStart('v')
$questNpmVersion = (& npm.cmd --version).Trim()
if ($questNodeVersion -ne $QuestVersions.node -or $questNpmVersion -ne $QuestVersions.npm) {
    throw "Use system Node $($QuestVersions.node) and npm $($QuestVersions.npm). Found Node $questNodeVersion / npm $questNpmVersion. See README.md."
}

$questVswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (!(Test-Path -LiteralPath $questVswhere)) { throw 'Install Visual Studio C++ Build Tools and the Windows SDK. See README.md.' }
$questVs = (& $questVswhere -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -format json | Out-String) | ConvertFrom-Json
if (@($questVs).Count -eq 0) { throw 'The Desktop development with C++ workload is required. See README.md.' }
$questWebviews = @(foreach ($questReg in @('HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients', 'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients', 'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients')) {
    if (Test-Path -LiteralPath $questReg) {
        Get-ChildItem -LiteralPath $questReg | ForEach-Object { Get-ItemProperty -LiteralPath $_.PSPath } | Where-Object { $_.PSObject.Properties.Name -contains 'name' -and $_.name -like '*WebView2*' }
    }
})
if ($questWebviews.Count -eq 0) { throw 'Install Microsoft Edge WebView2 Runtime. See README.md.' }

foreach ($questDirectory in @($QuestEnv, (Join-Path $QuestEnv 'downloads'), $env:CARGO_HOME, $env:RUSTUP_HOME, $env:npm_config_cache)) {
    New-Item -ItemType Directory -Path $questDirectory -Force | Out-Null
}

function Get-QuestDownload {
    param([string]$Url, [string]$Name, [string]$Sha256)
    $questDownload = Join-Path $QuestEnv "downloads\$Name"
    if (!(Test-Path -LiteralPath $questDownload) -or (Get-FileHash -LiteralPath $questDownload -Algorithm SHA256).Hash -ne $Sha256) {
        Write-Host "Downloading $Name..."
        Invoke-WebRequest -Uri $Url -OutFile $questDownload -UseBasicParsing -TimeoutSec 300
    }
    if ((Get-FileHash -LiteralPath $questDownload -Algorithm SHA256).Hash -ne $Sha256) { throw "Checksum mismatch: $Name" }
    return $questDownload
}

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
    Invoke-QuestCommand -File 'npm.cmd' -Arguments @('install', '--package-lock-only', '--ignore-scripts')
    Invoke-QuestCommand -File (Join-Path $env:CARGO_HOME 'bin\cargo.exe') -Arguments @('generate-lockfile', '--manifest-path', 'src-tauri/Cargo.toml')
}
foreach ($questLock in @('package-lock.json', 'src-tauri\Cargo.lock')) {
    if (!(Test-Path -LiteralPath (Join-Path $QuestRoot $questLock))) { throw "Missing $questLock. Maintainers can explicitly use -RefreshLocks." }
}
foreach ($questManifest in @('package.json', 'package-lock.json', '.npmrc')) {
    Copy-Item -LiteralPath (Join-Path $QuestRoot $questManifest) -Destination (Join-Path $QuestEnv $questManifest) -Force
}
Write-Host 'Installing locked npm dependencies into env/node_modules...'
Invoke-QuestCommand -File 'npm.cmd' -Arguments @('ci', '--prefix', $QuestEnv, '--no-audit', '--no-fund')
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
    visualStudio = @($questVs | Select-Object displayName,installationVersion)
    msvcToolsets = @(foreach ($questVsInstance in $questVs) {
        Get-ChildItem -LiteralPath (Join-Path $questVsInstance.installationPath 'VC\Tools\MSVC') -Directory | Select-Object -ExpandProperty Name
    })
    windowsSdks = @(Get-ChildItem -LiteralPath (Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\Lib') -Directory | Select-Object -ExpandProperty Name)
    webview2 = @($questWebviews | Select-Object name,pv)
    windows = [Environment]::OSVersion.Version.ToString()
    npmLockSha256 = (Get-FileHash -LiteralPath (Join-Path $QuestRoot 'package-lock.json')).Hash
    cargoLockSha256 = (Get-FileHash -LiteralPath (Join-Path $QuestRoot 'src-tauri\Cargo.lock')).Hash
}
$questRecord | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $QuestEnv 'installed-versions.json') -Encoding UTF8
Write-Host 'Ready. Start with .\run-dev.ps1 or build with .\run-build.ps1.' -ForegroundColor Green
