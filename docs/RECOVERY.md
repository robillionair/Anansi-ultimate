## Resume point — 2026-10-31 visualization fallback

Focused Windows-verified runtime: `cdae30bcac2c231c2e12f6fde5b7210544559fb5`; review branch `codex/review-m8-visualization-repair` adds docs only. [Evidence](reviews/windows-m8-visualization-verified.md). Preserve closed document/URL and visualization repair findings. Section26 Three.js and remaining GPU/adaptive-quality work stay deferred. Coordinate release integration independently; retain owner previews and data.

## Resume point — 2026-10-31

Last focused Windows-verified URL runtime: `d0a86d5e34884a2d604b5cf21ab586876e170786`. Review branch `codex/review-m8-url-input-repair2` adds documentation only. [Evidence](reviews/windows-m8-url-input-verified-2026-10-31.md). Preserve this runtime and the closed document/URL probes. Release tooling source `df7241829` remains separate: integrate only its three assigned directories onto a new branch from this reviewed application tip. Do not replace the runtime with the tooling's old application baseline. Native/release/production gates stay open.

# ANANSI Ultimate — Recovery Guide

## Current verified lineage checkpoint — 2026-10-30

Use `E:/Anansi/worktrees/review-m8-overnight`, branch `codex/review-m8-lineage`, verified incoming runtime `06a3f573a`. [Focused Windows evidence](reviews/windows-m8-lineage.md): five old probes, extended resumed-work chain, 74 library cases, nine vault cases, real v10 import/live migration, UI/contracts 90/90 and browser 4/4 pass. R1–R4 tested defects are closed in this scope. Do not restart those repairs without new evidence. Preserve genuine v10 fixture/inspection and prior failure records; schema11 data roots stay separate from older builds. No preview/native/release promotion or automatic cloud/CI/push/audit monitoring. Antigravity tooling integration remains separate and must not overwrite this newer runtime with its older baseline.

## Current M8 URL input — 2026-10-30

On branch `muse/m8-url-input`, from reviewer tip
`144acc715`. Explicit URL entry plus a separate Fetch
action for either collider beam: a backend-owned
bounded fetch service (public HTTP/HTTPS only, no
credentials in URLs, loopback/private/link-local/
reserved addresses refused, every resolved address
validated, connection to the validated destination,
redirect/time/byte/text bounds, normal TLS validation,
no cookies/authorization headers/JavaScript/external
resources, plain text + conservatively extracted HTML
only). Fetching saves nothing and starts no inference;
only the explicit Collide press sends the reviewed
text, and generation-time text + provenance survive
save, restart, approved Forge transfer, and
backup/restore. A saved URL record is inert text: its
URLs are shape-checked and its text hash re-verified,
but it is never re-resolved or re-fetched at
start/restore/transfer. Linux verification only;
independent Windows rerun is pending. Owner
previews/data and Antigravity tooling remain untouched.
No cloud calls, pushes, CI or promotion.

## Current M8 URL input repair — 2026-10-30

On branch `muse/m8-url-input-repair`, from reviewer
tip `db8d4cb93`. Three focused security repairs over
the URL slice, reviewer probes preserved unchanged:
globally-routable address classification (IPv6
allowlist over `2000::/3` minus special-purpose
exclusions; mapped/6to4 by embedded IPv4; mixed DNS
answers refused; connection still pinned to the
validated address; redirect targets re-validated),
one total deadline across DNS/connect/TLS/writes/
headers/bodies/trailers/redirects (20s unchanged;
every socket wait uses the remaining budget), and the
loopback test allowance compile-gated to test/debug
builds (release builds are unconditionally strict).
Linux verification only; independent Windows rerun
is pending. Owner previews/data and Antigravity
tooling remain untouched. No cloud calls, pushes, CI
or promotion.

## Current M8 visualization repair — 2026-10-31

On branch `muse/m8-visualization-repair`, from
reviewer tip `5a050f18`. VIS-01: helper renamed to
`colliderVisualizationScene.ts` (case-only
collision with the component broke Windows builds);
all imports updated, casing checks intact,
folded-name scan clean. VIS-02: canvas labels now
white over a dark backing, verified in inspected
screenshots (paused, reduced-motion, high DPI).
Canvas/list stays the accessible fallback; Three.js
and remaining §26 requirements stay unfinished.
Linux verification only; independent Windows rerun
is pending. Owner previews/data and Antigravity
tooling remain untouched. No cloud calls, pushes, CI
or promotion.

## Current M8 visualization — 2026-10-31

On branch `muse/m8-visualization`, from reviewer
tip `795aab00`. Saved-experiment visualization only
(live panel untouched): pure derivation from the
parsed experiment document, section-flow-only edges,
deterministic layout, canvas + text/list selection
with exact source text and pinned provenance,
reduced-motion static view, identity-keyed stale
state, zero API calls, render as data. No backend,
schema, or contract change; no new persistence. The
renderer is Canvas 2D (Three.js swap open, not
claimed). Linux verification only; independent
Windows rerun is pending. Owner previews/data and
Antigravity tooling remain untouched. No cloud calls,
pushes, CI or promotion.

## Current M8 URL input repair 2 — 2026-10-30

On branch `muse/m8-url-input-repair2`, from reviewer
tip `239dc58e`. The fetch deadline is now enforced
beneath rustls: a deadline-aware transport re-checks
the deadline and shrinks the socket timeout before
every underlying socket read/write, so assembling one
encrypted record, alerts, post-handshake traffic, and
fragmented writes cannot outlive the deadline (the
first repair's deadline sat above `StreamOwned::read`
and one dripped record bypassed it). Certificate
validation, address pinning, size bounds, and the 20s
production deadline are unchanged; no timeout was
increased. Linux verification only; independent
Windows rerun is pending. Owner previews/data and
Antigravity tooling remain untouched. No cloud calls,
pushes, CI or promotion.

## Current M8 document input repair 2 — 2026-10-30

On branch `muse/m8-document-input-repair2`, from reviewer
tip `d3f004bc`. DI-03 repair, frontend only: every new
document selection invalidates the beam's load session
before any early return, so even a pre-read refusal
(oversized file) retires older pending reads; the
rejection clears obsolete loading state and shows the
current error, and the last applied beam and attribution
are preserved. DI-01/DI-02 fixes, restore, transfer and
provenance behavior are unchanged. Linux verification
only; independent Windows rerun is pending. Owner
previews/data and Antigravity tooling remain untouched.
No cloud calls, pushes, CI or promotion.

## Current M8 document input repair — 2026-10-30

On branch `muse/m8-document-input-repair`, from reviewer
tip `97ab19b4`. Two frontend-only repairs over the
document slice: oversized files are refused on `File.size`
before any read (decoder post-read check retained), and
document load failures are generation-gated exactly like
successes, so an obsolete failure cannot display its
error, clear the loading flag, or enable dispatch while a
newer read is pending. Restore, transfer, provenance and
the saved-experiment contract are unchanged. Linux
verification only; independent Windows rerun is pending.
Owner previews/data and Antigravity tooling remain
untouched. No cloud calls, pushes, CI or promotion.

## Current M8 document input — 2026-10-30

On branch `muse/m8-document-input`, from reviewer tip
`c50d6b03`. Saved collider experiments may carry optional
per-beam document provenance (`documentA`/`documentB`:
label, kind, BOM flag, file bytes, file/text SHA-256,
text chars). Restore and transfer validation recompute
every field against the saved beam text: forged, stale
(edited-beam) or inconsistent records refuse before
destination publication, exactly like the R1 provenance
guards. Experiments without document fields restore
unchanged, and later changes to a source file never rewrite
a saved experiment — the record is generation-time only.
No schema change accompanies this slice. Linux verification
only; independent Windows rerun is pending, including
native file-picker behavior. Owner previews/data and
Antigravity tooling remain untouched. No cloud calls,
pushes, CI or promotion.

## Current M8 lineage repair — 2026-10-30

Use branch `muse/m8-lineage-repair`, from reviewer tip `e03c9bde`.
The resumed-work lineage regression is repaired on Linux (see the
repair design and addendum in `docs/handoffs/` and the BUILD_STATUS
entry): restore writes the full alias set under a widened
`restore_origins` key, schema 10 → 11, with a v10 legacy
verification path that migrates only the staging copy. Validation
is unchanged (exact pair lookup). Independent Windows rerun by
ChatGPT is still pending and is the next review step, including a
real v10 backup restore; do not treat Linux evidence as closing
it. Owner previews/data and reviewer worktrees remain untouched.
No cloud calls, pushes, CI or promotion.

## Current provenance Windows review — 2026-10-30

Use `E:/Anansi/worktrees/review-m8-overnight`, branch `codex/review-m8-r1-provenance`, incoming `b6a3251c`. [Verified outcomes and R4](reviews/windows-m8-r1-provenance.md): original four probes pass, but new work created after a restore loses intermediate identity authority on a later recovery. Do not promote; preserve exact mapping authority, immutable snapshots and prior R1–R3 fixes. Muse owns the bounded lineage implementation; reviewer additions only test/document the failure. Antigravity tooling and owner previews/data are separate. No automatic cloud, CI, pushes or audit-monitor activation.

## Current M8 R1 provenance repair — 2026-10-30

Use branch `muse/m8-r1-provenance-repair`, from reviewer tip
`7388b7fa`. The two remaining R1 provenance findings are repaired
on Linux (see the repair design and addendum in `docs/handoffs/`
and the BUILD_STATUS entry): no early transform success for absent
optional context, and id substitutions require an exact
backend-recorded restore-origins pair, with ultimate origins
recorded so multi-hop restores validate. Independent Windows rerun
by ChatGPT is still pending and is the next review step; do not
treat Linux evidence as closing it. Owner previews/data and
reviewer worktrees remain untouched. No cloud calls, pushes, CI or
promotion.

## Current M8 repair review — 2026-10-30

Use `E:/Anansi/worktrees/review-m8-overnight`, branch `codex/review-m8-overnight-repair`, incoming `d0a6c1e1`. [Exact findings](reviews/windows-m8-overnight-repair.md): original three reviewer probes now pass on Windows, but two added R1 attacks still accept fabricated structured text/identities. Preserve R2/R3 fixes and all assertions; Muse owns the remaining bounded R1 implementation. Do not promote or claim native/security/release completion. Antigravity release tooling is separate; do not modify its worktree. No automatic cloud, CI, push or monitor activation.

## Current M8 overnight Windows repair — 2026-10-30

Use branch `muse/m8-overnight-windows-repair`, from reviewer tip
`240af4ef`. The three Windows findings are repaired on Linux (see the
repair design and addendum in `docs/handoffs/` and the BUILD_STATUS
entry): forged restored-hash provenance refuses, selected concepts pin
final restored hashes after all fix-ups, and saved overview links
resolve to their own saved result. Independent Windows rerun by ChatGPT
is still pending and is the next review step; do not treat Linux
evidence as closing it. Owner previews/data and reviewer worktrees
remain untouched. No cloud calls, pushes, CI or promotion.

## Current M8 overnight review — 2026-10-30

Use `E:/Anansi/worktrees/review-m8-overnight`, branch `codex/review-m8-overnight`, incoming `edba7ed3`. Read [Windows findings](reviews/windows-m8-overnight.md). Acceptance is blocked by R1 forged restored-hash provenance, R2 post-fixup stale context hash, and R3 saved/live anchor collision. Reviewer changes only add expected-failing probes/docs. Muse owns repairs; preserve assertions and do not promote. Current-source boundary and focused UI/build checks pass, but this does not close recovery/native/production gates. The older Slice 4 review worktree and its uncommitted probes remain preserved separately. No automatic cloud calls, audit monitoring, pushes or CI.


## Current verified Windows integration — 2026-10-30

Use `E:/Anansi/worktrees/review-m8-benchmark`, branch
`codex/review-ask-windows-findings`, implementation `4ef90406d` over incoming
`c15fe5d6`. [Verification and scope](reviews/windows-ask-findings-verified-2026-10-30.md).
All four findings pass focused Windows checks. Native verify-only root/catalog
and batch checks yield zero permission-helper launches. Same 16-note Ask: 2,264 ms;
full realistic corpus 68/68 answerable, 80/80 bindings, p95 4,383.3 ms. Use this
integrated checkpoint rather than the incoming branch alone. Preserve incoming
ZIPs, frozen cloud prompts and historical measurements. Model quality/token replay,
desktop preview promotion, production gates and known full-core failures remain
open. No automatic cloud calls, pushes, CI or audit-monitor activation.

## Current scale-hardening Windows review — 2026-10-29

Use `E:/Anansi/worktrees/review-m8-benchmark`, branch
`codex/review-ask-scale-hardening`, incoming `825f7561`.
Read [exact findings and commands](reviews/windows-ask-scale-hardening-2026-10-29.md).
Speed improves to 6,910 ms on 16 notes, but permission-widen refusal, Windows
accounting and strict Clippy are open. Preserve the explicit ignored reproduction
and its FAILED output; it must become a true refusal pass, not a permitted read.
No runtime changes or full-scale Windows pass are claimed by this review. Frozen
cloud measurement remains held. Preserve owner data, previews and audit-monitor
state; no automatic model calls, pushes, CI or new feature work.

## Current Windows retrieval review — 2026-10-29

Use `E:/Anansi/worktrees/review-m8-benchmark`, branch
`codex/review-ask-retrieval-verify`, incoming `f90f9e850`.
Read [findings and evidence](reviews/windows-ask-retrieval-verify-2026-10-29.md).
One Windows Ask on 16 notes took 27,637 ms while retrieving the right evidence.
Do not infer Windows scale readiness from Linux timing or run the expensive
1,007-record public-path suite unbounded before resolving process-launch overhead.
Preserve ACL/reparse/owner/hash checks and the original scale questions. Model
replay and token measurements remain held; no cloud calls are authorized by
resuming this review. Existing previews, data and audit-monitor state stay intact.

## Ask recall repair completed — 2026-10-29

Use `E:/Anansi/worktrees/review-m8-benchmark`, branch `codex/ask-retrieval-recall`.
Verified implementation `24d3df5c7`; read [repair evidence](reviews/ask-retrieval-recall-repair-2026-10-29.md).
All 40 missing evidence sets recovered on the unchanged 100-case fixture; 90/90
answerable cases contain all facts and 10/10 genuinely empty cases remain empty.
No new model answers or API usage measurement. Preserve both immutable fixtures
and original cloud results. Do not repeat cloud calls, claim 100% model accuracy,
promote a preview or resume the external-audit monitor automatically.
The owner's fix request supersedes historical no-production-change language below
only for this bounded retrieval repair, not for unrelated milestone work.

## Large free-model benchmark completed — 2026-10-29

The authorized 100-case test is complete. Measured protocol commit `21b1afb0b`.
Read [measured result](reviews/large-free-benchmark-2026-10-29.md): ANANSI 60/100,
basic 70/100, full 100/100; 79.69% lower input versus full does not preserve quality.
All 30 scored calls and one smoke reported zero cost. Raw data and exact sums are
preserved. No further cloud call or repeat is automatically authorized by this
completed experiment. Keep the savings-led LinkedIn draft on hold.

The owner's newer request authorizes a new bounded test on this computer.
Use branch `codex/benchmark-large-free` in `E:/Anansi/worktrees/review-m8-benchmark`.
Read [frozen acceptance plan](handoffs/M8-large-free-benchmark-design.md).
Artifacts are under `E:/Anansi/benchmarks/large-free-2026-10-29`.
100 natural questions use actual lexical Ask preparation; cloud replay is limited
to the verified free Nemotron model, 30 scored batch requests plus the successful
smoke. Stop on error/rate limit/cost/model mismatch; no retries or paid fallback.
No credentials are stored in source or artifacts. Earlier no-new-round language
below remains historical and is superseded only for this explicitly scoped run.
Preserve partial results and immutable prompts. Do not run the cloud runner again
automatically. Production changes, previews, CI, pushes and publication remain out
of scope. The external-audit monitor stays off.

## Current benchmark review resume authority — 2026-10-29

Use `E:/Anansi/worktrees/review-m8-benchmark`, branch
`codex/review-m8-benchmark`, incoming `8639c44c30eb98848961dfd31fe1057e769d53e4`.
Read [independent review](reviews/windows-m8-benchmark.md) and latest BUILD_STATUS.
Reporting correction `655c4f615` changes no application behavior or raw measurements.
Eight provider reasoning-counter anomalies remain open; do not claim independently
verified total computation savings. No further cloud round is authorized. Preserve
previews, personal data, reviewed ancestry and original delivery. Audit monitor stays off.

## Current M8 Slice 3 review resume authority — 2026-10-28

Use `E:/Anansi/worktrees/review-m8-slice3`, branch `codex/review-m8-slice3`.
Incoming `776259a4` plus local consent/provenance correction `f54e46695` and its
test strengthening/synchronization commits is the reviewed implementation.
Read [exact Windows evidence](reviews/windows-m8-slice3.md) and current BUILD_STATUS;
focused acceptance passed, while native promotion and complete M8/production
certification remain open. Preserve all existing previews/data and other worktrees.
Use E: for builds/temp and the shared application-test lease. No new slice, push,
CI or generated execution is authorized by this review. The audit monitor remains
off at the owner's instruction. Earlier resume authorities below are historical.

## Current M8 review resume authority — 2026-10-27

Use `E:/Anansi/worktrees/review-m8-acceptance-repair`, branch
`codex/review-m8-acceptance-repair`. Incoming repair `1ccda62d` plus local
receipt-integrity correction `5b3cf958f` is the reviewed implementation; read
[the exact Windows evidence](reviews/windows-m8-acceptance-repair.md) and the
latest BUILD_STATUS before continuing. Focused backend/browser acceptance is
passed; native promotion and whole-milestone/production completion are not implied.
Preserve existing previews/data and other worktrees. Use E: for builds and temporary
fixtures and the shared application-test lease for application tests. No new
feature slice, push, CI or generated execution is authorized by this review.
The owner's instruction to turn off the audit monitor remains in force; historical
monitoring language below does not authorize restarting it. Earlier M2/M3 resume
restrictions are historical and do not replace this current checkpoint.

## Current resume authority — 2026-10-20

Resume M2–M4 reconciliation in `C:/Users/robin/Documents/ChatGPT/Anansi ultimate`, branch `codex/m2-m3-integration`. Preserve the separate M2 checkout at `bd2e2a9`. [Ownership and acceptance](handoffs/M2-M4-integration-owner.md) supersede older isolation/stop instructions below within the Owner's new task scope. Read actual HEAD/status before resuming. Do not downgrade schema, reset user data or infer production acceptance from synthetic providers. [Readiness](evidence/M5-READINESS.md) tracks unfinished work. Audit monitoring is paused; CI/protected publication remain deferred. No M5 code until readiness requirements pass.

## Owner amendment — M2.7-Core isolation (2026-10-19)

Increment 2 recovery: use only the isolated `codex/m2-7-vault-core` worktree, [active plan](../plans/M2-vault-security.md) and [evidence](evidence/M2-7-increment2.md). Schema 9 is opt-in through the new core API, never automatic in the live application. Preserve the entire closed root, matching M2.5 source backup and all generation/staging files before diagnosis. Never downgrade user_version, unlink lock files, steal a lease by PID/age, reset or purge originals. OS process death releases admission locks; durable generation and epoch come from SQLite. Before commit the previous state remains authoritative; after commit the newly committed state is authoritative even when a post-commit callback fails. Inspect actual active generation and integrity before retrying. An incomplete new stage is not authority and is never automatically executed. Explicit rollback requires authentication of the selected retained generation and is a new epoch-advancing transaction. Independent old backup keys remain usable for those old copies.

Fresh ciphertext restore authenticates all members before publication, requires a new recovery acknowledgement and creates new workspace/record/revision/graph IDs and keys. It does not overwrite the source or pre-existing destination workspaces. Retained legacy body files, old SQL/WAL remnants and M2.5 backups can contain plaintext; this increment makes no erasure or whole-root at-rest certification claim. Application-native/browser vault recovery and job/scheduler integration remain gated. Source bundles exclude private roots and secrets. The external `M2-7-core-increment2-20261019/source-verification.json` records the actual checkpoint/tree/bundle/restore/CI results after creation; do not infer success from the path. Do not merge or publish the active M3 checkout from here.

[Owner risk acceptance](approvals/M2-6-owner-risk-acceptance.md) explicitly supersedes historical G1 pre-implementation stop language below. G1-External-Audit remains open for post-M3/pre-production/commercialization certification; no audit coverage is fabricated. M2.5 source/CI recovery baseline was reverified. M2.7-Core is authorized in a separate worktree; M3 Search/UI wiring and shared-checkout files remain untouched. G2–G4 are unchanged. No vault completion, production certification or implemented UI disclosure is claimed. AN-018/019/031 remain in progress.

## Current resume gate — M3.1 with M2/G1 retained

Local M3.1 functional acceptance is complete at implementation `734a94b`. Full browser run initially passed 12/13; the sole stale planned-Search assertion was corrected and all three onboarding tests passed on rerun. Do not report a second local full run. Finish corrected exact-SHA CI/protected publication and inspect `C:/Users/robin/Documents/ChatGPT/Anansi-source-recovery/M3-1-title-search-20261019/source-verification.json` for actual checkpoint/tree/bundle/restore/results. The implementation source bundle restored matching HEAD/tree and passed documentation/hygiene validation. Prior audit checkpoint `8798fad` is now published on protected master. Do not publish the superseded search checkpoint whose Linux run contains the stale assertion. Next functional scope is the M3.2 detailed design; no body indexing, inference or crypto implementation starts automatically.

The latest user explicitly authorized independent next-phase work without waiting for audit feedback. Resume only unfinished [M3.1 title-search acceptance](../plans/03-search-and-ollama.md), following [its evidence](evidence/M3-1.md) and ADR-0024. This supersedes earlier stop-before-M3 text below for M3.1 only. Schema stays 8; no persistent index or model is introduced. Source rollback reverts code without rewriting user data. An hourly follow-up is now authorized and active; older no-monitor wording below describes the prior checkpoint. Do not revert an active search branch to satisfy the earlier audit-only schedule. Preserve unsaved/user changes and consult current exact-SHA CI before protected publication.

Upstream [maintainer response](https://github.com/RustCrypto/password-hashes/issues/940#issuecomment-6256202461) and [authorized acknowledgement](https://github.com/RustCrypto/password-hashes/issues/940#issuecomment-6256248503) have been handled. Do not send duplicate messages. His qualified no-known-audit statement leaves G1 open; no new independent audit is authorized or funded. Pure Rust and p=4 remain required. No crypto/dependency implementation or M2 completion is implied by M3.1.

2026-10-19 follow-up: retain pure Rust and p=4 as explicitly selected by the user. Read [audit evidence](evidence/M2-6-audit-review.md), [the comparison decision](../plans/M2-vault-library-options.md), ADR-0023 and [the prepared inquiry](reviews/M2-6-audit-inquiry.md). G1 remains open; source checksums and the historical AEAD/opaque-ke reports do not establish current Argon2 coverage. The user explicitly authorized publishing the prepared inquiry; it was sent once as [upstream issue 940](https://github.com/RustCrypto/password-hashes/issues/940). Read the response before considering duplicate contact; no paid audit is authorized; the later explicitly requested hourly follow-up is active as noted above. Do not start M2.7 or a native FFI replacement. The external `C:/Users/robin/Documents/ChatGPT/Anansi-source-recovery/M2-6-audit-review/source-verification.json` records checkpoint/restore/CI state after execution; inspect actual results rather than inferring publication from this path.

Published M2.5 checkpoint `6c193ef8c9891e966c547451b4e01b7db74215b4` passed all three implementation/checkpoint CI jobs and independent source recovery. Read [the M2.6 reconciliation](../plans/M2-vault-security.md) and [evidence](evidence/M2-6.md). The user's conditional approval is preserved; [the corrections](approvals/M2-6-corrections.md) are adopted and G1 audit evidence remains open, not an absent approval. Do not add crypto dependencies, run sealed migrations, purge staging, reset a vault or claim integrated M2/M3 certification while that condition remains unresolved. M3.1 alone has the explicit scheduling exception above. No vault exists in schema 8. Source bundles cannot recover private application data or future recovery keys.

## M2.5 backup recovery

Schema 8 requires a compatible binary; never downgrade a live root. Preserve the original closed root and use a fresh compatible copy when recovering. Source bundles exclude private catalogs and are distinct from application backups.

In Backups, explicitly consent to plaintext snapshots in private managed storage. Create a workspace snapshot (only that workspace) or the separately labeled application data snapshot (all workspaces and referenced artifacts). Save a Completed artifact from Jobs and export to another disk if protection from disk loss is needed. Scheduling defaults off; enabling requires destination/retention/privacy consent. It runs only while the native app or API is open, reports missed intervals and keeps all versions without pruning.

To recover a saved archive, create/open a local landing workspace in a compatible installation, select the TAR file in Backups and choose Import saved backup. Only verified owned artifacts can be previewed. Review schema/scope/counts/privacy/size/hash and choose an unused restored workspace name. Confirm fresh IDs; restore creates new workspace(s), preserving originals and retaining ID provenance. Application recovery also copies completed job artifacts and disables imported schedules. Reopen the restored workspace and verify its data before relying on it. Unknown schema, missing/hash-invalid members, links/traversal or Sealed content fail closed; no reset or plaintext fallback is performed.

Failed attempts retain private `backup-staging` data, incomplete uploads and unreferenced output for diagnosis. Do not manually merge it into the catalog, delete originals, restore a live SQLite main file alone or claim secure deletion. Ordinary packages are plaintext and must not contain credential secrets; audited vault/confidential backup work remains separate. Final execution evidence is in [M2.5](evidence/M2-5.md).

1. Read `AGENTS.md`, `docs/MASTER_SPEC.md`, `docs/BUILD_STATUS.md`, the active plan, and `docs/DECISIONS.md`.
2. Check `git status` and inspect the last verified commit recorded in build status.
3. Do not discard uncommitted work. Identify the active phase and its acceptance criteria.
4. Re-run the phase's verification commands before changing code.
5. Continue only the unfinished acceptance criteria, then record evidence and update status.
6. If requirements conflict, stop implementation and record the conflict in `docs/DECISIONS.md`.

Recovery checkpoints should contain source, tests, and the corresponding status/documentation updates. Never treat a build artifact alone as a recoverable project state.

## Historical reconciled resume entrypoint (M2.3 checkpoint)

Begin with [the M2 detailed plan](../plans/02-memory-system.md), the active/next task in project-state.json, and [the canonical roadmap](ROADMAP.md). M0 and M1.0–M1.5 are complete. The user accepted the M2 plan. M2.1, M2.2 and the M2.3 acceptance repair are verified locally; the last verified implementation is 5b83338. M2.4 persistent jobs/open-format export is next and remains unstarted. Verify the latest checkpoint/publication before proceeding. Preserve the M1 completion tag at 97babb3. Check BUILD_STATUS.md for actual acceptance/publication. Read the detailed acceptance plan before coding. Milestone IDs follow master section 82; legacy numeric plan filenames do not define milestone numbers. Run `node --test tests/repository-hygiene.test.mjs` and `node scripts/validate-planning.mjs --self-test` for current foundation checks; these do not run product tests or replace source recovery checks.

## Workspace data recovery after M1.3

M2.1 uses schema 3. Recognized schema-1/2 catalogs migrate forward without removing original workspace fields; older binaries cannot open schema 3. Preserve a complete closed-root copy before manual application rollback. Do not downgrade user_version or copy only selected rows/files. Onboarding-complete is only a UI preference and grants no cloud-sharing/recording consent.

Source bundles exclude application data. Close both desktop and API before preserving the entire configured storage root, including .anansi-root, workspaces.sqlite3, all content/ Markdown revisions and any WAL/SHM/journal files. Keep the original unchanged outside Git and use only a verified compatible copy for manual recovery. Never copy just the main database while a client is running, delete a failed initialization, lower a schema version or reset an unknown catalog automatically. Read-only, corrupt and newer-schema failures leave a recoverable error; interrupted first initialization may require inspection and manual recovery. Automatic user-data backup/restore remains future work. See [M1.3 evidence](evidence/M1-3.md).

If a documentation checkpoint records the previous verified implementation commit, inspect subsequent metadata commits as well. Never reset to a recorded SHA just because it is older than HEAD. Fresh recovery uses `git clone https://github.com/robillionair/Anansi-ultimate.git`; bundle recovery uses `git bundle verify <bundle>` followed by `git clone <bundle> <new-directory>`. M0 independent backup/restore has been proved and recorded in docs/evidence/M0.md; follow RECOVERY_AND_BACKUP.md to restore or create subsequent source backups.

## M1.5 installer recovery

For M2.1 integrity errors, preserve the complete root unchanged and use a compatible copy for diagnosis. Unreferenced Markdown may be a failed/conflicted save; never infer that it should be imported or deleted. Do not repair a missing body by creating an empty file or editing revision pointers. Source bundles exclude all application data. Restore/backup UI remains M2.5/M2.7, not a current product recovery feature.

Installer verification uses a fresh external temporary target and refuses existing ANANSI registration/remembered locations. A failed verification retains its exact target and installer-verification.json. Inspect that record and compare the registered InstallLocation and installed payload hash before running only the generated test uninstaller. Never remove a real application, edit its registration or delete data merely to unblock the test. Successful verification removes only its originally absent, test-owned remembered-location product key after exact location readback. Private workspace fixtures remain outside source.

Development installer artifacts are rebuildable and are excluded from source bundles. CI release artifacts retain installer SHA256 and payload/build manifests for 14 days; they are unsigned development builds. No API service, model downloads, updater or signed-release guarantee is bundled. M2 requires its separate detailed plan and approval; do not start it at the M1 completion boundary.

## M1 completion boundary

M1 development desktop/browser foundation is verified complete; see [M1.5 evidence](evidence/M1-5.md). The completion tag is m1-desktop-foundation-verified at 97babb3, applied after protected publication. Source/tag readback and independent restore passed. Subsequent user continuation accepted M2.0 and authorized M2.1. Its schema-3 memory implementation is separate from M1; see evidence/M2-1.md. Preserve the M1 tag and distinguish source bundles from private-data backups.

## M2.2 schema and history recovery

M2.2 moves recognized schema-1/2/3 roots forward to schema 4 without rewriting original revision JSON or Markdown. Preserve the entire closed root before migration; schema-3 binaries must refuse it. Never lower user_version to run an older binary. Failed migration retains its previous schema and records. Source bundles do not contain private application data.

Archive and expiry retain bytes and history. Include archived/expired records explicitly to reopen them. Restore archive returns to the saved prior state; an expired record remains filtered until explicitly requested. Compare history and restore selected content as a new revision to retain a recoverable audit trail. This preserves current privacy and lifecycle; it does not restore a database snapshot, erase originals, or implement M2.5 backup recovery. Stale writes must reopen the latest revision rather than overwriting it. See [M2.2 evidence](evidence/M2-2.md).

## M2.3 entities and relationships recovery

M2.3 advances recognized schema-1/2/3/4 roots forward to schema 5, creating the `relationships` table with composite foreign keys `(workspace_id, source_id)` and `(workspace_id, target_id)` referencing `memories(workspace_id, id)`. Preserve the entire closed root before forward migration; older binaries (< schema 5) refuse to open schema 5 databases. Never manually lower `user_version` to run an older binary. Failed forward migrations rollback atomically within a transaction and retain the prior schema version and records. Source bundles exclude private application data.

All nine user knowledge kinds (`memory`, `note`, `project`, `task`, `person`, `organization`, `decision`, `event`, `document`) and type-specific attributes (`dueDate`, `completed`, `projectId`, `eventDate`) are stored and validated at the boundary. Archiving or deleting a record does not cascade-delete relationships automatically; relationships pointing to archived records remain intact in the relationship graph for auditing and history. Cross-workspace relationships and self-loops are rejected by SQLite composite foreign keys and core validation. To recover or repair relational data, inspect relationships using `listRelationships` and verify both endpoints exist in the workspace. Stale or conflicting writes must re-read existing relationships rather than force-overwriting. See [M2.3 evidence](evidence/M2-3.md).


### M2.3 acceptance repair recovery

Schema 6 migrates recognized earlier catalogs forward without changing applied migrations. Preserve the entire closed storage root before upgrading. Do not downgrade user_version or run schema-5 binaries against the upgraded root. A migration error rolls back schema/version together. Surviving schema-5 links retain their IDs/metadata with legacy_schema5 provenance; previously deleted links and unrecorded past actions cannot be reconstructed. Link removal now archives with a required expectedVersion; inspect scoped relationship history and restore the latest archived version. Record revision snapshots and independent relationship action history have different scopes; neither should be rewritten to match the current graph. See ADR-0019 and [M2.3 repair evidence](evidence/M2-3.md).


The M2.3 completion source checkpoint is independently bundled/restored after protected publication. Its machine ledger is `C:/Users/robin/Documents/ChatGPT/Anansi-source-recovery/M2-3-verified-b4a8e110d61042fbb11a3faf434bf9b4/source-verification.json`; read its actual HEAD/tree/digest/results rather than infer a self-referential commit from this document. Actual memory/entity verification JSON and screenshots are retained in the same external directory. This source ledger is separate from private-data backup/recovery.

## M2.4 recovery contract

The active branch is `codex/m2-4-jobs-export`; local acceptance passed at 8f7ff87 in [M2.4 evidence](evidence/M2-4.md). Schema 7 migrates forward from recognized schemas 1–6 without changing existing migrations. Preserve the entire closed private root before rollback; schema-6 binaries must refuse schema 7 rather than modifying its version. SQLite remains canonical for job state, histories and ownership. Interrupted Running jobs become Paused after lease expiry (120 seconds); explicitly resume to rebuild the snapshot stage. Cancel/retry never publishes old partial attempts. Keep unreferenced private export files for diagnosis; no automatic deletion or import is authorized. A portable knowledge export is not a full application backup or a product restore package. No M2 milestone tag is created. M2.5 remains unstarted until this task is safely published/recoverable and the user continues.

M2.4 source recovery package: `C:/Users/robin/Documents/ChatGPT/Anansi-source-recovery/M2-4-checkpoint-mvd70n47`. The final `source-verification.json` records actual published HEAD/tree, source.bundle SHA256, fresh-clone checks, CI URLs/status and real screenshot hashes. The implementation-only source checkpoint precedes the acceptance metadata to avoid self-reference. Use the final ledger and exact GitHub checks when reconciling pending-at-authoring CI statements; do not assume a source bundle contains application data.
