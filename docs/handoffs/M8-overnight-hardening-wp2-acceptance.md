# M8 overnight hardening — WP2 acceptance criteria

WP2 hardens the existing project-picker selection workflow. It
adds no new project system, no new provider dispatcher and no
parallel context registry: every change reuses the existing
controllers, contracts and backend bounds. Selection, loading,
version changes and review start no inference; the pre-send
review still describes exactly what submission will send, and
submission still sends exactly the reviewed refs.

Criteria are committed before implementation (WP1 gate passed
at `158cc241`: collider 34/34, transfer 19/19,
forge_restore 4/4, backup_restore 7/7, node pass,
Clippy/fmt/tsc/contracts clean).

## What already holds (verified by inspection 2026-10-30, not assumed)

- Review epoch discipline: memory review, project review and
  project version lists drop late responses for superseded
  selections; project switch clears choices, review and
  version options synchronously and bumps both epochs.
- List effects use cleanup cancellation, so a late list for
  the previous project cannot populate the new project.
- Submit sends exactly the reviewed refs, never re-read
  heads; failed starts preserve selection and review, so a
  retry needs no re-review; unmount cancels the active run
  through the controller reset.
- Backend already refuses cross-picker duplicates, overlong
  items, over-count selections and ineligible types before
  provider traffic. WP2 moves those refusals earlier, into
  the UI, with explanations — it does not change backend
  rules.

## Gaps to close (each with its acceptance test)

### W1. Pagination: lists must not silently truncate

- Today the picker calls `listMemories` with no limit, so
  the backend default page (50, max 100) silently truncates
  the project dropdown, a project's member list and a
  member's revision list. `listProjectArtifacts` and
  artifact version lists are unpaged and unaffected.
- Fix: a paging helper that follows `nextOffset` at
  limit 100 until exhaustion, used at the three memory
  list sites, honouring effect cancellation between
  pages. A page cap (200 pages) ends the loop with a
  visible list error, never a silent truncation.
- Tests (node, deferred fake client): multi-page list
  reassembles in order; exact page boundary (100, 200)
  loads completely; empty list stays empty; a failure
  mid-pages surfaces the list error and keeps no partial
  list as if complete. E2E: a project with 51 members
  shows the 51st member.

### W2. List failures must not masquerade as empty

- Today a failed project/members/artifacts load sets an
  empty list, so the picker reports "no selectable items"
  for what was actually a failed load, with no retry.
- Fix: distinct list-error state per list, rendered as an
  alert, suppressing the empty message while set, plus a
  Retry control that reloads the same list without
  changing the selection.
- Tests: node loader tests for error propagation; e2e
  with a failed list response shows the alert and
  recovers via Retry.

### W3. Pre-send review shows the privacy label

- Today project (and memory) review entries carry no
  privacy: the consent boundary shows title, kind,
  version and body, but not the label under which the
  item will be sent.
- Fix: the review loaders capture privacy from the read
  responses (`read.memory.privacy`,
  `read.artifact.privacy`) and the review renders
  "· {privacy}" beside each entry, for both pickers.
- Tests: node loader test asserts privacy capture;
  e2e asserts the label renders in the review section.

### W4. Cross-picker duplicates are prevented, not just refused

- Today a memory can be checked in both pickers; only
  the backend refusal at submit explains it.
- Fix: a member checkbox disables when its id is already
  selected in the memory picker, and a memory checkbox
  disables when its id is already chosen as a project
  memory item, each with a short note naming the other
  context. Unchecking in one picker re-enables the other.
- Tests: e2e selects in one picker, asserts the other
  picker's checkbox is disabled with the note, unchecks,
  asserts re-enable.

### W5. Count and content bounds are visible before submit

- Today the 9th checkbox can be checked and an overlong
  item reviewed; only the backend refusal at submit
  explains it.
- Fix: at 8 selected items in a picker, further
  checkboxes in that picker disable with a note ("8
  items maximum for this context"). A review entry
  whose body exceeds 8000 characters, or whose picker
  total exceeds 32000, renders a warning in the review
  section and blocks submission (the Collide control
  disables) with the bound named. Bounds mirror the
  backend exactly (8 / 8000 / 32000 per picker); nothing
  is truncated.
- Tests: e2e at 8 items asserts the 9th disables; a
  review entry over 8000 chars (seeded via API) asserts
  the warning and the disabled submit.

### W6. Failed review reads can retry without losing the selection

- Today a failed review read leaves only an error: the
  user must deselect and reselect to retry.
- Fix: a Retry control in each review section that
  reloads the current choices unchanged (fresh epoch,
  same choice list). Retry must not alter choices,
  versions or the other picker.
- Tests: e2e with a failed then succeeding read
  response asserts the review recovers with the same
  choices.

### W7. Filtered-out items are explained, not silently hidden

- Today ineligible artifact types and project records are
  filtered from the lists with no trace.
- Fix: when the fetched lists contain filtered items, a
  help note states how many were hidden and why
  ("approval records are never offered as context";
  project records are containers, not content). Counts
  only — no hidden titles or bodies.
- Tests: node test for the counting; e2e with an
  approval-row artifact present asserts the note.

### W8. Races stay safe (verify, fix only if a test fails)

- Project switch while list/read responses are delayed;
  version switch while an older review is still loading;
  deselect/reselect of the same identity; late responses
  after a newer explicit choice; navigation/unmount
  while a read or start is in flight.
- Mechanism: Playwright route delays (deferred/gated
  responses, never arbitrary sleeps or raised timeouts).
  Assertions: the UI always shows the newest explicit
  selection's items/review; a stale response never
  overwrites it; unmount during a delayed start cancels
  without errors.
- Existing epoch/cancellation discipline is expected to
  pass these; any failure is repaired in the same
  discipline (epochs, cleanup flags), not by sleeps,
  timeout increases or weakened assertions.

## Non-goals for WP2

- No backend rule changes: bounds, eligibility and
  refusal semantics are WP1's and the backend's own.
- No new selection affordances beyond the notes, limits
  and retries above (no "select all", no search).
- No change to what submission sends: still exactly the
  reviewed refs, still no inference before Collide Ideas.

## Focused acceptance gate for WP2

- New node unit tests (paging helper, review loaders,
  filtered counts) pass; new/updated e2e specs pass
  sequentially on the real API with a synthetic provider.
- Pre-existing collider / transfer / forge_restore /
  backup_restore / node suites pass unchanged (counts
  recorded, no assertion weakened).
- Strict Clippy, fmt, tsc, contracts check clean.
- The WP2 record (this file, updated with before/after
  evidence) is committed before WP3 criteria are written.
