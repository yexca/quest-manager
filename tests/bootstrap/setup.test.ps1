# Host-only regression tests. No downloads, elevation, system installation or devices.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '..\..\scripts\Environment.ps1')
. (Join-Path $PSScriptRoot '..\..\scripts\SystemPrerequisites.ps1')
$questTestCount = 0

function Assert-SetupTest {
    param([bool]$Condition, [string]$Message)
    if (!$Condition) { throw $Message }
}

function Assert-SetupThrows {
    param([scriptblock]$Action, [string]$Pattern)
    try { & $Action | Out-Null } catch {
        if ($_.Exception.Message -notmatch $Pattern) { throw }
        return
    }
    throw "Expected failure matching: $Pattern"
}

function Test-Setup {
    param([string]$Name, [scriptblock]$Action)
    & $Action
    $script:questTestCount++
    Write-Host "PASS: $Name"
}

function New-SetupState {
    param([string]$Cpp = 'compatible', [string]$Sdk = 'compatible', [string]$Webview = 'compatible')
    [pscustomobject]@{
        visualStudio = @(); windowsSdks = @(); webview2 = @()
        checks = @(
            [pscustomobject]@{ id = 'cpp'; name = 'C++'; status = $Cpp; required = 'synthetic' }
            [pscustomobject]@{ id = 'sdk'; name = 'SDK'; status = $Sdk; required = 'synthetic' }
            [pscustomobject]@{ id = 'webview'; name = 'WebView2'; status = $Webview; required = 'synthetic' }
        )
    }
}

Test-Setup 'version boundaries reject absent, malformed and older versions' {
    foreach ($questVersion in @('', 'unknown', '0.0.0.0', '109.0.9999.0')) {
        Assert-SetupTest (!(Test-QuestMinimumVersion $questVersion '110.0.1531.0')) 'An incompatible runtime was accepted.'
    }
    Assert-SetupTest (Test-QuestMinimumVersion '110.0.1531.0' '110.0.1531.0') 'Exact minimum rejected.'
    Assert-SetupTest (Test-QuestMinimumVersion '152.0.1.0' '110.0.1531.0') 'Newer runtime rejected.'
    Assert-SetupTest (!(Test-QuestMinimumVersion '18.0' '17.0' '18.0')) 'Unsupported VS major accepted.'
}

Test-Setup 'classification keeps missing, incomplete and compatible installations distinct' {
    Assert-SetupTest ((Get-QuestRequirement test test @() test).status -eq 'missing') 'Missing detection failed.'
    Assert-SetupTest ((Get-QuestRequirement test test @([pscustomobject]@{ compatible = $false }) test).status -eq 'incompatible') 'Incomplete installation accepted.'
    Assert-SetupTest ((Get-QuestRequirement test test @([pscustomobject]@{ compatible = $false }, [pscustomobject]@{ compatible = $true }) test).status -eq 'compatible') 'Side-by-side compatible installation missed.'
}

Test-Setup 'a missing SDK resource compiler makes its payload incomplete' {
    function Test-Path { param($LiteralPath, $PathType) return $LiteralPath -notlike '*\rc.exe' }
    Assert-SetupTest (!(Test-QuestFiles 'C:\EXAMPLE SDK' @('Include\um\Windows.h', 'bin\x64\rc.exe'))) 'Missing resource compiler accepted.'
}

Test-Setup 'repair selects a compatible VS 2022 instance before unsupported major versions' {
    $questState = New-SetupState
    $questState.visualStudio = @(
        [pscustomobject]@{ installationVersion = '18.0'; supportedVersion = $false; compatible = $false }
        [pscustomobject]@{ installationVersion = '17.14'; supportedVersion = $true; compatible = $false }
        [pscustomobject]@{ installationVersion = '17.12'; supportedVersion = $true; compatible = $true }
    )
    Assert-SetupTest ((Get-QuestCppRepairTarget $questState).installationVersion -eq '17.12') 'Repair selected the wrong VS instance.'
}

Test-Setup 'existing VS repair targets only the approved instance and waits without reboot' {
    $script:questInstallerCalls = @()
    function Invoke-QuestMicrosoftInstaller {
        param($Path, $Arguments, [switch]$Elevate)
        $script:questInstallerCalls += [pscustomobject]@{ arguments = $Arguments; elevated = $Elevate.IsPresent }
    }
    Install-QuestCppPrerequisites ([pscustomobject]@{ installationPath = 'C:\EXAMPLE Tools\VS' })
    Assert-SetupTest ($script:questInstallerCalls.Count -eq 2) 'Expected update followed by component modification.'
    Assert-SetupTest ($script:questInstallerCalls[0].arguments[0] -eq 'update' -and $script:questInstallerCalls[1].arguments[0] -eq 'modify') 'Unexpected repair actions.'
    foreach ($questCall in $script:questInstallerCalls) {
        Assert-SetupTest ($questCall.elevated -and $questCall.arguments -contains '--norestart' -and $questCall.arguments -contains 'C:\EXAMPLE Tools\VS') 'Repair did not preserve its target and reboot policy.'
        Assert-SetupTest ($questCall.arguments -notcontains '--wait' -and $questCall.arguments -notcontains '--force') 'Unsupported setup.exe or force-close argument.'
    }
    Assert-SetupTest ($script:questInstallerCalls[1].arguments -contains $QuestVersions.systemPrerequisites.windowsSdkComponent) 'SDK was not requested.'
}

Test-Setup 'a fresh C++ install uses the Build Tools bootstrapper with its wait flag' {
    function Get-QuestMicrosoftInstaller { param($Kind) Assert-SetupTest ($Kind -eq 'cpp') 'Wrong installer kind.'; 'EXAMPLE-BuildTools.exe' }
    function Invoke-QuestMicrosoftInstaller {
        param($Path, $Arguments, [switch]$Elevate)
        Assert-SetupTest ($Path -eq 'EXAMPLE-BuildTools.exe' -and $Elevate -and $Arguments -contains '--wait' -and $Arguments -contains '--norestart') 'Bootstrapper flags incorrect.'
        Assert-SetupTest ($Arguments -contains 'Microsoft.VisualStudio.Workload.VCTools') 'C++ workload missing.'
    }
    Install-QuestCppPrerequisites $null
}

Test-Setup 'compatible systems never prompt or install' {
    function Get-QuestSystemPrerequisites { New-SetupState }
    function Confirm-QuestSystemInstall { throw 'Unexpected prompt' }
    function Install-QuestCppPrerequisites { throw 'Unexpected installation' }
    function Get-QuestMicrosoftInstaller { throw 'Unexpected download' }
    $questState = Ensure-QuestSystemPrerequisites
    Assert-SetupTest ($questState.checks.Count -eq 3) 'Incomplete result.'
}

Test-Setup 'check-only and noninteractive failures never prompt or install' {
    function Get-QuestSystemPrerequisites { New-SetupState -Sdk missing }
    function Confirm-QuestSystemInstall { throw 'Unexpected prompt' }
    function Install-QuestCppPrerequisites { throw 'Unexpected installation' }
    Assert-SetupThrows { Ensure-QuestSystemPrerequisites -CheckOnly } 'missing or incompatible'
    Assert-SetupThrows { Ensure-QuestSystemPrerequisites -NonInteractive } 'missing or incompatible'
}

Test-Setup 'declining installation has no download or installer side effects' {
    function Get-QuestSystemPrerequisites { New-SetupState -Cpp missing }
    function Confirm-QuestSystemInstall { $false }
    function Install-QuestCppPrerequisites { throw 'Unexpected installation' }
    function Get-QuestMicrosoftInstaller { throw 'Unexpected download' }
    Assert-SetupThrows { Ensure-QuestSystemPrerequisites } 'declined'
}

Test-Setup 'an SDK-only failure still requests C++/SDK repair and rechecks it' {
    $script:questRepairs = 0
    function Get-QuestSystemPrerequisites {
        if ($script:questRepairs -eq 0) { New-SetupState -Sdk incompatible } else { New-SetupState }
    }
    function Confirm-QuestSystemInstall { $true }
    function Install-QuestCppPrerequisites { $script:questRepairs++ }
    Ensure-QuestSystemPrerequisites | Out-Null
    Assert-SetupTest ($script:questRepairs -eq 1) 'SDK repair was not run once.'
}

Test-Setup 'successful installer exit does not bypass failed component detection' {
    function Get-QuestSystemPrerequisites { New-SetupState -Sdk missing }
    function Confirm-QuestSystemInstall { $true }
    function Install-QuestCppPrerequisites { }
    Assert-SetupThrows { Ensure-QuestSystemPrerequisites } 'checks still fail'
}

Test-Setup 'outdated WebView2 is upgraded only after consent and then rechecked' {
    $script:questWebviewInstalls = 0
    function Get-QuestSystemPrerequisites {
        if ($script:questWebviewInstalls -eq 0) { New-SetupState -Webview incompatible } else { New-SetupState }
    }
    function Confirm-QuestSystemInstall { $true }
    function Get-QuestMicrosoftInstaller { 'EXAMPLE-WebView2.exe' }
    function Invoke-QuestMicrosoftInstaller { $script:questWebviewInstalls++ }
    Ensure-QuestSystemPrerequisites | Out-Null
    Assert-SetupTest ($script:questWebviewInstalls -eq 1) 'Unexpected WebView2 installer calls.'
}

Test-Setup 'untrusted system installers are rejected' {
    function Get-AuthenticodeSignature { [pscustomobject]@{ Status = 'NotSigned'; SignerCertificate = $null } }
    Assert-SetupThrows { Assert-QuestMicrosoftSignature 'EXAMPLE-installer.exe' } 'valid Microsoft signature'
}

Test-Setup 'a valid signature from another publisher is rejected' {
    function Get-AuthenticodeSignature { [pscustomobject]@{ Status = 'Valid'; SignerCertificate = [pscustomobject]@{ Subject = 'CN=Example, O=Example Corporation' } } }
    Assert-SetupThrows { Assert-QuestMicrosoftSignature 'EXAMPLE-installer.exe' } 'valid Microsoft signature'
}

Test-Setup 'installer failure and restart requirements stop setup' {
    function Assert-QuestMicrosoftSignature { }
    function Start-Process { [pscustomobject]@{ ExitCode = 3010 } }
    Assert-SetupThrows { Invoke-QuestMicrosoftInstaller 'EXAMPLE.exe' @('--quiet') } 'requires a restart'
    function Start-Process { [pscustomobject]@{ ExitCode = 1603 } }
    Assert-SetupThrows { Invoke-QuestMicrosoftInstaller 'EXAMPLE.exe' @('--quiet') } 'exit 1603'
}

Test-Setup 'installer arguments preserve paths with spaces and never request reboot' {
    function Assert-QuestMicrosoftSignature { }
    function Start-Process {
        param($FilePath, $ArgumentList, $WorkingDirectory, $WindowStyle, [switch]$Wait, [switch]$PassThru, $Verb)
        Assert-SetupTest ($ArgumentList -eq '"modify" "--installPath" "C:\EXAMPLE Tools\VS" "--quiet" "--norestart"') 'Installer argument quoting changed.'
        Assert-SetupTest ($Wait -and $PassThru -and $Verb -eq 'RunAs') 'Elevation/wait flags missing.'
        [pscustomobject]@{ ExitCode = 0 }
    }
    Invoke-QuestMicrosoftInstaller 'EXAMPLE.exe' @('modify', '--installPath', 'C:\EXAMPLE Tools\VS', '--quiet', '--norestart') -Elevate
}

Test-Setup 'missing project Node is rejected even when a system Node is available' {
    $questSavedNode = $script:QuestNode
    try {
        $script:QuestNode = Join-Path $QuestEnv 'EXAMPLE-missing-node.exe'
        Assert-SetupThrows { Assert-QuestNode } 'Project-local Node/npm are missing'
    } finally { $script:QuestNode = $questSavedNode }
}

Write-Host "$questTestCount bootstrap tests passed."
