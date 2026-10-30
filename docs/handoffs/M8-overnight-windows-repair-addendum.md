# M8 overnight Windows repair — after record

Branch `muse/m8-overnight-windows-repair`, from reviewer tip `240af4ef`
(`edba7ed3` plus reviewer probes/docs only). Design committed before code:
`docs/handoffs/M8-overnight-windows-repair-design.md`. Reviewer probes are
unchanged; the only existing browser spec updated is `collider.spec.ts`,
whose two saved-view locators now use the saved instance's scoped
combinations heading (the legacy id no longer exists on saved views).

## Before (reproduced on Linux, same as Windows)

- Rust reviewer probes: 0/2 — forged snapshot restored instead of
  refusing; concept-context probe printed
  `selected concept pins final restored target hash: false` and the
  second fresh restore failed with `InvalidRequest`.
- Browser reviewer probe: 0/1 — only the final saved-target assertion
  failed (`saved experiment section link resolves outside its artifact
  view`); all earlier live/saved steps passed.

## Repairs

- R1: `allowed_restore_transform` in `crates/core/src/restore.rs`.
  `restoredContentHash` is untrusted metadata, never authorization.
  Pristine rows must be byte-identical to the snapshot; diverged rows
  must differ only by path-scoped id substitutions (new package id,
  old id absent) and pinned-hash rewrites that equal the referenced
  package row's hash, for the ten known transformed formats. All other
  differences refuse before destination publication.
- R2: `apply_pinned_hash_fixups` in `restore.rs` runs all six pinned-hash
  passes to a global fixpoint (cap `rows + 1`), so every pinned field
  names a final materialized row. A same-version cycle keeps changing
  hashes every round and refuses at the cap.
- R3: `colliderResultHeadingId` in `ColliderOverview.tsx` scopes the six
  result headings, their `aria-labelledby` associations and the overview
  links by instance. Live keeps legacy ids; a saved view uses
  `saved-{artifactId}-v{version}`. The live collider disclosure is scoped
  to `collider-providers-disclosure-title` and its overview link points
  there. Deliberate exception: the pipeline heading itself
  (`collider-pipeline-heading`) is not a link target and stays legacy in
  both instances, because the reviewer probe locates the saved overview
  by it inside the Artifacts region.

## After (sequential, Linux)

- Focused probes: Rust reviewer probes 2/2; new swapped-legitimate-body
  attack test 1/1; restore lib binding tests 4/4 including the new
  same-version cycle unit test; browser reviewer probe 1/1; new
  live-vs-saved instance-anchors browser spec 1/1 (DOM targets and
  keyboard Tab/Enter, not href strings alone).
- Affected suites: collider 37/37, transfer 19/19, forge_restore 4/4,
  backup_restore 7/7, node UI/contracts 90/90 (was 89/89; +1 scoped-id
  unit test).
- Affected browser specs, one file per invocation, 23 tests passing:
  collider, collider-memory-context, collider-memory-acceptance (2),
  collider-project-context, collider-picker-reliability (5),
  collider-integrated-acceptance, collider-pipeline-overview,
  review-collider-anchors, collider-instance-anchors, transfer,
  transfer-identity, transfer-concurrency, forge, forge-review,
  forge-revision, forge-deliverable, backups, export.
- Strict Clippy `--workspace --all-targets -D warnings` clean, fmt clean,
  contracts `--check` clean (generated files regenerated for the
  restored-hash comment), TypeScript clean, docs validation PASS
  (173 Markdown files), API/web builds pass.
- Broadened once: full core `--no-fail-fast -- --test-threads=1`
  349 passed / 4 failed — exactly the known live-Ollama
  `semantic_workflow` environment failures (same four names as every
  prior round); reconciliation 345 (WP4) + 2 reviewer probes + 1 swapped
  test + 1 cycle unit test = 349. Providers+API 144/0.
- Not rerun: the full canonical e2e battery end to end — only the
  affected specs above, as in WP4. The real-semantic `search.spec.ts`
  environmental failure family is unchanged and was not re-proven here.

## Still open (Linux evidence does not close these)

Independent Windows rerun of this repair by ChatGPT (checklist in the
delivery MANIFEST); Windows verification of Slice 4; M7 native desktop
acceptance + preview promotion; M5 production gates; G1 audit; §32
execution sandbox. M8 is not complete: 3D, URL fetching and document
ingestion remain unfinished.
