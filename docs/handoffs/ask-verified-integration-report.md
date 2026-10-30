# Ask verified integration — report (2026-10-30)

Branch: `muse/ask-verified-integration`, from
`codex/review-ask-windows-findings` at
`532afccf9333522c3e15c97cd42105c71807bf35`, including the
integrator checkpoint `4ef90406d` and the incoming findings
repair `c15fe5d6`. The reviewed branch is preserved untouched;
this branch only adds the frozen quality criteria, the preview
integration checkpoint, this report, and the BUILD_STATUS entry.
No cloud calls, credentials, paid inference, downloads,
generated-code execution, pushes, master merges, remote CI, or
publication in this round.

## Baseline verification

- Package ZIP SHA-256
  `90aa53ac42523a93e3f679b8177130927c3b5598618c4c52029c4b8cc67497b3`,
  bundle SHA-256
  `7c6ec7c22e7481967cce2a1ea7111f572732e435e54d081676b45fcd07f6cc51`
  (matches the package manifest), exact tip fetch, and ancestry
  through `c15fe5d6` and `6e355b02`.
- Integrator change (`4ef90406d`, read in full): Windows
  verify-only root/catalog calls and operation batches now use
  the native owner/DACL verifier instead of PowerShell; the
  batch script path is removed; creation/repair still uses the
  script; a `process_spawns` counter records actual helper
  launches separately from logical calls. Shared
  (non-Windows) behavior is unchanged: the Linux code paths,
  queries, compilation, and evidence caps are identical to
  `c15fe5d6`. Windows-only additions are the native parity
  test, the bulk synthetic setup helper, and test-only setup
  reporting (total vs filler setup milliseconds).
- The review's Windows measurements are recorded, not repeated
  here: probe 27,637 → 6,299 → 2,264 ms; accounting at all
  sizes with zero helper spawns; realistic corpus 68/68 with
  p95 4,383.3 ms under the 5,000 ms bar. Those are Windows
  observations, not Linux claims.

## 1. Linux regressions (sequential, same battery as last round)

- Content unit tests 7/7; batch-proof integration 4/4.
- Permission accounting 1/1: identical counts to the previous
  round at 0/1/16/128/1010 notes (singles 8/12/42/266/2030,
  rechecks 0/2/17/129/1011), plus `process_spawns=0` at every
  size on Linux, as the new counter requires.
- Realistic corpus 2/2: 68/68 complete, 6/6 disjoint, 6/6
  natural, 80/80 disclosure, 80/80 budget, 0 leaks, 600
  additional sources, query p95 355.8 ms (Linux).
- Large-scale acceptance 2/2.
- Strict Clippy, formatting, contracts, TypeScript, docs
  validation, API and web builds: all clean. The web build
  still shows only the known chunk-size warning.
- Full core suite: 322 passed, 5 failed — IDENTICAL to the
  previous round. The 5 are the same known failures, none
  caused by this branch: 4 live-Ollama environment failures
  (embedding model absent on disk, downloads forbidden) and
  1 pre-existing benchmark A6 (partial-match vs empty
  expectation, proven on a pristine baseline last round).
  No new failure, no assertion weakened.
- Providers + API suites: 144 passed, 0 failed, matching the
  earlier round.
- Canonical browser Ask workflow: 1/1 (isolated lease
  harness, test data only — not the owner's preview).

## 2. Offline audit of the Windows evidence: reproduced semantically, not byte-identically

Correction (2026-10-30, A6 round): an earlier delivery
message described this reproduction as "byte-identical".
That wording was wrong. Ran
`scripts/audit-realistic-evidence.py` on the package's
`windows-evidence/realistic-evidence.json` (SHA-256
`71ae70c2511d92231a10c0404a55e55b52fad18c15bfaa64ceda14bb50a82528`).
The output matches the supplied independent audit in every
field (all 15 values identical), but the two files differ
in formatting: the reproduced file uses LF line endings
(515 bytes) while the supplied file uses CRLF (530 bytes;
`cmp` first differs at char 2). The results match
semantically; the files are not byte-identical. Values:
80 cases, 68 complete, 80 bound facts,
6 empty disjoint, 6 natural negative context checks, max
evidence 1,554 bytes, combined prompt bytes 121,280, 600
additional source appearances, mean 4,124.0 ms, p95
4,383.2552 ms (nearest-rank convention), max 4,598.641 ms,
0 cloud calls. The superseded audit differs only in p95
(4,399.1372 ms, next order statistic); raw timings and all
other fields are unchanged, confirming the review's
convention note. This audit checks evidence structure,
bindings, budgets, disclosure, and canary exclusion only; it
does not score model answers or tokens.

## 3. Frozen cloud experiment: inspected, unchanged, unmeasured

- Fixture SHA-256 `738ecc96…` matches; freeze manifest and
  the sampled payload are byte-identical to the earlier
  frozen copies. Settings unchanged: model
  `nvidia/nemotron-3-ultra-550b-a55b:free`, temperature 0,
  max_tokens 4096, no fallbacks, usage included, 10 batches ×
  10 cases × 3 approaches = 30 payloads, cap 30 scored + 1
  smoke, 0 attempts made, credential field records absence
  only (no secret present).
- No cloud attempt was made or implied. Earlier prompt-byte
  and estimate-based comparisons are NOT attributed to the
  repaired app; only actual runtime token usage from a real
  run counts, and none exists yet.

## 4. Local quality comparison: blocked plainly, criteria frozen

- No suitable local model is installed: Ollama's blob store
  is empty, no `.gguf` file exists on disk, no Hugging Face
  cache, no llama.cpp runtime. Downloads and cloud inference
  are forbidden, so no comparison ran and no quality or token
  number is claimed from this round.
- `docs/handoffs/ask-local-quality-acceptance.md` freezes the
  experiment before any measurement: same frozen prompts,
  same scorer (`scripts/score-large-benchmark.py`,
  unmodified), same settings, model-selection rule
  (strongest installed model fitting memory with room for
  OS/runtime/harness), separate scoring of factual
  correctness, source-supported citations, and
  insufficient-evidence responses, actual-vs-estimated token
  separation, every-case reporting, no tuning on scored
  questions, and fresh held-out cases after any repair.
- The scorer's rules are recorded for the future run:
  exact-value facts, both-values conflicts,
  `INSUFFICIENT_EVIDENCE` with no citations for unanswerable
  cases, citation validity plus required-memory coverage plus
  fact-span visibility for grounded correctness, and
  malformed/truncated/non-stop outputs counted as failures,
  never silently dropped.

## 5. Preview integration checkpoint (isolated, not launched)

- New launcher `Start ANANSI Preview Ask Verified.cmd` and
  handoff `docs/handoffs/ask-preview-integration-checkpoint.md`
  model the existing preview pattern: separate checkout,
  separate build, fresh data root
  `%LOCALAPPDATA%\ANANSI-Preview\ask-verified`, credential
  environment cleared, refuse-to-start if unbuilt.
- The owner's running preview, existing preview branches,
  and existing launchers are untouched. Nothing was launched
  beyond the isolated lease-harness browser test above.
- No Windows/native acceptance, preview promotion, master
  merge, remote CI, production gate, G1 audit, or
  generated-execution certification is claimed from Linux.

## Plain-English verdict

- Did answer quality hold? UNMEASURED. Retrieval
  completeness holds on both platforms (Linux 68/68 and
  Windows 68/68 with 80/80 bindings), but model-answer
  quality has not been measured with any model: the cloud
  round awaits a credential and the local round awaits an
  installed model. No quality recovery is claimed.
- How many actual tokens were saved? NONE MEASURED. Actual
  input, output, reasoning, and total tokens exist for
  neither the cloud nor the local comparison. Prompt-byte
  reductions (e.g. 175,658 → 158,999 on the fixture, and
  121,280 combined Windows prompt bytes) are text sizes,
  not token savings.
- What remains unmeasured or broken? (a) The local paired
  comparison (needs an installed model); (b) the frozen
  cloud round (needs the credential through the secure
  flow); (c) native desktop acceptance and preview
  promotion; (d) M5 production gates, §32 sandbox, G1
  audit, remote CI; (e) the 4 live-Ollama environment
  failures and the pre-existing benchmark A6 invariant,
  both recorded, neither caused by this branch.
- What should the Windows reviewer test next? (1) Build
  and launch this checkpoint in isolation with the new
  launcher and fresh data root; run the preview acceptance
  steps in the checkpoint handoff. (2) Re-run refusal,
  accounting, and strict Clippy on the integrated tree
  (already green on `4ef90406d`; confirm on this branch).
  (3) When a local model is installed, run the frozen
  local quality comparison exactly as specified — no
  prompt, setting, or model substitution. (4) Decide the
  A6 invariant (partial retrieval vs genuinely empty) as
  an owner decision; do not silently edit the benchmark.
