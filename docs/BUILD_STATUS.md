# ANANSI Ultimate — Build Status

## Current state — 2026-10-16

M0, M1.0, M1.1 and M1.2 are locally verified complete. M1.2 delivers shared readiness through a real Windows Tauri application and authenticated local browser API. Both Foundation jobs passed on 570b4c0; its additional native CI window verifier failed on unavailable CDP. A debug-only test-config repair passes locally and awaits its CI rerun. Protected publication remains pending. M1 remains incomplete and has no completion tag.

Prior verified checkpoint: 91f53fcf7f13897951766afe6376c4cd1e5d6b1b. The following metadata checkpoint will record the newly published source SHA after actual CI succeeds. See [M1.1 history](evidence/M1.md) and [M1.2 evidence](evidence/M1-2.md).

## M1.2 acceptance and verification

- Shared Rust DTOs/domain validation, deterministic schema/TypeScript/static validator generation, direct scoped native command and authenticated loopback HTTP transport work.
- Actual production browser workflows passed: pairing/live refresh, wrong-token/unavailable recovery, forget/reload without credential persistence, keyboard form and narrow layout.
- Actual debug and release Tauri/WebView2 windows passed live readiness/refresh, contract rejection and explicit shell ACL denial. Screenshots inspected; CSP remains strict without unsafe-eval.
- Root desktop build produced an optimized executable; it is not an installer. Release SHA256 and precise commands are recorded in evidence.
- 12 Rust tests, 6 contract/client tests, 3 browser workflows, 36 foundation Node tests and 13 recovery checks passed. Formatting, Clippy with warnings denied, TypeScript, contract drift, frozen installation, documentation and whitespace checks passed. Original master hash preserved; all 90 sections, 16 plans and 33 requirements retained.

## Remaining scope and next task

No local blocker remains. M1.3 SQLite workspace create/reopen and durable settings is next and has not started. No database migration, memory, inference, provider, onboarding/navigation or installer completion is claimed. M1.4/M1.5 and M2–M15 retain their approved scope. Requirement areas AN-002/003/018/020 remain in progress, with readiness evidence rather than full-requirement completion.
