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

function Install-QuestNode {
    # Restore the complete official distribution, including its bundled npm.
    $questArchive = Get-QuestDownload -Url $QuestVersions.nodeArchive.url -Name "node-v$($QuestVersions.node)-win-x64.zip" -Sha256 $QuestVersions.nodeArchive.sha256
    Expand-Archive -LiteralPath $questArchive -DestinationPath (Join-Path $QuestEnv 'node') -Force
    Assert-QuestNode
}

function Get-QuestRustupVersion {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)][string]$ExpectedVersion)
    $questAutoInstall = [Environment]::GetEnvironmentVariable('RUSTUP_AUTO_INSTALL', 'Process')
    try {
        # rustup --version also queries rustc and can auto-install a missing toolchain.
        # Capture all output before selecting a line: a native pipeline ending in
        # Select-Object -First 1 can advance while rustup is still running.
        $env:RUSTUP_AUTO_INSTALL = '0'
        # Windows PowerShell treats native stderr (including rustup's info lines)
        # as errors. This preference is function-local; check the actual exit code.
        $ErrorActionPreference = 'Continue'
        $questOutput = @(& $Path --version 2>$null)
        if ($LASTEXITCODE -ne 0) { throw "Rustup version query failed (exit $LASTEXITCODE)." }
        if ($questOutput.Count -eq 0 -or [string]$questOutput[0] -notlike "rustup $ExpectedVersion *") {
            throw 'Local rustup version does not match toolchain.versions.json.'
        }
        return [string]$questOutput[0]
    } finally {
        [Environment]::SetEnvironmentVariable('RUSTUP_AUTO_INSTALL', $questAutoInstall, 'Process')
    }
}
