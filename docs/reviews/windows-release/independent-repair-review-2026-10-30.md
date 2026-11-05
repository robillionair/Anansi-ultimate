# Release-tooling repair independent review — 2026-10-30

Incoming `3aa2ff51a89948d3a2b0508ed52c542987dafddd` over reviewer `c85509e86955bec21230a71563429fb18f30a44d`. ZIP SHA256 matches `c3e82e04de7d83b384e42e9ade7833ff0773fe2d352ab3ea721db54c9240e949`. Existing reviewer probes unchanged. Assigned directories only. All existing28 tests independently pass on Windows.

Verdict: original examples repaired; two broader acceptance conditions remain open. No production integration.

## WR-01b (P1): early zero exit and unrelated existing files qualify

`runGuardedLaunch` only rejects an early exit when exitCode is nonzero, and derives readiness from any directory entries. Independent control: existing Windows whoami.exe with no args exits0 immediately, while a synthetic data root contains only unrelated.txt. Both phases return early; evidence declares readinessVerified:true, persistenceVerified:true and ok:true. This is not application storage initialization, health, integrity or re-admission. The current code checks names, not accessibility/contents/database state.

Reject every premature desktop exit, regardless of code. Define a truthful readiness contract for synthetic application state; arbitrary files cannot satisfy it. Verify restart identity/data with meaningful checks, not directory counts alone. For a weaker startup-only mode, label it explicitly and never claim persistence/integrity acceptance. The earlier cleanup request also remains incomplete by inspection: ensureProcessTerminated checks only the parent PID, and finally releases the lease even if the last cleanup fails. Track/confirm owned process-tree cleanup and retain a blocked lease with actionable evidence if termination cannot be confirmed.

## WR-02b (P1): fields are named but their values are not constrained

`collectDiagnostics` copies launchId/timestamp/smokeDurationMs/phase exit codes from parsed JSON without validating types or bounds. A launchId object containing a synthetic private sentence is exported intact; the same happens to strings in numeric fields. Removing raw log text fixes the original probe, but a field-name allowlist alone does not provide the promised typed/bounded export.

Validate every exported field against strict types, lengths and numeric ranges; omit/reject invalid values and unknown nested data. Apply equivalent treatment to all manifest/toolchain/evidence-derived output. Do not special-case the canary. Cover arbitrary objects, arrays, strings in numeric fields, oversize strings, null and boundary values.

## Evidence

`node --test tests/windows-release/*.test.mjs` before new probes:28/28. New `reviewer-followup.test.mjs`:0/2, both expected-success assertions reproduce the findings. Existing reviewer file unchanged. No tool implementation changed by reviewer. System where.exe/whoami.exe are existing Windows utilities; no generated software executed. Tests use E: synthetic roots and the shared lease, now released. No owner previews/data, cloud, credentials, build, install, CI, push or integration.
