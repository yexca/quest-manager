. (Join-Path $PSScriptRoot 'scripts\Environment.ps1')
Assert-QuestInstalled
Invoke-QuestCommand -File 'npm.cmd' -Arguments @('run', 'tauri', '--', 'dev', '--', '--locked')
