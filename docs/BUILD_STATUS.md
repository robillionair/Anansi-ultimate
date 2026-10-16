# ANANSI Ultimate — Build Status

## Current state — 2026-10-16

M0 and M1.0–M1.4 are verified complete. M1.4 adds the shared first-run flow, navigation, privacy review, skippable AI setup and durable intent/motion settings. Checkpoint 4f29dbf passed all three jobs in CI run 38908703118 and was published to protected master. M1 remains incomplete and has no completion tag.

Verified implementation checkpoint: 4f29dbf596ce143cdd53556022373c4704be1143. This completion record references the prior verified implementation SHA to avoid self-reference. Independent source-bundle restore matched HEAD/tree and passed restored docs, frozen installation, types and 11 client/component tests. See [M1.4 evidence](evidence/M1-4.md), [M1.3 history](evidence/M1-3.md) and [active plan](../plans/PHASE-001-runnable-shell.md).

## M1.4 acceptance and verification

- Both editions offer the five specified intent choices, local workspace create/open, truthful privacy review, AI setup skip and working workspace/settings/connection navigation. Completed onboarding and changeable intent persist per workspace.
- Actual optimized Windows application and production browser pass onboarding, cross-adapter settings parity and both-process restart. Keyboard-only browser onboarding/settings, labeled native controls and focus, skip link, OS/saved reduced motion, 390px/1280px layouts and error recovery are verified. Actual screenshots are inspected; no manual assistive-technology certification is claimed.
- Schema 2 adds constrained intent/completion columns transactionally. Version-1 fixture migration preserves IDs/names/motion under concurrent opens; failed forward migration rolls back original rows/version. Existing security/permissions/corrupt/newer-schema tests remain passing.
- 26 Rust tests, 11 client/component tests, 7 browser workflows, 36 Node foundation tests and 13 source-recovery checks passed. Actual release desktop build, native readiness/schema rejection/shell denial, types, formatting, Clippy, contract generation/drift, frozen install, docs/coverage and whitespace checks are recorded in evidence.

## Remaining scope and next task

Next approved task is M1.5 development Windows installer and milestone certification; it has not started. M3 hardware/model setup and M5 provider connections remain retained dependencies, shown unavailable with no fabricated success. Memory, inference, vault/encryption and automatic user-data backups remain future work. AN-002/003/018/020/023 remain in progress; no full product row is advanced by planned navigation entries. Source/master specification and all 90 section owners, 16 plans and 33 requirements are preserved.
