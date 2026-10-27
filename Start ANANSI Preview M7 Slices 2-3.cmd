@echo off
setlocal
set "ANANSI_STORAGE_ROOT=%LOCALAPPDATA%\ANANSI-Preview\m7-slices2-3"
set "ANANSI_VAULT_TEST_PROVIDER="
set "ANANSI_PROVIDER_TEST_CREDENTIAL="
set "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="
set "TAURI_CONFIG="
set "PREVIEW_EXE=E:\Anansi\build-cache\preview-m7-slices2-3\debug\anansi-desktop.exe"
if not exist "%PREVIEW_EXE%" (
  echo This preview has not been built. See M7-PREVIEW.md.
  pause
  exit /b 1
)
start "ANANSI Preview M7 Slices 2-3" /wait "%PREVIEW_EXE%"
if errorlevel 1 pause
