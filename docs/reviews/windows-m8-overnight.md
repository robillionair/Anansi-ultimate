# M8 overnight hardening — Windows review (2026-10-30)

Incoming `edba7ed32db2f7c5ea999466061c4e1c3966b2e7`, on isolated `codex/review-m8-overnight`.

## Acceptance before review probes

- Verify package identity and reviewed ancestry; preserve Slice 4 review worktree and owner previews/data.
- Confirm current-source refusals and eligible historical controls on Windows.
- Original provenance must not become trusted merely because a generic editor supplies a matching `restoredContentHash`. Forged snapshot content with its own internally coherent hash plus the actual target hash must refuse restore before destination publication.
- Every supported selected artifact must pin its final restored target hash after all fix-ups. A concept selected as context, whose source experiment also changes on remap, must survive two fresh restores without stale hashes.
- Run affected UI, contract, build and browser checks. Report failures; Linux evidence is not Windows acceptance. No production changes, cloud inference, pushes or promotion in this review.

- Each overview link must resolve within its own live/saved result, including when both are mounted. Duplicate global IDs must not send saved-history navigation into the live panel.

## Verdict: acceptance blocked by three independently reproduced findings

Package ZIP SHA-256 `cca22f4a4fea76800fd9975670cd7d125f1a13786376d062223f05e7be1f734f`; bundle SHA-256 `5ab8ce9db6a9a858a9404e22693975b51c78398d8544dbb9252b557d52162dc7`. Exact tip and ancestry verify. Review changes only add tests and this evidence; incoming runtime/UI code remains unchanged.

### R1 — P1: restoredContentHash is a client-forgeable provenance bypass

At `crates/core/src/restore.rs`, `validate_original_experiment_bindings` accepts a source row that does not match the pinned snapshot when the body-supplied `restoredContentHash` matches the row hash. The field has no restore-owned authority. Through the existing generic artifact revision operation, the probe replaces a selected note snapshot with invented content, hashes that invented content, and sets `restoredContentHash` to the actual unchanged note hash. Backup/fresh restore succeeds instead of refusing. No SQL tampering of the experiment, source row, receipt or backup is needed.

Reproduction: `node scripts/cargo.mjs test -p anansi-core --locked --test collider review_forged_snapshot_cannot_self_authorize_with_target_restored_hash -- --exact --nocapture`. Expected refusal/no destination publication; observed accepted restore. This fails the unchanged expected-refusal assertion in `assert_restore_refused`. The supplied fabricated-hash test only checks a hash that does not match the real target, so it misses this case.

Required repair: establish original-to-restored provenance from backend-controlled authority or a verifiable allowed transformation; do not treat an editable body field as authorization. Keep legitimate repeated restore and immutable generation-time snapshot semantics. Preserve refusal before destination publication. Include generic registration/revision attacks and valid restored controls.

### R2 — P1: a selected concept's restored hash goes stale after later fix-ups

`fix_experiment_project_artifact_hashes` converges only experiment-context hashes and runs before `fix_transfer_experiment_hashes` / `fix_transfer_receipt_hashes`. A supported concept selected as context changes again in the later pass. Probe: save a memory-context experiment, transfer it, select the concept as context in another experiment, save, restore, then back up/restore again. First restore succeeds; selected `restoredContentHash` differs from the final restored concept row. The second fresh restore fails with `InvalidRequest`.

Reproduction: `node scripts/cargo.mjs test -p anansi-core --locked --test collider review_concept_context_pins_final_hash_after_transfer_fixups_and_restores_twice -- --exact --nocapture`. Observed diagnostic: `selected concept pins final restored target hash: false`; second restore refused.

Required repair: resolve all relevant artifact dependencies against final materialized rows, including concept/receipt and chained experiment dependencies. Do not merely move the same one-off pass to another point without testing combinations. Detect/refuse unsupported cyclic graphs boundedly. Preserve original pre-validation and receipt/Forge admission rules.

### R3 — P2: saved overview links resolve to the live result

`ColliderOverview.tsx` and `ColliderResultView` use global fixed IDs. `Projects.tsx` mounts the live Collider before Artifacts; both results remain mounted after a saved experiment opens. The saved overview's `#collider-combinations-heading` therefore resolves to the live panel, not the saved source/version. The existing browser test scopes its locator to the saved overview but does not check its actual target.

Reproduction: `node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/review-collider-anchors.spec.ts`. The canonical workflow passes through save/reopen, then the added strict DOM target assertion fails: expected true, received false. Give each result/overview instance a stable unique prefix and keep headings/aria references/navigation aligned. Also scope provenance links when multiple disclosure panels exist. Preserve keyboard and saved-version behavior.

## Verified Windows scope

- Current-source boundary tests: **7/7 passed** (both pickers, archive/Sealed/expiry/reassignment, eligible historical control).
- Independent restore probes: **0/2 passed**, both expected-success/refusal assertions fail as detailed above; these are actual defects, not acceptance passes.
- UI/contracts **89/89 passed**; TypeScript, strict core all-target Clippy, formatting, generated contract drift and documentation checks passed. Contract generation emits existing ignored-`uint` format warnings; no drift failure. API/web builds passed; web retains its bundle-size warning.
- Browser on real Windows API/web with synthetic provider: integrated selection/transfer/restart/two-restores **passed**; canonical project-context workflow **passed**; independent overview-target probe **failed** at the new final assertion. Total **2 passed / 1 failed**. No timeout or existing assertion changed.
- Shared application-test lease was acquired by the repository wrapper and released. Owner previews/data untouched, all builds/temp/evidence on E:. No cloud calls, downloads, generated execution, push, master merge, CI or preview promotion.
- This is focused review, not a full-core rerun or native desktop certification. Earlier Slice 4 incoming backend suites passed 54 tests on `0b6ba0e8`; those passes are historical baseline evidence and are not transferred to this new runtime tree.

## Handoff

Use `codex/review-m8-overnight` (incoming `edba7ed3` plus additive reviewer probes/docs) as the repair baseline. Preserve every assertion and original delivery; do not promote. Muse owns implementation of R1–R3; reviewer owns independent Windows rerun. Evidence directory: `E:/Anansi/benchmarks/m8-overnight-windows-2026-10-30`.

