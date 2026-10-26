@echo off
setlocal
cd /d "%~dp0"
set "ANANSI_STORAGE_ROOT=%LOCALAPPDATA%\ANANSI-Preview\m7"
set "ANANSI_VAULT_TEST_PROVIDER="
set "ANANSI_PROVIDER_TEST_CREDENTIAL="
set "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="
set "TAURI_CONFIG="
set "PREVIEW_EXE=E:\Anansi\build-cache\preview-m7\debug\anansi-desktop.exe"
if not exist "%PREVIEW_EXE%" (
  echo Preview executable is not built. See M7-PREVIEW.md.
  pause
  exit /b 1
)
start "ANANSI Preview M7" /wait "%PREVIEW_EXE%"
if errorlevel 1 pause
