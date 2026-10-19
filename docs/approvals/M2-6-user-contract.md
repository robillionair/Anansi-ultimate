# M2.6 — Vault Security Design and Approval Gate

Status: **APPROVED FOR M2.7 IMPLEMENTATION** (Pending/Concurrent with M2.5 Verification)
Authorizing Architect: Robin-Kevin Vettik (Robillionair OÜ)
Date: 2026-10-18

## Scope
Defines the normative cryptographic contract for the Sealed-memory vault (`vault.rs`) and encrypted backup envelope prior to M2.7 implementation. `Private`, `Standard`, and `Public` records remain governed by the existing policy; `Sealed` operations fail closed whenever the vault is in the `Locked` state.

---

## 1. Audited Cryptographic Library & Pinned Versions (Decision 1)
Provenance correction under the [Owner amendment](M2-6-owner-risk-acceptance.md): use exact-pinned pure-Rust RustCrypto primitives and cross-platform OS secret storage. NCC reviewed specified historical AEAD source; no matching independent Argon2 audit or externally reviewed current AEAD delta is established. M2.7 engineering proceeds under explicit unaudited-primitive risk acceptance; G1-External-Audit stays open. No blanket current RustCrypto audit or universal constant-time guarantee is claimed:

* **AEAD Cipher:** `chacha20poly1305 = "=0.11.0"` (or `"=0.10.1"` if workspace MSRV < 1.85) — Using **`XChaCha20Poly1305`** (256-bit key, 192-bit extended nonce, 128-bit Poly1305 tag) to eliminate random nonce collision risks across local snapshots.
* **Key Derivation (KDF):** `argon2 = "=0.6.0"` (or `"=0.5.3"` if workspace MSRV < 1.85) — Using **`Argon2id` v0x13** (RFC 9106 hybrid memory-hard KDF resistant to both GPU cracking and side-channel timing attacks).
* **Memory Zeroization:** `zeroize = { version = "=1.9.1", features = ["derive"] }` — Enforcing `Zeroize` and `ZeroizeOnDrop` on all key, passphrase, and plaintext buffer structs.
* **OS Credential Store:** `keyring = { version = "=3.6.3", features = ["windows-native", "apple-native", "sync-secret-service"] }` — Native bindings for Windows Credential Manager, macOS Keychain, and Linux Secret Service.
* **CSPRNG:** `rand_core` / `getrandom` via `OsRng` — Operating system hardware entropy source exclusively.

---

## 2. Authenticated Binary Envelope Format (Decision 2)
Every encrypted `Sealed` record and encrypted backup blob MUST serialize into the following strict big-endian binary envelope. **Bytes `0..64` (the entire header) MUST be passed as Associated Authenticated Data (AAD)** into `XChaCha20Poly1305::encrypt_in_place` so any header or parameter tampering fails authentication prior to plaintext emission.

| Byte Offset | Length | Field | Value / Validation Rule |
| :--- | :--- | :--- | :--- |
| `0..4` | 4 B | `magic` | Exact ASCII bytes `b"ANSV"` (`0x41 0x4E 0x53 0x56`). Reject immediately if mismatched. |
| `4..5` | 1 B | `version` | `0x01`. Any unknown version (`!= 0x01`) MUST fail closed with `VaultError::UnsupportedVersion` before KDF/AEAD execution. |
| `5..6` | 1 B | `alg_id` | `0x01` (`XChaCha20-Poly1305`). Any other value fails with `VaultError::UnsupportedAlgorithm`. |
| `6..7` | 1 B | `kdf_id` | `0x01` (`Argon2id v0x13`) or `0x00` (Direct DEK envelope when unlocked via wrapped Master Key). |
| `7..8` | 1 B | `flags` | Bitfield (`0x01` = Sealed Record, `0x02` = Encrypted Backup Archive, `0x04` = Recovery-Wrapped DEK).Reserved bits `0xF8` must be `0`. |
| `8..12` | 4 B | `m_cost_kib` | `u32` BE: Argon2id memory cost in KiB. Default `65536` (64 MiB). Hard bounds: `19456..=1048576` (19 MiB to 1 GiB). |
| `12..16` | 4 B | `t_cost` | `u32` BE: Argon2id time iterations. Default `3`. Hard bounds: `1..=16`. |
| `16..20` | 4 B | `p_cost` | `u32` BE: Argon2id parallelism lanes. Default `4`. Hard bounds: `1..=16`. |
| `20..36` | 16 B | `salt` | 128-bit CSPRNG salt (`OsRng`). Must not be all-zeros. |
| `36..60` | 24 B | `nonce` | 192-bit CSPRNG `XNonce` (`OsRng`). Generated fresh on every single write/seal operation. |
| `60..64` | 4 B | `ct_len` | `u32` BE: Exact byte length of `ciphertext` (excluding the 16-byte tag). Reject if `total_len != 64 + ct_len + 16`. |
| `64..64+N` | `N` B | `ciphertext` | Encrypted payload bytes. |
| `64+N..80+N`| 16 B | `auth_tag` | 128-bit Poly1305 authentication tag verifying `AAD (0..64) || ciphertext`. |

**Pre-Decryption Fail-Fast Rule:** Any payload shorter than `80 bytes` (64B header + 0B body + 16B tag), any unknown `magic`/`version`/`alg_id`, out-of-bounds KDF parameters, or length mismatch MUST abort at the parser boundary before allocating KDF memory or invoking AEAD decryption.

---

## 3. Key Hierarchy, Derivation, Recovery, Lifetime & Zeroization (Decision 3)
* **Two-Tier Envelope Key Hierarchy (KEK + DEK):**
  1. **Master Data Encryption Key (DEK):** A 256-bit (`[u8; 32]`) random key generated via `OsRng` upon vault initialization. All `Sealed` records are encrypted with the DEK.
  2. **Passphrase Key Encryption Key (KEK):** Derived from user passphrase + 16B `salt` via `Argon2id` (`m=64MiB, t=3, p=4`) into a 32-byte key that wraps the DEK using the Section 2 envelope.
  3. **Recovery Key:** A separate 256-bit CSPRNG key generated once during setup, displayed once to the user as 8 hyphenated Base32 groups, and used to create a second independent wrapped-DEK slot (`vault.recovery.keyslot`).
* **Rotation:**
  * *Passphrase Rotation:* Re-derives a new KEK with a fresh salt and re-wraps the existing 32-byte DEK in $O(1)$ time without touching record ciphertexts.
  * *Full DEK Compromise Rotation:* Generates a new DEK and executes an atomic shadow re-encryption of all `Sealed` records (see Section 6).
* **Unlock Lifetime & Lock-on-Restart:**
  * Unwrapped DEK resides solely in volatile RAM inside a `Zeroizing<[u8; 32]>` guard inside the Rust process state.
  * **Idle Auto-Lock:** 15 minutes (`900s`) of inactivity automatically drops and zeroizes the in-memory DEK.
  * **Absolute Session Cap:** 4 hours (`14400s`) maximum before mandatory re-authentication.
  * **Lock-on-Restart:** Process termination, panic, or service restart immediately destroys the in-memory DEK; the vault starts in `Locked` state on boot.
* **Zeroization & Redaction Limits:**
  * All key structs implement `#[derive(Zeroize, ZeroizeOnDrop)]` and a manual `std::fmt::Debug` implementation outputting `"SecretKey([REDACTED])"`.
  * Passphrases, KEKs, DEKs, Recovery Keys, and decrypted `Sealed` payloads MUST NEVER be written to application logs, `tracing` spans, panic crash dumps, or standard unencrypted exports/backups.

---

## 4. Platform Credential Storage & Browser Session Boundary (Decision 4)
* **OS Credential Store Ownership:**
  * Owned strictly by the local Rust backend (`anansi-core` / daemon) via `keyring = "=3.6.3"` under service namespace `"com.robillionair.anansi.vault"`.
  * Target backends: **Windows Credential Manager** (`windows-native` CredWriteW/CredReadW), **macOS Keychain** (`apple-native`), and **Linux Secret Service** (`sync-secret-service`).
  * Only an optional, user-consented wrapped key token may be stored in OS Credential Manager; if the OS store is unavailable or locked, the vault falls back strictly to interactive passphrase entry.
* **Browser / UI Session Isolation:**
  * Browser or frontend clients NEVER receive passphrases, KEKs, or DEKs.
  * Upon unlock, the Rust backend issues an ephemeral, random 128-bit **Authorization Handle** (`session_handle` held only in JavaScript memory variable, strictly forbidden from `localStorage`, `sessionStorage`, `IndexedDB`, or non-HttpOnly cookies).
  * When the vault locks or the backend restarts, all active `session_handle` tokens are invalidated immediately.

---

## 5. Locked vs. Unlocked Access Control & Metadata-Leak Matrix (Decision 5)
To prevent side-channel and metadata leaks while `Locked`, `Sealed` records store their titles, tags, semantic edges, and content **inside** the encrypted payload. Only an opaque `record_id` (UUIDv4), `tier = Sealed`, and coarse `updated_at` bucket exist in the outer index.

| Operation | `Public / Standard / Private` (While Vault Locked) | `Sealed` Records (While Vault **LOCKED**) | `Sealed` Records (While Vault **UNLOCKED**) |
| :--- | :--- | :--- | :--- |
| **`list`** | Allowed (normal behavior). | **DENY** metadata & content. Returns only opaque count `{"sealed_locked_count": N}` without titles, tags, snippets, or timestamps. | **ALLOW** (decrypts titles, tags, and summaries in memory). |
| **`read`** | Allowed. | **DENY** (`VaultError::VaultLocked`). Fail closed. | **ALLOW** (verifies Poly1305 tag, returns plaintext in memory). |
| **`history`** | Allowed. | **DENY** (`VaultError::VaultLocked`). Revision count and diffs hidden. | **ALLOW** (decrypts historical revisions). |
| **`relationship`**| Allowed for non-sealed edges. | **DENY** (`VaultError::VaultLocked`). All graph edges touching a `Sealed` node are stored inside the encrypted envelope and omitted from graph queries while locked. | **ALLOW** (merges decrypted edges into active memory graph). |
| **`export`** | Allowed (plaintext export). | **DENY** (`VaultError::VaultLocked`). Sealed records are strictly excluded from ordinary plaintext exports even when unlocked unless explicit `--include-sealed-encrypted` envelope export is invoked. | **ALLOW** only as encrypted `.ansv` envelopes; plaintext export requires explicit per-action confirmation. |
| **`backup`** | Allowed (standard snapshot). | **ALLOW (Ciphertext-Only):** Copies opaque, already-encrypted `.ansv` envelopes + keyslots verbatim into backup bundle without decrypting or leaking metadata. | **ALLOW** (encrypted `.ansv` envelopes preserved in backup). |
| **`restore`** | Allowed. | **DENY** live overwrite of sealed store while locked; stages encrypted `.ansv` blobs only after verifying outer manifest integrity. | **ALLOW** after verifying Poly1305 authentication tags on all restored `.ansv` records before committing. |

---

## 6. Migration, Interrupted Re-Encryption, Tamper & Recovery Policy (Decision 6)
* **Two-Phase Shadow Staging for Migration & Re-Encryption:**
  1. Encrypting existing records into `Sealed` or rotating the DEK writes all new `.ansv` envelopes into an isolated staging directory/table (`vault_migration.staging`) alongside a signed SHA-256 progress journal (`migration.journal`).
  2. Every staged record undergoes a mandatory **read-back decrypt & Poly1305 verification check** against the original plaintext hash before the migration transaction can advance.
  3. **Interrupted Migration:** If power fails, the process crashes, or an I/O error occurs mid-migration, startup recovery detects the incomplete `migration.journal`, purges `vault_migration.staging`, and retains the untouched original records (`RollbackClean`).
  4. **Original Plaintext Preservation:** Original plaintext records are NEVER deleted automatically during initial M2.7 migration; they remain backed up in the pre-migration M2.5 snapshot until an explicit, user-confirmed `purge_migrated_plaintext` command is executed after verification.
* **Wrong Key & Tamper Handling:**
  * Wrong passphrase, wrong recovery key, corrupted ciphertext, or modified AAD header bytes MUST return a single uniform error (`VaultError::AuthenticationFailed`) with exponential backoff (`500ms * 2^attempt`, capped at `30s`) and MUST NOT modify or delete any stored bytes.
* **Lost Passphrase + Lost Recovery Material:**
  * Because `XChaCha20-Poly1305` + `Argon2id` has no backdoor, losing both the passphrase and the 256-bit Recovery Key renders `Sealed` ciphertexts permanently unrecoverable. The user may invoke a destructive `vault_reset_sealed_only` which archives the unrecoverable `.ansv` files to `vault.orphaned/` and initializes a fresh DEK without harming `Public`, `Standard`, or `Private` records.

---

## 7. Required Approval Evidence

### 7.1 Threat Model & Trust Boundaries
* **Trusted Computing Base (TCB):** Local Rust process (`anansi-core`) during the `Unlocked` window and the OS kernel/Credential Manager.
* **Untrusted / Mitigated Threat Vectors:**
  1. *Cold Disk / Stolen Laptop / Cloud Backup Leak:* Mitigated by `Argon2id (64 MiB)` + `XChaCha20-Poly1305` authenticated envelopes.
  2. *Ciphertext / Header Bit-Flipping or Downgrade Attack:* Mitigated by binding the entire 64-byte header (`version`, `alg_id`, `kdf_params`, `salt`, `nonce`, `ct_len`) into Poly1305 AAD and failing closed on unknown versions.
  3. *Metadata & Graph Inference While Locked:* Mitigated by encapsulating titles, tags, and relationship edges inside the encrypted payload and omitting sealed nodes from locked list/graph queries.
  4. *Browser / Frontend Compromise (XSS / LocalStorage scraping):* Mitigated by never sending keys or passphrases to the UI; browser holds only a revocable in-memory session handle.
  5. *Log & Crash Dump Leakage:* Mitigated by `ZeroizeOnDrop` wrappers and redacted `Debug`/`Display` traits.

### 7.2 Mandatory M2.7 Certification Test Matrix (`tests/vault_certification.rs`)
M2.7 cannot be marked complete until all 7 automated deterministic tests pass:
1. `test_known_vector_xchacha20poly1305_argon2id`: Verifies deterministic key derivation and AEAD output against pinned test vectors.
2. `test_envelope_header_and_aad_tamper_fails_closed`: Flips single bits across `magic`, `version`, `alg_id`, `m_cost`, `salt`, `nonce`, `ciphertext`, and `auth_tag`; asserts every mutation fails prior to plaintext emission.
3. `test_wrong_passphrase_and_wrong_recovery_key`: Asserts decryption with an incorrect passphrase or recovery key returns `VaultError::AuthenticationFailed` and leaves storage untouched.
4. `test_recovery_key_unwraps_dek_and_rotates_passphrase`: Locks vault, discards passphrase, recovers DEK via Recovery Key, sets new passphrase, and verifies full read access to `Sealed` records.
5. `test_restart_lock_and_zeroization`: Simulates service restart / state drop; asserts vault boots `Locked` and previous `session_handle` is rejected.
6. `test_locked_allow_deny_and_zero_metadata_leak`: Queries `list`, `read`, `history`, `relationship`, and `export` while `Locked`; asserts zero `Sealed` titles, tags, or edges appear in responses.
7. `test_interrupted_migration_atomic_rollback`: Injects a simulated crash after encrypting 50% of records during migration; verifies automatic rollback restores 100% of original records intact.

### 7.3 Explicit User Approval Record
- [x] **APPROVED:** Architect Robin-Kevin Vettik explicitly approves this M2.6 Vault Security Design and authorizes proceeding to M2.7 (`vault.rs` implementation and Cargo dependency addition) immediately upon M2.5 verification.