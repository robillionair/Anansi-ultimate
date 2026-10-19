# M2.7 trusted vault runtime acceptance criteria

This increment defines the missing internal orchestration component required before adapter wiring:

- `VaultRuntime` owns `VaultStore`, process-local `Sessions`, and a non-serializable pending `AcknowledgedSetup`.
- `NativeCredentialProvider` is the only credential input boundary. Setup/unlock passphrases and recovery keys are zeroizing values supplied by a trusted native adapter; browser callers are rejected before credential input.
- Recovery display is one-shot and acknowledgement requires the saved-copy flag, challenge match, canonical recovery key and authenticated recovery-slot unwrap.
- Setup does not publish a vault without a verified M2.5 `BackupReference`; the acknowledged setup remains private in the runtime until explicit sealing.
- Lock delegates to the durable epoch bump and clears process-local sessions.

Executed:

- `node scripts/cargo.mjs test -p anansi-core --test vault_runtime --locked` — 1 passed.
- `node scripts/cargo.mjs clippy -p anansi-core --all-targets --locked -- -D warnings` — passed.

The current test verifies browser setup is rejected before any provider credential callback. Native Windows prompt implementation, adapter command transport, browser authorization plumbing, live Sealed record migration, encrypted backup/restore workflow, and M2.8 remain subsequent integration work.
