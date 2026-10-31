# M8 §27 artifacts & Forge provisioning — delivery addendum

Branch: `muse/m8-artifacts-forge`, from
reviewer tip `7632f94765cbf1f886fd12fb6e59476fb1e8b75b`
(the Three.js visualization runtime). Design and
acceptance criteria committed before code
(`docs/handoffs/M8-artifacts-forge-design.md`).
Release tooling directories are NOT touched. No
paid APIs, no model downloads, no generated-code
execution. No new dependency was added anywhere.

## What was built

- Six deterministic markdown artifacts derived
  purely from the saved experiment document
  (`crates/core/src/collider_artifacts.rs`,
  canonical; `packages/ui/src/colliderArtifacts.ts`,
  byte-identical mirror). The save path embeds
  their SHA-256 + byte counts in the document's new
  optional `artifacts` field; older experiments
  without the field still parse and validate.
  `validate_experiment_doc` recomputes every
  document when the field is present and rejects
  ANY hash or byte-count mismatch fail-closed —
  which covers restore and transfer, since both
  already run that gate.
- Content honesty rule, enforced by tests: the
  documents organize the record's own fields and
  add no new claims. Risk severity/likelihood,
  performance budgets, module lists, interfaces,
  and phase breakdowns are written as "Not stated
  in the source record", never invented. The node
  suite asserts no invented severity/likelihood
  values appear in any derived document.
- Artifact viewer in the saved experiment view
  (alongside the visualization): six tabs, a safe
  markdown renderer (headings, bold, lists, fenced
  code; zero raw-HTML injection — asserted by
  grep over the new files), section-aware search
  that renders matches as plain lines so a filter
  cannot corrupt a code fence, per-artifact copy,
  and only the active tab rendering at a time.
- Deterministic export bundle: a hand-rolled STORE
  ZIP writer (fixed 1980-01-01 timestamp, fixed
  order, CRC-32) plus a MANIFEST with byte lengths
  and SHA-256 hashes for the six documents and
  `experiment.json` (the stored body, byte for
  byte). Export verifies the fresh derivation
  against the document's stored hashes first and
  refuses on any mismatch. Download is Blob +
  anchor, the established Jobs export precedent.
- "Build This Product": a one-click action in the
  saved experiment view that calls the EXISTING
  `collider-transfer` op — no new op, no new Forge
  entity. The approved brief is the record's own
  concept brief (the text CONCEPT.md carries in
  its Concept brief section); the full CONCEPT.md
  text would exceed the op's 4000-character brief
  bound, so it is not used as the brief. The op
  embeds the derived ARCHITECTURE.md as the
  concept's new optional `initialSpecification`
  field and pins experiment id, version, and
  content hash. Re-running Build returns the same
  concept with `alreadyTransferred`. The concept
  view renders the initial specification with the
  safe renderer, and labels the brief correctly:
  Build-path concepts say "the record's concept
  brief, used verbatim by Build This Product",
  edit-path concepts still say "user-authored".
  Transfer still approves NO specification and
  starts NO provider run; ForgeSpecPanel remains
  the separate explicit spec action.

## Corrections found by testing (not by inspection)

1. Substring-heading collision (real defect, fixed
   in this slice): the new `<h3>Experiment
   artifacts</h3>` made the shared e2e helper's
   non-exact `getByRole('heading', {name:
   'Artifacts'})` match twice (Playwright name
   matching is case-insensitive substring by
   default), failing the Three.js spec on its
   second `openProject`. The helper's two
   assertions are now `exact: true` — more
   precise, not weaker. The full battery passes
   after the fix.
2. `fmt --check` failed on whitespace drift in the
   new Rust module and in the transfer edit;
   `cargo fmt` applied, formatting-only diffs,
   committed separately, `fmt --check` clean after.
3. Parser `null` handling: the UI parser first
   rejected `artifacts: null`, while the backend's
   Option field parses null as absent. Fixed to
   treat null as absent, matching the backend.

## What was deliberately NOT built, and why

- Native save dialog for export: the existing
  native save is a Windows-only PowerShell dialog
  command (`save_export`) for server-side job
  artifacts. A native path for a client-derived
  bundle would need a new desktop command plus a
  zip dialog script in Windows-only code that
  cannot be compiled or tested on this Linux VM.
  Shipping it unverified would risk the Windows
  build, so this slice ships the browser download
  (which also works in the Tauri webview) and
  records the native dialog as a small, separately
  approved follow-up: a `save_experiment_export`
  command mirroring `save_export` plus a zip
  SaveFileDialog script. It is NOT claimed here.
- No new Forge "project" entity: in ANANSI's
  model the Forge-side entity created by an
  approved handoff IS the transferred concept, and
  spec generation is the separate explicit
  ForgeSpecPanel action. Inventing a parallel
  project entity would duplicate Slice 2.

## Linux evidence (sequential)

- Rust workspace: fmt clean (after the fix above),
  Clippy `-D warnings` clean, tests **553 passed,
  4 failed** — the 4 are the known live-Ollama
  `semantic_workflow` environmental failures
  (embedding backend unavailable:
  `StorageUnavailable` where the test expects
  `RevisionConflict`), unrelated to this slice's
  files. New: 3 artifact lib tests (determinism +
  structure, tamper fail-closed, pinned hashes).
- Node `test:ui`: **125/125**
  (115 + 10 new: derivation hashes, export/zip/
  manifest, markdown rendering incl. a 1 MB+
  document).
- TypeScript exit 0, Vite web build exit 0,
  contracts `--check` clean, docs validation PASS
  (208 files before this addendum; 209 with it).
- Browser battery **12/12 specs** (13 tests):
  artifacts-forge (viewer, byte-identical repeat
  exports, manifest-vs-entry-vs-stored-hash
  verification, Build, concept + Forge panel with
  no auto-spec, backup/restore leg with
  already-transferred re-Build), visualization,
  visualization screenshots, three visualization
  (2/2 after the heading fix), three fallback,
  document input, URL input, pipeline overview,
  transfer, forge, collider.
- Screenshots shipped (2 new, inspected):
  `artifacts-viewer.png` (RISKS.md tab with
  search, copy, and export controls) and
  `artifacts-concept.png` (transferred concept
  with the verbatim brief label and the rendered
  ARCHITECTURE.md initial specification).
- Credentials sweep clean (only the standard
  ephemeral local pairing-token plumbing shared by
  all e2e specs). No Rust schema change; the only
  contract change is the two additive optional
  fields (`artifacts`, `initialSpecification`).

## What is still open (not claimed)

- Independent Windows rerun of this slice by
  ChatGPT (checklist in the MANIFEST), especially
  the case-sensitive build, frozen contracts, and
  Windows screenshots of viewer + concept.
- Native save dialog for export (sketched above).
- M7 native desktop acceptance + preview
  promotion; M5 production gates; §32 sandbox;
  Windows verification of Slice 4; Windows
  verification of the Three.js slice. This slice
  completes §27's artifact suite and the §28
  bridge as designed above; it does not claim M8
  complete.
