# Draft — limited pilot evidence, not a competitive benchmark

**Hold this savings-led draft.** The newer [100-case lexical-only benchmark](large-free-benchmark-2026-10-29.md)
found 60/100 supported correct outcomes versus 100/100 for full context. The older
pilot input arithmetic is unchanged, but posting it without the broader quality
finding would give an incomplete picture. Improve retrieval and independently
validate on held-out questions before making a general efficiency claim.

I have been testing ANANSI's approach to selecting relevant memory before sending
context to a language model.

In a small synthetic pilot — eight distinct answerable questions, repeated twice
per approach using the same model, source material and generation settings — it
used 37.1% fewer API-reported input tokens than sending the full source bodies.
All 32 saved answers were checked against the expected facts and supplied sources.

The result depends on workload: the largest-document case contributes most of the
savings. Removing it leaves 19.4% fewer input tokens. Short inputs can add overhead.
This is preliminary evidence, not a comparison against competing products or a
claim of proven production cost savings.

We also found inconsistent reasoning-token subcounters in the provider responses,
so I am withholding claims about total computation saved until that is resolved.
The next step is independent review and broader representative evaluation.

Editorial note: this draft describes the supplied saved-response audit, not an
independent rerun of the cloud model. Do not substitute “31% less computation.”
