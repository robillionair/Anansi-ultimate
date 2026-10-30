# M8 document input repair — delivery addendum (Linux verification)

Baseline: reviewer tip `97ab19b4` over delivery tip `faafa1c26`.
Design committed before code in
`docs/handoffs/M8-document-input-repair-design.md`. The
reviewer probe `tests/e2e/review-collider-document-input.spec.ts`
is preserved unchanged.

## Findings repaired

- DI-01 (P2): `pickDocument` refused oversized files only
  after `file.arrayBuffer()` had already read them. The
  pick now checks `file.size` against the 65,536-byte limit
  first and refuses with the same visible message, without
  setting loading state or starting a load. The decoder's
  post-read length check is retained for files whose size
  changes mid-read.
- DI-02 (P2): the load session resolved successes with a
  generation but rejected failures, so the component's
  catch path could not tell an obsolete failure from the
  current one — an obsolete failure displayed its error
  and cleared the loading flag, enabling Collide Ideas
  while a newer read was still pending. `load` now returns
  a uniform outcome `{ generation, result, error }` that
  never rejects, and `pickDocument` applies one generation
  gate to both paths: a superseded outcome changes neither
  error nor loading state. Edit and clear already
  invalidate the session, so failures superseded by either
  are stale by the same rule. The dispatch guard is
  unchanged: Collide Ideas stays disabled while either
  beam's loading flag is set.

## Verification (sequential, Linux)

- Before repair, reviewer probe unchanged: 1 failed —
  soft assertion "oversized file must be refused BEFORE
  reading its contents" (read list contained the oversized
  file) and hard assertion "obsolete failed read must not
  enable dispatch while newer read is pending" (button
  enabled). Log kept as `before-review-probe.log`.
- After repair, reviewer probe unchanged: 1/1 passed,
  including the inherited save/restart/backup-restore
  workflow and both finding assertions.
- Canonical document spec: 1/1 passed.
- Affected browser battery: reviewer anchors, instance
  anchors, project context, integrated acceptance,
  collider, memory context, memory acceptance,
  picker reliability, pipeline overview, transfer,
  transfer identity, transfer concurrency, forge, forge
  review, forge revision, forge deliverable, backups,
  export — all passing (see `after-browser-battery.txt`).
- Node UI/contracts: 95/95 (94 + 1 new test covering
  superseded failures after newer selection, edit, and
  clear, plus a current failure surfacing its error).
- Focused Rust collider suite: 45/45 (+1 ignored real-v10
  fixture test). Strict Clippy clean, fmt clean, contracts
  `--check` clean, TypeScript clean, docs validation PASS,
  API/web builds pass.
- No backend, schema, or contract change; no new feature.

## Still unfinished (explicitly retained)

Independent Windows rerun of this repair by ChatGPT
(including native file-picker behavior), Windows
verification of Slice 4, M7 native acceptance + preview
promotion, M5 production gates, §32 sandbox. URL fetching,
other document formats, OCR/transcription and 3D remain
unfinished. No pushes, master merges, CI, or preview
promotion.
