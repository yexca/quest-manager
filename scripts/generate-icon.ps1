# Derive application icons from the selected mascot using the pinned Tauri CLI.
. (Join-Path $PSScriptRoot 'Environment.ps1')
Assert-QuestInstalled
$questIconSource = Join-Path $QuestRoot 'assets\branding\mint-pilot.png'
$questGeneratedIcons = Join-Path $QuestEnv 'generated-icons'
$questIconDirectory = Join-Path $QuestRoot 'src-tauri\icons'
if (!(Test-Path -LiteralPath $questIconSource -PathType Leaf)) {
    throw 'The Mint Pilot source artwork is missing from assets/branding.'
}
Invoke-QuestCommand -File 'npm.cmd' -Arguments @('run', 'tauri', '--', 'icon', $questIconSource, '--output', $questGeneratedIcons)
foreach ($questIconName in @('icon.png', 'icon.ico')) {
    Copy-Item -LiteralPath (Join-Path $questGeneratedIcons $questIconName) -Destination (Join-Path $questIconDirectory $questIconName) -Force
}
Copy-Item -LiteralPath (Join-Path $questGeneratedIcons 'icon.ico') -Destination (Join-Path $QuestRoot 'public\favicon.ico') -Force
Write-Host 'Updated Windows icons, shared app artwork and browser favicon.' -ForegroundColor Green
