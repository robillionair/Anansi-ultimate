# M8 benchmark — independent source, evidence and Windows review

Incoming `8639c44c30eb98848961dfd31fe1057e769d53e4`, baseline
`df8072595e672ead29ad0df82393c31de7b6f7d8` confirmed as ancestor. ZIP and
sidecar match SHA-256 `e805800e3b3a6a3bb7559fd788b887f2da9213255b94e7d8e51a4f626b2727e7`;
bundle matches `f43be9960a5a7825cecf4c637800f170d8719da5f37e2904f67bdafd22c6e3d5`,
complete history verified. Review branch `codex/review-m8-benchmark` in
`E:/Anansi/worktrees/review-m8-benchmark`. Preview and personal data untouched.

## Acceptance plan

Independently sum raw provider records, check response identity/finish/usage and
answers against their actual numbered source entries, inspect before/after replay
records and production repair diffs, and validate reporting commands. Run focused
Windows migration, context, Ask/streaming and synthetic benchmark checks, UI/TS,
builds, strict Clippy/contracts, formatting/docs and the browser Ask workflow under
the shared lease. No cloud calls, model downloads or new measurement rounds.

## Findings established before local reporting corrections

- Raw Nemotron records: 32 distinct generation ids, 8 unique answerable questions,
  two runs per approach. Model ids match, all finish `stop`, recorded cost/cache
  counters zero. Recomputed totals: ANANSI 9,802 input + 2,336 output = 12,138;
  full-body baseline 15,584 + 1,990 = 17,574. Input reduction 37.1022%, total
  reduction 30.9321%. This verifies the supplied records, not a new provider run.
- Read all 32 visible answers and checked cited source entries: rubric facts and
  substantive cited claims are supported, including A3's two-source arithmetic,
  A5's unresolved survey conflict and A13's logs 0–39 in entries 1–4. Minor A13
  "remaining records" wording is imprecise as noted by the original audit.
- Concentration: A2 alone saves 3,831 of the 5,436 tokens saved (70.4746%). Removing
  A2 leaves ANANSI 9,817 versus baseline 11,422, a 14.0518% reduction. The report's
  assertion that one example does not dominate is unsupported. Show the sensitivity
  calculation; the overall 30.9% arithmetic remains correct.
- Linux final log reports 329 passes, but it includes API and contracts targets:
  305 core + 24 API, not 329 core tests. There are 59 executed test binaries and
  two zero-test doctest summaries. Earlier 300 was core-only; the 29 increase is
  five additional core tests plus 24 API tests, not 29 additional core tests.
- The report CLI still prints a naive tokens-per-correct figure using ANANSI's
  correct count and divides replay tokens by original-run harness grades, despite
  replay answers being separate samples. Neither denominator is valid without
  independent grades. Harness substring checks are diagnostic proxies, not the
  manual rubric. Suppress unsupported per-correct numbers rather than invent them.
- The documented `--replay` invocation fails on the supplied JSONL: option values
  are accidentally included among input JSON documents (`JSONDecodeError: Extra
  data`). Fix argument parsing, preserve raw data and add focused CLI regressions.

Acceptance for the bounded CLI correction: the documented command must work;
input/replay measurements remain separate; unsupported quality denominators stay
unavailable; no raw file changes. Add report sensitivity and correct test scopes.
Production repairs were inspected; no new production change is planned.

## Provider accounting qualification

Eight of 32 raw responses report reasoning tokens greater than completion tokens.
Subtracting them produces negative visible-output counts for those responses.
Do not clamp, add the counters together, or infer corrected totals. The supplied
total counters equal input plus completion and reproduce the 30.9321% reduction,
but the reasoning breakdown is unresolved. Describe these as API-reported totals,
not independently measured computation or cost savings. No new cloud call was made.
The 37.1022% API-reported input reduction is unaffected by that subtraction.
Excluding A2, input is 7,746 versus 9,606: 19.3629% lower.

## Bounded reporting repair

The CLI now parses `--replay` correctly and withholds per-correct-answer figures
where independent grades are unavailable. Harness substring checks are labelled
diagnostic proxies. Two focused CLI regressions failed before and pass after.
The documented replay command now succeeds on the original JSONL; raw measurements
were not edited. Offline audit deliberately exits 1 for the eight counter anomalies.
This is a qualified measurement review, not an all-clear for every headline.

## Focused Windows verification

Context compiler 10/10; workspace roundtrip 11/11 including the 20×16 barrier
migration regression; Ask 5/5; streaming and backend cancellation 1/1;
canonical synthetic Ask browser workflow 1/1 under the shared application lease;
UI/contracts 81/81; reporting CLI regressions 2/2. TypeScript, web/API builds,
contracts drift, strict core all-target Clippy, formatting, documentation validation
and Git diff checks passed. The larger synthetic harness is still pending.

No full Windows suite, independent real-model replication, native acceptance or
preview promotion is claimed. Prior production gates remain open. Source repairs
were inspected and no application code was changed by this review. Public wording
is limited to the small synthetic pilot and API-reported input reduction.
