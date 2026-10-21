# M6 R15 repair — addendum (eviction-test synchronization + strict clippy)

Date: 2026-10-21 · Branch `muse/m6-r15-repair` from the R15
tip `d56ed52`. Scope: the two remaining findings from the
owner's Windows review of R15 — nothing else. **No production
code changed**: the diff is two test files plus this
addendum and `BUILD_STATUS.md`. The R15 recovery
implementation is preserved byte-for-byte. All results below
are **Linux (this VM)**; Windows re-verification of the
rewritten test is the owner's / independent reviewer's step —
nothing here claims it.

## What the Windows review of R15 established

Passed on Windows at `d56ed52`: `backup_artifacts` 5/5
(workspace/application round trips, legacy packages,
corruption refusal, repeat-restore handling), `evaluations`
3/3, `ideas_validation` 4/4, `ideas_generation` 6/7 including
concurrent-save protection, UI/contracts 47/47, TypeScript,
documentation validation, core-library Clippy with warnings
denied.

Remaining acceptance failure: one test, plus a strict-clippy
failure on the backup test target. Both are repaired here.

## Finding 1 — `cap_eviction_cancels_the_oldest_active_stream`

Windows failure at `ideas_generation.rs:644`: "the evicted
stream was never cancelled: {}", reproduced in the suite and
in isolation (44.25 s). The owner's diagnosis is adopted:
this was a **test-design defect, not a proven backend
defect**. The old test started 32 runs against streams that
each finish after ~6 s of dripping, started sequentially; on
a machine with Windows storage/permission overhead the
oldest stream could finish naturally before the evicting
33rd start, so there was no live connection left to cancel.
Any fix that merely lengthens the drip would keep the same
race on a slower machine.

### The repair: explicit synchronization, no timing assumptions

The synthetic provider gains a **gated mode**
(`crates/core/tests/ideas_generation.rs`; all pre-existing
modes unchanged):

- A gated stream writes its response headers and first chunk,
  then **holds behind a server-controlled gate** until the
  test opens it. It cannot finish on its own, however slow
  the machine is.
- While gated, the server watches its socket with a short
  read timeout; a client close (eviction / cancellation) is
  observed **directly as EOF** and recorded per connection.
- Connections are attributed to runs by a **topic marker**
  (`evictprobeNN` in the brief) parsed from the request body,
  replacing the old accept-order assumption (connection 0/1).
  The server tracks per-connection `begun` / `finished` /
  `broken` state.

The rewritten test then proves, in order:

1. **Oldest and neighbour active before eviction** — each of
   the 32 starts is synchronized on the server having begun
   that run's stream; runs 0 and 1 report `running` from the
   app, and `providers-usage` shows 32 established requests
   with exactly one uniform token reservation held per
   stream.
2. **Eviction closes the oldest connection** — after the 33rd
   start, polling run 0 returns `NotFound` (it left the
   registry), the server's broken set is exactly
   `{connection of run 0}`, and run 1's connection is open
   with run 1 still `running`, as is the new run.
3. **The evicted reservation is released** — usage settles
   to 33 total requests with exactly 32 token holds (31
   survivors + the new run). A leaked eviction hold would
   read 33 holds; a collateral cancellation would read
   fewer.
4. **Cleanup** — the gate opens, every remaining test-owned
   stream is polled to `complete`, every connection except
   the evicted one is observed `finished`, and reserved
   tokens/requests end at exactly 0.

### Non-vacuity check (mutation, reverted, not committed)

With the eviction path's cancel call disabled in
`crates/core/src/ideas.rs`, the rewritten test fails
immediately: the 33rd start is refused by the provider layer
because the evicted stream still occupies its slot. The
mutation was reverted before commit; `ideas.rs` is identical
to R15 in this checkpoint.

## Finding 2 — strict clippy on the backup test target

`clippy -p anansi-core --test backup_artifacts -- -D warnings`
failed on two lints the lib-only clippy scope did not cover;
reproduced verbatim on Linux before fixing:

- `backup_artifacts.rs:685` — `legacy_package` took a
  `scratch: &Core` parameter its body never used (it works
  from `scratch_root`). Parameter and both call-site
  arguments removed.
- `backup_artifacts.rs:97` — `for rubric_version in [1]`, a
  one-element loop left from before the rubric-v2 evaluation
  existed. The v1 evaluation is now saved directly.

No warning suppressions anywhere; fixture behaviour is
unchanged (`backup_artifacts` still 5/5).

## Linux evidence (this checkpoint)

- Owner's exact commands: the isolated eviction test passes
  (`--exact`); the strict `backup_artifacts` clippy command
  is clean. Clippy `-D warnings` also clean for the core lib,
  the `ideas_generation` target, and `--all-targets`;
  `cargo fmt --check` clean.
- Eviction test stability: 10/10 consecutive runs and 3/3
  under 4-way CPU contention, each ≤ ~0.9 s (the gated design
  removes the sequential slow-stream accumulation entirely —
  there is no longer a ~6 s-per-stream clock to race).
- Focused suites: `ideas_generation` 7/7,
  `ideas_validation` 4/4, `backup_artifacts` 5/5,
  `backup_restore` 7/7, `artifacts_registry` 6/6,
  `evaluations` 3/3, core lib 34/34.
- Full `anansi-core` suite: **193 passed / 6 failed of
  199** — identical to the R15 baseline, and the 6 are
  exactly the known set: the 4 live-Ollama
  `semantic_workflow` tests (no Ollama on this VM) and the 2
  `vault_increment2` assertions root-caused at R15 to the
  pre-0010 schema ladder, left unchanged for the vault
  owner.

## Still open (unchanged by this checkpoint)

- **Windows re-verification of the rewritten eviction
  test** — the Linux evidence above does not close it.
- **The two `vault_increment2` assertions** — a vault-domain
  decision, tests not weakened or edited here.
- **Application acceptance** and **preview promotion** —
  still gated on the Windows verification above.
- M5 pending checks (Windows native/manual acceptance,
  live-provider verification, G1 external audit, remote CI)
  remain open and unclaimed. M6 Slice 4 is not started.
