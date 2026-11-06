# SwarmTank v2 — Calibration plan: comparing simulated decisions against held-out interviews or real pilot behaviour

## Status

**Not performed.** This is a plan for a future calibration round, not a
result. Until a calibration round has been run and reported, every
SwarmTank v2 outcome is a **conditional simulation result**, not validated
demand, conversion, or market evidence. Worldwide dropdown coverage (262
CLDR territories), researched regional profiles (3 starter profiles), and
real-world validation (this plan, not performed) are three separate
statuses.

## What calibration is (and is not)

Calibration compares the simulation's decisions against behaviour that was
**held out before the simulation ran** — real pilot interviews, real pilot
usage, or a real pilot's buying decisions — on the same product, market
and price. It measures agreement; it does not tune the simulation to
match the held-out set, and it never overwrites the measured version.

It is NOT: tuning agents on the held-out set, cherry-picking the interviews
that agree, presenting a small synthetic result as a universal saving, or
claiming a result against competing products.

## The six steps (design §6)

### 1. Hold out the real behaviour first

Before running the simulation, set aside the real pilot material that
will be the comparison set: interview transcripts, pilot usage logs, or
recorded buying decisions. The held-out set is frozen (hashed, dated) and
never used to choose personas, write agent goals, set prices, or pick
scenario events. Any material used to build the simulation is NOT in the
held-out set.

Minimum useful size: 5–10 interviews (or pilot accounts) per cohort
(segment × market). Below that, report the sample size prominently and
do not report per-cohort agreement at all — report only the overall
agreement with the sample size attached.

### 2. Run the simulation on the same product, market and price

Run the AI simulation (or Quick simulation, and say which) with the same
product description, price, market profile (or Custom assumptions, and say
which), and scheduled events as the pilot faced. Record the run's
assumptions, the profile version, the model identity, and the seed. The run
is the measured version; keep it.

### 3. Compare per cohort, per objection

For each cohort (segment × market) and for the top cited objections,
compare:

- **Simulated decision distribution** (share of recommend / defer /
  request_evidence / reject, or Convert / Confused / Reject for Quick)
  vs **held-out decision distribution** (share who bought / deferred /
  asked for evidence / declined, coded the same way).
- **Top cited objections**: the simulation's most-cited objections vs
  the objections actually raised in the held-out interviews, ranked.
- **Agreement per cohort and per objection**, with the sample size (n)
  for each. State disagreements plainly — where the simulation was more
  optimistic, more pessimistic, or cited objections nobody raised.

### 4. Report agreement with sample sizes

The calibration report states, for each cohort and each top objection:
the simulated share, the held-out share, the difference, and n. It states
plainly where the simulation disagreed. It does not hide a disagreement
behind an overall average. A cohort with n < 5 is reported as
"insufficient data", not as an agreement rate.

### 5. Any change following a calibration round is a NEW version

If the calibration suggests a change (a profile assumption, an agent goal,
a price-sensitivity range), the change is made as a **new profile version**
or a **new scenario version**. The old version is kept, with its measured
result. Never overwrite the measured version, and never re-run the
calibration against the same held-out set after tuning on it — that set is
spent. A new calibration round needs a fresh held-out set.

### 6. Never present as validated until a calibration round has been run

Before a calibration round has been run and reported, the simulation is a
conditional simulation result. After a round, the report states the
agreement that was measured (per cohort, with n) and the version it was
measured on. "Validated" means "a calibration round was run and reported",
not "the simulation was right".

## What to record in the calibration report

- The held-out set (what it is, n per cohort, how it was frozen).
- The simulation run (artifact id, profile version, model, seed,
  assumptions, price, market).
- Per-cohort and per-objection agreement tables (simulated vs held-out,
  difference, n).
- Disagreements, stated plainly.
- The version the measurement applies to, and what changed (if anything)
  as a new version afterwards.
- The date and who ran it.

## Honest boundaries (carried from the design)

- SwarmTank is scenario planning, not a substitute for real customer
  interviews, conversion data, A/B tests, or statistically valid market
  research (§34).
- Confidence scores are internal model indicators, not empirically
  calibrated probabilities unless validated (§37).
- Do not claim N separately reasoned AI agents if one model created N
  persona records (§36). In AI mode the run records how many provider
  calls were actually made — report that number.
- Never invent measured real-world demand (§40).
- A price-slider update is a synthetic sensitivity calculation, not fresh
  AI evaluation or evidence that real customers changed their minds (§39).
