# M6 Slice 4 addendum — project-scoped Opportunity Discovery, as built

Date: 2026-10-21 · Branch `muse/m6-slice4-opportunity-discovery`
over the integrated preview checkpoint `2665e22`. Design +
acceptance criteria: `M6-slice4-design.md` (written before
code). Owner-approved scope (2026-10-21): discovery uses
**only deliberately supplied information** — text the user
types or pastes, a supplied document excerpt, and explicitly
selected memories. No background memory scanning, no web
research, no connectors, no autonomous actions, no
inference on opening a screen.

## What shipped

- **Core** (`crates/core/src/opportunities.rs`, mirroring the
  Slice 2 idea engine on the same M5 substrate — no parallel
  provider/project system): `opportunities-generate-start /
  -poll / -cancel` and `opportunities-save` on the existing
  `ai_command` providers arm, behind the unchanged M5 gates
  (model resolution from trusted config, per-segment privacy
  classes, Zero-Cost Lock, paid-inference permits, explicit
  cloud authorization, bounded run registry with
  eviction-cancels-victim, claim-and-wait idempotent save).
  Supplied sources carry run-assigned ids (`supplied-1…n`)
  at privacy class `private`; each is framed on the wire as
  data to analyse, never instructions. Selected memories are
  sent exactly as in Slice 2 (own privacy class, source id =
  memory id) and their exact revision is recorded. At least
  one source is required; archived projects refuse; the
  context budget (16 sources / ~64k chars) matches Slice 2.
- **Grounding, enforced in the parser**: the output is the
  §21/§22 twelve-field concept shape plus per-opportunity
  `sourceReferences` and `assumptions`. References to ids
  that were not supplied are dropped; a set in which any
  opportunity ends up citing nothing supplied is refused as
  ungrounded; 5–10 bounds, field limits and truncation
  honesty are inherited from the strict Slice 2 parser. The
  system instruction forbids invented statistics, novelty
  and validation claims; the saved set records every source
  (supplied text verbatim, memory id + revision) and is
  labelled hypotheses, not research.
- **Save & evaluation**: one immutable artifact of the
  existing type `idea` whose body is
  `{format: "anansi-opportunity-set", formatVersion: 1,
  topic, note, sources, concepts}` — readable by Slice 3's
  `parse_idea_set`, so **Compare & evaluate works on saved
  opportunity sets with no copying and no new code**.
  Registry source references cite the selected memories.
  User assessments stay in evaluation artifacts, separate
  from the generated hypotheses, as before.
- **Restore identity**: backup/restore remaps the body's
  memory source ids (and concept citations to them) to the
  restored memories and recomputes the content hash — the
  same remap discipline as evaluations; `supplied-N`
  markers and supplied text pass through byte-identical; a
  memory source id missing from the restore map fails the
  restore closed rather than leaving a dangling reference.
- **API/contracts/client**: the four operations join the
  `/v1/ai/...` allowlist; contracts generated for
  `OpportunitySet` and the request/response shapes; client
  methods `startProjectOpportunities / poll / cancel /
  saveProjectOpportunities`.
- **UI** (Projects → "Opportunity discovery"): dynamic
  supplied-source rows (Problem statement / Customer
  complaint / Observation / Document excerpt, optional
  label), an explicit memory picker (sealed unselectable),
  optional focus, a **pre-send review listing exactly what
  will be sent**, Zero-Cost Lock, generate/cancel with the
  streamed provider disclosure, and results rendered with
  their supplied sources, per-opportunity citations and
  assumptions. `OpportunityRunController` mirrors
  `IdeaRunController`'s staleness discipline. Saved sets
  render in the artifact view with sources + citations and
  mount the existing evaluation panel unchanged.

## Verification (Linux, this machine, final tree `4e24f02`)

- Core: **208 passed / 4 failed** of 212 — the only failures
  are the 4 known environmental `semantic_workflow` tests
  requiring a live Ollama at 127.0.0.1:11434 (absent here).
  New: opportunities unit 5/5 (grounding, citation dropping,
  ungrounded refusal, remap incl. fail-closed),
  `opportunities_generation` 6/6 (happy path with wire
  capture + unselected-source exclusion + provenance,
  idempotent/concurrent save, restart reopen, Slice 3 parse
  compatibility; refusals incl. archived/foreign/no-source;
  provider-failure/malformed/ungrounded modes; cancel;
  cross-workspace isolation), `opportunities_restore` 1/1
  (backup → fresh restore remaps sources, hash verifies,
  restored set evaluates against the restored source).
  Regression suites on the restore change: artifacts 6/6,
  backup_artifacts 5/5, backup_restore 7/7, evaluations 3/3,
  ideas_generation 7/7, ideas_validation 4/4.
- Providers + API: **144 passed / 0 failed**. Node
  contracts + UI: **51/51** (incl. 4 new run-controller
  tests). `tsc` clean, contracts `--check` clean, web build
  ok, clippy `--all-targets -D warnings` clean (core,
  providers, api, contracts), fmt clean.
- Browser e2e, canonical subset run sequentially: **9/9** —
  app checkpoint, artifacts, ideas, evaluations, providers,
  vault, backups, integrated workflow, and the new
  `opportunities.spec.ts` (supplied problem + excerpt +
  approved memory → wire discipline asserted → citations/
  assumptions render → save → evaluate → backup → fresh
  restore → restart → restored set cites the *restored*
  memory and the restored evaluation resolves to the
  restored set; the original workspace is untouched).
- Two e2e locator repairs came with the slice (no assertion
  weakened): existing specs scope the idea form's context
  checkbox to its fieldset group — the new memory picker
  gives each memory a second, distinctly-grouped checkbox —
  and the new spec scopes its "Supplied sources" heading to
  the artifacts section, since results and detail
  legitimately show it at once.

## Not verified / still open

- **Windows verification of Slice 4 is pending** — all
  evidence above is Linux. The owner has confirmed the R16
  preview "looks good" and Windows `vault_increment2` 9/9;
  neither waives Slice 4's own Windows pass.
- M5 production gates remain open and unclaimed: Windows
  native acceptance (real CredUI + OS keyring), live-provider
  verification, G1 external audit, remote CI. Synthetic
  providers prove application behavior only.
- Real local-model discovery (e.g. a live Ollama) was not
  exercised: no model is installed on this machine and
  downloads are out of scope.

## Owner test guide (preview build)

1. Open a project → **Opportunity discovery**. Type a
   problem, add a source row as *Document excerpt* with a
   pasted paragraph, tick one memory under *Approved memory
   sources*. The review list shows exactly those three.
2. Leave Zero-Cost Lock on (with a local model configured)
   and choose **Discover opportunities**. Results cite the
   sources each opportunity derives from; *Explore* shows
   assumptions and required validation. The disclosure panel
   shows what was sent.
3. **Save opportunity set as artifact**, open it from the
   Artifacts list: sources render with the set, and
   **Compare & evaluate** scores it like any idea set.
4. Back the workspace up, restore into a fresh workspace,
   reopen the set: its memory source is the restored memory.
