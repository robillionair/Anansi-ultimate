# M2.6 G1 — Crypto library decision proposal

Status: user selected Option A on 2026-10-19: **“Retain pure Rust; obtain matching audit evidence.”** G1 remains open. Option B is retained as comparison research only and is not selected. No dependency, native binary, KDF profile or product behavior changes. The original [contract](../docs/approvals/M2-6-user-contract.md) and [adopted corrections](../docs/approvals/M2-6-corrections.md) remain normative. See [the follow-up evidence](../docs/evidence/M2-6-audit-review.md).

## Objective and acceptance

Establish primary audit evidence and exact source/version identity, distinguish historical scope from current-source coverage, identify implementation/profile/build changes required by alternatives, and leave a concrete review decision. Do not equate popularity, cryptanalysis, matching vectors, an audit of an application consuming a crate, or a blanket upstream statement with an audit of the proposed implementation. No M2.7 code or crypto execution in this task.

## Option A — Retain the approved pure-Rust selection

The requested RustCrypto chacha20poly1305 0.11.0 / argon2 0.6.0 pins preserve the approved API, Argon2id defaults m=65536 KiB/t=3/p=4, all supported parameter bounds and current ANSV definitions. Version availability was verified in [the main plan](M2-vault-security.md).

NCC's historical AEAD review is relevant to ChaCha20/Poly1305, but does not establish Argon2 implementation coverage. Further primary-source searches of RustCrypto and auditor sites still did not locate an independent report for the proposed Argon2 release. Unrelated projects' “audited RustCrypto” descriptions and C# ports are not primary audit evidence. This is an evidence gap, not a finding that the crate is insecure or that no report can exist.

To close A: obtain a report identifying the RustCrypto Argon2 source revision, variant and dependencies, map it to the actual selected release and record relevant changes/fixes. An existing report could satisfy this if its scope matches; otherwise an independent review is needed. The user separately authorized publishing the prepared audit-evidence question as upstream issue 940; this proposal authorizes no paid audit or commercial commitment.

## Option B — Investigate native libsodium with an explicit contract amendment

The [primary libsodium 1.0.12/1.0.13 assessment](https://www.privateinternetaccess.com/blog/libsodium-v1-0-12-and-v1-0-13-security-assessment/) provides stronger direct KDF evidence than was available for A. Its scope includes password hashing and authenticated encryption; section 3.5.8 explicitly discusses the Argon2id addition. The report is historical, not an audit of release 1.0.22 or of ANANSI. It records low-severity findings and must not be summarized as “no findings.” The [maintainer's introduction](https://doc.libsodium.org/) links this assessment.

Candidate source: [libsodium 1.0.22-RELEASE](https://github.com/jedisct1/libsodium/releases/tag/1.0.22-RELEASE), resolved through the annotated tag to commit `77e1ce5d6dee871c49ef211222ba18ef0c486bda`. Historical release 1.0.13 resolves to `7d5d9204e515ce37cce6d5e5015a1f94d1d92492`. These identities were read from GitHub's Git ref/tag/tree APIs, not guessed from a moving branch.

Candidate Rust FFI pin: libsodium-sys-stable `=1.24.3`, default features off. The registry release was not yanked. Its `.crate` archive SHA-256 was actually checked as `1f55c1c508d31e2cf0a649c9c889e4bcf554cbfb2a6803f049f219784b044283`; package metadata records VCS revision `353c0d2fe08dfb881f5be75daa95e6f29177c277`. This is release provenance, not an audit of the Rust bridge or proof its packaged files are identical to that Git revision. Exact packaged manifest/build logic was inspected without running it. The bridge identifies its library as libsodium_sys. Use only a narrow reviewed safe Rust wrapper around the required public functions; do not expose raw pointers or a generic FFI API to either frontend.

### Material contract differences requiring a decision

| Existing requirement | Native proposal and consequence |
|---|---|
| Pure-Rust RustCrypto primitives | Native C libsodium plus Rust FFI; additional build, ABI, packaging and unsafe-boundary review. This cannot be silently called the original pure-Rust design. |
| Argon2id default p=4; import p range 1..16 | The [pinned public implementation](https://raw.githubusercontent.com/jedisct1/libsodium/77e1ce5d6dee871c49ef211222ba18ef0c486bda/src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c) passes lanes=1 to its raw function. Propose m=65536 KiB/t=3/p=1, with p=1 required in imported passphrase envelopes and the corresponding direct-key canonical header fields. A changed p yields different keys, not an optimization that can be ignored. Reject p!=1 explicitly before KDF; never relabel a p=4 header or derive with p=1 underneath it. |
| RFC 9106 p=4 primitive fixture | Retain that fixture as an unsupported-profile rejection case for the native API. Use independent published libsodium p=1 KDF vectors plus XChaCha AEAD vectors for accepted profiles; preserve all seven named certification test objectives. This changes parameter coverage and must be explicit. |
| Existing ANSV header/AAD/context | Keep the exact 64-byte header AAD, algorithm IDs, nonce/tag/salt sizes, purpose/context keyslots and all G2–G4 corrections. The KDF algorithm is still Argon2id v0x13; a p=1-only backend must declare its supported profile. No existing encrypted vault data exists to migrate in schema 8, and no cross-profile decryption compatibility is claimed. |
| Fallible Rust OS entropy | Keep getrandom for application keys/nonces. Check native initialization and FFI return codes, validate lengths before entering C, and review native fatal/misuse behavior. Do not install a custom libsodium RNG or assume every C-level failure is a recoverable Rust Result. |

**Decision:** retain A and obtain matching evidence. The earlier proposal to investigate B was not selected by the user. Do not implement the native build proposal below, downgrade releases to match an old report, or use private/internal Argon2 symbols to evade the public p=1 constraint.

## Historical-to-current coverage map

[The reproducible selected-source inventory](../docs/evidence/M2-6-libsodium-components.json) compares Git blob identities in selected password-hashing, XChaCha/ChaCha/HChaCha/Poly1305/BLAKE2, RNG, utility and public-header subtrees. It found 59 selected files in 1.0.13 and 64 in 1.0.22: 15 identical paths/blobs, 41 changed paths, eight added and three removed. Moves can appear as add/remove; these counts are not vulnerability counts or a full transitive source audit.

For example, HChaCha20 core is byte-identical at Git blob `39ab26a6da894e75cc648a5868fd663237ffdcc2`, while Argon2 core, the reference block implementation and the public Argon2id wrapper differ. XChaCha AEAD moved paths and differs in source identity. Therefore the old report cannot be passed off as complete current-version coverage. The [pinned release history](https://raw.githubusercontent.com/jedisct1/libsodium/77e1ce5d6dee871c49ef211222ba18ef0c486bda/ChangeLog) also records later implementations/optimizations and security maintenance. A changed file may contain only maintenance, or security-relevant changes; a hash comparison alone cannot decide which.

To close B's audit condition, inspect the actual diff and selected compile/runtime dispatch path for every relevant changed/new source and build dependency, correlate fixes with upstream advisories, and obtain/record independent coverage where the retained audit requirement demands it. Classify unchanged, moved, formatting-only, implementation-changing and newly selected platform paths separately. Record residual coverage limits explicitly. A developer's source review and this inventory are not a new third-party audit. Never downgrade to 1.0.13 simply to match a report.

## Native build provenance proposal

The exact bridge package's build script can retrieve mutable distribution names such as LATEST.tar.gz; a Cargo crate pin alone is insufficient to pin native code. It also has system-library discovery and native binary fallback paths. Its SODIUM_LIB_DIR route links a caller-supplied library and reports an unknown native version; it does not independently verify a version or library hash. ANANSI must supply that provenance if B is adopted.

Propose a tracked native manifest and preparation script, deterministic per-target static source builds and SODIUM_LIB_DIR pointing only at their verified output. Forbid implicit system discovery, mutable latest downloads and automatic binary fallback in the controlled build wrapper. Verify trusted publisher signatures and committed hashes before extraction/build; build with the existing pinned MSVC toolchain on Windows and tested host tools in Linux CI. Keep acquisition/build outputs outside committed source. Builds and native API/version checks must run before feature activation.

GitHub release API metadata reported these candidate archive digests (the native archives and signatures were not downloaded or independently verified in this task):

| Asset | Reported SHA-256 |
|---|---|
| libsodium-1.0.22.tar.gz | `adbdd8f16149e81ac6078a03aca6fc03b592b89ef7b5ed83841c086191be3349` |
| libsodium-1.0.22-msvc.zip | `3e03a726fac4bc09cb61d8f29d658ef7a5eca0811de59082130414f7ca2e4279` |

Prefer rebuilding the source over distributing opaque prebuilt code. Version strings and hosted digests alone do not authenticate publisher identity. The future preflight must verify the publisher's detached signature with a pinned trusted key, source/content identity, actual static library hash, linked sodium_version_string and the target's complete native dependency closure. The MSVC ZIP is listed for provenance comparison, not selected as a default fallback.

If B is adopted, add exact planned owners to the main implementation plan: `scripts/prepare-sodium.ps1`, `scripts/prepare-sodium.sh`, `native/libsodium-manifest.json`, `crates/core/src/vault/crypto_backend.rs`, a narrow native bridge dependency in `crates/core/Cargo.toml`/Cargo.lock and native CI preparation. Do not create these product/build files before the design choice and G1 are resolved. macOS source/credential/input/release verification stays explicitly tracked rather than inferred from an x86 Windows build.

## Decision and remaining work

A is selected as the continuing research direction, not as a declaration that G1 passed. Pure Rust, p=4 defaults, supported bounds and the audit gate remain unchanged. Obtain the relevant independent scope/version evidence and then record a final gate decision; do not reopen G2–G4 or repeat the user's blanket vault approval. [The published inquiry](../docs/reviews/M2-6-audit-inquiry.md) identifies the missing evidence precisely. No keys, binaries, cryptographic dependencies, native builds, vault code or sealed migration were introduced or executed.
