# M2.6 — Vault security design and approval gate

Status: draft for explicit review. No cryptographic implementation is authorized by this document.

## Scope

Define the optional Sealed-memory vault and encrypted backup contract before M2.7 implementation. Private, Standard and Public records remain governed by the existing plaintext policy until the design is approved and implemented. Sealed operations continue to fail closed.

## Decisions required before implementation

1. Select an audited, maintained cross-platform cryptographic library and pin its exact version after reviewing its official security and platform documentation.
2. Define an authenticated envelope format with version, algorithm identifiers, nonce, salt/KDF parameters, ciphertext and authentication tag. Unknown versions and malformed lengths must fail before decryption.
3. Define key generation, passphrase derivation, recovery-key handling, rotation, unlock lifetime, lock-on-restart and zeroization limits. Secrets must never enter logs, ordinary exports or ordinary backups.
4. Define storage ownership for Windows Credential Manager and the equivalent supported platform stores. Browser sessions may hold an authorization handle only; they may not persist passphrases or keys.
5. Define the allow/deny table for list, read, history, relationship, export, backup and restore while locked, including metadata-leak limits.
6. Define migration and recovery behavior for interrupted re-encryption, wrong keys, tampering and lost recovery material. Original plaintext must remain recoverable until an explicit, reviewed migration policy says otherwise.

## Required approval evidence

- Threat model and trust-boundary review.
- Reproducible known-vector, tamper, wrong-key, recovery and restart-lock tests proposed for M2.7.
- Exact library/version, format, key-storage, migration and disposal decisions.
- Explicit user approval recorded before any `vault.rs`, encryption dependency or sealed-data migration is added.

## Current boundary

M2.5 snapshot/restore remains incomplete. M2.6 is a design gate only. M2.7 and M2.8 cannot be claimed, and M3 must not start, until M2.5 is verified, this design is approved, the vault is implemented and integrated certification passes.
