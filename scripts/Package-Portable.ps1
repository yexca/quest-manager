param([ValidateSet('Online', 'Offline', 'Both')][string]$Variant = 'Both')
. (Join-Path $PSScriptRoot 'Environment.ps1')
. (Join-Path $PSScriptRoot 'Downloads.ps1')
Assert-QuestInstalled
$questFixed = Get-Content -LiteralPath (Join-Path $QuestRoot 'packaging/webview2.json') -Raw | ConvertFrom-Json
$questVersion = (Get-Content -LiteralPath (Join-Path $QuestRoot 'package.json') -Raw | ConvertFrom-Json).version
$questExe = Join-Path $QuestRoot 'release/quest-manager.exe'
if (!(Test-Path -LiteralPath $questExe)) { throw 'Build the application first with run-build.ps1 -Portable Both.' }

function Assert-PortablePrivacy {
    param([string]$Directory)
    # Binary scans include UTF-16 native __FILE__ literals as well as Rust UTF-8 strings.
    $questScan = @'
const fs = require('node:fs'); const path = require('node:path');
const root = process.argv[2];
const needles = [process.env.USERPROFILE, process.argv[3]].filter(Boolean).flatMap(p =>
 [p, p.replaceAll('\\','/')].flatMap(s => [Buffer.from(s), Buffer.from(s, 'utf16le')]));
const walk = dir => { for (const e of fs.readdirSync(dir, {withFileTypes:true})) {
 const p=path.join(dir,e.name); if(e.isSymbolicLink()) throw Error('Linked file in portable package');
 if(e.isDirectory()) { walk(p); continue; } const b=fs.readFileSync(p);
 if(needles.some(n=>b.includes(n))) throw Error('Personal build path in '+path.relative(root,p));
 if(/^(?:build-environment|installed-versions|device-settings)\.json$|\.(?:pfx|p12|jks|keystore|log)$|^adbkey(?:\.pub)?$/i.test(e.name)) throw Error('Private file in '+path.relative(root,p));
} }; walk(root); console.log('Portable privacy scan passed.');
'@
    $questScan | & $QuestNode - $Directory $QuestRoot
    if ($LASTEXITCODE -ne 0) { throw 'Portable privacy scan failed. Rebuild with run-build.ps1 -Portable Both; do not patch executable bytes.' }
}

$questRuntime = $null
if ($Variant -in @('Offline', 'Both')) {
    $questCabName = "Microsoft.WebView2.FixedVersionRuntime.$($questFixed.version).x64.cab"
    $questCab = Get-QuestDownload -Url $questFixed.url -Name $questCabName -Sha256 $questFixed.sha256
    $questSignature = Get-AuthenticodeSignature -LiteralPath $questCab
    if ($questSignature.Status -ne 'Valid' -or !$questSignature.SignerCertificate -or $questSignature.SignerCertificate.Subject -notmatch '(^|,\s*)O=Microsoft Corporation(,|$)') { throw 'WebView2 CAB signature is not a valid Microsoft signature.' }
    $questRuntime = Join-Path $QuestEnv "webview2/Microsoft.WebView2.FixedVersionRuntime.$($questFixed.version).x64"
    $questExtract = Join-Path $QuestEnv 'webview2'
    New-Item -ItemType Directory -Path $questExtract -Force | Out-Null
    & "$env:SystemRoot\System32\expand.exe" $questCab '-F:*' $questExtract | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not extract the fixed WebView2 runtime.' }
    $questBrowser = Join-Path $questRuntime 'msedgewebview2.exe'
    if (!(Test-Path -LiteralPath $questBrowser) -or (Get-Item -LiteralPath $questBrowser).VersionInfo.ProductVersion -ne $questFixed.version) { throw 'Fixed WebView2 payload version mismatch.' }
}

$questOutput = Join-Path $QuestRoot ('release/portable-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N').Substring(0, 6))
New-Item -ItemType Directory -Path $questOutput | Out-Null
$questModes = @($Variant.ToLowerInvariant())
if ($Variant -eq 'Both') { $questModes = @('online', 'offline') }
$questCommit = (& git rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot record the source commit.' }
$questDirty = [bool](& git status --porcelain | Out-String).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot record source working-tree state.' }
$questResults = @()
Add-Type -AssemblyName System.IO.Compression.FileSystem
foreach ($questMode in $questModes) {
    $questName = "quest-manager-v$questVersion-windows-x64-portable-$questMode"
    $questStage = Join-Path $questOutput $questName
    New-Item -ItemType Directory -Path (Join-Path $questStage 'portable') -Force | Out-Null
    Copy-Item -LiteralPath $questExe -Destination $questStage
    foreach ($questTool in @('platform-tools', 'aapt2', 'apk-tools')) {
        Copy-Item -LiteralPath (Join-Path $QuestEnv $questTool) -Destination $questStage -Recurse
    }
    foreach ($questNotice in @('LICENSE', 'THIRD_PARTY.md')) { Copy-Item -LiteralPath (Join-Path $QuestRoot $questNotice) -Destination $questStage }
    Copy-Item -LiteralPath (Join-Path $QuestRoot 'packaging/Start-QuestManager.cmd') -Destination $questStage
    Copy-Item -LiteralPath (Join-Path $QuestRoot 'packaging/Launch.ps1') -Destination (Join-Path $questStage 'portable')
    if ($questMode -eq 'offline') { Copy-Item -LiteralPath $questRuntime -Destination (Join-Path $questStage 'webview2') -Recurse }
    $questManifest = [ordered]@{ version = $questVersion; mode = $questMode; architecture = 'x64'; minimumWebView2 = $QuestVersions.systemPrerequisites.webview2Minimum; fixedWebView2 = $(if ($questMode -eq 'offline') { $questFixed.version } else { $null }); sourceCommit = $questCommit; sourceHasLocalChanges = $questDirty }
    $questManifest | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $questStage 'portable/manifest.json') -Encoding UTF8
    $questReadme = @"
Quest Manager $questVersion - $questMode portable (Windows x64)

Extract the COMPLETE ZIP into a writable local folder. Double-click
Start-QuestManager.cmd. Do not launch from inside the ZIP preview.
Use the launcher: directly opening quest-manager.exe skips runtime setup.
Windows 10 2004 or later / Windows 11 x64 is required.

ONLINE: Reuses a compatible system WebView2. If it is missing/too old,
asks before downloading Microsoft's signed Evergreen installer. This installs
a shared runtime/updater outside the app folder. Internet is needed only for
this setup and features that inherently download content. Default consent is No.
OFFLINE: Uses bundled WebView2 $($questFixed.version); no WebView2 download or
system runtime installation. On Windows 10, grants AppContainer read/execute
access to the bundled webview2 folder as required by Microsoft. It must be
extracted to writable local NTFS storage. The fixed runtime updates with the app.
Online app features still require internet; offline refers to runtime setup.

Neither package needs Node/npm, Rust, Visual Studio, Windows SDK or global Java.
Quest USB access can require Meta's ADB driver, developer mode and headset
debugging authorization. A working app window does not prove device access.
Settings/cache/signing keys still live in the Windows user data directory.
These packages are unsigned builds. Automated builds are uploaded for review.
Full third-party binary-license/source review and clean-Windows testing remain
release gates. Keep LICENSE, THIRD_PARTY.md and all bundled NOTICE/legal files.

Read-only check: powershell -NoProfile -ExecutionPolicy Bypass -File portable\Launch.ps1 -CheckOnly
SHA-256: compare the ZIP with its accompanying .sha256 file before extraction.
Source: https://github.com/yexca/quest-manager
WebView2: https://developer.microsoft.com/microsoft-edge/webview2/
Bundled WebView2 notices are preserved, including its third-party license viewer.
WebView2 terms: https://developer.microsoft.com/microsoft-edge/webview2/
"@
    [IO.File]::WriteAllText((Join-Path $questStage 'PORTABLE-README.txt'), $questReadme, [Text.UTF8Encoding]::new($false))
    Assert-PortablePrivacy $questStage
    $questZip = Join-Path $questOutput "$questName.zip"
    [IO.Compression.ZipFile]::CreateFromDirectory($questStage, $questZip, [IO.Compression.CompressionLevel]::Optimal, $true)
    $questHash = (Get-FileHash -LiteralPath $questZip -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText("$questZip.sha256", "$questHash  $questName.zip`n", [Text.UTF8Encoding]::new($false))
    $questResults += [pscustomobject]@{ mode = $questMode; zip = $questZip; zipBytes = (Get-Item -LiteralPath $questZip).Length; expandedBytes = (Get-ChildItem -LiteralPath $questStage -Recurse -File | Measure-Object Length -Sum).Sum; sha256 = $questHash }
}
$questResults | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $questOutput 'comparison.json') -Encoding UTF8
$questResults | Format-Table mode, zipBytes, expandedBytes
Write-Host "Portable comparison packages: $questOutput" -ForegroundColor Green
if ($env:GITHUB_ACTIONS -eq 'true' -and $env:GITHUB_OUTPUT) {
    "portable-directory=$questOutput" | Out-File -LiteralPath $env:GITHUB_OUTPUT -Encoding utf8 -Append
}
