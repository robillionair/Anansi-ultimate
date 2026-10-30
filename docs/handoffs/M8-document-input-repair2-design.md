# M8 document input repair 2 — bounded design (Windows finding DI-03)

Baseline: reviewer tip `d3f004bc` over repair tip `ef0b9843`.
DI-01 and DI-02 are closed and preserved. The reviewer
probes — `review-collider-document-input.spec.ts` (DI-01/
DI-02), `review-collider-document-rejected-selection.spec.ts`
(DI-03), and the canonical `collider-document-input.spec.ts`
— are preserved unchanged. Before repair, the DI-03 probe
fails only its identity assertion: releasing a gated read
after a newer oversized selection replaces the reviewed
beam with the obsolete content.

## Cause

The DI-01 pre-read size branch in `pickDocument`
(`packages/ui/src/Collider.tsx`) returns before
`session.load()`, so it never advances the load session's
generation. A gated read from an earlier selection still
owns the current generation and lands when released,
despite the newer rejected selection. Every other
selection path advances the generation (a started load
bumps it; edit and clear invalidate it); the pre-read
refusal is the only selection that does not.

## Fix (one boundary, in `pickDocument` only)

Every new selection invalidates the beam's load session
first, before any early return:

```ts
session.invalidate();
if (file.size > MAX_DOCUMENT_BYTES) {
  if (mountedRef.current) {
    setLoading(false);
    setError(`The file must be at most ${MAX_DOCUMENT_BYTES} bytes.`);
  }
  return;
}
setLoading(true);
setError('');
const loaded = await session.load(/* ... */);
```

Consequences, all required by the finding:

- The gated read's outcome is stale by the existing
  generation gate, whether it later succeeds or fails —
  it changes neither beam nor attribution nor error nor
  loading state.
- The rejected selection clears the obsolete loading
  state (the invalidated read will resolve silently
  without clearing it itself) and shows the current
  rejection.
- The last applied beam and attribution are untouched by
  a rejection: the branch sets only loading and error.
- The pre-read size guard, the decoder's post-read check,
  the DI-02 failure gate, edit/clear/unmount behavior,
  and same-name reselect are unchanged. No backend,
  schema, or contract change.

## Acceptance

- All three reviewer/canonical browser specs pass
  unchanged, including the DI-03 identity assertion and
  the inherited save/restart/backup-restore workflow.
- Focused node coverage: an obsolete success and an
  obsolete failure after a rejected selection (a bare
  invalidate with no new load), plus a fresh load still
  applying afterward.
- Affected browser battery, UI/contracts, TypeScript,
  contracts drift, docs, and focused Rust suites
  unchanged in result.

## Explicitly out of scope

No new features, no native picker change, no backend or
contract change, no preview promotion, no cloud calls,
pushes, or CI.
