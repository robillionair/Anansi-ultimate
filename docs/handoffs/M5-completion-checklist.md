# M5 completion checklist — requirements, evidence, classification

Date: 2026-10-21 · Tree: `integration/m5-into-m2-m3` (merge of
`codex/m2-m3-integration` at `fc2540f` with
`muse/m5-provider-integration` at `29e8da2`).

Classification legend: **Verified** = implemented with
executable evidence (Linux runs here + the owner's independent
Windows passes of r4–r9, cited per round in the addenda).
**Implemented** = code complete, evidence partial or
platform-scoped. **Externally blocked** = cannot proceed
without something outside this work (credentials, a human at a
native dialog, an external party). **Deferred** = consciously
postponed with rationale (design decisions M5-D1…D8).

This checklist deliberately does NOT equate the provider
package with all of M5: the package (M5-R1…R17) is one part;
application integration, owner-boundary rounds, and the UI
workflow are the rest, and several M5-adjacent gates remain
open below.

## Package requirements (from M5-requirements-and-acceptance.md)

| ID | Classification | Evidence / rationale |
|---|---|---|
| M5-R1 provider-independent abstraction | **Verified** | One dispatcher drives all adapters through the trait; 120 provider tests on Linux + Windows (r9 pass). |
| M5-R2 §14 breadth: local/OAuth/free/API-key/enterprise/custom | **Implemented** | Catalog covers all 20 §14 names with explicit status; OAuth-connected use is contract-only (see R4); Bedrock/Vertex deferred (R16); Cloudflare catalog-only. No entry is faked as live. |
| M5-R3 auth verified against official docs | **Verified** | `M5-provider-verification.md` (checked 2026-10-20); catalog auth fields match it; unit test forbids fabricated OAuth entries. Re-verification against live docs is due again before any live-provider work. |
| M5-R4 OAuth-first (PKCE etc.) | **Deferred** | Contracts + RFC 7636 vector + state validation unit-tested. Live exchange needs the system-browser hand-off + loopback redirect + provider client registrations — none exist in the product yet, and no provider in the catalog was verified to offer user OAuth for inference (R3 record). Unblocks: an owner decision to register an OAuth client with a specific provider that supports it. |
| M5-R5 API-key path via OS vault | **Verified** | References-only config; native-prompt → keyring lifecycle (r9) proven prompt→keyring→resolver→wire against synthetic servers; real OS store untested (see pending checks). |
| M5-R6 no scraping/cookie/password capture | **Verified** | No such code paths; endpoint policy tests; the native prompt is the OS's own control (the vault pattern), not a capture form. |
| M5-R7 Zero-Cost Lock | **Verified** | Gate tests with zero-hit assertions; UI default-on; e2e refusal stages (r8, r9). |
| M5-R8 real usage or honest Unavailable | **Verified** | Usage parsed from responses; `QuotaReport::Unavailable` first-class; no invented allowances anywhere in the crate (grep-auditable). |
| M5-R9 eight routing modes + fallback | **Verified** | Router unit tests per mode incl. Private-never-cloud; wire-binding round (r5) proved gates/permit/wire/disclosure describe the same candidate. |
| M5-R10 budgets + pre-dispatch estimates + disclosure | **Verified** | QuotaLedger with atomic reservations (concurrency-proven, r4); estimates in every disclosure record; UI usage panel. §67 profiles are qualitative in the spec; ceilings are always caller numbers (M5-D7). |
| M5-R11 §19 pre-transmission pipeline + disclosure | **Verified** | PrivacyGate ordering; Sealed refused for every provider kind incl. local; disclosure DTO end-to-end in UI ("View what ANANSI sent"). "Restricted content" is caller-classified (M5-D4) — a repo-level definition remains an owner decision. |
| M5-R12 native enforcement, SSRF/spending protections | **Verified** | Endpoint policy + transport deadline tests; sliced-read transport (r7); spending bounded by permits + ledger reservations. |
| M5-R13 OS credential storage; no secrets in backups/Git | **Verified** | Keyring service separate from the vault's; configs hold references only; tests assert secrets never appear in errors/Debug output. |
| M5-R14 streaming/cancellation/bounded timeouts | **Verified** | Synthetic-server suites incl. idle-stream cancel (r7, Windows-verified); one overall deadline (r3 addendum). |
| M5-R15 sanitized errors, retry behavior | **Verified** | Error taxonomy + redaction tests; billable-safe retry semantics (post-send ambiguity is terminal for Paid/Unknown). |
| M5-R16 custom/enterprise endpoints | **Implemented** | Custom OpenAI-compatible + Azure implemented and synthetic-tested. Bedrock/Vertex **Deferred**: IAM signing (SigV4 / Google ADC) is a separate verified design; catalogued Deferred, never stubbed. |
| M5-R17 no paid inference without authorization | **Verified** | Permit gate with zero-traffic refusals at every layer (crate, core, API, UI); no paid call has ever been made by this work. |

## Application-integration requirements (owner rounds r3–r9)

| Item | Classification | Evidence |
|---|---|---|
| Core/API wiring of all provider ops | **Verified** | API two-session boundary suite; merged-tree regression. |
| Authorization boundary (no policy in dispatch DTOs; backend-held permits) | **Verified** | r4 round + adversarial core scenarios; forged callerContext overwritten (API test). |
| Derived locality (no caller-asserted local/cloud) | **Verified** | Masquerade rejected both directions; tampered persisted config rejected identically. |
| Logical workspace scoping + isolation | **Verified** | Two-workspaces-one-root isolation tests. |
| Backend lock released during network I/O | **Verified** | Concurrency tests (slow dispatch vs poll/cancel/other workspace). |
| Owner authorization for configure/authorize (host or live grant) | **Verified** | r5 API test (403 before grant / 200 after); r9 approval-queue workflow over real HTTP. |
| Windows storage-latency defect (r5 finding) | **Verified** | Root-caused to powershell-per-open; poll/cancel storage-free + positive-only cache; owner's Windows rerun passed. |
| Idle-stream cancellation defect (r6 finding) | **Verified** | Sliced reads; closed by owner's Windows verification at `7301022`. |
| Full UI workflow (configure → authorize → dispatch → stream → cancel → disclosure/usage) | **Verified** | Playwright e2e on the real stack (synthetic provider); component contracts 36/36. |
| Credential setup/removal via trusted native controls | **Implemented** | Flow proven against keyring mock + debug test prompt; the real Windows CredUI interaction is a pending manual check (below). |
| Host browser-session approval/revocation UI | **Verified** | Queue + card + granted-browser HTTP workflow test. |
| UI cancellation honesty ('cancelling' until backend confirmation) | **Verified** | State machine + e2e incl. timed unmount-cleanup stage. |
| Configuration persistence (restart-safe, hardened) | **Verified** | Cross-process restart tests; failure-injection + symlink + race tests (r9). |

## Pending checks — kept visible, never counted as passed

1. **Windows native/manual acceptance** — real Win32 CredUI
   provider-credential prompt (set/remove/status against the
   real Credential Manager), and the pre-existing manual vault
   dialog check from the M2–M4 line. *Owner-in-the-loop;
   computer-use rules forbid automating OS auth dialogs.*
2. **Live-provider verification** — any real provider call
   (incl. a local Ollama through the M5 dispatcher, as opposed
   to the M2–M4 Ask path) needs owner authorization and, for
   cloud providers, real credentials. Synthetic evidence does
   not establish real-model behavior.
3. **External audit (G1-External-Audit)** — open pre-production
   gate under the owner's recorded risk acceptance; unchanged
   by M5.
4. **Remote CI / protected publication** — deferred by owner
   capacity policy; nothing in this line has been pushed by the
   M5 agent; `master` untouched.
5. **Real OS keyring on Windows** (vs the mock) — folds into
   check 1.

## Merged-tree regression (this checklist's run, Linux)

- providers 120/120 · core provider suite 18/18 · API 21/21 ·
  UI/contracts 36/36 · tsc clean · contracts drift check clean ·
  docs validation clean (85 files) · fmt clean.
- Full anansi-core suite: only the four pre-existing
  `semantic_workflow` failures that require a live Ollama at
  127.0.0.1:11434 (not present on this VM) — identical to the
  pre-merge baseline on both parent branches.
- Browser checkpoint (merged tree): workspace open → memory
  create/edit → body search hit → provider unary dispatch +
  disclosure → stream + cancel → API restart → dispatch from
  persisted configuration. `tests/e2e/app_checkpoint.spec.ts`.
