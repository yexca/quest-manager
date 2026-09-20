$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
. (Join-Path $root 'scripts/Assert-ReleaseSource.ps1')
$script:releaseTests = 0
function Check($Condition, [string]$Message) { if (!$Condition) { throw $Message } }
function Expect-Failure([scriptblock]$Action, [string]$Text) {
    try { & $Action; throw 'Expected failure did not occur.' } catch {
        if ($_.Exception.Message -notlike "*$Text*") { throw }
    }
}
function Test-Release([string]$Name, [scriptblock]$Action) { & $Action; $script:releaseTests++; Write-Host "PASS $Name" }
$fixture = Join-Path $root ('env/test-artifacts/release-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path (Join-Path $fixture 'src-tauri') -Force | Out-Null
foreach ($file in @('package.json', 'package-lock.json', 'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock', 'src-tauri/tauri.conf.json')) {
    Copy-Item -LiteralPath (Join-Path $root $file) -Destination (Join-Path $fixture $file)
}
$version = Get-QuestReleaseVersion $root
Test-Release 'all application version locations agree' { Check ((Get-QuestReleaseVersion $fixture) -ceq $version) 'Wrong version.' }
Test-Release 'mismatched lockfile version blocks release' {
    $path = Join-Path $fixture 'package-lock.json'
    $original = Get-Content -LiteralPath $path -Raw
    try {
        $pattern = '(?s)("packages"\s*:\s*\{\s*""\s*:\s*\{.*?"version"\s*:\s*")[^"]+'
        ($original -replace $pattern, '${1}99.0.0') | Set-Content -LiteralPath $path -Encoding UTF8
        Expect-Failure { Get-QuestReleaseVersion $fixture } 'versions disagree'
    } finally { Set-Content -LiteralPath $path -Value $original -Encoding UTF8 }
}
Test-Release 'tag version mismatch blocks release before Git operations' {
    function git { throw 'Git should not run.' }
    Expect-Failure { Assert-QuestReleaseTag $fixture 'v99.0.0' $version } 'exactly match'
}
Test-Release 'tag pointing elsewhere blocks release' {
    function git { $global:LASTEXITCODE = 0; if ($args -contains 'HEAD') { 'a' * 40 } else { 'b' * 40 } }
    Expect-Failure { Assert-QuestReleaseTag $fixture "v$version" $version } 'does not resolve'
}
Test-Release 'dirty tagged checkout blocks release' {
    function git { $global:LASTEXITCODE = 0; if ($args -contains 'status') { ' M example.txt' } else { 'a' * 40 } }
    Expect-Failure { Assert-QuestReleaseTag $fixture "v$version" $version } 'clean working tree'
}
Test-Release 'clean matching tag is accepted' {
    function git { $global:LASTEXITCODE = 0; if ($args -notcontains 'status') { 'a' * 40 } }
    Assert-QuestReleaseTag $fixture "v$version" $version
}
Test-Release 'CI system setup refuses self-hosted or local runners' {
    $saved = @{}
    foreach ($key in @('GITHUB_ACTIONS', 'RUNNER_ENVIRONMENT', 'RUNNER_OS')) { $saved[$key] = [Environment]::GetEnvironmentVariable($key, 'Process') }
    try {
        $env:GITHUB_ACTIONS = 'true'
        $env:RUNNER_OS = 'Windows'
        $env:RUNNER_ENVIRONMENT = 'self-hosted'
        Expect-Failure { & (Join-Path $root 'scripts/Initialize-CiRunner.ps1') -InstallWebView2 } 'only for disposable'
        $env:GITHUB_ACTIONS = 'false'
        $env:RUNNER_ENVIRONMENT = 'github-hosted'
        Expect-Failure { & (Join-Path $root 'scripts/Initialize-CiRunner.ps1') -InstallWebView2 } 'only for disposable'
    } finally {
        foreach ($key in $saved.Keys) { [Environment]::SetEnvironmentVariable($key, $saved[$key], 'Process') }
    }
}

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$fixed = Get-Content -LiteralPath (Join-Path $root 'packaging/webview2.json') -Raw | ConvertFrom-Json
$required = @('quest-manager.exe', 'Start-QuestManager.cmd', 'portable/Launch.ps1', 'PORTABLE-README.txt', 'LICENSE', 'THIRD_PARTY.md', 'platform-tools/adb.exe', 'platform-tools/AdbWinApi.dll', 'platform-tools/AdbWinUsbApi.dll', 'aapt2/aapt2.exe', 'apk-tools/apktool.jar', 'apk-tools/apksigner.jar', 'apk-tools/zipalign.exe', 'apk-tools/libwinpthread-1.dll', 'apk-tools/jre/bin/java.exe', 'apk-tools/jre/bin/keytool.exe')
function New-TestArchive {
    param([string]$Mode = 'online', [bool]$Dirty = $false, [string]$Omit = '', [string]$Extra = '')
    $directory = Join-Path $fixture ([guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $directory | Out-Null
    $name = "quest-manager-v$version-windows-x64-portable-$Mode"
    $zip = Join-Path $directory "$name.zip"
    $archive = [IO.Compression.ZipFile]::Open($zip, [IO.Compression.ZipArchiveMode]::Create)
    try {
        $entries = $required
        if ($Mode -eq 'offline') { $entries += 'webview2/msedgewebview2.exe' }
        if ($Extra) { $entries += $Extra }
        foreach ($entry in $entries) { if ($entry -cne $Omit) { $null = $archive.CreateEntry("$name/$entry") } }
        $manifest = @{version=$version; mode=$Mode; architecture='x64'; sourceCommit=('a'*40); sourceHasLocalChanges=$Dirty; fixedWebView2=$(if ($Mode -eq 'offline') { $fixed.version } else { $null })}
        $writer = [IO.StreamWriter]::new($archive.CreateEntry("$name/portable/manifest.json").Open())
        try { $writer.Write(($manifest | ConvertTo-Json)) } finally { $writer.Dispose() }
    } finally { $archive.Dispose() }
    $hash = (Get-FileHash -LiteralPath $zip).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText("$zip.sha256", "$hash  $name.zip`n")
    return $directory
}
function Verify-TestArchive([string]$Directory, [string]$Mode = 'Online', [string]$Commit = ('a'*40)) {
    & (Join-Path $root 'scripts/Test-PortableArchives.ps1') -Directory $Directory -Variant $Mode -ExpectedCommit $Commit -RequireCleanSource
}
Test-Release 'complete online archive passes identity and checksum checks' { Verify-TestArchive (New-TestArchive) }
Test-Release 'complete offline archive passes identity and checksum checks' { Verify-TestArchive (New-TestArchive -Mode offline) Offline }
Test-Release 'modified ZIP is rejected by sidecar checksum' {
    $directory = New-TestArchive
    $zip = Get-ChildItem -LiteralPath $directory -Filter '*.zip' | Select-Object -First 1
    Add-Content -LiteralPath $zip.FullName -Value 'changed'
    Expect-Failure { Verify-TestArchive $directory } 'checksum mismatch'
}
Test-Release 'dirty source manifest is rejected' { Expect-Failure { Verify-TestArchive (New-TestArchive -Dirty $true) } 'modified source' }
Test-Release 'wrong source identity is rejected' { Expect-Failure { Verify-TestArchive (New-TestArchive) Online ('b'*40) } 'requested source' }
Test-Release 'missing ADB DLL is rejected' { Expect-Failure { Verify-TestArchive (New-TestArchive -Omit 'platform-tools/AdbWinApi.dll') } 'Missing portable resource' }
Test-Release 'missing offline runtime is rejected' { Expect-Failure { Verify-TestArchive (New-TestArchive -Mode offline -Omit 'webview2/msedgewebview2.exe') Offline } 'runtime is absent' }
Test-Release 'private build records cannot enter upload archives' { Expect-Failure { Verify-TestArchive (New-TestArchive -Extra 'build-environment.json') } 'Private file' }
Test-Release 'path traversal entries are rejected' { Expect-Failure { Verify-TestArchive (New-TestArchive -Extra '../outside.txt') } 'Unsafe path' }
Write-Host "$script:releaseTests release validation tests passed. No release was created."
