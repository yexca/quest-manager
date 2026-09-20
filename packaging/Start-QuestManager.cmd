@echo off
setlocal
set "QUEST_PS=%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe"
if exist "%SystemRoot%\Sysnative\WindowsPowerShell\v1.0\powershell.exe" set "QUEST_PS=%SystemRoot%\Sysnative\WindowsPowerShell\v1.0\powershell.exe"
"%QUEST_PS%" -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0portable\Launch.ps1"
if errorlevel 1 (
  echo.
  echo Quest Manager could not start. See the message above and PORTABLE-README.txt.
  pause
  exit /b 1
)
