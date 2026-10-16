# ANANSI Ultimate — Build Status

## Current state — 2026-10-16

M0 and M1.0–M1.3 are verified complete. M1.3 implements local workspace create/open and durable reduced-motion settings through the actual Windows desktop and authenticated browser adapters. Checkpoint 0f58a9f passed all three CI jobs in run 38902347007 and was published to protected master. M1 remains incomplete and has no completion tag.

Verified implementation checkpoint: 0f58a9f310294fb2dda3ce49c21b1814bb6d14b1. This completion record references the prior verified implementation SHA to avoid self-reference. See [M1.3 evidence](evidence/M1-3.md), [M1.2 history](evidence/M1-2.md) and [active plan](../plans/PHASE-001-runnable-shell.md). Independent source-bundle restore matched HEAD/tree; restored docs, frozen installation, types and seven client tests passed.

## M1.3 acceptance and verification

- Shared core list/create/open/settings operations persist a stable workspace ID and validated reduced-motion preference in a private versioned SQLite catalog. Both editions use generated contracts and the same UI.
- Real release Windows Tauri/WebView2 and production browser workflow passed shared create/reopen, updates in both directions, both process restarts and actual reduced-motion CSS. Screenshots inspected. Existing native readiness, schema rejection and shell-denial checks also passed.
- Transactional migration rollback retains original rows; foreign keys, WAL, busy timeout, duplicates/invalid names/paths, initialized concurrent access, read-only permissions, links, corrupt/unknown/newer schema preservation and authenticated HTTP parity are tested. Existing catalogs are inspected read-only before write access. No destructive reset exists.
- 24 Rust tests, 7 contract/client tests, 4 browser workflows, 36 foundation Node tests and 13 source-recovery checks passed. Formatting, Clippy with warnings denied, types, contract drift, frozen install, actual optimized desktop build and documentation/whitespace checks are recorded in evidence. Master specification is unchanged; 90 source sections, 16 plans and 33 requirements are retained.

## Remaining scope and next task

Next approved task is M1.4 first-run/navigation/shared accessible design; it has not started. M1.5 installer and M2–M15 retain their approved scope. Workspace storage is metadata/settings only: no memory, AI, encryption/vault, automatic user-data backup or installer completion is claimed. AN-002/003/018/020/023 remain in progress. Source recovery excludes private catalogs; preserve the whole closed storage root for manual data recovery.
