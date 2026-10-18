# M2.6 G1 — Pure-Rust audit evidence follow-up

Review date: 2026-10-19. Status: evidence review completed; G1 remains open. The user selected **“Retain pure Rust; obtain matching audit evidence.”** This preserves Argon2id m=65536 KiB/t=3/p=4, supported import bounds, the original conditional authorization and adopted G2–G4 corrections. Native libsodium is comparison evidence only, not a selected implementation. No M2.7 work is authorized by an unverified audit claim.

## Acceptance and method

Identify exact proposed published sources, inspect independent reports for actual scope, rule out misleading audit associations, preserve the user's choice, and prepare an actionable evidence request. No product code, Cargo manifest/lockfile change, cryptographic execution or paid review is part of this checkpoint. After preparing the request, the user explicitly authorized publishing it.

[The direct-package inventory](M2-6-rustcrypto-sources.json) records downloaded archive SHA-256 values checked against registry checksums, packaged VCS metadata, original manifests and selected source-file fingerprints for argon2 0.6.0, chacha20poly1305 0.11.0, zeroize 1.9.1 and keyring 3.6.3. Archives were read passively in memory; package build scripts were not run. These are direct packages, not a resolved transitive dependency graph. VCS metadata is a package claim, not independent proof that every packaged byte matches Git. Checksums establish source identity, not audit coverage.

| Proposed primitive | Packaged VCS revision | Coverage result |
|---|---|---|
| argon2 0.6.0 | `b1e0ad6fe229b1ba74e4696c7359ab45d7e931f0` | Matching independent report not located; remains blocking |
| chacha20poly1305 0.11.0 | `e37a978ccf0992d9053fbc039470d6527108e393` | Historical review found; current revision/dependency/feature mapping remains incomplete |

## Independent report scope

The NCC 2020 MobileCoin RustCrypto review covers AES-GCM and ChaCha20Poly1305 with supporting cipher/hash/trait repositories. Its scope lists these revisions:

| Repository | Audited revision |
|---|---|
| AEADs | `a15698fdba23ffb17b84d9ecaa2c9c80706ecf03` |
| block-ciphers | `e385f1ebb2ec48547194e51c5193309ee328d93b` |
| stream-ciphers | `1235638004c21dee4e76af4cc932cf1cd815e8f9` |
| traits | `4569d256f02ac0ecefa393baf225fb4a6df35875` |
| universal-hashes | `1ab06bd79542e75490468b227dd3c2cbe42d3d92` |

Source provenance: [publisher announcement](https://www.nccgroup.com/research/public-report-rustcrypto-aesgcm-and-chacha20pluspoly1305-implementation-review/); the publisher's old PDF link redirected to a landing page. The six-page NCC-authored report was recovered from [a third-party mirror](https://pentestreports.com/files/reports/nccgroup/NCC_Group_MobileCoin_RustCrypto_AESGCM_ChaCha20Poly1305_Implementation_Review_2020-02-12_v1.0.pdf), scope on printed page 2. The mirror copy was not independently authenticated against a publisher-supplied digest; later re-fetch also failed. This is useful historical scope evidence with that provenance limitation. It does not cover password-hashes/Argon2 or establish coverage of the proposed current AEAD release. [Upstream audit follow-up](https://github.com/RustCrypto/AEADs/issues/87) provides additional primary maintainer context, not a new audit.

A separate [NCC opaque-ke assessment, December 10, 2021, v1.3](https://www.nccgroup.com/media/0uspzge5/_ncc_group_whatsappllc_opaque_report_2021-12-10_v13.pdf) was read from the publisher. Printed page 2 targets opaque-ke v0.5.0, with limited retesting of later changes. Printed page 25 explicitly excludes the later switch from scrypt to Argon2 from that targeted release. It identifies the Argon2 addition as `535b9b8ee41392c2a0c100f71f5cd04497a61817`. Thus this application/protocol review cannot establish an independent audit of RustCrypto argon2 0.6.0. No findings from that different product are attributed to ANANSI or this crate.

## Search limits and closure work

Primary RustCrypto repository/README/security policy, GitHub audit-related issues and auditor-site searches did not yield a matching Argon2 implementation report. Cargo/RustSec audit CI, algorithm analysis, downstream “audited” descriptions and ports are not independent implementation reports. Search absence is not proof no report exists and is not evidence of insecurity.

The packaged manifests expose dependency ranges and optional features. They do not close the selected build's full dependency/feature/platform coverage. Future feature minimization, zeroization behavior, lane handling, current AEAD changes and primitive vectors still require review and execution after the gate permits implementation. Nothing here changes the approved p=4 profile or silently substitutes a sequential p=1 API.

Next: obtain an auditor/maintainer-provided report or explicit scope evidence, authenticate its provenance, map source revisions/features/dependencies and addressed findings to the selected implementation, and record residual gaps. If no suitable report exists, independent review is required under the retained gate. The user authorized [the concrete inquiry](../reviews/M2-6-audit-inquiry.md), which was published once as [upstream issue 940](https://github.com/RustCrypto/password-hashes/issues/940). A request is not audit evidence; G1 remains open pending a verified response/report. No ongoing monitor was configured. Commissioning a paid review needs separate explicit authorization. This is not a repeated vault approval request.

## Validation and recovery

Executed successfully: `pnpm check` (50 Markdown files, 90 section owners, 16 milestone plans, 33 requirements); `pnpm test` (40 Node tests and 13 PowerShell source-recovery checks); `node scripts/validate-planning.mjs --self-test` (seven invalid fixtures rejected); `git diff --check`. Independently re-fetched all four package archives and matched their archive digests, all 33 recorded file fingerprints, packaged VCS metadata and original manifests. No package build scripts executed.

The master remains blob `b95eae4a3813fb2cd87c00360fcdd9b2489cedec`; peeled M0/M1 tags remain `268533e4` / `97babb3a`. Approved runtime architecture and requirement status remain unchanged; AN-018/019/031 remain in progress. No native/browser vault workflow, KDF vector or encrypted migration was run. Source checkpoint and independent restore results belong in the external `M2-6-audit-review` recovery ledger, with the actual commit/tree rather than a self-referencing SHA. Branch publication is not protected-master publication; check that ledger for exact CI and remote state before continuing.
