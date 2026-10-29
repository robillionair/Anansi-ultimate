@echo off
setlocal
set "ANANSI_STORAGE_ROOT=%LOCALAPPDATA%\ANANSI-Preview\ask-verified"
set "ANANSI_VAULT_TEST_PROVIDER="
set "ANANSI_PROVIDER_TEST_CREDENTIAL="
set "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="
set "TAURI_CONFIG="
set "PREVIEW_EXE=E:\Anansi\build-cache\preview-ask-verified\debug\anansi-desktop.exe"
if not exist "%PREVIEW_EXE%" (
  echo This preview has not been built. See docs/handoffs/ask-preview-integration-checkpoint.md.
  pause
  exit /b 1
)
start "ANANSI Preview Ask Verified" /wait "%PREVIEW_EXE%"
if errorlevel 1 pause
