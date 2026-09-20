# Detection is read-only. Installer downloads and elevation happen only after consent.
function Test-QuestMinimumVersion {
    param([string]$Actual, [string]$Minimum, [string]$MaximumExclusive = '')
    $questVersion = $null
    if (![version]::TryParse($Actual, [ref]$questVersion)) { return $false }
    return $questVersion -ge [version]$Minimum -and (!$MaximumExclusive -or $questVersion -lt [version]$MaximumExclusive)
}

function Test-QuestFiles {
    param([string]$Root, [string[]]$Files)
    foreach ($questFile in $Files) {
        if (!(Test-Path -LiteralPath (Join-Path $Root $questFile) -PathType Leaf)) { return $false }
    }
    return $true
}

function Get-QuestVisualStudio {
    $questVswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (!(Test-Path -LiteralPath $questVswhere)) { return }
    $questJson = & $questVswhere -products '*' -all -format json
    if ($LASTEXITCODE -ne 0) { throw 'Visual Studio discovery failed. Repair Visual Studio Installer and retry.' }
    $questInstances = $questJson | Out-String | ConvertFrom-Json
    foreach ($questInstance in $questInstances) {
        if (!$questInstance) { continue }
        $questVersionFile = Join-Path $questInstance.installationPath 'VC\Auxiliary\Build\Microsoft.VCToolsVersion.default.txt'
        $questMsvc = ''
        if (Test-Path -LiteralPath $questVersionFile) { $questMsvc = (Get-Content -LiteralPath $questVersionFile -Raw).Trim() }
        $questComplete = $false
        if ($questMsvc -match '^\d+\.\d+\.\d+$') {
            $questComplete = Test-QuestFiles -Root (Join-Path $questInstance.installationPath "VC\Tools\MSVC\$questMsvc") -Files @('bin\Hostx64\x64\cl.exe', 'bin\Hostx64\x64\link.exe', 'include\vcruntime.h', 'lib\x64\vcruntime.lib')
        }
        $questSupportedVs = Test-QuestMinimumVersion $questInstance.installationVersion $QuestVersions.systemPrerequisites.visualStudioMinimum $QuestVersions.systemPrerequisites.visualStudioMaximumExclusive
        [pscustomobject]@{
            displayName = $questInstance.displayName
            installationPath = $questInstance.installationPath
            installationVersion = $questInstance.installationVersion
            msvcVersion = $questMsvc
            supportedVersion = $questSupportedVs
            compatible = $questSupportedVs -and ($questInstance.PSObject.Properties.Name -contains 'isComplete') -and $questInstance.isComplete -and ($questInstance.PSObject.Properties.Name -contains 'isLaunchable') -and $questInstance.isLaunchable -and $questComplete -and (Test-QuestMinimumVersion $questMsvc $QuestVersions.systemPrerequisites.msvcMinimum)
        }
    }
}

function Get-QuestWindowsSdk {
    $questRoots = @(foreach ($questKey in @('HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots', 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows Kits\Installed Roots')) {
        $questValue = Get-ItemProperty -LiteralPath $questKey -Name KitsRoot10 -ErrorAction SilentlyContinue
        if ($questValue) { $questValue.KitsRoot10 }
    })
    $questRoots += Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10'
    foreach ($questSdkRoot in @($questRoots | ForEach-Object { $_.TrimEnd('\') } | Sort-Object -Unique)) {
        $questLib = Join-Path $questSdkRoot 'Lib'
        if (!(Test-Path -LiteralPath $questLib)) { continue }
        foreach ($questSdk in Get-ChildItem -LiteralPath $questLib -Directory) {
            $questSdkVersion = $null
            if (![version]::TryParse($questSdk.Name, [ref]$questSdkVersion)) { continue }
            $questFiles = @("Lib\$($questSdk.Name)\um\x64\kernel32.lib", "Lib\$($questSdk.Name)\ucrt\x64\ucrt.lib", "Include\$($questSdk.Name)\um\Windows.h", "Include\$($questSdk.Name)\ucrt\stdio.h", "bin\$($questSdk.Name)\x64\rc.exe")
            [pscustomobject]@{
                version = $questSdk.Name
                root = $questSdkRoot
                compatible = (Test-QuestMinimumVersion $questSdk.Name $QuestVersions.systemPrerequisites.windowsSdkMinimum) -and (Test-QuestFiles $questSdkRoot $questFiles)
            }
        }
    }
}

function Get-QuestWebView {
    # Microsoft documents this product GUID; a display-name match also accepts unrelated products.
    foreach ($questKey in @('HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients', 'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients', 'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients')) {
        $questValue = Get-ItemProperty -LiteralPath "$questKey\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -Name pv -ErrorAction SilentlyContinue
        if ($questValue) {
            [pscustomobject]@{
                name = 'Microsoft Edge WebView2 Runtime'
                pv = [string]$questValue.pv
                compatible = Test-QuestMinimumVersion ([string]$questValue.pv) $QuestVersions.systemPrerequisites.webview2Minimum
            }
        }
    }
}

function Get-QuestRequirement {
    param([string]$Id, [string]$Name, [object[]]$Installed, [string]$Required)
    $questStatus = 'missing'
    if ($Installed.Count -gt 0) { $questStatus = 'incompatible' }
    if (@($Installed | Where-Object { $_.compatible }).Count -gt 0) { $questStatus = 'compatible' }
    [pscustomobject]@{ id = $Id; name = $Name; status = $questStatus; required = $Required }
}

function Get-QuestSystemPrerequisites {
    $questVs = @(Get-QuestVisualStudio)
    $questSdks = @(Get-QuestWindowsSdk)
    $questWebviews = @(Get-QuestWebView)
    $questPolicy = $QuestVersions.systemPrerequisites
    [pscustomobject]@{
        visualStudio = $questVs; windowsSdks = $questSdks; webview2 = $questWebviews
        checks = @(
            Get-QuestRequirement 'cpp' 'Visual Studio C++ tools' $questVs "VS $($questPolicy.visualStudioMinimum) to below $($questPolicy.visualStudioMaximumExclusive), MSVC >= $($questPolicy.msvcMinimum), complete x64 compiler/linker"
            Get-QuestRequirement 'sdk' 'Windows SDK' $questSdks ">= $($questPolicy.windowsSdkMinimum), x64 libraries, headers and resource compiler"
            Get-QuestRequirement 'webview' 'WebView2 Runtime' $questWebviews ">= $($questPolicy.webview2Minimum)"
        )
    }
}

function Show-QuestSystemPrerequisites {
    param($State)
    foreach ($questCheck in $State.checks) { Write-Host "[$($questCheck.status)] $($questCheck.name): $($questCheck.required)" }
    foreach ($questVs in $State.visualStudio) { Write-Host "  Found $($questVs.displayName) $($questVs.installationVersion), MSVC $($questVs.msvcVersion)" }
    foreach ($questSdk in $State.windowsSdks) { Write-Host "  Found Windows SDK $($questSdk.version)" }
    foreach ($questWebview in $State.webview2) { Write-Host "  Found WebView2 $($questWebview.pv)" }
}

function Assert-QuestMicrosoftSignature {
    param([string]$Path)
    $questSignature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($questSignature.Status -ne 'Valid' -or !$questSignature.SignerCertificate -or $questSignature.SignerCertificate.Subject -notmatch '(^|,\s*)O=Microsoft Corporation(,|$)') {
        throw 'The system installer does not have a valid Microsoft signature. Installation refused.'
    }
}

function Get-QuestMicrosoftInstaller {
    param([ValidateSet('cpp', 'webview')][string]$Kind)
    $questUrl = 'https://aka.ms/vs/17/release/vs_buildtools.exe'
    $questName = 'vs_buildtools.exe'
    if ($Kind -eq 'webview') {
        $questUrl = 'https://go.microsoft.com/fwlink/p/?LinkId=2124703'
        $questName = 'MicrosoftEdgeWebview2Setup.exe'
    }
    $questDirectory = Join-Path $QuestEnv 'downloads\system-installers'
    New-Item -ItemType Directory -Path $questDirectory -Force | Out-Null
    $questFile = Join-Path $questDirectory $questName
    Write-Host "Downloading Microsoft installer: $questUrl"
    Invoke-WebRequest -Uri $questUrl -OutFile $questFile -UseBasicParsing -TimeoutSec 300
    Assert-QuestMicrosoftSignature $questFile
    return $questFile
}

function Invoke-QuestMicrosoftInstaller {
    param([string]$Path, [string[]]$Arguments, [switch]$Elevate)
    Assert-QuestMicrosoftSignature $Path
    # Start-Process joins arguments into one command line. Quote every token;
    # never pass installer arguments through a shell or an expression evaluator.
    $questQuoted = foreach ($questArgument in $Arguments) {
        if ($questArgument -match '["\r\n]' -or $questArgument.EndsWith('\')) { throw 'Invalid system installer argument.' }
        '"' + $questArgument + '"'
    }
    $questParameters = @{ FilePath = $Path; ArgumentList = ($questQuoted -join ' '); WorkingDirectory = $QuestRoot; WindowStyle = 'Hidden'; Wait = $true; PassThru = $true }
    if ($Elevate) { $questParameters.Verb = 'RunAs' }
    try { $questProcess = Start-Process @questParameters } catch { throw "Could not run the Microsoft installer (permission may have been declined): $($_.Exception.Message)" }
    if ($questProcess.ExitCode -in @(3010, 1641)) { throw 'The system installer requires a restart. Restart Windows yourself, then rerun .\run-install.ps1.' }
    if ($questProcess.ExitCode -ne 0) { throw "Microsoft installer failed (exit $($questProcess.ExitCode)). Complete setup manually and rerun .\run-install.ps1." }
}

function Get-QuestCppRepairTarget {
    param($State)
    # Prefer an already usable VS 2022 instance; do not modify other major versions.
    $State.visualStudio | Where-Object { $_.supportedVersion } | Sort-Object @{Expression = { $_.compatible }; Descending = $true}, @{Expression = { [version]$_.installationVersion }; Descending = $true} | Select-Object -First 1
}

function Install-QuestCppPrerequisites {
    param($Target)
    $questComponents = @('--add', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '--add', $QuestVersions.systemPrerequisites.windowsSdkComponent)
    if ($Target) {
        $questSetup = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\setup.exe'
        Invoke-QuestMicrosoftInstaller -Path $questSetup -Arguments @('update', '--installPath', $Target.installationPath, '--channelUri', 'https://aka.ms/vs/17/release/channel', '--quiet', '--norestart') -Elevate
        Invoke-QuestMicrosoftInstaller -Path $questSetup -Arguments (@('modify', '--installPath', $Target.installationPath, '--channelId', 'VisualStudio.17.Release', '--quiet', '--norestart') + $questComponents) -Elevate
    } else {
        $questSetup = Get-QuestMicrosoftInstaller 'cpp'
        Invoke-QuestMicrosoftInstaller -Path $questSetup -Arguments (@('--quiet', '--wait', '--norestart', '--add', 'Microsoft.VisualStudio.Workload.VCTools') + $questComponents) -Elevate
    }
}

function Confirm-QuestSystemInstall {
    param([string]$Action, [switch]$NonInteractive)
    Write-Host $Action
    if ($NonInteractive -or [Console]::IsInputRedirected) { return $false }
    try { $questAnswer = Read-Host 'Proceed with this system change? [y/N]' } catch { return $false }
    return $questAnswer.Trim() -match '^(?i:y|yes)$'
}

function Ensure-QuestSystemPrerequisites {
    param([switch]$NonInteractive, [switch]$CheckOnly)
    $questState = Get-QuestSystemPrerequisites
    Show-QuestSystemPrerequisites $questState
    $questMissing = @($questState.checks | Where-Object { $_.status -ne 'compatible' })
    if ($questMissing.Count -eq 0) { return $questState }
    if ($CheckOnly -or $NonInteractive) {
        throw 'System prerequisites are missing or incompatible. Run .\run-install.ps1 interactively to choose installation, or see docs/getting-started.md for manual setup.'
    }
    if (@($questMissing | Where-Object { $_.id -in @('cpp', 'sdk') }).Count -gt 0) {
        $questTarget = Get-QuestCppRepairTarget $questState
        $questAction = 'Install Visual Studio 2022 Build Tools with x64 C++ tools and Windows SDK 26100 alongside any other Visual Studio versions.'
        if ($questTarget) { $questAction = "Update $($questTarget.displayName) at '$($questTarget.installationPath)' to the current VS 2022 release and add/repair x64 C++ tools and Windows SDK 26100." }
        $questAction += ' This changes system components, can download several GB, and requires administrator approval (possibly twice). Close Visual Studio first. No automatic reboot is requested.'
        if (!(Confirm-QuestSystemInstall $questAction)) { throw 'System installation declined. Install compatible C++ tools/SDK manually; see docs/getting-started.md.' }
        Install-QuestCppPrerequisites $questTarget
        $questState = Get-QuestSystemPrerequisites
        if (@($questState.checks | Where-Object { $_.id -in @('cpp', 'sdk') -and $_.status -ne 'compatible' }).Count -gt 0) {
            Show-QuestSystemPrerequisites $questState
            throw 'C++/SDK checks still fail after installation. Repair the reported components in Visual Studio Installer, then rerun .\run-install.ps1.'
        }
    }
    if (@($questState.checks | Where-Object { $_.id -eq 'webview' -and $_.status -ne 'compatible' }).Count -gt 0) {
        if (!(Confirm-QuestSystemInstall 'Install or update Microsoft Edge WebView2 Evergreen Runtime using the official bootstrapper. This changes the shared runtime and its automatic updater; installation normally uses the current user unless elevated.')) { throw 'WebView2 installation declined. Install a compatible runtime manually; see docs/getting-started.md.' }
        $questSetup = Get-QuestMicrosoftInstaller 'webview'
        Invoke-QuestMicrosoftInstaller -Path $questSetup -Arguments @('/silent', '/install')
        $questState = Get-QuestSystemPrerequisites
    }
    Show-QuestSystemPrerequisites $questState
    if (@($questState.checks | Where-Object { $_.status -ne 'compatible' }).Count -gt 0) { throw 'System prerequisites still fail after installation. Complete setup manually, then rerun .\run-install.ps1.' }
    return $questState
}
