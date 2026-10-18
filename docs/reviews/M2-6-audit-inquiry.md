# M2.6 — Prepared audit evidence inquiry

Status: UNSENT. Prepared 2026-10-19. No maintainer contact, issue, email, paid review or audit commission has occurred. Proposed destination: one public documentation question in [RustCrypto/password-hashes issues](https://github.com/RustCrypto/password-hashes/issues). Read-only GitHub API checks confirmed issues enabled, discussions disabled, and no repository-local issue template or CONTRIBUTING.md at the checked paths. This is an audit-documentation question, not a vulnerability report; do not use private security reporting without a vulnerability. User authorization to publish the message is still required.

## Proposed message

Title: Independent implementation audit scope for RustCrypto argon2

We are evaluating RustCrypto argon2 for a local-first desktop vault whose project requirements retain an independent implementation-audit gate. Our proposed pin is argon2 0.6.0, package SHA-256 `134c52ddac6d63c576bef8168db10c83c49c26444ecbc68060fef078925a901c`, packaged VCS revision `b1e0ad6fe229b1ba74e4696c7359ab45d7e931f0` (argon2 directory). The intended raw KDF profile is Argon2id v0x13, m=65536 KiB, t=3, p=4, with bounded import parameters. No vault implementation has begun.

Could you point us to any independent review report that actually includes this Rust Argon2 implementation, with the audited revision, dependency/feature scope, findings and remediation references? If the review predates this release, is there a published mapping or follow-up for relevant changes? A public report or auditor-authenticated scope statement would help; an algorithm review or an application merely depending on Argon2 would not establish the implementation coverage we need.

We found the NCC 2020 AEAD review, which does not include password-hashes, and the NCC 2021 opaque-ke report, which explicitly excludes its later Argon2 switch from the targeted release. We have not located matching evidence; we are not claiming that no review exists or that the implementation is insecure. If there is no shareable report, knowing that would help us scope an independent review rather than infer coverage.

Thank you for any pointers.

## Evidence needed before G1 can close

Record report publisher/authenticity, auditor/date, exact source/version and feature/platform/dependency scope, findings/fixes, and mapping to the selected published package. Also finish the current AEAD closure mapping in [the evidence review](../evidence/M2-6-audit-review.md). Resolve actual transitive source identities and compile paths in an isolated review artifact before treating direct-package checksums as sufficient. Do not commit new product dependencies to manufacture audit evidence.

If a new independent review is needed, request an explicit scope and cost proposal covering Argon2id core/BLAKE2 dependencies, parameter validation and memory handling, selected feature/platform paths, current AEAD dependencies and their historical changes. Such a proposal is not authorization to spend money or start M2.7. G2–G4 and the pure-Rust/p=4 contract remain adopted.
