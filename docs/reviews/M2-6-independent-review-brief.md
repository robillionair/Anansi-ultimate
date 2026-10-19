# ANANSI M2.6 G1 — Independent review brief

Prepared 2026-10-19. Draft scoping artifact only; not a commissioned audit, proof of security, gate closure or permission to implement vault cryptography. Stored outside the shared checkout because another AI owns M3. Repository plans and adopted contracts remain authoritative.

## Objective

Obtain independently attributable implementation review evidence for the selected pure-Rust vault primitives. Preserve Argon2id v0x13, default m=65536 KiB, t=3, p=4 and approved bounded import profiles. Do not replace the implementation with native libsodium, reduce parallelism to p=1 or waive G1.

## Source baseline

- Proposed argon2 0.6.0 package SHA-256: `134c52ddac6d63c576bef8168db10c83c49c26444ecbc68060fef078925a901c`.
- Packaged Argon2 VCS revision: `b1e0ad6fe229b1ba74e4696c7359ab45d7e931f0`, `argon2` directory.
- Proposed chacha20poly1305 0.11.0 packaged VCS revision: `e37a978ccf0992d9053fbc039470d6527108e393`.
- Supporting proposed packages: zeroize 1.9.1 and keyring 3.6.3. Consult `docs/evidence/M2-6-rustcrypto-sources.json` for the previously checked archive and file identities.

These are proposed direct-package identities, not a frozen transitive build. Before a reviewer accepts scope, independently verify archives against the registry, compare packaged source to the claimed Git revision and supply the exact dependency graph, enabled features, compiler/targets and platform dispatch paths from an isolated review workspace. Do not alter the M3 checkout or treat a consumer's Cargo.lock as audited evidence.

## Review questions and required coverage

1. Argon2id implementation: variant/version selection, memory and iteration arithmetic, lane and slice processing at p=4 and other approved imported values, address generation, finalization, error paths and malformed/bounded input handling. Identify whether lane count is implemented correctly regardless of thread execution strategy.
2. Dependency closure: identify selected Blake2/digest and supporting implementations, features, target-specific paths and unsafe code. Explicitly list excluded paths and their effect on the intended Windows/browser-adapter architecture. The untrusted browser must not gain native crypto or secret-store capabilities merely through this review.
3. Resource handling: allocation failure, arithmetic overflow, denial-of-service bounds, fallible API behavior, password/key lifetime, copies and zeroization limits. Distinguish best-effort memory cleanup from forensic-erasure guarantees.
4. AEAD coverage: authenticate the historical NCC report provenance, map reviewed source to the proposed current ChaCha20Poly1305/XChaCha implementation and selected dependencies/features, and identify implementation changes requiring independent review. Do not infer current coverage from a historical project-wide label.
5. Findings: provide severity, affected source revisions/configurations, reproducer or rationale, remediation revision and independent retest result. Record unresolved findings and exclusions explicitly.

Primitive vectors, fuzzing, advisory scans and developer code review can support this work but do not substitute for the retained independent implementation review. A primitive report does not certify ANANSI's later vault integration, migration, export or recovery workflows.

## Deliverables needed for a gate assessment

- Reviewer identity and independence; report date and authenticated publisher/report location or verifiable private report provenance.
- Immutable reviewed source identifiers and archive digests, dependency versions/features, platform scope and method.
- Explicit inclusion of the selected Rust Argon2 implementation and intended p=4 profile, rather than only the algorithm or a downstream application.
- Findings and remediation/retest ledger, including residual risks and excluded dependencies or platform paths.
- Historical-to-selected-version mapping where the report predates a selected package; changed implementation paths cannot be called covered without supporting evidence.
- A project evidence matrix mapping every G1 condition to report sections, with gaps left open. Only then assess whether the retained gate can close under the repository's approval rules.

## Known evidence and boundaries

RustCrypto member feedback at https://github.com/RustCrypto/password-hashes/issues/940#issuecomment-6256202461 says he does not think argon2 has ever been audited. This is a qualified statement, not proof about all private reports or a vulnerability finding. The existing acknowledgement is comment 6256248503; no further message is requested by this brief. At preparation, the issue API showed only those two previously handled comments.

The historical NCC AEAD report does not cover Argon2. The previously inspected opaque-ke report excludes its later Argon2 switch. Existing report provenance limitations remain recorded in `docs/evidence/M2-6-audit-review.md`.

## Next actions and acceptance

Continue the existing evidence monitor. If a matching report arrives, verify its provenance and scope against this checklist before proposing closure. If none arrives, obtaining independent review requires an explicitly authorized engagement; this draft authorizes neither contact nor payment. Keep G1 open and do not start M2.7. Another AI retains all M3 implementation, CI repair, commits and publication.

Acceptance for this planning artifact: exact known baseline recorded; unresolved transitive/feature scope explicit; independent deliverables defined; no claim of new audit evidence; no shared repository edits or product execution. No runtime tests are applicable to this text-only draft.
