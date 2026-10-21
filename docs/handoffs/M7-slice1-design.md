# M7 Slice 1 design — Product Forge: concept → specification → source package (export-for-review)

Date: 2026-10-21 · Owner-approved bounded first slice of M7
(MASTER_SPEC §§28–33; plan `plans/05-product-forge.md`;
requirement AN-012). Baseline: the verified R17 preview
checkpoint `e915f63`.

## Scope

One vertical workflow, one template:

**Saved concept → product specification → development plan
→ generated source package → inert inspection → versioned
save → export → restart/reopen/revise → backup/restore.**

The single approved template is a **small, dependency-free
static web application** (HTML/CSS/JavaScript). Everything
is labelled a **prototype / source package**: not SaaS, no
authentication, no database, no deployed service. Per §32,
**no execution sandbox exists in this slice, so automatic
execution is disabled and export-for-review is the
deliverable** — generated code is never run, previewed as
HTML, installed, or deployed. §30 stage statuses are shown
honestly: specification / plan / source generation can
complete; build, test, preview and deployment stages are
**Unavailable** with the §32 reason. Nothing is ever marked
built, tested, working or completed beyond what occurred.

## Flow and pinning

1. The user opens a saved idea/opportunity artifact and
   picks a concept. Forge reads the artifact at the
   **exact pinned version** (never "latest" silently).
2. Essential questions only (§29): purpose, intended users,
   must-have behavior — three bounded free-text answers.
3. `forge-spec-*`: the model (existing M5 substrate, all
   gates unchanged) drafts a product specification from the
   pinned concept snapshot + answers. Strict-parsed,
   bounded. The user reviews it; **approval is the explicit
   save** as an immutable `specification` artifact citing
   `artifact://<source>@v<n>`.
4. `forge-build-*`: the model drafts a development plan +
   source files from the **pinned specification version**
   (selected explicitly, revisable beforehand through the
   registry — a revise creates a new pinned version).
5. The user inspects files as inert text with the provider
   disclosure and the package limitations, then saves the
   package as an immutable `source-package` artifact.
6. `forge-export` (artifacts command) returns a
   deterministic USTAR tarball (base64 + SHA-256) of the
   pinned package version plus an `ANANSI-PACKAGE.json`
   manifest (provenance + per-file hashes + honesty
   labels). The UI downloads it; nothing executes it.

## Output safety (validated in core, defense in depth)

- Files: 2–12 entries; path ≤ 96 chars, relative, `/`-
  separated segments of `[A-Za-z0-9._-]` not starting with
  `.`, no `.`/`..` segments, no backslash, no leading `/`,
  no drive letters; extensions `.html .css .js .md .txt`
  only; paths unique (exact and case-insensitive); each
  file ≤ 32 KiB, package ≤ 192 KiB, no NUL bytes.
- Required: root `index.html` and root `README.md`.
- The tar writer emits only regular files, mode 0600,
  uid/gid 0, mtime 0 — no links, devices, or executable
  archive metadata.
- Model output that fails any check ends the run `invalid`
  with the reason; nothing is saved. Source text is data:
  concept/spec content is sent as data segments, never as
  instructions; the model is told the deliverable limits.

## Provenance and restore

Spec bodies embed the source artifact id + version and a
concept snapshot; package bodies embed the spec id +
version (and through it the source). Restore remaps these
artifact references like evaluations (fail closed when the
referenced artifact is not in the package) and recomputes
hashes. Registry `source_references` use `artifact://…@v…`
throughout, so the generic restore rewriting covers them.

## Acceptance criteria

1. Full workflow (browser e2e, synthetic provider): open
   saved opportunity set → pick concept → answer questions
   → generate + save specification → generate plan/source
   → inspect a file as text → save package → export
   (download verifies: tar members = package files +
   manifest, hash matches) → restart → reopen → revise the
   specification to v2 → generate a second package pinned
   to v2 → backup → fresh restore → restored spec/package
   resolve to restored sources.
2. Pinning: generation reads exactly the requested
   artifact versions (stale-version reads verified in core
   tests — a concept edited in a newer version does not
   leak into a run pinned to the older one).
3. Safety validation: traversal/absolute/backslash/drive
   paths, duplicates, disallowed extensions, oversized
   file/package, too many files, missing index/README,
   malformed JSON and refusal text → run `invalid`, nothing
   saved (unit + integration).
4. Lifecycle: provider failure → terminal `failed`;
   cancel → `cancelled` with nothing saved; duplicate and
   concurrent saves → one artifact; cross-workspace runs
   and artifacts refused; archived project refuses;
   artifact privacy propagates to segments (sealed source
   artifacts stay local-only under the unchanged M5 gate).
5. Honesty surfaces: the package view and export manifest
   carry the prototype limitations verbatim; pipeline
   stages for build/test/preview/deploy render
   "Unavailable — no execution sandbox in this slice (§32);
   export is for review" and never a completed state.
6. Regressions: existing M6/provider/recovery suites pass;
   contracts/tsc/clippy/fmt/docs clean.

## Explicitly not in this slice

Execution/build/preview/deployment (§32 sandbox is a
later, separately approved slice — Antigravity reviews
that boundary independently), additional templates or
product types (SaaS/desktop/agents/APIs stay planned per
§28 and DECISIONS), dependency installation, licensing
generation (the manifest states no license is granted or
implied), market/launch material (§33 commercial extras),
and any background or autonomous generation.
