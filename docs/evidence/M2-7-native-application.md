# M2.7 native application workflow evidence

Status: locally verified Windows desktop workflow; remote CI/publication deferred; external cryptographic audit remains open for pre-production certification.

The Windows Tauri adapter owns a persistent `VaultService` and process-memory session table. Renderer commands contain workspace and verified-backup identifiers only. Passphrases and recovery acknowledgement use native Windows controls; DEKs and session tokens never enter serialized responses or browser storage. A compile-time debug-only synthetic provider enables repeatable application acceptance and is absent unless the explicit `vault-test-provider` feature is selected.

`scripts/verify-vault-desktop.mjs` ran the actual debug Tauri executable through its WebView2 IPC boundary against an isolated synthetic root. It created a workspace and Private record, completed a real M2.5 backup, acknowledged recovery, sealed the workspace, unlocked and authenticated the encrypted snapshot, locked and observed an unauthorized read, restarted and observed an unauthorized read, then unlocked and read the sealed title/body again. Evidence root: `C:/Users/robin/AppData/Local/Temp/anansi-vault-desktop-Reecld`.

This verifies the native vertical slice only. The retained pre-seal backup remains plaintext by approved design. Search exclusion does not imply whole-root encryption or secure erasure of retained originals, SQLite/WAL remnants, swap, screenshots or independent backups. Browser authorization, encrypted portable backup/restore and full M2.8 acceptance remain separate work.
