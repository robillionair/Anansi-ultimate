# ANANSI Ultimate — Build Status

## Current state — 2026-10-16

- M0.1 and M0.2 complete; M0 remains incomplete. Stop before M0.3.
- Next approved task: M0.3 independent source backup/restoration in the approved M0 plan.
- Current task authorization: user continuation after M0.1; only M0.2 executed.
- Last verified implementation checkpoint: 873f10b1c4789a4202130ef3b59c5997dac9212d (pushed to origin/master; Windows/Linux CI success).
- Project-state records completed M0.2 and next M0.3; subsequent publication metadata commits intentionally refer to the verified implementation checkpoint.
- Product implementation, desktop/web builds, installer, migrations and provider integrations: not available.

## M0.2 acceptance and evidence

Dependency-free documentation/state validator, positive/negative fixture tests, honest root commands and SHA-pinned least-privilege Windows/Linux CI are implemented. Legacy planning command remains compatible. Existing architecture and master specification are preserved.

- `pnpm check`, `pnpm check:docs`: passed; 90 source sections, 16 milestone plans, 33 requirements, local files/headings/reference links and project state/evidence checked.
- `pnpm test`: 28 passed, 0 failed/skipped. `pnpm test:docs`: 23 passed. Existing Git hygiene checks pass.
- Frozen dependency install passed. No dependencies or product packages added.
- [Actual CI run 38837748893](https://github.com/robillionair/Anansi-ultimate/actions/runs/38837748893) succeeded on both ubuntu-latest and windows-latest, including tests, lockfile and whitespace checks.
- Master protection applied and independently read back: strict required Foundation checks on both platforms, GitHub Actions app 15368, admin enforcement ON, force push/deletion OFF.
- All M0.2 acceptance items passed. Detailed measurements, API status and limitations are in [M0 evidence](evidence/M0.md).
- `pnpm build` and `pnpm backup:source` intentionally exit 1 explaining unimplemented scope; tested, not product-build or backup evidence.

## Remaining

M0.3 must implement and prove independent source backup/restore; no M0 completion tag yet. Its selected test runner is plain PowerShell assertions, without a Pester dependency. No backup implementation/restore drill ran in M0.2.

Rust/Cargo and standard Windows native tools remain future M1 prerequisite gaps. No product behavior, inference or private data processing tested. AN-022 remains in progress until all source recovery requirements pass.

Current master protection applies to administrators. Publish/check a task branch before updating master; do not bypass protection. Final completion records are published separately from the implementation checkpoint and verified in the task report.
