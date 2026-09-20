. (Join-Path $PSScriptRoot 'scripts\Environment.ps1')
Assert-QuestInstalled
Invoke-QuestCommand -File $QuestNpm -Arguments @('run', 'tauri', '--', 'dev', '--', '--locked')
