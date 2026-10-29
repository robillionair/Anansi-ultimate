# Local paired answer-quality acceptance — frozen criteria (2026-10-30)

Status: FROZEN, UNMEASURED. No local model is installed on this
machine (verified 2026-10-30: Ollama blob store empty, no `.gguf`
file on disk, no Hugging Face cache, no llama.cpp runtime), and
downloads and cloud inference are forbidden for this assignment.
This document freezes the experiment so a later run with an
installed model proceeds unchanged. Nothing here has been
measured, tuned, or scored.

## Experiment (frozen, reused unchanged)

- Questions and source material: the repaired fixture
  `repaired-cases.json` (SHA-256
  `738ecc96a2e11dd88f8a8a4f9a0c6e94b6d7bc1f9c8c3d01b46a77aecd1f836e`):
  100 questions, 90 answerable and 10 genuinely unanswerable.
- Approaches: the three frozen prompts per batch (ANANSI, basic,
  full) from the frozen cloud round — 30 payloads (10 batches ×
  3 approaches), byte-identical to the frozen copies. Prompts,
  rubric, and settings are NOT changed for the local run.
- One request per payload: the payload's single user message is
  sent as-is to the local model. Generation settings match the
  frozen round exactly: temperature 0, max_tokens 4096, no
  fallback, no retry on anomaly.
- Model selection (when a model becomes available): the
  strongest already-installed model that fits available memory
  with room for the OS, the runtime, and the harness (on this
  8 GiB VM, a model needing more than ~4 GiB of resident memory
  is unsuitable). Record the model name, quantization, size, and
  why it was selected. The same model and settings serve every
  approach; approach is the only variable.

## Scoring (frozen, reused unchanged)

Scorer: `scripts/score-large-benchmark.py`, unmodified.
Per answer it reports, separately:

- `fact_correct`: exact requested value (whitespace/case
  normalized); conflicts require both values pipe-separated;
  insufficient cases require `insufficient=true` with text
  exactly `INSUFFICIENT_EVIDENCE`.
- `grounded_correct`: fact correct AND all citation indices
  valid AND every required supporting memory cited AND every
  exact fact span visible in the cited texts.
- `citation_indices_valid` and `missing_or_malformed`
  (non-JSON, empty answer, or bad citation list).
- Insufficient-evidence handling is scored on its own: a
  correct `INSUFFICIENT_EVIDENCE` with no citations counts;
  an answer on an unanswerable case counts as wrong.

The runner must emit scorer-compatible records for every
request: approach, tag, parsed answers, usage, and finish
reason. A request that fails to parse, truncates, or ends
without `stop` is recorded as malformed/failed, never silently
dropped, and excluded from scoring only by the scorer's own
existing rule (`valid_for_accounting` and `finish_reason`).

## Usage reporting (frozen)

- Report actual input, completion, reasoning (when the runtime
  supplies it), and total tokens per approach, summed over the
  30 requests. Mark any field the runtime does not supply as
  unavailable; never estimate it into the actuals.
- Keep three things separate: prompt bytes (measured text
  size), estimated tokens (any tokenizer estimate), and actual
  runtime token usage. Previous token savings from prompt-size
  or estimate comparisons must NOT be attributed to the
  repaired app; only this run's actuals count.
- Report the reasoning-tokens anomaly rule from the scorer
  (reasoning exceeding completion is flagged, not hidden).
- Cost is zero for a local run; record that, not a cloud price.

## Acceptance and integrity rules

- Every case, failure, truncation, and unfavorable result is
  reported, including cases where the repaired approach scores
  worse than a baseline.
- No production code is tuned on the scored questions, and no
  prompt, fixture, or rubric change is made to improve a
  headline. If a real defect appears, preserve the failing
  evidence, make a bounded repair, and evaluate the repair on
  fresh held-out cases, never by re-scoring the same cases.
- Bars (for the future run, not claims today): quality holds
  if ANANSI's grounded-correct count is at least the basic
  approach's on the same questions with the same model, and
  insufficient-evidence handling does not regress; token
  savings are whatever the actuals show, favorable or not.
- Windows/native acceptance, preview promotion, and any
  production gate remain open and are not established by this
  local run.

## Dependency status (2026-10-30)

BLOCKED on a local model: none installed, downloads
forbidden, cloud forbidden. The frozen cloud round remains
unmeasured for the same reason (no credential in session).
Either dependency unblocks its own run; neither may be
substituted with a different model, prompt set, or setting.
