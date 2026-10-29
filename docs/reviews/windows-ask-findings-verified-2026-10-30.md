# Ask Windows findings verified and native verification integrated

Review started 2026-10-29, completed 2026-10-30. Incoming
`c15fe5d6b9b0ea73fd105d08027e9ef61f74d331` preserves reviewed `6e355b02d`.
The ZIP contains a completed report, despite its accompanying progress message.
Observed ZIP SHA-256:
`64e181734bed0476df6d43d57bbc4dea62c7d6507fe91e07f7d26a58c1dbf7b4`.
Bundle matches its manifest SHA-256:
`3665cfe857ef51c02532d7ca9dc88f4c4a6c6c46ad1a92de677250c50c1886c8`.
Bundle verification, exact fetch and reviewed ancestry pass.

## Outcome and implementation

All four incoming findings pass focused Windows verification: the original ACL
widening refusal, platform accounting, strict Clippy, and the source-binding
later-correct-occurrence regression. The refusal assertion is preserved and the
test is no longer ignored.

Integrator checkpoint `4ef90406d8726757f225e75a36e3bf57e01ebde6`, branch
`codex/review-ask-windows-findings`, additionally removes verification-helper
process cost. The existing native owner/DACL policy now handles verify-only
root/catalog calls and operation batches. Permission creation/repair still uses
the original script. No positive ACL cache, schema, consent, query or model change.
Use the final reviewed branch, not incoming `c15fe5d6` alone, for this improvement.

The added Windows parity test compares native checks directly with the original
script for private files/directories, Everyone-read widening, widened ancestors
and enabled inheritance. All refuse/restore correctly. Hashes, size caps, links,
reparse refusal, Sealed exclusion and current-permission checks are retained.
These are tested point-in-time checks, not a formal proof against every malicious
concurrent filesystem swap or a new security audit.

## Windows measurements

Same unchanged public 16-note probe, synthetic data, refused loopback endpoint:

| Stage | Ask ms |
|---|---:|
| Pre-batch review | 27,637 |
| Earlier batch-only checkpoint | 6,910 |
| Incoming findings repair `c15fe5d6` | 6,299 |
| Integrated native verify-only path `4ef90406d` | 2,264 |

These are individual observations, not a repeated speed study. Timings include
the refused runtime connection, not model generation. No cross-OS speed claim.

Accounting across all original sizes passes:

| Notes | Setup ms | Ask ms | Logical batches | Covered paths | Helper spawns |
|---:|---:|---:|---:|---:|---:|
| 0 | 0 | 285 | 0 | 0 | 0 |
| 1 | 1,930 | 2,229 | 1 | 4 | 0 |
| 16 | 2,849 | 2,258 | 1 | 34 | 0 |
| 128 | 6,246 | 2,512 | 1 | 258 | 0 |
| 1,010 | 38,808 | 4,071 | 1 | 2,022 | 0 |

Eight single root/catalog checks remain native calls, not process launches.
`process_spawns` separately records successful permission-helper launches.
Do not sum logical batches and function calls and label that an OS process count.

## Full realistic corpus

The unchanged fixture reports 1,008 total records including one foreign canary
(1,007 in the main workspace), four fictional domains and 80 questions. All use
public Ask, real rebuild/compiler, canonical bodies, hashes and privacy checks.

| Gate | Windows result |
|---|---:|
| Complete answerable questions | 68/68 |
| Required fact/source bindings | 80/80 |
| Term-disjoint empty-evidence cases | 6/6 |
| Natural-negative partial-context cases | 6/6 |
| Disclosure and budget | 80/80 each |
| Canary/excluded-record leaks | 0 |
| Additional source appearances, answerable cases | 600 |
| Ask mean / p95 / max | 4,124.0 / 4,383.3 / 4,598.6 ms |

The original p95 bar is 5,000 ms. Test suite 2/2 passes in 472.25 seconds.
Total setup: 141,712 ms, of which filler setup is 34,389 ms. Earlier fixture code
timed only filler insertion; both total and filler setup are now reported.

Windows synthetic setup bulk-writes canonical rows/files from a real public-created
template, then protects every new path before Ask. Accounting uses this setup;
realistic acceptance uses it for the same 910 filler notes. Primary source/revision
operations remain public API calls. Linux retains original public-creation setup.
No question, body, binding or pass bar was weakened. This does not certify bulk
product ingestion or ordinary create-note performance.

Raw prompts/citations/bindings/timings: `realistic-evidence.json`, SHA-256
`71ae70c2511d92231a10c0404a55e55b52fad18c15bfaa64ceda14bb50a82528`.
The separate auditor reconstructs exact citation headers, confirms all 80 bindings,
no listed canary spans, disclosure equality and at most 1,554 evidence bytes.
Combined prompt bytes: 121,280; not provider tokens. The first audit used the next
order statistic for p95; the corrected version matches nearest-rank test output.
Both outputs are retained, the first explicitly named superseded. Raw timings and
test results were unchanged.

Extra sources remain substantial. No model was called: negative context checks
do not establish model refusal; evidence completeness does not establish answers.

## Final validation and limits

- 94 focused Rust tests: library 63; Ask 5; Ask adversarial 4; permission batch 1;
  recall 2; streaming 1; body search 1; context 10; hybrid 6; retrieval 1.
- Accounting 1/1, realistic 2/2, unchanged probe 1/1: 98 distinct final Rust tests.
  Earlier individual reruns are not added again.
- Strict core all-target Clippy, fmt, contracts drift, TypeScript and docs pass.
  API/web builds pass; the existing web chunk-size warning remains.
- UI/contracts 81/81; Python runner/scorer 8/8; canonical browser Ask 1/1
  (12.9 s test, 17.0 s runner), real-model mode disabled. Lease released.

Complete workspace/core was not rerun. Delivered four Ollama environment failures
and A6 partial-match/empty expectation remain recorded, not claimed green. The
older 1,007-record suite was not duplicated; the realistic suite supplies full-scale
Windows acceptance here. Native dialogs/desktop promotion, G1, remote CI and
generated-execution certification remain open. No cloud calls, credentials,
pushes, master merge, preview changes or audit-monitor activation.

## Reproduction and resume

Use E: for TEMP/TMP/CARGO_TARGET_DIR. Optional ANANSI_REALISTIC_OUT must name a NEW
absolute artifact file; output cannot overwrite prior evidence.

```powershell
node scripts/cargo.mjs test -p anansi-core --locked --test ask_permission_accounting -- --test-threads=1 --nocapture
node scripts/cargo.mjs test -p anansi-core --locked --test ask_realistic_corpus_acceptance -- --test-threads=1 --nocapture
node scripts/cargo.mjs test -p anansi-core --locked --test ask_windows_scale_probe -- --test-threads=1 --nocapture
node scripts/cargo.mjs clippy -p anansi-core --all-targets --locked -- -D warnings
python scripts/audit-realistic-evidence.py realistic-evidence.json new-audit.json
```

Raw evidence: `E:/Anansi/benchmarks/ask-windows-findings-review-2026-10-29`.
No private owner workspace was used. This checkpoint is ready for the next
isolated preview integration and separately bounded model measurement. Frozen
cloud prompts and historical model results remain unchanged; no replay implied.
