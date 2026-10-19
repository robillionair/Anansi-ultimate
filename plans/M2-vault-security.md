# M2.6 — Vault security design and approval reconciliation

Status: M2.7-Core engineering authorized under Explicit Unaudited-Primitive Risk Acceptance; G1-External-Audit remains open. No implemented-vault or completion claim.

Authority: [master specification](../docs/MASTER_SPEC.md), sections 3, 10, 66, 70–76, 82–84; [Memory OS plan](02-memory-system.md); [unchanged user-approved contract](../docs/approvals/M2-6-user-contract.md). The conditional M2.7 authorization is recognized. M2.5 is published at `6c193ef8c9891e966c547451b4e01b7db74215b4`, with all three CI jobs passed and independent source recovery verified. Requirements are preserved; unresolved technical contradictions are not waived by continuation.

The user subsequently replied **“Adopt these corrections; retain audit gate.”** [The normative addendum](../docs/approvals/M2-6-corrections.md) records that authorization and concrete purpose/keyslot/journal/schema/resource definitions. It supersedes the original conflicting journal purge, physical-erasure and plaintext-export wording. The register below retains the reasons for G2–G4; their proposed corrections are now adopted, not open approval requests. Where the design discussion below reserves finalization to G2–G4, use the addendum's concrete definitions. The Owner amendment below supersedes G1 as a pre-implementation prerequisite; G1-External-Audit remains a pre-production/commercialization requirement.

Owner amendment (2026-10-19): [Explicit Unaudited-Primitive Risk Acceptance](../docs/approvals/M2-6-owner-risk-acceptance.md) supersedes historical pre-implementation audit blocks below. M2.7 engineering is authorized after verified M2.5; G1-External-Audit remains open before production/commercialization certification. G2–G4 and all deterministic/integration tests remain mandatory.

## M2.6 acceptance criteria

2026-10-19 direction: the user selected **“Retain pure Rust; obtain matching audit evidence.”** [The library comparison](M2-vault-library-options.md) records this choice; native libsodium and p=1 are not selected. [Follow-up evidence](../docs/evidence/M2-6-audit-review.md) records checksum-verified direct sources, historical audit revisions and the opaque-ke scope exclusion. [The explicitly authorized inquiry](../docs/reviews/M2-6-audit-inquiry.md) is published as upstream issue 940. G1 remains open; neither source fingerprints nor an audit of a dependent application closes it.

Preserve approval/provenance; verify dependency/platform/audit facts; define threats, format, keys, operation policy, migration/recovery, exact file ownership and reproducible tests; record conflicts and closure evidence; validate and publish a recoverable design checkpoint. M2.7-Core dependency and implementation work is now authorized under the Owner amendment; do not claim M2.7 or M2 complete without verification.

## Verified dependency and platform facts

Registry reads on 2026-10-18 confirmed these exact proposed versions, all not yanked. No dependency or lockfile was changed. Checksums identify registry packages, not audits.

| Crate | Proposed pin | rust-version | Registry SHA-256 |
|---|---|---|---|
| chacha20poly1305 | =0.11.0 | 1.85 | `9b89e1c441e926b9c82a8d023f6e1b7ae0adcfaa7d621814e4d60789bac751cb` |
| argon2 | =0.6.0 | 1.85 | `134c52ddac6d63c576bef8168db10c83c49c26444ecbc68060fef078925a901c` |
| zeroize | =1.9.1 | 1.85 | `e13084392c5e4bc371903e2935a5eaeed24905a7511356b883835e18a78f6879` |
| keyring | =3.6.3 | 1.75 | `eebcc3aff044e5944a8fbaf69eb277d11986064cba30c468730e8b9909fb551c` |

Primary registry endpoints: [cipher](https://crates.io/api/v1/crates/chacha20poly1305/0.11.0), [KDF](https://crates.io/api/v1/crates/argon2/0.6.0), [zeroize](https://crates.io/api/v1/crates/zeroize/1.9.1), [keyring](https://crates.io/api/v1/crates/keyring/3.6.3). Pinned Rust 1.99.1/edition 2024 exceeds the stated minima, so the supplied older-version fallback condition does not apply. Actual feature/transitive resolution, advisory review and target builds remain preflight, not passed tests.

[NCC's report page](https://www.nccgroup.com/research/public-report-rustcrypto-aesgcm-and-chacha20pluspoly1305-implementation-review/) describes a December 2019 review of specified AES/GCM and ChaCha20+Poly1305 implementations. The [cipher maintainers](https://raw.githubusercontent.com/RustCrypto/AEADs/master/chacha20poly1305/README.md) reference it and qualify constant-time behavior by CPU. This is relevant historical evidence, not blanket coverage of current RustCrypto releases, Argon2, keyring or this application. The official PDF download redirected to a research landing page during inspection; exact audited source revisions were not recovered there. The [Argon2 maintainer README](https://raw.githubusercontent.com/RustCrypto/password-hashes/master/argon2/README.md) supplied no corresponding audit report. This does not prove no other audit exists. Search results are not a lockfile security scan.

The current [getrandom 0.4.3](https://docs.rs/getrandom/0.4.3/getrandom/) pin provides fallible OS randomness; reuse it without adding an unpinned rand_core. OS randomness is not necessarily hardware-only entropy. Fresh 192-bit random nonces reduce collision probability rather than eliminate it. Entropy failure must leave canonical data unchanged.

[keyring 3.6.3 documentation](https://docs.rs/crate/keyring/3.6.3) describes platform stores, synchronous calls, concurrency constraints and mock fallback. Proposed target-specific features: Windows windows-native, macOS apple-native, Linux sync-secret-service plus crypto-rust, with documented libdbus prerequisites in [its manifest](https://docs.rs/crate/keyring/3.6.3/source/Cargo.toml.orig). Serialize each credential's access on a blocking thread; refuse mock backend in production. That statement described the design checkpoint. Isolated M2.7-Core now compiles and has verified one synthetic real Windows wrapped-slot store workflow; see M2-7 evidence. Linux Secret Service is enabled only by explicit core feature linux-secret-service, requiring system D-Bus prerequisites and separate platform acceptance. Without it Linux credential operations fail closed rather than selecting mock. No Linux requirement is marked complete. Windows desktop/browser is M2's current acceptance target; actual macOS/Apple Silicon install remains M14. Unsupported native secret-entry environments must stay locked, not fall back to browser password capture.

## Conflict and closure register

| Gate | Conflict or missing prerequisite | Concrete closure path |
|---|---|---|
| G1: audit coverage | The approval calls the chosen primitives audited by NCC; located evidence covers narrower historical AEAD implementations. Exact current release/transitive coverage, especially Argon2, is unverified. Master section 75 requires audited libraries. | Obtain primary reports and map audited algorithms/revisions, fixes and subsequent changes to exact dependency closure. If unavailable, explicitly review an alternative audited-library design; do not silently substitute or call test vectors an audit. Owner risk acceptance now permits exact-pinned dependency addition and engineering while G1-External-Audit remains open; no third-party audit closure is claimed. |
| G2: adopted journal/recovery correction | “Signed SHA-256” defines no signing key, MAC/signature algorithm or encoding. Automatic startup purge conflicts with retaining failed staging and cannot authenticate encrypted progress while locked. | Adopted amendment: purpose-bound ANSV-authenticated manifest containing file hashes; quarantine incomplete generations at startup, authenticate after unlock, retain originals, and perform cleanup only explicitly. The addendum defines canonical purpose/encoding and replaces automatic purge with preserved staging. A hash alone is not authentication. |
| G3: adopted lifecycle/export correction | Immediate physical key destruction on panic/termination and exclusion from all OS dumps exceed zeroize guarantees. Ordinary encrypted-only Sealed export also coexists with unspecified per-action plaintext export. | Adopted amendment: enforce logical revocation and best-effort Rust-owned buffer erasure; document OS/swap/dump/privileged-inspection limits. Keep ordinary Sealed plaintext export denied. The approved addendum explicitly removes the plaintext-export exception for this implementation. |
| G4: adopted context/format definitions | Header AAD binds format but not workspace/record/generation identity. Direct-key KDF fields, keyslot purpose/context, journal purpose, full-backup resource cap and plaintext metadata ownership are not fully encoded in the approved contract. | The approved addendum defines purpose combinations, authenticated identities and bounded payload schema. Reject valid-ciphertext swaps and rollback/context confusion. Do not deploy an app-specific envelope whose security-critical fields remain implementation guesses. |

Your approval remains recorded. G1-External-Audit is an open certification prerequisite; G2–G4 are adopted corrections under the addendum, with the register retained as rationale. Implementation tests remain mandatory. The Owner amendment authorizes the isolated M2.7-Core increment.

## Threat model and trust boundaries

Protect closed storage and copied encrypted backups against passive disclosure; detect tampering before releasing plaintext. Retain ownership, pairing/Host/Origin, path/reparse protection, resource limits and Rust-native enforcement. AEAD does not by itself prevent replay of an older valid archive or authenticate its human author.

Trusted while unlocked: Rust core, OS kernel/store, narrowly scoped native secret controls and the authorized user. A compromised frontend can read deliberately displayed plaintext and use its current handle; a handle does not defeat XSS. Privileged debuggers, malware in the unlocked process, screen capture, swap/dumps and already-rendered copies cannot be excluded by zeroize. [Documented zeroize limits](https://docs.rs/zeroize/1.9.1/zeroize/) cover copies/reallocation and mechanisms beyond its scope. BitLocker/FileVault are separate OS controls; application lock neither configures nor proves disk encryption. No cloud disclosure, generic browser filesystem/process grant, generated software execution or paid inference.

## Format, keys and session design

The supplied contract's 64-byte header/table, BE integer offsets, 16-byte salt, 24-byte nonce, detached 16-byte tag, algorithm IDs and AAD exactly equal to original bytes 0..64 remain authoritative. Do not reserialize the AAD. Minimum length is 80; exact total is 80 + ct_len using checked arithmetic, with no trailing bytes. Validate magic/version/algorithm/flags/purpose/KDF/length/bounds before KDF or allocation. Reject unknown versions and unsupported purpose combinations; no downgrade or unauthenticated plaintext emission.

Argon2id v0x13 defaults remain m=65536 KiB, t=3, p=4 and 32-byte output, corresponding to a profile in [RFC 9106](https://www.rfc-editor.org/rfc/rfc9106.html). Retain approved hard bounds m=19456..1048576 KiB, t=1..16, p=1..16, plus algorithm consistency checks. Enforce one KDF at a time per host, a bounded queue/backoff and memory admission before execution; never silently lower parameters. Direct-key mode runs no KDF; the adopted canonical default m/t/p and fresh nonzero salt preserve the supplied universal field bounds.

Generate independent random 32-byte DEK/recovery key per vault, fresh salt and nonce per seal/rewrap. Guard secret buffers, prohibit Copy/secret serde, redact Debug/Display, and never put secrets in command arguments, environment, logs or evidence. Recovery text proposal: canonical unpadded RFC 4648 Base32 (52 chars) in eight groups 7/7/7/7/6/6/6/6; validate final padding bits. Show once through trusted native controls and require acknowledgement of an externally saved copy; no automatic clipboard or browser storage.

Passphrase change stages and verifies a new salt/KEK-wrapped DEK slot before switching; existing record ciphertext stays unchanged. Recovery unwraps DEK, requires a new passphrase and revokes sessions. Full compromise rotation stages and verifies every history/graph envelope under a new DEK. Old copies remain decryptable using old compromised keys; rotation cannot revoke independent backups. Lost passphrase plus lost recovery material means unrecoverable ciphertext, not a backend reset bypass.

Encrypted typed payloads must bind vault/source workspace/record/revision/generation/purpose/privacy identities as well as complete metadata/body/history/relationships. Verify identities after successful AEAD and against authorized context before returning or committing. Copying a valid envelope into another record must fail. Fresh restore authenticates source identities, maps IDs and re-encrypts destination identities with a new vault key; it cannot relabel opaque ciphertext. Wrapped-key slots also need authenticated vault/generation/purpose context; the exact 130-byte context encoding is defined by the addendum, not yet implemented.

### Native input and two-process authorization

The approval says passphrases never enter the frontend. A React password field would violate that rule. Proposed Windows resolution: Rust-owned generic credential controls with fixed ANANSI vault labels, no OS account authentication and no implicit credential persistence. [Microsoft's API documentation](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-creduipromptforcredentialsw) documents generic/non-persistent prompting and clearing returned buffers. The application integration, limits, UTF-16 conversion and native recovery-key display must be tested before activation. Cancel/timeout/parent exit discards buffers; browser requests cannot supply a parent PID, native path or arbitrary prompt content.

A browser user explicitly requests unlock; the local trusted prompt names the paired requesting origin/workspace and requires confirmation. No prompt on page load or remote/headless fallback. A cancelled prompt emits no handle. Browser/native plaintext editor output is still sensitive even though no keys enter JS.

Desktop and API are independent Rust processes sharing schema-8 storage. Unlock separately; no DEK transfer through SQLite, pairing files or IPC between them. Issue a random 128-bit handle scoped to caller/transport/root/workspace/process instance/generation/revocation epoch. JS memory only, never localStorage/sessionStorage/IndexedDB/cookie persistence. Enforce 900-second idle and 14400-second absolute deadlines using a monotonic injected test clock. Check authorization before decrypting and again at publication.

Manual lock/recovery/rotation advances a durable epoch under cross-process serialization. Every sensitive operation and commit checks it; old handles are logically invalid even in another process. Other processes erase guarded keys on notification/poll or next access. This is different from instant physical erasure on all processes (G3). Restart always starts locked, with no persisted session or automatic OS-store unlock.

OS-store consent is optional. Only a wrapped-DEK token is permitted under service com.robillionair.anansi.vault and a managed vault account ID; no passphrase/KEK/raw DEK/recovery key. Such a cached wrapped slot still requires interactive passphrase derivation; it is not passwordless unlock. Store unavailable/locked must produce an actionable error or interactive native entry, never plaintext persistence.

## Allow/deny and metadata policy

Non-Sealed paths retain current authorization/behavior. Current Sealed operations continue to fail closed until implementation is verified.

| Operation | Sealed locked | Sealed unlocked |
|---|---|---|
| create/revise/lifecycle | Deny | Authorized guarded encryption, no plaintext canonical revision |
| list/count | Only approved opaque sealed_locked_count; no item metadata/history counts | Decrypt authorized workspace metadata in memory |
| read/history | VaultLocked, no partial content | Authenticate complete envelope and identity before output |
| edges/project/supersession | Hide every touching reference, including plain endpoint snapshots | Authorized encrypted graph/history only |
| ordinary export | Deny/exclude with explicit scope | Still exclude; separate encrypted export; no plaintext exception under the adopted addendum |
| backup | Copy ciphertext and wrapped slots only, never plaintext Sealed metadata | Same; separate whole-archive encryption where selected |
| restore | Bounded staging/outer hashes only, clearly authentication pending | Authenticate all envelopes, map/re-encrypt, then publish fresh target atomically |
| recovery/rotation | Explicit native recovery request, never renderer secret | Explicit confirmation, revoke handles and verify staged replacement |

Outer storage necessarily reveals opaque IDs, ownership/vault association for authorization, lengths/file counts and approved coarse update buckets/count. “Zero metadata leak” in a test name means none of the forbidden fields, not zero information. Encrypt titles/tags/body/sources/project assignments/edge labels/history counts and sensitive references. Cover archived/expired revisions and incoming/outgoing edges, plaintext counterpart relationship snapshots, supersession, error/count hints and future FTS/vector/context paths. No persisted decrypted Sealed index or cloud eligibility.

## Migration, backup and recovery

Plan a new schema barrier `0009_vault.sql`; never modify applied migrations 1–8. Use the addendum's schema ownership, context keyslots/journal and exact encrypted-backup resource ceilings. No new schema or migration is created here.

Before opt-in migration, preserve a complete closed root and verified M2.5 snapshot, explicitly disclosed as plaintext. Preview the selected records, all revisions and every sensitive reference. Stage private encrypted generation, read-back authenticate and compare original hashes/domain values, then fence edits/locks/jobs and switch canonical generation in a short transaction. No plaintext SQL metadata/current body or historic graph reference may remain active for newly Sealed content. Refuse partial-history sealing until complete migration is available. Preserved old roots/backups still contain plaintext; do not claim those are retroactively encrypted.

Crash points before/after every write/sync/authentication/transaction/keyslot switch must reopen to exactly one valid committed generation. Interrupted/ambiguous/tampered journals preserve original and staging with visible recovery errors, not inferred imports or resets. Startup stays locked; quarantine/authenticate-before-cleanup is the adopted G2 correction replacing automatic purge. Explicit purge/reset operations need separately defined previews, retained orphans and confirmation before implementation. Do not promise forensic deletion across SQLite/WAL, SSD remanence, swap or old backups.

Plain M2.5 packages cannot admit raw Sealed plaintext. Future versioned packages must identify ciphertext and wrapped keyslot members, with schema-8 clients rejecting unknown formats. Locked preview verifies outer structure/hash only. Portable full-archive encryption must retain recoverable wrapped source keyslots inside encrypted content, exclude OS-store contents/raw keys, enforce a total cap before allocation, and require trusted native passphrase/recovery input. Wrong-key/tamper/interruption leaves source bytes and canonical roots unchanged; fresh restore verifies every tag/context before destination publication. Missing required keyslot fails; do not publish unrecoverable records.

## M2.7-Core isolated execution contract — 2026-10-19

Owner: this agent, branch codex/m2-7-vault-core in its own worktree. Active M3 checkout remains untouched. No merge or protected-master publication until coordinated integration. Shared Git objects are safe; never switch, reset, clean or stage files in the M3 checkout.

Immediate owners: new crates/core/src/vault.rs and vault/{envelope,keys,session,policy,migration,credential_store}.rs; dedicated core vault tests and fixtures, new scripts/vault-reference-vector.mjs; crates/core/Cargo.toml, Cargo.lock and the vault module declaration in crates/core/src/lib.rs. Governance files changed only in this isolated branch. No Search, UI, adapters, generated contracts, CI or M3 test files. Existing core domain/storage modifications require inspection and an explicit file list before execution; independent worktree isolation prevents live-file collision but does not eliminate later merge conflicts.

Order and acceptance: (1) governance validation and M2.5 source/CI recheck; (2) exact dependency resolution and strict ANSV parser/AEAD with independent primitive fixtures; (3) context-bound guarded keyslots, recovery/rotation, restart-locked sessions and filtering hook; (4) serialized real OS wrapped-slot store binding with synthetic-only tests; (5) recoverable authenticated shadow staging and fenced publication; (6) non-M3 domain enforcement and integration regressions. Each step must pass meaningful tests before its completion is recorded. No live vault activation until complete-history/graph, schema/privacy/revocation and all seven certification tests actually pass. Search/UI wiring and native/browser acceptance remain an explicit integration boundary; core-only tests cannot certify those workflows.

Disclosure before full verification: engineering in progress; third-party cryptographic audit pending (G1 open). Preserve the Owner's exact implemented wording as a prospective requirement, not a current capability claim. Maintain G1-External-Audit open.

## Exact M2.7 file ownership (planned, not created)

M3.1 scheduling handoff: when G1 eventually permits vault implementation, include `crates/core/src/search.rs` and shared `Search.tsx` in policy/revocation tests. Title matching is currently request-local and excludes Sealed; it has no persistent index to migrate. Vault activation must still guard encrypted title metadata before matching/counts and recheck authorization at result/body publication, including lock or migration during an in-flight search. No new search path may bypass the approved context/session/privacy rules. This is a retained integration obligation, not crypto implementation or a waiver of G1. See ADR-0024.

| Responsibility | Create or extend |
|---|---|
| Format/keys/session/policy/migration/store | New `crates/core/src/vault.rs` and `crates/core/src/vault/envelope.rs`, `keys.rs`, `session.rs`, `policy.rs`, `migration.rs`, `credential_store.rs`, `native_prompt.rs` under that vault directory |
| Schema/core entry | New `crates/core/migrations/0009_vault.sql`; extend `crates/core/src/lib.rs`, `readiness.rs`, `storage.rs`, `privacy.rs` |
| Existing domain owners inspected | Extend `crates/core/src/memory.rs`, `lifecycle.rs`, `relationships.rs`, `entities.rs`, `content.rs`, `export.rs`, `snapshots.rs`, `backup_commands.rs`, `backup_upload.rs`, `restore.rs`, `jobs.rs` where each policy is exercised |
| Dependencies/contracts | `crates/core/Cargo.toml`, `Cargo.lock`, `crates/contracts/src/main.rs`, `scripts/contracts.mjs`, generated `packages/contracts/schemas/readiness.json`, `src/generated.ts`, `src/validators.mjs`, `src/validators.d.mts`, and `packages/contracts/src/client.ts` |
| Adapters/capabilities | `crates/api/src/lib.rs`, desktop `apps/desktop/src-tauri/src/lib.rs`, new narrowly scoped vault permission file and existing default capability; no generic native grant |
| UI | New `packages/ui/src/Vault.tsx`; extend shared Workspace/Navigation/Memory/KnowledgeRecords/EntityHistory/Backups/Jobs and tokens in `packages/ui/src/` |
| Tests | New `crates/core/tests/vault_certification.rs`, `vault_roundtrip.rs` in that test directory, `crates/api/tests/vault.rs`, `tests/e2e/vault.spec.ts`, `scripts/vault-workflow.mjs`, `scripts/verify-vault.mjs` |
| Evidence | New `docs/evidence/M2-7.md`; update plan/status/architecture/decisions/recovery/matrix/state and CI's sequential native verifier |

The supplied shorthand `tests/vault_certification.rs` maps to the core integration-test directory so Cargo actually discovers it. No invented `memory_commands.rs` module is required; actual existing ownership was inspected.

## Ordered implementation and certification

1. Record the Owner risk-acceptance exception while retaining G1-External-Audit open; use the adopted addendum's canonical payload/keyslot/journal/schema/resource/native-input specifications and independent vector fixtures. Preserve approval history; do not treat unmet conditions as renewed user-approval absence.
2. Preserve source/synthetic schema-8 recovery roots; resolve exact dependencies/features/advisories/target support without unrelated updates. Fail on entropy errors, mock store or unsupported prerequisites.
3. Implement/test strict parser and independent vectors, keyslots and clocks/authorization first, then one complete native/browser create/unlock/read/lock/restart slice. Production must have no deterministic keys or fault bypass.
4. Implement complete-history/graph opt-in migration, recovery/passphrase/DEK rotation and cross-process revocation fencing with injected crash/failure recovery.
5. Add explicit encrypted export/backup/import/fresh restore with no raw Sealed data in ordinary paths, then full existing memory/entity/export/backup regressions.
6. Run actual Windows/native browser workflows, inspect errors/metadata/previews/layout, publish verified source and independently restore it. Stop before M2.8.

### Required tests (future, not executed)

Preserve all seven approved test names in `crates/core/tests/vault_certification.rs`: `test_known_vector_xchacha20poly1305_argon2id`, `test_envelope_header_and_aad_tamper_fails_closed`, `test_wrong_passphrase_and_wrong_recovery_key`, `test_recovery_key_unwraps_dek_and_rotates_passphrase`, `test_restart_lock_and_zeroization`, `test_locked_allow_deny_and_zero_metadata_leak`, `test_interrupted_migration_atomic_rollback`.

Independent vectors: [RFC 9106 section 5.3](https://www.rfc-editor.org/rfc/rfc9106.html#section-5.3) Argon2id and [XChaCha draft Appendix A.3](https://www.ietf.org/archive/id/draft-irtf-cfrg-xchacha-03.txt) AEAD inputs/outputs. Commit exact synthetic fixtures with provenance; independently test each primitive before composition. Computing expected bytes using the code under test is not an independent vector. Small vector KDF profiles do not change production defaults.

Add parser rejection callback counters (no KDF/AEAD), every-header-byte/truncation/overflow/flag/purpose mutation, bounded high-cost admission, serialized KDF/credential concurrency, entropy/store/prompt cancel errors, buffer redaction/no browser persistence, wrong workspace/process/epoch/transport handles, idle/absolute deadlines, swapped valid envelopes/keyslots/generations, replay disclosures, all nine entity kinds/history/lifecycle/edges, simultaneous API/native lock and publication, wrong-key/no canonical writes, complete keyslot restore, old backup recovery after rotation, unknown schema refusal and exact original-byte preservation after every injected crash. Observe controlled buffer drop, not unprovable absence from OS dumps.

Actual workflow: trusted native initialization and recovery display; sealed create/edit/history/links; lock and inspect all forbidden paths; desktop/API restart and old-handle rejection; independent unlock; recovery/new passphrase; encrypted backup/save/import/preview/fresh restore; wrong-key/tamper refusal; ordinary data while locked; native/browser layout and zero external requests. OS-store tests use unique synthetic identities, explicit opt-in and exact owned-entry cleanup, never existing credentials. Check plaintext canaries in active catalog/WAL/current metadata/artifacts/logs, explicitly distinguishing preserved pre-migration originals.

### Verification and recovery commands

M2.6: `pnpm check`; `pnpm test`; `node scripts/validate-planning.mjs --self-test`; `git diff --check`; `git hash-object docs/MASTER_SPEC.md` (expected `b95eae4a3813fb2cd87c00360fcdd9b2489cedec`). Compare approval bytes/digest with the source and inspect diff paths for no product/dependency/schema changes.

M2.7 after gate closure: all common commands in [the Memory OS plan](02-memory-system.md#verification-commands-and-evidence), plus `node scripts/cargo.mjs test -p anansi-core --test vault_certification --locked`, `node scripts/cargo.mjs test -p anansi-core --test vault_roundtrip --locked`, `node scripts/cargo.mjs test -p anansi-api --test vault --locked`, `pnpm exec playwright test tests/e2e/vault.spec.ts`, `node scripts/verify-vault.mjs --configured-cdp`. Generate changed DTOs first and check drift; sequence native verifiers due to fixed ports. These future files/commands are not current passing tests.

Follow [recovery](../docs/RECOVERY.md): codex/ branch, existing Git identity, commit/push, exact-SHA required CI, normal protected-master publication, verify remote/clean tree and independently bundle/clone source. Do not move tags or rewrite master specification. Source bundles exclude application/credential data. Record final publication/source results in an external ledger to avoid self-referencing commits.

## Definition of Done

Reconciliation Done: approval and provenance preserved, sources inspected, corrections G2–G4 adopted with concrete definitions, and test/file plan published with accurate gate status. M2.7 engineering readiness follows the Owner risk-acceptance exception and reverified M2.5 baseline; G1-External-Audit remains unmet for production certification. M2.7 Done requires executed cryptographic/failure/native/browser/recovery/regression evidence. M2 completion additionally requires M2.8. No encryption capability or application security audit is claimed here.
