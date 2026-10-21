@echo off
setlocal
cd /d "%~dp0"
set "ANANSI_STORAGE_ROOT=%LOCALAPPDATA%\ANANSI-Preview\r17"
set "ANANSI_VAULT_TEST_PROVIDER="
set "ANANSI_PROVIDER_TEST_CREDENTIAL="
set "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="
set "TAURI_CONFIG="
if not exist "target\debug\anansi-desktop.exe" (
  echo Preview executable is not built. See PREVIEW.md.
  pause
  exit /b 1
)
start "ANANSI Preview R17" /wait "target\debug\anansi-desktop.exe"
if errorlevel 1 pause
