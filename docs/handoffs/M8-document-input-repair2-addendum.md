# M8 document input repair 2 — delivery addendum (Linux verification)

Baseline: reviewer tip `d3f004bc` over repair tip `ef0b9843`.
DI-01 and DI-02 are closed and preserved. Design committed
before code in
`docs/handoffs/M8-document-input-repair2-design.md`. All
reviewer probes are preserved unchanged.

## Finding repaired

- DI-03 (P2): the DI-01 pre-read size branch returned
  before `session.load()`, so it never advanced the load
  session's generation. A gated read from an earlier
  selection still owned the current generation and, when
  released, replaced the reviewed beam despite the newer
  rejected selection. The fix is one boundary in
  `pickDocument`: every new selection invalidates the
  beam's load session first, before any early return.
  The rejected selection clears the obsolete loading
  state (the invalidated read resolves silently without
  clearing it itself) and shows the current rejection,
  while the last applied beam and attribution stay
  untouched. The pre-read size guard, the decoder's
  post-read check, the DI-02 failure gate, edit/clear/
  unmount behavior, and same-name reselect are unchanged.

## Verification (sequential, Linux)

- Before repair, the new DI-03 probe unchanged: 1 failed
  — only its identity assertion ("rejecting a newer
  oversized selection must invalidate older pending
  content": beam became "Obsolete content must not win"
  instead of retaining "Keep this reviewed text").
  Log kept as `before-rejected-probe.log`.
- After repair, all three document probes unchanged:
  rejected-selection 1/1, DI-01/DI-02 reviewer probe 1/1,
  canonical document spec 1/1, each including its
  inherited save/restart/backup-restore workflow.
- Affected browser battery: reviewer anchors, instance
  anchors, project context, integrated acceptance,
  collider, memory context, memory acceptance,
  picker reliability, pipeline overview, transfer,
  transfer identity, transfer concurrency, forge, forge
  review, forge revision, forge deliverable, backups,
  export — all passing (see `after-browser-battery.txt`).
- Node UI/contracts: 96/96 (95 + 1 new test: obsolete
  success and obsolete failure after a rejected
  selection, plus a fresh load applying afterward).
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
