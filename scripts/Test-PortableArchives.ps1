param(
    [Parameter(Mandatory)][string]$Directory,
    [ValidateSet('Online', 'Offline', 'Both')][string]$Variant = 'Both',
    [Parameter(Mandatory)][string]$ExpectedCommit,
    [switch]$RequireCleanSource
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Assert-ReleaseSource.ps1')
$questVersion = Get-QuestReleaseVersion (Split-Path -Parent $PSScriptRoot)
$questFixed = Get-Content -LiteralPath (Join-Path $PSScriptRoot '../packaging/webview2.json') -Raw | ConvertFrom-Json
$questModes = @($Variant.ToLowerInvariant())
if ($Variant -eq 'Both') { $questModes = @('online', 'offline') }
Add-Type -AssemblyName System.IO.Compression.FileSystem
if ($ExpectedCommit -cnotmatch '^[0-9a-f]{40}$') { throw 'Expected source commit must be a full Git SHA.' }
$questZips = @(Get-ChildItem -LiteralPath $Directory -File -Filter '*.zip')
if ($questZips.Count -ne $questModes.Count) { throw 'Unexpected ZIP count in portable output.' }
foreach ($questMode in $questModes) {
    $questName = "quest-manager-v$questVersion-windows-x64-portable-$questMode"
    $questZip = Join-Path $Directory "$questName.zip"
    $questHash = (Get-FileHash -LiteralPath $questZip -Algorithm SHA256).Hash.ToLowerInvariant()
    $questSidecar = (Get-Content -LiteralPath "$questZip.sha256" -Raw).Trim()
    if ($questSidecar -cne "$questHash  $questName.zip") { throw 'Portable ZIP checksum mismatch.' }
    $questArchive = [IO.Compression.ZipFile]::OpenRead($questZip)
    try {
        $questEntries = @($questArchive.Entries | ForEach-Object { $_.FullName.Replace('\', '/') })
        if (@($questEntries | Sort-Object -Unique).Count -ne $questEntries.Count) { throw 'Duplicate ZIP entries.' }
        foreach ($questEntry in $questEntries) {
            if (!$questEntry.StartsWith("$questName/", [StringComparison]::Ordinal) -or $questEntry -match '(^|/)\.\.(/|$)|:') { throw 'Unsafe path in portable ZIP.' }
            if ($questEntry -match '(^|/)(build-environment|installed-versions|device-settings)\.json$|\.(pfx|p12|jks|keystore|log)$|(^|/)adbkey(\.pub)?$') { throw 'Private file in portable ZIP.' }
        }
        foreach ($questRequired in @('quest-manager.exe', 'Start-QuestManager.cmd', 'portable/Launch.ps1', 'portable/manifest.json', 'PORTABLE-README.txt', 'LICENSE', 'THIRD_PARTY.md', 'platform-tools/adb.exe', 'platform-tools/AdbWinApi.dll', 'platform-tools/AdbWinUsbApi.dll', 'aapt2/aapt2.exe', 'apk-tools/apktool.jar', 'apk-tools/apksigner.jar', 'apk-tools/zipalign.exe', 'apk-tools/libwinpthread-1.dll', 'apk-tools/jre/bin/java.exe', 'apk-tools/jre/bin/keytool.exe')) {
            if ($questEntries -cnotcontains "$questName/$questRequired") { throw "Missing portable resource: $questRequired" }
        }
        $questManifestEntry = $questArchive.Entries | Where-Object { $_.FullName.Replace('\', '/') -ceq "$questName/portable/manifest.json" }
        $questReader = [IO.StreamReader]::new($questManifestEntry.Open())
        try { $questManifest = $questReader.ReadToEnd() | ConvertFrom-Json } finally { $questReader.Dispose() }
        if ($questManifest.version -cne $questVersion -or $questManifest.mode -cne $questMode -or $questManifest.architecture -cne 'x64' -or $questManifest.sourceCommit -cne $ExpectedCommit) { throw 'Portable manifest does not match the requested source/version/variant.' }
        if ($RequireCleanSource -and $questManifest.sourceHasLocalChanges -cne $false) { throw 'Portable archive was built from modified source.' }
        if ($questMode -eq 'offline') {
            if ($questEntries -cnotcontains "$questName/webview2/msedgewebview2.exe" -or $questManifest.fixedWebView2 -cne $questFixed.version) { throw 'Offline runtime is absent or mismatched.' }
        } elseif ($questManifest.fixedWebView2 -or @($questEntries | Where-Object { $_.StartsWith("$questName/webview2/") }).Count) { throw 'Online package unexpectedly includes a fixed runtime.' }
    } finally { $questArchive.Dispose() }
    Write-Host "Verified $questMode ZIP: checksum, resources and source identity."
}
