# M2.6 — Owner unaudited-primitive risk acceptance

Authority: direct Owner Directive, 2026-10-19, subsequently approved for repository recording and execution. This is an explicit engineering exception to master §75's audited-library prerequisite; `docs/MASTER_SPEC.md` remains unchanged. It supersedes Correction 5's pre-implementation block and the corresponding G1 implementation prerequisites in the vault and Memory OS plans. G2–G4 remain normative without relaxation.

G1-External-Audit remains **open**, deferred until post-M3 and before pre-production/commercialization/enterprise certification. This amendment does not establish independent audit coverage, certify production security, or mark M2.7 complete. No commissioned review is authorized or funded. The Owner's categorical audit assertions below are preserved as their factual/risk acceptance record; the independent evidence remains the narrower qualified maintainer statement and recorded historical report/source delta gaps.

## Owner Directive — verbatim

"As Repository Owner and Authorizing Architect, I accept your `ANANSI M2.6 G1 — Independent review brief` as an accurate factual record:

1. `argon2 0.6.0` (`134c52ddac6d63c576bef8168db10c83c49c26444ecbc68060fef078925a901c`, rev `b1e0ad6fe229b1ba74e4696c7359ab45d7e931f0`) does **not** have an independent third-party security audit.
2. `chacha20poly1305 0.11.0` (rev `e37a978ccf0992d9053fbc039470d6527108e393`) has drifted since the February 2020 NCC Group report (`2020-02-12 v1.0`) and its delta is unreviewed by an external audit firm.
3. Because Robillionair OÜ is at the pre-seed bootstrap stage ($0 external audit budget), no commissioned third-party cryptography report will arrive on the evidence monitor during M2.

**Authoritative Contract Amendment (Owner Directive):**

Do **not** falsify audit coverage or claim G1 is satisfied by a third-party audit report. Instead, I am formally amending the repository's M2.6/M2.7 gate contract as follows:

- **Retain the `ANANSI M2.6 G1 — Independent review brief`** as an open pre-production/commercialization audit requirement (`G1-External-Audit`), deferred to post-M3 / pre-enterprise certification. Documentation and UI must explicitly label Sealed Vault cryptography as *'Implemented using pinned RustCrypto primitives (`argon2 0.6.0`, `chacha20poly1305 0.11.0`); third-party cryptographic audit pending (G1 open)'*.
- **Unblock M2.7 Engineering Implementation (`vault.rs`):** Modify the M2.6 $\rightarrow$ M2.7 implementation gate in the authoritative plan so that M2.7 implementation is authorized to proceed under **Explicit Unaudited-Primitive Risk Acceptance**, subject to:
  1. Preserving `Argon2id v0x13` (`m=65536 KiB, t=3, p=4`), `XChaCha20-Poly1305`, `zeroize 1.9.1`, and `keyring 3.6.3` with exact pinned versions.
  2. Verifying M2.5 snapshot/restore completion first.
  3. Implementing all M2.7 deterministic vector tests (`p=4` lane consistency, tamper fail-closed, wrong-key handling, recovery key rotation, and restart lock) without touching any files owned by the active M3 agent.

Please tell me which exact authoritative plan/contract file(s) define the M2.6 G1 blocker so we can commit this Owner Risk-Acceptance Amendment and start M2.7 once M2.5 is verified."

## Execution and disclosure boundary

The subsequent Owner instruction authorizes recording/synchronizing this amendment and beginning M2.7-Core in an isolated worktree. No shared M3 checkout edits, Search/UI changes or automatic merge/publication into the active M3 branch. Search filtering gets an internal vault-owned hook; wiring and UI disclosure remain deferred until ownership transfer/integration. The required “Implemented…” wording is prospective and must not be displayed as current capability until implementation passes verification. Pre-implementation status is **engineering authorized; unimplemented; third-party cryptographic audit pending (G1 open)**.

M2.5 baseline `6c193ef8c9891e966c547451b4e01b7db74215b4` was rechecked on 2026-10-19: exact-SHA run 39164526567 has all three jobs successful; the existing checkpoint-restored clone is clean and matches HEAD/tree `03370d47c4f21c4e04c62d4c274455f7f2fd0bee`; checkpoint.bundle SHA-256 matches `D4860C1846CC77508B33556809D6C51214822B756C0E820DC2BCBCEC0CD371AD`. This rechecks source recovery and recorded completion, not a new application restore workflow or a guarantee about live private data.

The independent review brief is retained at `C:/Users/robin/Documents/ChatGPT/Anansi-source-recovery/M2-6-audit-review/independent-review-brief.md`; a tracked copy accompanies this amendment as [the external review brief](../reviews/M2-6-independent-review-brief.md).
