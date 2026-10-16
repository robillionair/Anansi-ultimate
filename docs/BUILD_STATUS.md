# ANANSI Ultimate — Build Status

## Current state — 2026-10-16

M0, M1.0, M1.1 and M1.2 are verified complete. M1.2 delivers shared readiness through a real Windows Tauri application and authenticated local browser API. Checkpoint a04b017 passed both Foundation platforms and the actual native CI window check in run 38850910632; protected master accepted that exact SHA. The initial CI debugging failure is resolved and retained in the evidence history. M1 remains incomplete and has no completion tag.

Verified implementation checkpoint: a04b0173d745d1a9eb639ff9ca261b3808d8cbf6. This completion record deliberately references the prior verified source SHA to avoid self-reference. See [M1.1 history](evidence/M1.md) and [M1.2 evidence](evidence/M1-2.md).

## M1.2 acceptance and verification

- Shared Rust DTOs/domain validation, deterministic schema/TypeScript/static validator generation, direct scoped native command and authenticated loopback HTTP transport work.
- Actual production browser workflows passed: pairing/live refresh, wrong-token/unavailable recovery, forget/reload without credential persistence, keyboard form and narrow layout.
- Actual debug and release Tauri/WebView2 windows passed live readiness/refresh, contract rejection and explicit shell ACL denial. Screenshots inspected; CSP remains strict without unsafe-eval.
- Root desktop build produced an optimized executable; it is not an installer. Release SHA256 and precise commands are recorded in evidence.
- 12 Rust tests, 6 contract/client tests, 3 browser workflows, 36 foundation Node tests and 13 recovery checks passed. Formatting, Clippy with warnings denied, TypeScript, contract drift, frozen installation, documentation and whitespace checks passed. Original master hash preserved; all 90 sections, 16 plans and 33 requirements retained.

## Remaining scope and next task

No current blocker remains. Independent source-bundle restore matched the verified commit/tree; restored frozen installation, docs, types and client tests passed. M1.3 SQLite workspace create/reopen and durable settings is next and has not started. No database migration, memory, inference, provider, onboarding/navigation or installer completion is claimed. M1.4/M1.5 and M2–M15 retain their approved scope. Requirement areas AN-002/003/018/020 remain in progress, with readiness evidence rather than full-requirement completion.
