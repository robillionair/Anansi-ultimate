# M5 addendum 4 — wire binding, logical workspaces, lock discipline, owner authorization

Agent: Muse (M5 implementation agent) · Date: 2026-10-20
Branch: `muse/m5-provider-integration`, continuing from `893956d`
(r4, independently verified on Windows: 112 provider tests, nine
core integration tests, Clippy, formatting, docs). This round
closes the four findings from the r4 review.

## 1. The wire request now names the selected candidate

At r4 the gates, permit, budget estimate and disclosure all
described the **routed candidate**, but adapters built the wire
body from the caller's *requested* model. Under the Zero-Cost
Lock, a dispatch routed to a free model would still have
transmitted the requested paid model — gates and wire described
different dispatches.

The dispatcher now clones the request per candidate and overrides
its model with the candidate's before `build_request`, in both
the unary and streaming paths. Gates, permit, budget, wire body
and disclosure describe one and the same dispatch.

Evidence (`crates/providers/tests/wire_binding.rs`, synthetic
servers record the exact wire bodies):

- Paid requested model + free routed candidate under the
  Zero-Cost Lock: the paid candidate is refused pre-network
  (0 hits); the free server receives `"model":"llama-free"` and
  never the string `gpt-paid`.
- Fallback across two free candidates: each server's recorded
  body carries its own candidate's model.
- Streaming under the lock: same binding, asserted on the
  recorded stream request.
- Choose-Model control: chosen model on the wire, as before.
- Permit/candidate binding: a permit for model A refuses
  candidate B pre-network; the chain proceeds to the candidate
  the permit actually covers.

## 2. Provider state is scoped to logical workspaces

At r4 the workspace scope was the storage-root fingerprint — but
several logical workspaces share one root, so root scoping did
not isolate them. Provider operations now carry a `workspaceId`
(the pattern every other domain operation already uses) which
`Core` validates through the existing storage registry
(`Storage::list`); unknown ids are refused before any provider
state is touched. Configuration, permits, grants, accounting and
stream runs are all keyed by (root, workspace id).

Evidence (`two_workspaces_in_one_root_are_fully_isolated`): two
workspaces created through the domain operation in one root get
independent configurations; traffic and usage stay in their own
workspace; a permit issued in one does not exist in the other; a
stream started in one is `NotFound` from the other.

## 3. The backend mutex is never held across a network call

Dispatch is two-phase: policy resolution (model universe,
permit, runtime) atomically under the lock, then the network call
unlocked on an `Arc<ProviderRuntime>` (`dispatch` now takes
`&self`; all dispatch-touched state is immutable or `Arc`-shared).
Stream start adds a third phase that re-checks the retained cap
under the lock after establishment, cancelling the just-opened
stream rather than exceeding the bound.

Evidence (`slow_dispatch_does_not_block_poll_cancel_or_other_workspaces`):
with a 1.5 s dispatch in flight in workspace 1, stream poll and
cancel in workspace 1 and a dispatch in workspace 2 all complete
in well under the in-flight duration (asserted < 800 ms), the
cancellation is effective, and the slow dispatch completes
correctly. (An earlier revision of this round kept the r4 shape
and the test caught it — poll waited the full remaining 1.2 s.)

## 4. Owner authorization on configure / authorize — verified

Moving policy into `providers-configure` never established
authorization by itself. Both `providers-configure` and
`providers-authorize` now require the **owner**: the host session
(the desktop process — the vault's Desktop transport), or an API
session holding a live grant. Grants are issued and revoked only
by the host (`providers-grant-session` /
`providers-revoke-session`), are per-workspace, expire
(30–3600 s), and are swept with the other expiries. A stamped
session can neither configure, authorize, nor grant itself.

Evidence: core test `configure_and_authorize_require_owner_authorization`
(denied → host grant → allowed → revoked → denied, zero provider
traffic throughout) and the API test, which now also asserts the
browser session's configure is **403 before the grant and 200
after** the host grants it. (The API's `ai` handler also gained
the missing `Forbidden → 403` / `Unauthorized → 401` status
mappings; both previously fell through to 400.)

## Remaining

The provider configuration / approved credential setup /
disclosure UI is the next slice; its surface is unchanged by this
round apart from the `workspaceId` field and the two grant
operations, both already in the generated contracts. OAuth
completion, Bedrock/Vertex and real-provider verification remain
deferred; no real credentials or paid inference were used.
