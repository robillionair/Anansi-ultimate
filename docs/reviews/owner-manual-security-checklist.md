# ANANSI Owner Manual Security Acceptance Checklist: M2 Vault & M5 Credential Dialogs

**Document Version**: 1.0.0  
**Date**: November 9, 2026  
**Audience**: Repository Owner & Authorizing Architect (Robillionair OÜ)  
**Status**: PENDING MANUAL OWNER VERIFICATION (Retained Manual Boundary)  

---

## 1. Context & Security Boundary Policy

Under project policy and computer-use safety rules:
- Automated test harnesses and agentic assistants must **never** automate, intercept, or spoof operating system credential prompts (Win32 CredUI, Windows Security dialogs, Windows Hello, or UAC prompts).
- The automated test suites verify the complete cryptography pipeline, state machines, and API boundaries using synthetic/test providers and simulated storage mocks.
- **Real OS Credential Manager storage and interactive Win32 prompts are reserved for owner-in-the-loop manual qualification.**

This checklist defines the manual verification procedure to be executed by the repository owner on a dedicated, non-automated Windows 11 host.

---

## 2. Milestone 2 (M2) — Sealed Vault Real OS Credential Store & Passphrase

### Prerequisites
- Build `anansi-desktop.exe` without test mock environment variables (`ANANSI_VAULT_TEST_PROVIDER` unset).
- Launch `anansi-desktop.exe` directly on Windows 11.

### Verification Steps
1. **Vault Initialization & Master Key Generation**:
   - Open a clean workspace.
   - Navigate to **Settings** -> **Sealed Vault**.
   - Click **Initialize Sealed Vault**.
   - [ ] **Check 2.1**: Verify that the native master key creation prompt appears requesting a master passphrase.
   - [ ] **Check 2.2**: Enter a strong passphrase. Verify that the 24-word recovery seed phrase is presented with a mandatory acknowledgement requirement.
   - [ ] **Check 2.3**: Verify that the master key encryption uses `Argon2id` (`m=65536 KiB, t=3, p=4`) with zero passphrase exposure in logs or UI DOM.

2. **Windows Credential Manager Keyring Entry**:
   - Open Windows **Credential Manager** (`control /name Microsoft.CredentialManager`) -> **Generic Credentials**.
   - [ ] **Check 2.4**: Verify that a credential entry exists with target name `anansi-vault/<workspace-id>`.
   - [ ] **Check 2.5**: Verify that no unencrypted plaintext secrets appear in `AppData/Roaming/anansi/storage`.

3. **Lock, Restart, and Unlock Cycle**:
   - In the ANANSI desktop app, click **Lock Vault**.
   - [ ] **Check 2.6**: Verify that all sealed memories and secrets become inaccessible (`SEALED_VAULT_LOCKED`).
   - Close `anansi-desktop.exe` and relaunch it.
   - [ ] **Check 2.7**: Verify that the vault remains securely locked across application restart.
   - Click **Unlock Vault**, enter the passphrase into the native prompt.
   - [ ] **Check 2.8**: Verify that sealed memories unlock cleanly and that entering an incorrect passphrase fails closed without revealing information.

4. **Passphrase Recovery Key Rotation**:
   - Select **Recover Vault** with the 24-word recovery phrase.
   - [ ] **Check 2.9**: Verify that the master key rotates and the new passphrase re-encrypts the vault without data corruption.

---

## 3. Milestone 5 (M5) — AI Provider Credential Win32 CredUI Dialogs

### Prerequisites
- Launch `anansi-desktop.exe` without mock providers.
- Have a valid (or test-scoped) API key for a commercial provider (e.g. OpenAI / Anthropic).

### Verification Steps
1. **Interactive Credential Entry via Win32 CredUI**:
   - Navigate to **AI Providers** in the ANANSI desktop app.
   - Select **Add Provider** -> choose an API-key provider (e.g. OpenAI).
   - Click **Configure API Key**.
   - [ ] **Check 5.1**: Verify that a genuine native Windows CredUI prompt ("Windows Security: Enter your credentials for ANANSI AI Provider") appears.
   - [ ] **Check 5.2**: Verify that the prompt is rendered by Windows `credui.dll`, not by HTML/DOM inputs.
   - Enter the key and confirm.
   - [ ] **Check 5.3**: Verify that the credential is saved to Windows Credential Manager under `anansi-provider/<provider-id>`.
   - [ ] **Check 5.4**: Verify that the API key never appears in the web UI, configuration JSON, application logs, or git commits. Configuration files store references only (`credential: "keyring:anansi-provider/..."`).

2. **Credential Revocation & Removal**:
   - In **AI Providers**, click **Remove Credential**.
   - [ ] **Check 5.5**: Verify that the credential entry is deleted from Windows Credential Manager.
   - [ ] **Check 5.6**: Verify that subsequent inference dispatches immediately fail with `PROVIDER_CREDENTIAL_MISSING` and Zero-Cost Lock holds.

3. **Zero-Cost Lock & Paid Inference Gates**:
   - [ ] **Check 5.7**: Verify that the **Zero-Cost Lock** toggle defaults to **Enabled**.
   - [ ] **Check 5.8**: Verify that attempting any paid inference request with Zero-Cost Lock enabled is blocked locally before any network packet is dispatched.
   - [ ] **Check 5.9**: Verify that un-locking Zero-Cost Lock requires explicit confirmation.

---

## 4. Milestone 8 & Milestone 13 — Interactive Win32 Common File Dialog (#32770) from Collider

### Prerequisites
- Build `anansi-desktop.exe` (`cargo build --release -p anansi-desktop`).
- Prepare a test folder with a Unicode file: `proov-küljendus-2026.md` (e.g. Estonian UTF-8 text with diacritics).
- Launch `anansi-desktop.exe` directly on Windows 11.

### Verification Steps
1. **Interactive File-Picker Opening from Collider**:
   - Open a workspace and navigate to **Projects** -> Select or create a project (e.g. `Harbor App`).
   - Scroll to **Product Creation Pipeline** -> **Invent: Open LEXI-COLLIDER**.
   - Under **Beam A**, click **Choose File** (`#collider-document-a`).
   - [ ] **Check 4.1**: Verify that the genuine Win32 Common Item Dialog (`IFileDialog`, Window Class `#32770`, Title "Open") opens modally over the application window.

2. **File Filters Verification**:
   - In the Open dialog, click the file type filter dropdown at the bottom right.
   - [ ] **Check 4.2**: Verify that the filter options reflect `.txt` and `.md` (`Text / Markdown Files (*.txt; *.md)` and `All Files (*.*)`).

3. **Cancel Action**:
   - Click the **Cancel** button (or press `Escape`).
   - [ ] **Check 4.3**: Verify that the dialog closes cleanly, Beam A text remains empty/unmodified, and zero error alerts appear in the UI.

4. **Unicode Filename Selection**:
   - Click **Choose File** again.
   - Navigate to `proov-küljendus-2026.md` and click **Open**.
   - [ ] **Check 4.4**: Verify that Beam A text populates with the decoded UTF-8 contents.
   - [ ] **Check 4.5**: Verify that the attribution label reads: `Document: proov-küljendus-2026.md · Markdown · <bytes> bytes · <characters> characters · Text SHA-256: <hash>...`.

5. **Same-Name Reselection After Modifying File Contents**:
   - Modify the contents of `proov-küljendus-2026.md` on disk (e.g. add new lines).
   - Click **Choose File** and reselect `proov-küljendus-2026.md`.
   - [ ] **Check 4.6**: Verify that Beam A text immediately updates to the modified text, character and byte counts update, and the displayed SHA-256 hash reflects the new file hash.

---

## 5. Acceptance Certification Record

| Item | Required Evidence | Verifier | Date | Status |
|---|---|---|---|---|
| **M2 Real Windows Vault** | Windows Credential Manager entry, Argon2id derivation, restart lock | Owner | PENDING | Pending Owner Execution |
| **M2 Recovery Key Rotation** | 24-word phrase recovery re-encryption | Owner | PENDING | Pending Owner Execution |
| **M5 Win32 CredUI Dialog** | Native OS security prompt, keyring reference storage | Owner | PENDING | Pending Owner Execution |
| **M5 Zero-Cost Lock** | Zero network traffic on paid models | Owner | PENDING | Pending Owner Execution |
| **M8/M13 Win32 File Dialog** | Win32 `#32770` dialog, Unicode name, Cancel, same-name reselection | Owner | PENDING | Pending Owner Execution |

*Note: All automated software boundaries for M2, M5, and M8/M13 have passed 100% on Windows NTFS in synthetic test suites. This manual checklist serves as the formal handoff to the Owner for live OS-level credential dialog and Win32 file picker sign-off.*
