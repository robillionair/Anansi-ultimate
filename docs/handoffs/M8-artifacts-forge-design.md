# M8 §27 versioned artifacts & Forge provisioning — bounded design and acceptance criteria

Baseline: `7632f94765cbf1f886fd12fb6e59476fb1e8b75b`
(branch `muse/m8-threejs-visualization`), strictly
continuous from verified tip `9147e349`.
Branch: `muse/m8-artifacts-forge`.
Design committed before any implementation code.
Ownership: application code (`crates/core/`,
`crates/api/`, `packages/ui/`, `apps/web/`,
`apps/desktop/`) and associated tests. Release
tooling directories are NOT touched. No paid APIs,
no model downloads, no generated-code execution.

## Authoritative sources and what already exists

- MASTER_SPEC §27 is terse: each run produces a
  versioned experiment; possible artifacts
  `CONCEPT.md`, `ARCHITECTURE.md`, `FEASIBILITY.md`,
  `ROADMAP.md`, `RISKS.md`, `TEST_PLAN.md`; the user
  can select **Build This Product**, which creates a
  Product Forge project using approved outputs; do
  not automatically execute generated code. §§28–30
  describe Forge generally (concept → spec → build,
  spec generation a separate explicit action).
- Already built, REUSED (never duplicated):
  - M8 Slice 2 `collider-transfer` op
    (`crates/core/src/transfer.rs`): model-free,
    binds an exact experiment version + recomputed
    content hash + one selected combination + an
    approved brief into a durable `concept` artifact
    (format `anansi-forge-concept` v1); idempotency
    is structural (same experiment/version/
    combination/brief → same artifact, flagged
    `alreadyTransferred`); restore pre-validates +
    remaps fail-closed. UI: `ColliderTransferPanel`
    (edit-before-approve) in the saved experiment
    view, opening the concept + `ForgeSpecPanel`.
    Slice 2 boundary preserved: transfer approves
    NO specification, NO paid inference, NO execution.
  - M7 Forge spec flow (`ForgeSpecPanel`,
    `forge-spec-start`): spec generation stays a
    separate explicit provider action.
  - Jobs export precedent (`Jobs.tsx`): download is
    a verified Blob + anchor download in both
    browser and native shells.
  - The saved experiment view (`Projects.tsx`)
    already composes `ColliderExperimentView`
    (incl. the 3D visualization) + transfer panel;
    the viewer, export, and Build action join that
    composition — no new view, no parallel system.

## Design decisions (recorded, not fabricated)

### 1. Six artifacts: derivation, storage, hashes

- Derivation is PURE and deterministic from the
  saved `ColliderExperimentDoc` alone: fixed
  templates + verbatim record fields, `\n` line
  endings, fixed section order. New Rust module
  `crates/core/src/collider_artifacts.rs` is
  canonical; `packages/ui/src/colliderArtifacts.ts`
  mirrors it exactly. Drift guard: the Rust tests
  pin hashes for a shared fixture, and the TS tests
  assert the SAME hashes for the SAME fixture.
- Content honesty rule (load-bearing): the six
  documents ORGANIZE the record; they do not invent
  analysis. Anything the record does not state —
  risk severity/likelihood, performance budgets,
  interface details beyond the recorded architecture
  text — is written as "Not stated in the source
  record", never as an invented value. Each document
  carries a header stating it is derived from the
  pinned experiment (id, version, provider, model)
  and that its contents are the record's own fields.
- Storage (no parallel persistence): the artifacts
  ride INSIDE the experiment document. The doc gains
  one optional field:
  `artifacts: Option<ColliderArtifacts>` — six
  entries, each `{ sha256, bytes }` for the derived
  markdown. The save path (`collider.rs::save`)
  derives + embeds them before registering the
  artifact row, so every newly saved experiment
  permanently carries its artifact hashes. The
  artifact CONTENTS are always re-derivable from
  the doc, so backup/restore needs no new rows or
  foreign keys: the experiment artifact already
  restores with remapping, and validation
  re-checks the hashes there.
- Backward compatibility: `artifacts` is
  `#[serde(default, skip_serializing_if)]`; older
  saved experiments (no field) still parse and
  validate. The UI parser's strict allowlist gains
  `artifacts` with strict shape-checking when
  present (six entries, 64-hex hashes, matching
  byte counts recomputed against the derivation —
  see below). Contracts are regenerated from the
  Rust types; the hand-maintained client lists are
  updated where the new types surface.
- Validation (fail-closed): `validate_experiment_
  doc` recomputes all six documents when `artifacts`
  is present and compares every hash and byte count;
  ANY mismatch (truncated, corrupt, or swapped
  content) rejects the document. This gate already
  runs at restore (2 sites) and transfer (2 sites),
  so tampered artifacts fail closed on every path
  that re-admits a record — no new gate needed,
  only the recomputation inside it.

### 2. Artifact viewer (saved experiment view)

- New component `ColliderArtifactViewer.tsx`,
  rendered in the saved experiment view alongside
  the visualization (which keeps its own section).
  Tabs for the six documents; only the active tab
  renders (bounded work for large documents).
- Markdown rendering is a small SAFE renderer
  (`colliderMarkdown.tsx`): headings (`#`/`##`/
  `###`), bold (`**`), unordered/ordered lists,
  fenced code blocks, paragraphs; everything else
  renders as a plain text line. Zero `innerHTML`,
  zero `dangerouslySetInnerHTML`, no new dependency
  (no markdown library is installed, and the
  generated documents use only this subset — the
  renderer is specified for exactly the subset the
  derivation emits, and degrades to plain lines
  otherwise). A light code-span style stands in for
  "syntax highlighting" of inline code and fences;
  no highlighter dependency is claimed.
- Search/filter: a text input filters the active
  document's lines (case-insensitive substring,
  matching lines shown with their section heading
  context); clearing restores the full document.
- Copy: one button per artifact copies its full
  markdown via `navigator.clipboard` with a
  "Copied" confirmation state; clipboard failure
  surfaces as visible text, never silently.
- The viewer renders ONLY for parsed saved
  experiments. An unapproved, incomplete, or
  unparsable collision has no viewer, no export,
  and no Build action — export/Build fail closed by
  absence, not by an error dialog.

### 3. Export bundle (deterministic, browser download)

- New module `packages/ui/src/colliderExport.ts`:
  a minimal deterministic ZIP writer (STORE method,
  no compression): fixed DOS timestamp (1980-01-01),
  fixed entry order (six docs, `experiment.json`,
  `MANIFEST.json`), CRC-32 implementation, no
  extra fields. Same inputs → byte-identical output.
- "Export Full Experiment" button in the saved
  experiment view builds the bundle from the
  already-loaded document: the six derived docs,
  `experiment.json` (the canonical saved document
  JSON), and `MANIFEST.json` (entry byte lengths,
  SHA-256 hex per entry computed with
  `crypto.subtle`, the experiment artifact id /
  version / timestamp from the artifact record,
  provider + model disclosure). The bundle is
  downloaded via Blob + anchor, the established
  Jobs export precedent.
- Verification contract: MANIFEST hashes MUST equal
  the SHA-256 of the zip entries AND the hashes
  stored in the document's `artifacts` field (the
  node tests assert the derivation hashes; the
  browser spec asserts MANIFEST-vs-entry consistency
  for a downloaded bundle).

### 4. "Build This Product" (Forge provisioning)

- NO new backend op and NO new Forge project
  entity. In ANANSI's model the Forge-side entity
  created by an approved handoff IS the transferred
  concept, and spec generation remains the separate
  explicit `ForgeSpecPanel` action. The slice adds:
  - A "Build This Product" action in the saved
    experiment view: a combination picker (same
    combinations as the transfer panel) plus one
    button. It calls the EXISTING
    `collider-transfer` op with `approvedBrief` =
    the derived `CONCEPT.md` text (the approved
    concept brief from CONCEPT.md, verbatim) for
    the selected combination. Idempotency is the
    op's own: re-submission returns the same concept
    with `alreadyTransferred: true`; the UI
    surfaces that state instead of duplicating.
    On success it uses the existing `onTransferred`
    navigation (concept artifact + ForgeSpecPanel).
  - `TransferConceptDoc` gains ONE optional field,
    `initialSpecification: Option<String>`, set by
    the transfer op from the source experiment's
    derived `ARCHITECTURE.md` at transfer time
    (both the Build path and the existing edit path
    get it — the op, not the button, owns it).
    `TransferConceptView` renders it as "Initial
    technical specification (derived from the
    pinned experiment's ARCHITECTURE.md)" when
    present. Old concepts (no field) render as
    before. The field is covered by the existing
    transfer-approval pinning (approval pins the
    concept body hash) and by restore remapping;
    it changes no approval semantics.
- Explicitly NOT built in this slice (recorded, not
  silently dropped): a native save dialog for export
  (would need `plugin-dialog` + `plugin-fs` + a
  capability review; the browser download is the
  supported path, exactly as Jobs export does
  today). If the owner wants it, it is a separate,
  small, separately approved addition — not a
  reason to hold this slice.

### 5. Restore lineage survival

- The experiment doc (with artifact hashes) and the
  transferred concept (with `initialSpecification`)
  both travel through the EXISTING backup/restore
  machinery. Coverage: a Rust test proving a doc
  with artifacts validates after a tamper-free
  round-trip and fails after hash tampering; a
  browser spec leg proving backup → fresh restore →
  reopened experiment still shows all six viewer
  tabs, exports a MANIFEST whose hashes match the
  restored doc, and re-running Build for the same
  combination reports already-transferred (no
  duplicate concept).

### 6. Adversarial coverage (AC-7, concrete)

- Truncated/corrupt artifact record → Rust test:
  flip one hash char / shorten one byte count →
  `validate_experiment_doc` rejects.
- Export of an unapproved/incomplete collision →
  by construction: viewer/export/Build render only
  for `parseColliderExperimentBody != null` (a
  saved, validated experiment); the live panel has
  none of them. Browser spec asserts their absence
  on the live (unsaved) panel.
- Large documents → derivation inputs are bounded
  by existing result validation (understanding
  ≤6000 chars, primitives ≤300 × 12, mappings ≤600
  × 12, combinations bounded, beams ≤8000 × 2), so
  derived docs are bounded (~100 KB); the viewer
  renders one tab at a time; a node test renders a
  1 MB+ markdown string through the safe renderer
  and asserts fast completion with zero `innerHTML`.

## Acceptance criteria (all must pass before delivery)

- AC-1: `ColliderExperimentDoc.artifacts` (optional)
  carries six SHA-256 + byte counts; old experiments
  parse + validate unchanged; contracts regenerated.
- AC-2: Rust and TS derivations are byte-identical
  on the shared fixture (pinned hash vectors agree).
- AC-3: Viewer shows all six tabs, renders the
  safe-markdown subset, filters by search, copies
  per artifact, zero `innerHTML` (node assertion
  over the new files).
- AC-4: Exported zip contains exactly the six docs
  + `experiment.json` + `MANIFEST.json`; MANIFEST
  hashes equal entry hashes equal doc hashes.
- AC-5: Build This Product creates (or reuses, via
  `alreadyTransferred`) exactly one concept for the
  (experiment, version, combination, brief) tuple,
  carrying the pinned experiment id/version/content
  hash and the derived ARCHITECTURE.md as
  `initialSpecification`; navigation lands on the
  concept + ForgeSpecPanel; no spec is auto-started.
- AC-6: Backup/restore leg proves artifacts +
  concept survive with identical hashes and no
  duplicate concept on re-Build.
- AC-7: Tamper tests fail closed; unapproved export
  absent by construction; 1 MB+ render test passes.
- AC-8: Node tests (derivation, hashing, manifest,
  zip structure, renderer), Rust tests (derivation,
  validation incl. tamper, transfer field), and a
  Playwright vertical slice (collide → viewer →
  export → Build → Forge concept → restore leg)
  all green; strict Clippy/fmt/tsc/contracts/docs/
  builds clean.
- Delivery: checksummed ZIP with Git bundle, exact
  baseline/tip, design/acceptance document,
  build/test logs, screenshots; Windows rerun
  checklist; native save dialog recorded as open,
  not claimed; no promotion, pushes, or CI; stop
  after this slice.
