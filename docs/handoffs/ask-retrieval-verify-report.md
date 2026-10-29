# Ask retrieval verification — large-scale retrieval passes, model replay held

Branch: `muse/ask-retrieval-verify`, from delivery tip
`95d384e3e24dfc53aeb73aa94a08891f7e9d101d` (implementation
`24d3df5c7cf7a90f849f7d16c08c9fe4fd3841dd`, parent `737d43691`).
Content commits on this branch: acceptance criteria `23d7a5cd`,
large-scale test `a257a1a7`, this report. The exact final tip,
bundle SHA-256, and ZIP SHA-256 are recorded in the delivery
`MANIFEST.json` and the delivery message. No production code changed
on this branch: only the criteria document, one new test, this report,
and a `BUILD_STATUS.md` entry. No push, merge, CI, preview promotion,
publication, or generated-code execution.

## Part 1 — Model replay: HELD, not failed (0/31 attempts spent)

The repaired frozen fixture was verified before any other work:
SHA-256 `738ecc96a2e11dd88f8a8a4f9a0c6e94b6d7bc1f9c8c3d01b46a77aecd1f836e`,
matching the repair delivery manifest and review. Offline coverage
recomputed from that file: ANANSI 90/90 answerable cases with all
required fact spans, 158,999 prompt characters, 10 zero-evidence
cases; basic retrieval 60/90, 196,512 characters; full context 90/90,
947,475 characters. These are retrieval and byte measurements, not
model scores or provider tokens.

Thirty scored payloads plus the smoke payload were frozen with the
existing runner's exact construction (temperature 0, max_tokens 4096,
`allow_fallbacks: false`, usage included, approach order rotated per
batch, ten cases per batch): 1,400,876 payload characters in total.
Frozen payloads contain no canaries, keys, or gold answers (checked).
The runner's own guards stay in force for any later run: free-pricing
check before inference, fixture re-hash before every request, 30-call
cap, stop with no retry on any failed, ambiguous, nonzero-cost, or
wrong-model response.

Public model check on 2026-10-29: `nvidia/nemotron-3-ultra-550b-a55b:free`
found, prompt/completion pricing both `0`, context length 1,000,000.
The cloud run did not proceed because `OPENROUTER_API_KEY` is absent
in this session: no request was sent, no cost was incurred, and no
current allowance figure can be stated (the key endpoint needs the
key; the prior run's postflight 31/50 is historical, not today's
allowance). What unblocks it: the owner supplies the credential
through the secure flow (never pasted into chat, code, logs, or
reports); the frozen smoke plus 30 scored requests can then run
unchanged, with no re-freeze and no production change while the
measurement runs.

Original results stay historical and are not overwritten: ANANSI
60/100 supported correct outcomes, basic 70/100, full 100/100, on the
old prompts. Prompt bytes are not provider tokens: repaired ANANSI
prompts are 9.48% smaller in bytes than the old ANANSI prompts
(158,999 vs 175,658), but no provider-token saving on repaired prompts
has been measured in this round.

## Part 2 — Larger retrieval acceptance: PASSED, no production change

Criteria were committed before implementation
(`docs/handoffs/ask-large-scale-acceptance.md`, `23d7a5cd`). The test
(`crates/core/tests/ask_large_scale_acceptance.rs`) seeds one shared
workspace with 1,007 records and asks 48 fresh questions through the
real `Core::ask_anansi` path with a refused loopback endpoint (local
only, no inference): plain 8, reworded 8, multi-source 6, fact
position 6 (early/middle/late in long notes), current-revision 4,
conflict 4, archived/expired-only 4, missing-evidence 4, near-match
disambiguation 4. No original ten templates, Harbor entities, or
copied question shapes were reused. Sealed and foreign-workspace
canary notes trap boundary leaks.

Final run (after formatting, rerun clean):

| Check | Result | Bar |
|---|---:|---:|
| Answerable evidence completeness | 40/40 | ≥36/40 |
| Insufficient-evidence correctness | 8/8 | 8/8 |
| Disclosure equality (cited = disclosed) | 48/48 | 48/48 |
| Budget (prompts under 5,500 bytes) | 48/48 | 48/48 |
| Sealed/foreign canary leaks | 0 | 0 |
| Archived/expired citations in any prompt | 0 | 0 |
| Exact Search still requires every term | 2/2 | 2/2 |
| Ask latency mean / p95 / max | 291.5 / 342.7 / 542.0 ms | p95 < 5,000 ms |
| Seeding | 1,007 records in ~18 s | — |
| Total prompt characters / cited sources | 57,265 / 146 | reported |

Two repair/retest cycles were used, both test-side:

1. Cycle 1 failed in setup, not in retrieval: creating a Sealed note
   through the normal API is refused fail-closed ("Sealed memory
   requires the future encrypted vault. No plaintext fallback was
   used."). That is designed vault behavior, not a defect. The test
   now seals notes by updating revision metadata after a private
   create, matching existing benchmark/collider test practice.
2. Cycle 2 exposed a probe-design flaw, not a retrieval defect: the
   first insufficient questions shared terms with eligible notes
   (Alder/Aldermere prefix, observatory, keeps/keep stemming, holds
   in filler), so OR retrieval legitimately returned partial-match
   evidence (Offline) instead of empty. The probes were redesigned to
   be term-disjoint from every eligible note, and a global assertion
   was added that no archived/expired id appears in any prompt's
   citations. Cycle 3 then passed every bar.

Findings carried forward, stated plainly:

- Irrelevant context is substantial: 96 of 146 citations go beyond
  the required sources, while budget and completeness still hold.
  OR recall trades precision for recall; the 4,000-byte cap, not
  precision, bounds the prompt.
- In a shared corpus, a natural-worded "insufficient" question
  returns partial-match evidence (Offline), and the model-facing
  abstention instruction — not zero-evidence status — is the guard.
  The 100-case fixture's 10/10 empty result depends on its
  insufficient questions being term-disjoint from their small
  corpora; it does not prove empty results for natural questions in
  large shared workspaces.
- No production defect was found at this scale, and no fixture
  dictionary, synonym list, or benchmark-specific answer was added.

## Part 3 — Final-tree verification (Linux only)

| Check | Result |
|---|---:|
| Focused Rust: lib + ask_anansi + ask_recall + ask_streaming + context_compiler + hybrid_retrieval + body_search + retrieval_evaluation + new large-scale test | 83/83 (82 repair set + 1 new) |
| Strict Clippy (`-p anansi-core --all-targets -- -D warnings`) | Pass |
| Formatting (`fmt --check`) | Pass |
| Contracts (`contracts.mjs --check`) | Pass |
| TypeScript (`tsc --noEmit`) | Pass |
| Documentation (`validate-docs.mjs`) | Pass, 148 Markdown files |
| API build (`-p anansi-api`) | Pass |
| Web build (vite) | Pass (chunk-size warning only) |
| Canonical browser Ask workflow (`ask_anansi.spec.ts`, `ANANSI_REAL_MODEL=0`): grounded citations, missing evidence, offline resilience, disclosure manifest | 1/1 passed |
| Cancellation | Covered at Rust streaming level (`ask_streaming` in the 83); not re-proven in the browser run |

Native Windows acceptance is explicitly pending for the Windows
reviewer. Linux results here do not close it.

## Verdict, in plain English

- **Did answer quality recover?** Not yet measured with a model.
  Retrieval evidence recovered (90/90 on the 100-case fixture) and
  holds at larger scale (40/40 on fresh questions), but model
  answers on the repaired fixture are unmeasured because the cloud
  run is held on the missing credential. Do not claim quality
  recovery until the bounded replay runs.
- **What actual token savings were measured?** None in this round:
  zero provider calls. The only new size figure is prompt bytes
  (−9.48% vs the old ANANSI prompts). The prior run's provider-token
  savings (79.69% input vs full, 8.20% vs basic) belong to the old
  prompts and must not be transferred to the repaired prompts.
- **What fails at larger scale?** Nothing failed against the frozen
  bars. Watch items: heavy irrelevant citation (96/146), partial-match
  evidence for natural insufficient questions, and the untested region
  beyond 1,007 records and the 4,000-byte cap.
- **What should the owner test next?** First, supply the credential
  through the secure flow and run the frozen smoke + 30 scored
  requests (no re-freeze, no code change during measurement). Second,
  Windows review of this checkpoint (focused Rust, browser Ask, and
  the new large-scale test on Windows). Third, fresh held-out
  questions in new domains — not these 48 — before any general claim.

## Integration handoff for the next isolated Windows preview

The suitable checkpoint is this branch, `muse/ask-retrieval-verify`,
at the final tip recorded in the delivery `MANIFEST.json`. It is
suitable as an isolated preview because it changes no production
behavior relative to `95d384e3` (docs plus one test only), carries
the repaired retrieval, the frozen Part 1 payloads, and the passing
large-scale acceptance test, and has a clean Linux verification
record above. The preview must still not be promoted or replace the
owner's running preview, and native Windows acceptance, M5 production
gates, the §32 sandbox, G1 audit, and remote CI remain open and
unchanged.
