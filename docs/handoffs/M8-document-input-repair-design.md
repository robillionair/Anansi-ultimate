# M8 document input repair — bounded design (Windows findings DI-01, DI-02)

Baseline: reviewer tip `97ab19b4` over delivery tip `faafa1c26`.
The reviewer probe `tests/e2e/review-collider-document-input.spec.ts`
is preserved unchanged; both of its finding assertions fail
before repair (oversized file read before refusal; obsolete
failure enables dispatch) and must pass after.

## DI-01 — refuse oversized files before reading

`pickDocument` in `packages/ui/src/Collider.tsx` checks
`file.size > MAX_DOCUMENT_BYTES` before calling
`file.arrayBuffer()` and refuses with the same visible
message the decoder uses ("at most 65536 bytes"). The check
touches no loading state: a refused pick never sets the
beam loading flag and never starts a load session. The
decoder's post-read byte-length check in
`decodeDocumentBytes` is retained unchanged — it covers a
file whose size changes between the pre-check and the read.
No backend change: the backend bound is untouched.

## DI-02 — generation-guard failures like successes

`DocumentLoadSession.load` in `packages/ui/src/documentInput.ts`
currently resolves successes as `{ generation, result }` and
rejects failures, so the component's catch path cannot tell
an obsolete failure from the current one. The load API
changes to a uniform outcome that never rejects:

```ts
type DocumentLoadOutcome = {
  generation: number;
  result: DocumentLoad | null;
  error: Error | null;
};
```

`pickDocument` then applies one gate to both paths: after
the mounted check, an outcome whose generation is not the
session's current generation returns without changing the
error, the loading flag, or the beam. Only a current
outcome clears loading; a current failure sets the visible
error and leaves the previous beam and attribution intact
(retry = pick again), exactly as before for current loads.
Edit and clear already invalidate the session, so a failure
superseded by either is stale by the same rule. The
dispatch guard is unchanged (`collide` refuses while either
beam's loading flag is set), so a pending newer read keeps
Collide Ideas disabled even after an older read fails.

## Acceptance

- Reviewer probe unchanged: oversized pick shows the
  65536 alert and its name never appears in the intercepted
  read list; obsolete failure leaves Collide Ideas disabled
  and the newer read's content wins; the inherited
  save/restart/restore workflow still passes.
- Focused node coverage for superseded failures after a
  newer selection, after edit (invalidate), and after clear
  (invalidate), plus a current failure still surfacing its
  error; stale successes remain dropped.
- Canonical document spec, UI/contracts, TypeScript,
  contracts drift, docs, and focused Rust suites unchanged
  in result; no backend, schema, or contract change.

## Explicitly out of scope

No new features, no native picker change, no backend or
contract change, no preview promotion, no cloud calls,
pushes, or CI.
