$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '../../packaging/Launch.ps1')
$script:portableTests = 0
function Check($Condition, [string]$Message) { if (!$Condition) { throw $Message } }
function Expect-Failure([scriptblock]$Action, [string]$Text) {
    try { & $Action; throw 'Expected failure did not occur.' } catch {
        if ($_.Exception.Message -notlike "*$Text*") { throw }
    }
}
function Test-Portable([string]$Name, [scriptblock]$Action) { & $Action; $script:portableTests++; Write-Host "PASS $Name" }
$online = [pscustomobject]@{ mode = 'online'; minimumWebView2 = '110.0.1531.0' }
$offline = [pscustomobject]@{ mode = 'offline'; minimumWebView2 = '110.0.1531.0'; fixedWebView2 = '153.0.4234.48' }
$savedRuntime = $env:WEBVIEW2_BROWSER_EXECUTABLE_FOLDER
try {
    Test-Portable 'compatible online runtime is reused without consent or install' {
        function Get-PortableWebViewVersion { [version]'153.0.4234.48' }
        function Confirm-PortableInstall { throw 'Unexpected prompt.' }
        function Install-PortableWebView { throw 'Unexpected system change.' }
        $env:WEBVIEW2_BROWSER_EXECUTABLE_FOLDER = 'C:\EXAMPLE\unrelated-runtime'
        $state = Initialize-PortableRuntime 'C:\EXAMPLE' $online
        Check ($state.runtime -eq 'system') 'Wrong runtime selected.'
        Check (!$env:WEBVIEW2_BROWSER_EXECUTABLE_FOLDER) 'Inherited runtime override was not cleared.'
    }
    Test-Portable 'check-only refuses outdated runtime without prompting' {
        function Get-PortableWebViewVersion { [version]'109.0.0.0' }
        function Confirm-PortableInstall { throw 'Unexpected prompt.' }
        function Install-PortableWebView { throw 'Unexpected system change.' }
        Expect-Failure { Initialize-PortableRuntime 'C:\EXAMPLE' $online -CheckOnly } 'is required'
    }
    Test-Portable 'noninteractive never authorizes installation' {
        Check (!(Confirm-PortableInstall -NonInteractive)) 'Consent was granted.'
        function Get-PortableWebViewVersion { }
        function Install-PortableWebView { throw 'Unexpected system change.' }
        Expect-Failure { Initialize-PortableRuntime 'C:\EXAMPLE' $online -NonInteractive } 'is required'
    }
    Test-Portable 'declining consent leaves system unchanged' {
        function Get-PortableWebViewVersion { }
        function Confirm-PortableInstall { $false }
        function Install-PortableWebView { throw 'Unexpected system change.' }
        Expect-Failure { Initialize-PortableRuntime 'C:\EXAMPLE' $online } 'is required'
    }
    Test-Portable 'successful installation is rechecked before use' {
        $script:installed = $false
        function Get-PortableWebViewVersion { if ($script:installed) { [version]'153.0.4234.48' } }
        function Confirm-PortableInstall { $true }
        function Install-PortableWebView { $script:installed = $true }
        $state = Initialize-PortableRuntime 'C:\EXAMPLE' $online
        Check ($script:installed -and $state.version -eq '153.0.4234.48') 'Installation was not rechecked.'
    }
    Test-Portable 'failed installation recheck blocks app launch' {
        function Get-PortableWebViewVersion { }
        function Confirm-PortableInstall { $true }
        function Install-PortableWebView { }
        Expect-Failure { Initialize-PortableRuntime 'C:\EXAMPLE' $online } 'after installation'
    }
    Test-Portable 'offline package does not fall back to system or download' {
        function Test-Path { $false }
        function Get-PortableWebViewVersion { throw 'Unexpected system runtime lookup.' }
        function Install-PortableWebView { throw 'Unexpected download.' }
        Expect-Failure { Initialize-PortableRuntime 'C:\EXAMPLE' $offline } 'Bundled WebView2 is missing'
    }
    Test-Portable 'offline check-only validates exact runtime without changing environment' {
        function Test-Path { $true }
        function Get-Item { [pscustomobject]@{ VersionInfo = [pscustomobject]@{ ProductVersion = '153.0.4234.48' } } }
        function Get-PortableWebViewVersion { throw 'Unexpected system runtime lookup.' }
        $env:WEBVIEW2_BROWSER_EXECUTABLE_FOLDER = 'C:\EXAMPLE\unchanged'
        $state = Initialize-PortableRuntime 'C:\EXAMPLE With Spaces' $offline -CheckOnly
        Check ($state.runtime -eq 'bundled') 'Wrong runtime selected.'
        Check ($env:WEBVIEW2_BROWSER_EXECUTABLE_FOLDER -eq 'C:\EXAMPLE\unchanged') 'Check-only changed environment.'
    }
    Test-Portable 'incorrect bundled runtime version is rejected' {
        function Test-Path { $true }
        function Get-Item { [pscustomobject]@{ VersionInfo = [pscustomobject]@{ ProductVersion = '152.0.0.0' } } }
        Expect-Failure { Initialize-PortableRuntime 'C:\EXAMPLE' $offline -CheckOnly } 'version is incorrect'
    }
    Test-Portable 'unknown manifest mode is rejected' {
        Expect-Failure { Initialize-PortableRuntime 'C:\EXAMPLE' ([pscustomobject]@{mode='invalid'}) } 'Invalid portable manifest'
    }
    Test-Portable 'unsigned installer is never executed' {
        function New-Item { }
        function Invoke-WebRequest { }
        function Test-Path { $false }
        function Get-AuthenticodeSignature { [pscustomobject]@{Status='NotSigned';SignerCertificate=$null} }
        function Start-Process { throw 'Installer must not execute.' }
        Expect-Failure { Install-PortableWebView } 'valid Microsoft signature'
    }
    Test-Portable 'signed installer failure is surfaced' {
        function New-Item { }
        function Invoke-WebRequest { }
        function Test-Path { $false }
        function Get-AuthenticodeSignature { [pscustomobject]@{Status='Valid';SignerCertificate=[pscustomobject]@{Subject='CN=Microsoft Corporation, O=Microsoft Corporation, C=US'}} }
        function Start-Process { [pscustomobject]@{ExitCode=1} }
        Expect-Failure { Install-PortableWebView } 'installation failed'
    }
    Test-Portable 'restart-required installer exit never launches the app' {
        function New-Item { }
        function Invoke-WebRequest { }
        function Test-Path { $false }
        function Get-AuthenticodeSignature { [pscustomobject]@{Status='Valid';SignerCertificate=[pscustomobject]@{Subject='CN=Microsoft Corporation, O=Microsoft Corporation, C=US'}} }
        function Start-Process { [pscustomobject]@{ExitCode=3010} }
        Expect-Failure { Install-PortableWebView } 'requires a restart'
    }
} finally { $env:WEBVIEW2_BROWSER_EXECUTABLE_FOLDER = $savedRuntime }
Write-Host "$script:portableTests portable launcher tests passed. No installers were run."
