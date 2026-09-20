param([switch]$CheckOnly, [switch]$NonInteractive)
$ErrorActionPreference = 'Stop'

function Get-PortableWebViewVersion {
    foreach ($key in @('HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients', 'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients', 'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients')) {
        $value = Get-ItemProperty -LiteralPath "$key\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -Name pv -ErrorAction SilentlyContinue
        $parsed = $null
        if ($value -and [version]::TryParse([string]$value.pv, [ref]$parsed)) { $parsed }
    }
}

function Confirm-PortableInstall {
    param([switch]$NonInteractive)
    if ($NonInteractive -or [Console]::IsInputRedirected) { return $false }
    Write-Host 'Microsoft WebView2 needs installation or updating. This downloads from Microsoft and installs a shared runtime/updater outside this portable folder.'
    return ((Read-Host 'Install/update WebView2 now? [y/N]').Trim() -match '^(?i:y|yes)$')
}

function Install-PortableWebView {
    # Mutable Evergreen endpoint: authenticate the Microsoft publisher before execution.
    $folder = Join-Path ([IO.Path]::GetTempPath()) ('QuestManager-WebView2-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $folder | Out-Null
    $installer = Join-Path $folder 'MicrosoftEdgeWebview2Setup.exe'
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -OutFile $installer -UseBasicParsing -TimeoutSec 300
        $signature = Get-AuthenticodeSignature -LiteralPath $installer
        if ($signature.Status -ne 'Valid' -or !$signature.SignerCertificate -or $signature.SignerCertificate.Subject -notmatch '(^|,\s*)O=Microsoft Corporation(,|$)') {
            throw 'The WebView2 installer does not have a valid Microsoft signature. Installation refused.'
        }
        $result = Start-Process -FilePath $installer -ArgumentList '/silent /install' -WindowStyle Hidden -Wait -PassThru
        if ($result.ExitCode -in @(3010, 1641)) { throw 'WebView2 requires a restart. Restart Windows yourself, then launch Quest Manager again.' }
        if ($result.ExitCode -ne 0) { throw "WebView2 installation failed (exit $($result.ExitCode))." }
    } finally {
        # Only remove the exact file and unique empty directory created above.
        if (Test-Path -LiteralPath $installer) { Remove-Item -LiteralPath $installer -Force }
        if (Test-Path -LiteralPath $folder) { Remove-Item -LiteralPath $folder }
    }
}

function Initialize-PortableRuntime {
    param([string]$Root, $Manifest, [switch]$CheckOnly, [switch]$NonInteractive)
    if ($Manifest.mode -notin @('online', 'offline')) { throw 'Invalid portable manifest mode.' }
    $minimum = [version]$Manifest.minimumWebView2
    if ($Manifest.mode -eq 'offline') {
        $runtime = Join-Path $Root 'webview2'
        $browser = Join-Path $runtime 'msedgewebview2.exe'
        if (!(Test-Path -LiteralPath $browser -PathType Leaf)) { throw 'Bundled WebView2 is missing. Extract the complete offline ZIP again.' }
        $version = [version](Get-Item -LiteralPath $browser).VersionInfo.ProductVersion
        if ($version -lt $minimum -or $version -ne [version]$Manifest.fixedWebView2) { throw 'Bundled WebView2 version is incorrect. Extract the complete offline ZIP again.' }
        if (!$CheckOnly) {
            # Microsoft requires these AppContainer read/execute permissions on Windows 10
            # for unpackaged Fixed Version >= 120. Only the bundled runtime is affected.
            if ([Environment]::OSVersion.Version.Build -lt 22000 -and $version.Major -ge 120) {
                & "$env:SystemRoot\System32\icacls.exe" $runtime /grant '*S-1-15-2-2:(OI)(CI)(RX)' '*S-1-15-2-1:(OI)(CI)(RX)' | Out-Null
                if ($LASTEXITCODE -ne 0) { throw 'Cannot prepare bundled WebView2 permissions. Extract to a writable local NTFS folder and retry.' }
            }
            $env:WEBVIEW2_BROWSER_EXECUTABLE_FOLDER = $runtime
        }
        return [pscustomobject]@{ mode = 'offline'; version = "$version"; runtime = 'bundled' }
    }
    $version = @(Get-PortableWebViewVersion | Where-Object { $_ -ge $minimum } | Sort-Object -Descending) | Select-Object -First 1
    if (!$version) {
        if ($CheckOnly -or !(Confirm-PortableInstall -NonInteractive:$NonInteractive)) { throw "WebView2 >= $minimum is required. Use the offline package or install WebView2 from https://developer.microsoft.com/microsoft-edge/webview2/." }
        Install-PortableWebView
        $version = @(Get-PortableWebViewVersion | Where-Object { $_ -ge $minimum } | Sort-Object -Descending) | Select-Object -First 1
        if (!$version) { throw 'No compatible WebView2 was detected after installation. Restart Windows if requested by the installer, then retry.' }
    }
    if (!$CheckOnly) { Remove-Item Env:WEBVIEW2_BROWSER_EXECUTABLE_FOLDER -ErrorAction SilentlyContinue }
    return [pscustomobject]@{ mode = 'online'; version = "$version"; runtime = 'system' }
}

function Start-PortableQuest {
    param([switch]$CheckOnly, [switch]$NonInteractive)
    if ($env:OS -ne 'Windows_NT' -or ![Environment]::Is64BitProcess -or $env:PROCESSOR_ARCHITECTURE -ne 'AMD64' -or [Environment]::OSVersion.Version.Build -lt 19041) {
        throw 'This package requires Windows 10 2004 or later / Windows 11, x64. Use Start-QuestManager.cmd with 64-bit PowerShell.'
    }
    $root = Split-Path -Parent $PSScriptRoot
    $manifest = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'manifest.json') -Raw | ConvertFrom-Json
    foreach ($file in @('quest-manager.exe', 'platform-tools/adb.exe', 'platform-tools/AdbWinApi.dll', 'platform-tools/AdbWinUsbApi.dll', 'aapt2/aapt2.exe', 'apk-tools/apktool.jar', 'apk-tools/apksigner.jar', 'apk-tools/zipalign.exe', 'apk-tools/jre/bin/java.exe')) {
        if (!(Test-Path -LiteralPath (Join-Path $root $file) -PathType Leaf)) { throw "Portable files are missing: $file. Extract the complete ZIP again." }
    }
    $state = Initialize-PortableRuntime -Root $root -Manifest $manifest -CheckOnly:$CheckOnly -NonInteractive:$NonInteractive
    Write-Host "WebView2 $($state.version) ($($state.runtime)); $($state.mode) portable."
    if (!$CheckOnly) { Start-Process -FilePath (Join-Path $root 'quest-manager.exe') -WorkingDirectory $root | Out-Null }
}

if ($MyInvocation.InvocationName -ne '.') {
    try { Start-PortableQuest -CheckOnly:$CheckOnly -NonInteractive:$NonInteractive } catch { Write-Host $_.Exception.Message -ForegroundColor Red; exit 1 }
}
