param([switch]$InstallWebView2)
# The workflow explicitly authorizes this change on a disposable hosted runner.
# Local/noninteractive run-install retains its fail-without-consent behavior.
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or $env:RUNNER_OS -ne 'Windows') {
    throw 'This entry point is only for disposable GitHub-hosted Windows runners.'
}
. (Join-Path $PSScriptRoot 'Environment.ps1')
. (Join-Path $PSScriptRoot 'SystemPrerequisites.ps1')
$questState = Get-QuestSystemPrerequisites
Show-QuestSystemPrerequisites $questState
if (@($questState.checks | Where-Object { $_.id -in @('cpp', 'sdk') -and $_.status -ne 'compatible' }).Count -gt 0) {
    throw 'The runner image lacks compatible VS 2022 C++ tools or Windows SDK. Update the workflow runner selection.'
}
if (@($questState.checks | Where-Object { $_.id -eq 'webview' -and $_.status -ne 'compatible' }).Count -gt 0) {
    if (!$InstallWebView2) { throw 'Pass -InstallWebView2 to authorize runtime setup on this disposable runner.' }
    $questSetup = Get-QuestMicrosoftInstaller 'webview'
    Invoke-QuestMicrosoftInstaller -Path $questSetup -Arguments @('/silent', '/install')
}
$null = Ensure-QuestSystemPrerequisites -NonInteractive
