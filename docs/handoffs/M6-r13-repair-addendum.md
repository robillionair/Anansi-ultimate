# M6 R13 reliability repair — addendum

Date: 2026-10-21 · Branch `muse/m6-slice2-idea-generation`
over the R12 tip `cb8c1cb`. Repairs the four findings of the
owner's independent Windows review of R12 (which confirmed
checksums, preview ancestry, ideas tests 9/9 and docs). No
feature work; no boundary weakened. Windows re-verification
of this repair is pending the owner.

## Finding 1 — atomic save idempotency

`ideas-save` now claims the save under the registry lock
(a per-run `saving` flag), registers WITHOUT holding the
lock, then publishes the artifact back under the lock.
Concurrent saves wait for the publication and resolve to
the same artifact; a failed registration releases the claim
so a waiter can perform the save itself. Regression
(`concurrent_saves_resolve_to_exactly_one_artifact`): two
barrier-synchronized saves return the identical artifact id,
exactly one reports `alreadySaved: false`, and the registry
holds exactly one artifact.

## Finding 2 — project-switch and polling lifecycle

The run lifecycle moved into a framework-free
`IdeaRunController` (`packages/ui/src/ideaRun.ts`), which
the Projects screen drives:

- `open()` and the New-project action await
  `controller.reset()` first: the previous project's active
  run is cancelled instead of abandoned (previously the
  tracking ref was merely cleared).
- An epoch + run id + cancel sequence invalidates in-flight
  continuations: a start resolving after a switch is itself
  cancelled and dropped; a delayed poll for the old project
  cannot overwrite the new project's state; a late "running"
  response cannot resurrect a cancelled run.
- Polling is serialized in the controller (a tick during an
  in-flight poll is skipped); cancel waits for the in-flight
  poll and invalidates its response.

Four tests with a deferred fake client cover
switch-during-generation, superseded starts, poll overlap,
and cancel-vs-poll ordering; the file joined the `test:ui`
script so the suite runs them (`pnpm test:ui`).

## Finding 3 — bounded registry and backend cleanup

`with_runs` no longer retains-and-discards silently.
Eviction is explicit: `collect_evictions` (TTL across
workspaces + the retained cap per workspace) returns
victims; the caller cancels the active ones AFTER the
registry lock is released — provider cancel plus a bounded
drain-to-terminal poll, which removes the provider stream
record and settles its ledger reservation through the M5
path. `start` makes room BEFORE the provider stream starts
(the provider layer refuses a 33rd stream, so insert-time
eviction alone could never free a slot), and every insert
path — including the provider-failure branch that bypassed
the cap — goes through `insert_run`. Runs retain their
CallerScope so a later eviction cancels with the original
authority. Regressions: in-crate unit tests (TTL/cap victim
selection; cancel only for active victims) and an
integration test that starts 32 slow generations plus a
33rd and asserts the evicted run leaves the registry AND
its provider connection is the one that closes while its
neighbour's stays open.

## Finding 4 — strict lint

`node scripts/cargo.mjs clippy -p anansi-core --lib
--locked -- -D warnings` is clean, fixed at the causes,
no allow attributes:

- `artifacts::ProjectMeta` is `pub(crate)` with `pub(crate)`
  fields, matching its `pub(crate)` accessor.
- The two `collapsible_if` sites in ideas poll are collapsed
  with let-chains.
- (Linux-only manifestation) `WindowsCredentialProvider`'s
  configuration fields are read only by the `cfg(windows)`
  CredUI flow; a manual `Debug` impl now surfaces the same
  non-secret configuration on every platform.

## Evidence (Linux, this branch) — and a correction

- Clippy (exact review command): 0 errors. fmt clean.
- Focused suites: ideas_generation 7/7, ideas_validation 4/4,
  artifacts registry 13/13, provider dispatch suite green,
  eviction unit tests 2/2, anansi-providers 120/120,
  anansi-api 23/23, UI/contracts 45/45, TypeScript +
  contracts drift + docs validation clean. Browser e2e:
  ideas + artifacts + app_checkpoint + providers — 4 passed.
- Full anansi-core suite: **181 passed, 6 failed of 187**.
  The failures: the 4 known live-Ollama semantic tests
  (no Ollama on this VM), plus 2 `vault_increment2` tests
  (`canonical_history_graph_epoch_rotation_and_explicit_rollback`,
  `deterministic_process_crash_cutover_and_rollback_matrix`)
  whose harness threads hit timing deadlines under the VM's
  current load. **Classification:** the identical failure
  (same test, same assertion, `left: 10, right: 9`) was
  reproduced on the pre-repair tip `cb8c1cb` in a clean
  worktree under the same conditions — pre-existing and
  load-sensitive, NOT a repair regression (the repair's
  vault change is a behavior-inert Debug impl). These tests
  passed in the R10–R12 sessions on this VM and in the
  owner's Windows runs.
- **Correction to R11/R12 evidence:** those reports cited
  ~497/506 anansi-core passes. The suite contains 187 tests
  (`cargo test -- --list`); the earlier totals evidently
  aggregated other suites' output in the summed logs. The
  failure SETS reported then were read from the same logs
  and stand (only the 4 Ollama tests), but the pass totals
  were wrong and are superseded by the per-binary accounting
  used here.

## Status

R12's pending items are unchanged: Windows verification of
this repair, M5's native/manual + live-provider + G1 audit
+ remote-CI checks. Antigravity's files were not touched.
