param([switch]$Device, [switch]$DeviceWrite)
. (Join-Path $PSScriptRoot 'scripts\Environment.ps1')
Assert-QuestInstalled
Invoke-QuestCommand -File 'npm.cmd' -Arguments @('run', 'build')
Invoke-QuestCommand -File 'cargo.exe' -Arguments @('fmt', '--manifest-path', 'src-tauri/Cargo.toml', '--', '--check')
Invoke-QuestCommand -File 'cargo.exe' -Arguments @('clippy', '--locked', '--manifest-path', 'src-tauri/Cargo.toml', '--all-targets', '--', '-D', 'warnings')
Invoke-QuestCommand -File 'cargo.exe' -Arguments @('test', '--locked', '--manifest-path', 'src-tauri/Cargo.toml')
if ($Device) {
    Invoke-QuestCommand -File 'cargo.exe' -Arguments @('test', '--locked', '--manifest-path', 'src-tauri/Cargo.toml', 'connected_device_smoke', '--', '--ignored', '--nocapture')
}
if ($DeviceWrite) {
    $questFixtureHash = (Get-FileHash -LiteralPath (Join-Path $QuestRoot 'tests\fixtures\verification.apk') -Algorithm SHA256).Hash
    if ($questFixtureHash -ne '6CB4FF7210C12AD42F21478B3B5C755250FBA960F31138EE412057E46F470844') { throw 'The verification APK checksum does not match the checked-in fixture.' }
    Invoke-QuestCommand -File 'cargo.exe' -Arguments @('test', '--locked', '--manifest-path', 'src-tauri/Cargo.toml', 'connected_device_task_roundtrip', '--', '--ignored', '--nocapture')
}
