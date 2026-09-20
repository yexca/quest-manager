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
