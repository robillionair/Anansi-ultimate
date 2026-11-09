# SOMNIA automatic memory compaction — design + acceptance criteria

## §0 Baseline, scope, and checkpoints

- **Baseline (verified, no substitute):** `muse/swarmtank-v2` at
  `bb1cc6e6dc779499105c27db70666cb2605f3e13` (SwarmTank v2 delivery:
  design + three checkpoints + calibration plan). The tree was clean
  at branch creation. Antigravity is reviewing Windows integration
  of that baseline in parallel; nothing in this slice touches the
  SwarmTank v2 code.
- **Branch:** `muse/somnia-compaction`.
- **What this slice is:** the compaction **library and tests** only —
  a bounded, deterministic compactor with source provenance, plus a
  frozen randomized benchmark and a separate oracle. No UI, no new
  API operation, no new artifact type, no contract change, no
  provider call. The library is pure Rust: no I/O, no clock, no
  model, no network.
- **What waits:** UI and provider-path integration, and the benchmark
  run through the actual SOMNIA application workflow. Both wait for
  Antigravity's accepted checkpoint, which includes newer SOMNIA work
  unavailable in this baseline. That integration is a separate
  checkpoint (see §8), not part of this delivery.
- **Priority rule (owner instruction):** any Windows review findings
  from Antigravity outrank expanding scope. If findings arrive
  mid-slice, this work pauses and the findings are repaired first.
- **Reviewable checkpoints for this component:**
  1. Design + acceptance criteria (this document, committed before
     code).
  2. Compaction library + unit tests.
  3. Frozen benchmark + oracle run at library level.
  4. (Deferred, §8) Integration: UI + provider path + app-workflow
     benchmark run.

## §1 The missing workflow

Today SOMNIA can replay synthetic scenarios against a reference
evaluator (rules R1–R5, §46), generate nightmares (§45), derive
inert consolidation proposals (§47), aggregate a dashboard (§50),
and build Forge QA scenarios (§51). What it cannot do is take a
history of records and prepare a **smaller history that is still
safe to reason from**. That is the missing workflow this component
serves, end to end once integrated:

```
import synthetic or explicitly approved records
  → prepare a compacted candidate
  → inspect retained and excluded material
  → approve
  → compare both histories with the same configured model
  → save the complete experiment
```

This slice builds the middle step — prepare the candidate — as a
library, and the benchmark that will later judge it through the
application. The import, inspection, approval, model comparison,
and save steps are integration work (§8).

Relationship to §47: the existing consolidation proposals are
**inert derivations over a saved evaluation doc** (they display,
never apply). The compaction candidate is a different object: a
concrete reduced history, produced from imported records, carrying
its own provenance and content hash, inspectable before any
inference runs. The two must not be conflated in code, docs, or UI
copy. In particular, compaction never promotes a lesson (§48) and
never schedules anything (§49); those remain separate, deferred
capabilities.

## §2 Supported input formats (explicit)

Only two input formats are supported. Anything else is refused,
not guessed at.

### Format A — structured JSON

Top level (deny unknown fields, contract-ready serde):

```json
{
  "format": "anansi-somnia-compaction-input",
  "version": 1,
  "records": [ { "id": "...", "key": "...", "source": "...",
                 "revision": 3, "kind": "fact",
                 "trust": "trusted", "privacy": "standard",
                 "text": "...", "supersedes": ["..."] } ]
}
```

Record fields:

- `id`: 1–64 chars, unique within the input. The stable handle
  every reason and every oracle check names.
- `key`: 1–128 chars. The fact or procedure the record concerns
  (e.g. `harbor.customs-fee`). Keys match **exactly**; see §3.7.
- `source`: 1–200 chars. The **exact source identity**, kept
  verbatim — never normalized, lower-cased, trimmed of meaning, or
  merged across sources. Two records from different sources are
  different evidence even when their text agrees.
- `revision`: unsigned integer. Monotonically increasing per
  (key, source) in well-formed histories; the compactor does not
  assume it — it orders by it.
- `kind`: `fact | instruction | uncertainty | deletion`.
- `trust`: `trusted | unverified`.
- `privacy`: `standard | private | public`. `sealed` is refused
  fail-closed wherever it appears, mirroring `somnia.rs`.
- `text`: 1–2000 chars, the record content as data.
- `supersedes`: optional list of record ids (same key and source,
  lower revision) this record explicitly replaces. May be empty or
  absent. See §3.3 for why supersession is explicit-only.

### Format B — text records

One record per line. Blank lines are skipped; lines starting with
`#` are comments and skipped. Every other line must match:

```
ID=<id> | SRC=<source> | REV=<n> | KEY=<key> | KIND=<kind>
  | TRUST=<trusted|unverified> | [PRIVACY=<standard|private|public> |]
  [SUPERSEDES=<id>,<id> |] TEXT=<text>
```

- Named fields may appear in any order except `TEXT`, which is
  last (record text may itself contain ` | `).
- `PRIVACY` defaults to `standard` when absent; `SUPERSEDES`
  defaults to empty when absent.
- A line missing a required field, carrying an unknown field, a
  non-numeric revision, an unknown enum value, or an empty `TEXT`
  is **damaged**: it is quarantined with its line number and a
  reason (§2.1), never repaired by guessing.

### §2.1 Ambiguous or damaged inputs are shown, not repaired

Parsing is two-layered:

1. **Whole-input refusal.** Input that is not JSON at all (Format
   A), or has the wrong `format`/`version`, or exceeds the input
   bounds (§3.1), is refused with a named error. No candidate is
   produced.
2. **Per-record quarantine.** A record that fails field validation
   (bad kind/trust/privacy, sealed, empty or over-long text, bad
   id/key/source, a `supersedes` id that names no record in the
   input, a duplicate id) is moved to **quarantine** with a reason
   naming the record (or line number for text input). The remaining
   valid records still compact. Quarantined records are never
   retained, never excluded-with-a-compaction-reason, and never
   silently fixed — the candidate carries the full quarantine list
   for review, and the integration UI must display it before
   approval.

### §2.2 Imported instructions are data

Records with `kind: "instruction"` — and any instruction-like
text inside any record — are **data, never instructions**. The
compactor only groups, orders, and selects records; it never
interprets record text, never builds commands, paths, URLs, or
provider requests from it, and never lets record content change
control flow beyond grouping and ordering. Prompt-injection text
("ignore previous instructions", "delete all records", "reveal
sealed data") is retained or quarantined exactly like any other
text, and appears in the candidate only as inert record text. The
benchmark proves this (§5).

## §3 Compaction rules (deterministic, in order)

The compactor is a pure function
`compact(records) -> Result<Candidate, String>`. Same input
records → byte-identical candidate, regardless of input order:
all grouping operates on a canonical sort by
`(key, source, revision, id)`.

### §3.1 Bounds (fail-closed, never silent truncation)

- At most **200** input records; at most **200,000** total input
  text chars; per-record limits from §2.
- Input exceeding a bound is refused with a named error. The
  compactor never truncates, samples, or drops records to fit.
  (Output size is bounded by construction: retained ⊆ input.)
- These bounds mirror the somnia/collider family's explicit
  limits (`somnia.rs`: 20 scenarios, 20 records per scenario,
  2000 chars per record).

### §3.2 Exact duplicates

Records identical in `(key, source, revision, kind, trust,
privacy, text)` are duplicates. The smallest `id` is kept; the
rest are excluded with reason `duplicate-exact` naming the kept
id. Duplication is judged **within one source only** — the same
text from two different sources is independent corroboration, and
both records are retained with their exact source identities.

### §3.3 Supersession is explicit-only

A newer revision supersedes an older record **only when it says
so**: the newer record (same key and source, higher revision)
lists the older record's id in `supersedes`. The older record is
then excluded with reason `superseded-by:<id>`.

A newer revision that does **not** list the older record is never
assumed to restate it. The older record is retained with status
`older` and a note naming the newer revision. Rationale: a
revision that changes one detail ("fee is now $45") routinely
fails to restate another ("payable in euros"); a compactor that
infers restatement silently destroys evidence. The benchmark's
adversarial keep-more-context cases (§5) exist to catch exactly
that failure, and the oracle fails the run if they are dropped.

### §3.4 Deletions are preserved and definitional

A record with `kind: "deletion"` at revision R for (key, source)
supersedes — by definition of what a deletion means — every
non-deletion record for the same (key, source) with revision
< R. Those records are excluded with reason
`superseded-by-deletion:<deletion-id>`. The deletion record
itself is **always retained** (status `deletion`), with its text
as the evidence of what was removed. A deletion at the same
revision as a fact for the same (key, source) is a conflict
(§3.5), not a supersession — both are retained.

### §3.5 Conflicts at the same revision are all preserved

Records sharing `(key, revision)` with differing text are a
conflict, whether from one source or several. **All** of them are
retained with status `conflict` and a flag naming the conflicting
record ids. The compactor never picks a winner, never merges
conflicting texts, and never averages them. A single record with
no same-revision disagreement is not a conflict.

### §3.6 Uncertainty is always retained

Records with `kind: "uncertainty"` are always retained (status
`uncertainty`), at any revision, from any source. Uncertainty
qualifies the facts around it; compacting it away would present
a confident history the evidence does not support.

### §3.7 Look-alike entities are never merged

Keys match exactly, byte for byte. Near-identical keys
(`harbor` vs `harbour`, `Müller` vs `Muller`, `Café` vs `Cafe`,
differing only by case, accent, or a confusable character) are
different keys; their records are never merged, never deduplicated
across each other, and never treated as the same entity. Unicode
names are compared as given — no normalization form is applied
to keys, sources, or text anywhere in the compactor.

### §3.8 Unverified material stays separate from trusted evidence

The candidate's retained section is split: `trusted` and
`unverified` are separate lists, and a record's trust value is
carried verbatim on the record. A trusted record never absorbs
an unverified one, and an unverified record is never promoted by
proximity to trusted records. The split is structural, not a
label the UI has to remember to apply.

### §3.9 Every exclusion carries a reason

Excluded records are enumerated, each with its id, a
machine-readable reason, and human-readable detail:

- `duplicate-exact` — names the kept record id.
- `superseded-by:<id>` — explicit supersession (§3.3).
- `superseded-by-deletion:<id>` — definitional deletion (§3.4).

There is no other exclusion reason. If a record is neither
retained nor excluded under one of these reasons, compaction is
buggy — the unit tests assert retained ∪ excluded ∪ quarantined
covers every input record exactly once.

### §3.10 Original history is retained

Compaction selects; it never rewrites. The candidate carries
`inputHash` (SHA-256 of the canonical input serialization) and
the full input record ids. The original history is preserved
alongside the candidate (at integration, the experiment artifact
stores input + candidate together); nothing in this slice deletes,
archives, or mutates an original record. The candidate is a
derivation, not a replacement.

## §4 Candidate shape — inspectable before inference

```rust
SomniaCompactionCandidate {
    format: "anansi-somnia-compaction",
    version: 1,
    input_hash: String,        // SHA-256 of canonical input
    retained: RetainedSections {
        trusted: Vec<RetainedRecord>,
        unverified: Vec<RetainedRecord>,
        conflicts: Vec<RetainedRecord>,
        deletions: Vec<RetainedRecord>,
        uncertainties: Vec<RetainedRecord>,
    },
    excluded: Vec<ExcludedRecord>,   // every exclusion + reason (§3.9)
    quarantined: Vec<QuarantinedRecord>, // damaged input + reason (§2.1)
    stats: CompactionStats {
        input_records, retained_records, excluded_records,
        quarantined_records, input_chars, retained_chars,
    },
    content_hash: String,      // SHA-256 of canonical candidate, hash field empty
}
```

- A retained record appears in exactly one section, by
  precedence: deletion → conflict → uncertainty → trusted /
  unverified. The record itself carries its `status`
  (`current | older | conflict | deletion | uncertainty`) and
  its flags, so sectioning loses no information.
- `content_hash` follows the `somnia.rs` evaluation-doc pattern:
  canonical serialization with the hash field empty, SHA-256.
  `validate_candidate` recomputes it and refuses any tampered
  candidate fail-closed.
- The candidate is complete **before** any inference runs: the
  integration UI renders retained sections, excluded-with-reason,
  and quarantine from this struct alone, and approval gates on
  it. No model call is needed to inspect a candidate, and none
  is made before approval.

## §5 Frozen benchmark (before evaluating)

The benchmark is frozen **before** any evaluation runs against
it, so the compactor cannot be tuned to the test set after the
fact. Freeze procedure: the generator (seeded, deterministic)
writes the cases + MANIFEST; the MANIFEST records the seed,
generator version, case count, and per-file SHA-256; the frozen
files are committed; a test asserts the MANIFEST hashes match the
files; re-running the generator into a scratch directory must
reproduce the files byte-identically.

### §5.1 Generator

- `crates/core/tests/fixtures/somnia-compaction/generate-benchmark.py`
  (stdlib only, following the `tests/fixtures/pdf/` generator
  precedent). Seeded PRNG (FNV-1a + xorshift32, the same integer
  arithmetic as the SOMNIA nightmare generator, so the sequence
  is reproducible without a model or a clock).
- Seed and case count are recorded in the MANIFEST and in the
  design handoff; changing either is a new benchmark version,
  never a silent edit.

### §5.2 Case categories (40 cases)

| Category | Cases | What it proves |
|---|---|---|
| updates | 6 | revision chains; explicit `supersedes` excludes the old record with reason; unlinked updates retain the older record |
| deletions | 5 | deletion record retained; earlier revisions excluded as `superseded-by-deletion`; same-revision deletion + fact is a conflict |
| conflicts | 5 | same-revision differing text from one or several sources: all retained, flagged, no winner |
| missing facts | 4 | the question's answer is absent from the history; the oracle expects abstention, and the compactor invents nothing |
| look-alike entities | 4 | near-identical keys / sources are never merged (harbor/harbour, Müller/Muller) |
| Unicode names | 4 | accented, CJK, and confusable names keep exact identities; no normalization |
| broken rows | 5 | malformed JSON, truncated text lines, bad revision, missing field, duplicate id → quarantine with reason, never repaired |
| prompt injection | 4 | injection text in records is inert data: retained/quarantined as text, never acted on, never an exclusion reason |
| keep-more-context (adversarial) | 3 | a newer revision that does not restate an older detail; the older record is the only carrier of a needed fact — dropping it fails the oracle |

Each case is one JSON file:

```json
{ "id": "case-001", "category": "updates",
  "records": [ /* Format A records */ ],
  "textInput": "…",               // Format B rendering of the same history, where applicable
  "questions": [ { "id": "q1", "text": "…",
                   "neededRecordIds": ["r2"],
                   "expectedAnswer": "…",
                   "abstain": false } ],
  "missingFact": null,            // for missing-facts cases: the string that must appear in NO retained record
  "forbiddenRetained": ["r9"] }   // record ids that must NOT be retained (broken / injected-acting)
```

### §5.3 The oracle (separate from the compactor)

The oracle is an independent checker that reads only the case
file and the candidate. It shares no code with the compactor
beyond the public candidate types, and it never reuses the
compactor's retention logic. Per case it checks:

1. Every `neededRecordIds` entry is in the candidate's retained
   ids (retention coverage — the evidence needed to answer is
   still there).
2. No `forbiddenRetained` id is retained.
3. For missing-facts cases: `abstain` is true and `missingFact`
   appears in no retained record text (nothing was invented).
4. For injection cases: injected text appears only as inert
   record text; no exclusion reason and no candidate field
   contains it as an instruction, and no record was excluded
   because of its content.
5. Retained ∪ excluded ∪ quarantined covers every input record
   exactly once; every excluded record has a reason.

The oracle verdict per case is pass/fail with the failing check
named. **Every case stays in the results** — failures are
listed, never dropped from the denominator.

### §5.4 What is recorded per case (and what is not yet measured)

Library-level run (this slice): compaction time per case (ms),
retained evidence ids, oracle pass/fail per check, failures with
reasons, total latency. Provider-reported input/output tokens:
**not measured** at library level — the results file records
`"not-measured"`, never a zero, following the SOMNIA precedent
that unmeasured model metrics are never zero-filled. Answer
accuracy at this level is oracle coverage (§5.3), not model
answers: whether a model answers correctly from the compacted
history is measured only by the app-workflow run after
integration (§8).

### §5.5 Benchmark runner

`crates/core/tests/somnia_compaction_benchmark.rs`: loads the
frozen fixtures via `include_str!` (the `vault-argon2-reference`
precedent — no working-directory dependence), runs `compact` +
oracle per case, and fails if any case fails. A companion run
writes the per-case results JSON (all 40 cases, timings, retained
ids, failures) to the delivery evidence directory; the results
file is evidence, not a test artifact that rewrites the fixtures.

## §6 Acceptance criteria

- **AC-C1** Design + acceptance criteria committed before code
  (this document, first commit on the branch).
- **AC-C2** Determinism: the same input records produce a
  byte-identical candidate (identical `content_hash`) across
  repeated runs and across shuffled input order.
- **AC-C3** Bounds fail closed: over-limit input is refused with
  a named error; nothing is truncated, sampled, or silently
  dropped to fit.
- **AC-C4** Newest valid revision is always retained and marked
  `current`; an explicitly superseded older record is excluded
  with reason `superseded-by:<id>`.
- **AC-C5** Deletion records are always retained; earlier
  same-(key, source) revisions are excluded as
  `superseded-by-deletion:<id>`; a same-revision deletion + fact
  is a retained conflict.
- **AC-C6** Same-revision conflicts are all retained, flagged
  with the conflicting ids; no winner is chosen.
- **AC-C7** Uncertainty records are always retained.
- **AC-C8** Exact source identities are preserved verbatim;
  look-alike keys are never merged; no normalization is applied
  to keys, sources, or text.
- **AC-C9** Unverified material is structurally segregated from
  trusted evidence in the candidate and never merged into it.
- **AC-C10** Every excluded record carries a reason; every
  quarantined input carries a reason; damaged input is never
  repaired by guessing.
- **AC-C11** Imported instructions are data: injection cases
  pass the oracle — injected text is inert, never an action.
- **AC-C12** The candidate is inspectable before inference
  (retained sections, excluded-with-reason, quarantine, stats)
  and its `content_hash` validates; tampering is refused.
- **AC-C13** Benchmark frozen: generator + 40 cases + MANIFEST
  committed; MANIFEST hashes match the files; regeneration
  reproduces them byte-identically.
- **AC-C14** Oracle is independent of the compactor and checks
  needed-id coverage per case; every case appears in the
  results, failures included.
- **AC-C15** Library-only slice: no new API operation, no new
  artifact type, no contract change, no UI change, no provider
  call; existing saved runs are untouched (SOMNIA + SwarmTank
  regressions green).
- **AC-C16** (Deferred to integration, §8 — recorded as pending,
  not claimed): app-workflow benchmark run recording compaction
  time, retained evidence, answer accuracy, abstentions,
  provider-reported input/output tokens, failures, and total
  latency; real-model verification only with an available
  authorized provider; synthetic-provider runs labelled.

## §7 Verification plan for this slice

- Rust workspace lib suite (new module tests + benchmark
  oracle test), strict Clippy (pre-existing warnings only),
  `cargo fmt --check` on touched files.
- `tsc --noEmit` 0, contracts `--check` 0 drift, docs validator
  PASS, UI tests unchanged count (no UI touched).
- Focused regression: SOMNIA e2e spec + SwarmTank market/AI
  specs (library is unreferenced by any op, but the crate
  changed, so the neighbours are re-run).
- Benchmark evidence: per-case results JSON (40/40 cases,
  timings, oracle verdicts) + generator determinism check
  (regenerate to scratch, byte-identical diff).
- Packaging per the standing delivery rule: verified Git bundle
  inside a ZIP with SHA-256 checksums, MANIFEST + evidence logs,
  scratch verification, copy hash-verified at `/mnt/data/`.

## §8 Integration slice (deferred — blocked on Antigravity's checkpoint)

After Antigravity's accepted checkpoint arrives (it includes
newer SOMNIA work unavailable in this baseline), a separate
slice wires the library in:

1. Import UI for the two formats (synthetic or explicitly
   approved records only; sealed refused; quarantine displayed
   for review).
2. Candidate inspection view (retained sections, excluded with
   reasons, quarantine) + explicit approval step. No inference
   before approval.
3. Provider path: compare both histories (original vs compacted)
   with the **same configured model** through the existing
   provider layer, local-first default, Zero-Cost Lock checked up
   front, existing privacy/disclosure/authorization/cost gates.
   A failed provider call is visible in the experiment history.
4. Save the complete experiment as a new artifact type
   (input + candidate + both runs + metrics, fail-closed hash),
   reopenable without rerunning inference.
5. Run the frozen benchmark through the actual SOMNIA
   application workflow and record, per case: compaction time,
   retained evidence, answer accuracy, abstentions,
   provider-reported input/output tokens, failures, total
   latency. Keep every case in the results. Real-model
   verification depends on an available authorized provider;
   synthetic-provider tests remain labelled.
6. Windows review findings, if any, are repaired before any
   scope expansion.

## §9 Honest boundaries and proposed status updates

- This component prepares compacted candidates; it does not
  prove a model answers better from them, does not promote
  lessons (§48), does not schedule runs (§49), and does not
  change any saved evaluation or memory. Those claims wait for
  the integration slice and its benchmark run.
- Library-level "answer accuracy" is oracle retention coverage
  (§5.3), not model accuracy. No model was involved in this
  slice; no token counts exist to report.
- **Proposed status updates: none to AN-015.** Milestone 10
  stays `verified` / "Milestone 10 complete" as reviewed; this
  is new scope on top of the verified milestone (the SwarmTank v2
  precedent on verified M9), not a reopening of it. No new
  requirement row is proposed in this slice; the integration
  slice will propose one when it wires the workflow.
- **Not claimed:** provider-backed evaluation and historical
  ingestion remain deferred in AN-015; this slice implements
  neither — it implements the deterministic compactor and the
  frozen benchmark that will judge the integrated workflow.
