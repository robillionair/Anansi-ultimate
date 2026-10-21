# M6 Slice 1 addendum — artifact registry, as built

Date: 2026-10-21 · Branch `muse/m6-slice1-projects-registry`
over the integrated checkpoint `2098c25`. Design:
`M6-slice1-design.md` (requirement mapping + acceptance
criteria, committed before code).

## What shipped

- **Identity**: artifacts attach to the existing §9 project —
  a memory with `kind='project'` — validated in-workspace on
  every write. No parallel project system, no project-optional
  artifacts.
- **Registry** (`crates/core/src/artifacts.rs`, migration
  `0010_artifacts.sql`): register (v1) / revise (v+1 under an
  expected-version check; stale → `RevisionConflict`, nothing
  stored) / list / versions / read. No update or delete paths
  exist — immutability is structural. Every read re-verifies
  the SHA-256 over the stored body; tamper → `ContentIntegrity`
  and the body is never returned. All nine §9 fields are stored
  and returned (generator = `{"kind":"manual"}` in this slice;
  validation status `unvalidated` — never fabricated).
- **Privacy**: standard/private round-trip; `sealed`
  registration refused with `VaultUnavailable` (fail-closed;
  no vault integration in this slice, recorded in the design).
  Archived projects refuse writes until restored through the
  existing memory lifecycle.
- **Surfaces**: generated contracts (8 schemas; the generator's
  recursion limit was raised 256 → 512 when the map outgrew
  it), `/v1/artifacts/*`, desktop `artifact_command`, typed
  client methods, and the Projects screen's Artifacts section
  (register, latest list, version history, per-version
  read-only view, revise form).

## Two same-class routing repairs (found by the new e2e)

In-process and raw-HTTP tests passed while every browser call
failed: (1) `httpClient` prefixed unknown operation groups
with `workspaces/` — `artifacts/` is now a first-class scope,
pinned by a contracts test; (2) the API boundary answers CORS
preflight only for `command_path()` paths — the five artifact
paths were added, **plus** the four M5 r9 paths
(providers-credential-set/-remove/-status, providers-session-access)
whose absence was a latent instance of the same bug (the r9
e2e never invoked them from the browser). Client failure
mapping for `artifacts/*` surfaces the core's composed refusal
verbatim, like provider dispatch detail.

## Schema-version resolution

The vault shares `workspaces.sqlite3` and owns user_version
8→9. The registry therefore landed as **v10**; the vault
catalog accepts {8, 9, 10}, applies its tables at v10 when
absent (never moving the counter backwards), and its seal
precondition accepts the pre-vault states {8, 10}. Older
migration fixtures were updated to drop `artifact_versions`
when rewinding versions and to assert the current final
version. The design doc's §"Data model" records this
correction.

## Evidence (Linux, this branch)

- Core registry suite 6/6 (lifecycle + §9 field-by-field,
  stale conflicts, isolation, sealed refusal, tamper,
  archived refusal, reopen) · full anansi-core: **497 passed,
  only the 4 pre-existing live-Ollama `semantic_workflow`
  failures** (baseline-identical).
- API 22/22 (incl. the new artifacts HTTP workflow and the
  vault boundary suite that caught the version collision) ·
  providers 120/120 · UI/contracts 39/39 · tsc clean ·
  contracts drift clean · docs validation clean · fmt clean.
- Browser (Playwright, local Chromium, synthetic workspace):
  `artifacts.spec.ts` (project → v1 → v2 → inspect both →
  archive refusal → restore → restart → reopen) plus the r10
  `app_checkpoint.spec.ts` and `providers.spec.ts` — **3 passed**.

## Not in this slice (by the owner's approval)

Idea generation, scoring, Opportunity Engine, pre-project
idea storage (all later decisions), sealed artifacts (vault
integration), artifact deletion/moves, generator/model
metadata beyond the recorded field. M5's pending checks —
Windows native/manual acceptance, live-provider verification,
G1 external audit, remote CI — remain open and unclaimed;
they did not block this model-free slice. Windows verification
of this slice is the owner's rerun, as before.
