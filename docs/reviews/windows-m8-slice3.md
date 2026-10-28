# M8 Slice 3 — independent Windows review

Incoming `776259a49819a0166e8199dc85e29b34cf452680`, with reviewed baseline
`4d3f8bfdaf904f452fe65c404a80bc2ef9aa83e2` confirmed as an ancestor.
ZIP SHA-256 `39373e975a3719c1d7e7aee89d17cf771f324b215acb66afdced96eea642d745`.
Bundle SHA-256 `53a0441b8eb6c3fd30ac586327e1954e714ecb7f0901af325fe849df134f8239`
matches the enclosed manifest. Bundle integrity, 4 refs and ancestry verified.
Isolated worktree `E:/Anansi/worktrees/review-m8-slice3`, branch
`codex/review-m8-slice3`; existing previews and personal roots are untouched.

## Acceptance plan

Run delivered Collider/transfer suites, restore-unit/Forge-restore checks, UI and
contracts, TypeScript, builds, strict core all-target Clippy, formatting and docs.
Run canonical memory-context and focused transfer browser checks under the shared
application-test lease, with synthetic data and loopback providers only.

Inspection identified two additional checks to establish before deciding repairs:

1. Pre-send consent: review immutable revision B, create C through another client,
   click Collide and inspect the actual wire request. It must contain reviewed B,
   never unreviewed C. Selection changes must clear stale reviews, pending reads
   must prevent submission, and unmount must not start a delayed run.
2. Restore provenance: original context version, title and privacy must agree with
   the referenced source revision, not just its content hash and owner. A fabricated
   label must refuse before restoration publishes anything. A focused unit probe
   targets the pre-validation function before testing any local correction.

No assertion or timeout may be weakened. Record before/after results and the exact
verified implementation. No new feature slice, preview promotion, native acceptance,
remote CI, paid inference, real credentials or generated execution is authorized.

Delivered UI/contracts 81/81, TypeScript, web build and initial docs (135 files)
passed. Existing Vite chunk-size advisory remains visible. Other results pending.

## Additional findings and bounded local corrections

The new browser probe `collider-memory-acceptance.spec.ts` failed both cases on
the delivered application: review-pinning reached the synthetic provider, but its
captured request contained revision 2's `UNREVIEWED REVISION C PRIVATE CANARY`,
not the revision-1 text still displayed before submission; pending-review held a
real read response behind a server-controlled gate, and Collide stayed enabled
through the unchanged 30-second assertion window. These are behavior failures,
not performance timeouts. The initial probes overlapped backend execution; final
application evidence will be run sequentially after backend work settles.

The UI correction submits the already-reviewed immutable references with no
pre-start re-read. It blocks loading/error/mismatched reviews, invalidates a review
immediately on selection change and unmount, drops superseded reads, and guards
duplicate starts and post-unmount completions. The existing run controller retains
start/cancel ownership; no alternate provider path is introduced.

The added restore-unit probe also failed before correction, accepting all three
fabricated labels (version, title, privacy) even though the referenced content was
valid. Pre-validation now compares those labels to source revision metadata before
any remapping/publication. The fixture now includes the real version/title fields;
all original refusal/control assertions remain. Both binding-unit tests pass after
the correction (2/2, including three variants in the new test). This establishes
provenance consistency, not cryptographic authenticity against arbitrary database
rewrites. Implementation correction:
`f54e46695e7aacbeca5676c11060bed4f04a6c1a`; strengthened project-switch test:
`ba37f9c03236b562109087117fc75953c4f141ee` (same production code).

## Windows results and exact scope

- Delivered implementation `776259a4`: Collider 19/19 (482.86 s), transfer 18/18
  (453.18 s), including memory ownership/Sealed/limits, actual selected wire content,
  generation-time B after later edits, malformed/cancel refusal, receipt integrity,
  context-bearing transfer restore and subsequent Forge admission. These binaries
  were built before the local corrections; do not relabel them a final-tree rerun.
- Corrected implementation: restore-binding units 2/2 (0.03 s), covering valid
  controls and Sealed/owner/hash/label refusals; Forge restore 4/4 (193.20 s), covering
  approval versions, revision parents, valid references/exports and absent-spec
  refusal. UI/contracts 81/81, TypeScript, web/API builds and strict core all-target
  Clippy passed. Contracts drift passed (existing ignored uint/float-format notices).
- Final browser checks: 3/3 (4.1 min), canonical Windows config, one worker, shared
  application-test lease, synthetic data and local servers, after backend suites
  completed. Review-pinning passed (33.8 s), gated pending-review/project-switch
  isolation plus a context-free collision passed (31.4 s), and the delivered full
  memory-context workflow passed through save, later memory edit, restart and fresh
  restore (2.9 min). Retained assertions and 240-second case limits are unchanged.

The first corrected-application browser run was 1/3: review-pinning passed, while
the new gate harness intercepted project-opening reads too and attempted route
cleanup before fulfilment; the delivered workflow also selected a checkbox while
the destination project was still loading (its existing Artifacts heading was
already visible for the previous project). The failure snapshots showed the later
panel's unchecked inputs, not a received/rejected review. These were root-caused
test synchronization errors. Commit `6a65cd031` restricts the gate to the selected
memory id, waits for its fulfilment before removing that handler, and waits for
the exact destination project heading. The canonical spec gained three identity
assertions, with none removed or weakened. Production code remained unchanged
between that diagnostic run and the complete 3/3 confirmation.

Final phase-boundary formatting, Git diff validation and documentation validation
(136 Markdown files) passed; the application-test lease was released normally.
No full Windows
core/API/providers suite, live Ollama or native desktop acceptance is claimed.

## Verdict and next baseline

Focused Windows acceptance **passed with local corrections** on
`codex/review-m8-slice3`, including production fix `f54e46695`, test strengthening
`ba37f9c03` and harness synchronization `6a65cd031`. Do not continue from incoming
`776259a4` alone. AN-013 remains in progress. No native acceptance or preview
promotion, URL/document/project ingestion, 3D work, generated-code execution,
paid inference, real credentials, live-provider certification, G1 external audit,
remote CI or protected publication is implied by this review. The source handoff
contains the exact review ref and diagnostic/confirming logs; existing previews,
launchers and personal roots are unchanged. All new work/cache/temp is on E:.

## Reproduction

Use an isolated checkout, E: temp/cache and the shared lease. The initial combined
Collider/transfer run preceded the local code correction; the remaining checks
below verify the corrected implementation. This is not a relabeled full final-tree
core regression run.

```text
node scripts/cargo.mjs test -p anansi-core --locked --test collider --test transfer
node scripts/cargo.mjs test -p anansi-core --lib --locked experiment_binding_tests
node scripts/cargo.mjs test -p anansi-core --locked --test forge_restore
pnpm test:ui
node node_modules/typescript/bin/tsc --noEmit
node scripts/contracts.mjs --check
pnpm build:web
node scripts/cargo.mjs build -p anansi-api --locked
node scripts/cargo.mjs clippy -p anansi-core --all-targets --locked -- -D warnings
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test tests/e2e/collider-memory-acceptance.spec.ts tests/e2e/collider-memory-context.spec.ts --config playwright.config.ts
node scripts/cargo.mjs fmt --all -- --check
node scripts/validate-docs.mjs
```
