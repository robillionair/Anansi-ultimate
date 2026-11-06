# SwarmTank v2 — Checkpoint 1: market selection + profile provenance

## Scope

Checkpoint 1 of the SwarmTank v2 design (`docs/handoffs/swarmtank-v2-design.md`
§2, AC-M1..M6). A searchable worldwide territory list from versioned Unicode
CLDR data, three researched starter profiles (Estonia, Dubai/UAE, United
States) with a source and date for every evidence-backed input and editable
assumptions for gaps, and a Custom assumptions state for every territory with
no researched profile. Geography shapes which buyers appear and what they can
spend; it never shapes a personality trait.

Preserved: the seeded Quick simulator, the LARP scene 08 matrix, the existing
SwarmTank runs and the browser regression. No provider call is made at any
step of this checkpoint.

## What changed

### CLDR territory data (AC-M1)

- `scripts/generate-cldr-territories.mjs` — generates the territory list from
  a pinned Unicode CLDR source (cldr-json tag **49.0.0**,
  `cldr-localenames-full/main/en/territories.json`; source SHA-256
  `1326fb3b…bf04`). Includes only two-letter codes, excludes `ZZ` (Unknown
  Region) and `EU` (political union, not a country/territory), excludes
  three-digit macro-regions and `*-alt-*` name variants. Output:
  `crates/core/src/markets/territories.json` — **262 territories**, CLDR 49,
  with format/version/source metadata in the file.
- The generator is re-runnable against a local file or the pinned URL; the
  CLDR source itself is not vendored as a dependency.

### Researched profiles (AC-M2, AC-M3)

Versioned JSON under `crates/core/src/markets/profiles/`:

- `ee.json` — **Estonia (EE)**, EUR, languages et (official) + en (business).
  Evidence (5): business language, company size (OECD, Financing SMEs and
  Entrepreneurs 2026: Estonia), digital readiness, buying authority,
  currency. Assumptions (2, editable): typical budget, procurement norm.
  Segments (3): micro (1–9, ×0.5), small (10–49, ×1.0), medium (50–249,
  ×2.0).
- `ae-dxb.json` — **Dubai, UAE (AE-DXB)**, territory AE, subdivision Dubai,
  AED, languages en (business) + ar (official). Evidence (5): business
  language, free zones (40+, 100% foreign ownership, mainland restriction,
  DIFC common-law framework), procurement norm, corporate tax (9% above
  AED 375,000), work week. Assumptions (2, editable): typical budget,
  buying authority. Segments (3): SME (1–49, ×0.8), mid (50–249, ×1.5),
  large (250+, ×3.0).
- `us.json` — **United States (US)**, USD, language en (business).
  Evidence (3): company size (SBA Office of Advocacy 2026: 36.2M small
  businesses, 99.9%), small-business definition (<500 employees, NAICS
  size standards), firm-size concentration. Assumptions (3, editable):
  business language, procurement norm, typical budget. Segments (4): micro
  (1–9, ×0.4), small (10–99, ×1.0), mid (100–499, ×2.5), large (500+, ×4.0).

Every evidence entry carries `field`, `value`, `source {name, url}`, and
`date`. Every assumption carries `field`, `value`, `rationale`, and
`editable: true`. Segment roles are drawn from the existing 12 archetype
roles only — a profile cannot invent a buyer role.

### Rust: market module + persona shaping (AC-M4, AC-M5, AC-M6)

- New `crates/core/src/swarmtank_market.rs`: `Territory`, `MarketProfile`,
  `MarketEvidence`, `MarketAssumption`, `MarketSegment`, `SwarmtankMarket`
  (the run's market selection), plus the `swarmtank-territories`,
  `swarmtank-market`, and `swarmtank-markets` ops. Data is embedded with
  `include_str!` — one source of truth, served to the UI through the API.
  `validate_profile` is fail-closed: format/version, CLDR version match,
  non-empty segments, every segment role must exist in the archetype table
  (`archetype_role_exists`), budget multiplier > 0, every evidence entry
  must have a non-empty source name, URL, and date. `validate_market` is
  fail-closed: an unknown profile id, an unknown territory code, or a
  custom selection naming a profile are all refused.
- `crates/core/src/swarmtank.rs`: `SwarmtankPersona` gains optional
  `language`; `SwarmtankRunDoc`, `SwarmtankPersonasRequest`, and
  `SwarmtankEvaluateRequest` gain optional `market`. New
  `generate_personas_for_market(seed, preset, market)`: with a researched
  profile, (archetype, segment) pairs are built round-robin across segments
  then roles, seeded-shuffled, per-persona budget = archetype base × segment
  multiplier × seeded 0.8–1.2, company size = the segment's, language =
  the segment's first language else the profile's business language.
  Archetype role, skepticism, price sensitivity, needs, priorities,
  constraints, and trust signals are unchanged — geography shapes which
  buyers appear and what they can spend, never their personality. With no
  profile (custom or unresearched territory) the call falls back to the
  unshaped `generate_personas` byte-for-byte; the run still records the
  selection, labelled custom.
- `personas_op` / `evaluate_op` validate the market when present and use
  the market-aware generator. `evaluate_op` stores `doc.market`.
- Three routing layers had to list the new ops (the third was found by a
  live curl probe returning `INVALID_REQUEST`, not by reading): the API
  path allowlist (`crates/api/src/lib.rs`), the `swarmtank_command`
  dispatcher (`crates/core/src/swarmtank.rs`), and the `ai_command`
  allowlist (`crates/core/src/ai_discovery.rs`). The market ops are static
  data, so `ai_command` treats them like `providers-catalog`: they need no
  `workspaceId` (root fingerprint as the scope workspace).
- Contracts: the market types are registered in
  `crates/contracts/src/main.rs`; `scripts/contracts.mjs --check` is clean.
  Client methods `listSwarmtankTerritories`, `listSwarmtankMarkets`,
  `getSwarmtankMarket` in `packages/contracts/src/client.ts`.

### UI

- New `packages/ui/src/SwarmtankMarket.tsx` (`SwarmtankMarketSelector`):
  searchable territory select (262 entries, CLDR version shown), researched
  profile view (evidence list with source link + date, editable assumptions
  with rationale, buyer segments), subdivision radio choice when a territory
  has several profiles, Custom assumptions section for unresearched
  territories (four editable fields, labelled "yours, not evidence"), and a
  Clear market button. The selection is published upward as a
  `SwarmtankMarket` (researched: profile id/version/territory; custom:
  territory + user-supplied assumptions only).
- `packages/ui/src/Swarmtank.tsx`: the selector sits above the description
  field; `market` is passed to generate/evaluate and to scenario reruns.
  `parsePersona` accepts optional `language`; `parseSwarmtankRunBody`
  accepts optional `market` via strict `parseMarket` (fail-closed: custom
  must not name a profile, a profile id requires a version, unknown fields
  refused). The report view shows the market label: `Estonia — researched
  profile v1`, `France (FR) — Custom assumptions`, or `Unspecified` for
  pre-market runs, plus a custom-assumption count when recorded.

### Location deviation from the design

The design (§2.2) said profiles live in `packages/contracts/src/markets/`.
They live in `crates/core/src/markets/` instead: the contracts package
exports only `./src/client.ts`, so the Rust core is the natural home for the
embedded data and its validator, and the UI receives it through the API
(one source of truth, consistent with the UI being an API client). No
behavioural difference; the design doc is not edited.

## Acceptance criteria — results (AC-M1..M6)

- **AC-M1** — searchable full CLDR list: 262 territories, CLDR 49, search
  narrows the list and reports no match. Proven by the e2e search test.
- **AC-M2** — three profiles show evidence with source + date and editable
  assumptions, distinguished: Estonia profile view shows all five evidence
  entries with source links and dates, the editable-assumptions section,
  and the buyer segments (screenshot `evidence/market-estonia-profile.png`).
- **AC-M3** — unresearched territory opens Custom assumptions and the run
  is recorded custom: France shows the Custom assumptions section; the
  saved run's report reads `France (FR) — Custom assumptions` with
  `2 custom assumption(s) recorded` (screenshot
  `evidence/market-france-custom.png`).
- **AC-M4** — a new region is added by data file only: a test parses a
  well-formed FR profile JSON literal through `parse_profile` and asserts
  it validates — the format, not a hardcoded id list, is what the loader
  validates. **Limitation, stated plainly:** `researched_profile_ids()`
  still matches on a hardcoded id list, so a fourth profile data file alone
  does not register until its id is added there. The data-only proof holds
  at the parser level; the registry lookup is the remaining one-line step
  for a future profile. This is not hidden: the test name says
  `a_fourth_profile_is_data_only`.
- **AC-M5** — personas use role/industry/size/authority/language/budget
  evidence; no persona field is derived from nationality: the market
  shaping test asserts EE personas' roles and company sizes are within the
  EE segments, languages are et/en, `synthetic` stays true, the same
  seed+market is deterministic, and the US population differs. Archetype
  personality fields are untouched by the market path.
- **AC-M6** — old saved runs (no market) still load, market shown
  `Unspecified`: the existing valid-parse test asserts
  `parsed.market === undefined`, and the report view renders `Unspecified`.

## Verification (Linux, this branch)

- Rust workspace lib: **270 passed, 0 failed** (api 4 + core 205 + desktop
  1 + providers 60). Baseline for this branch is the pilot merged baseline
  `3bf51af8` (core 200); +5 new market tests = 205. Do not compare against
  the PDF-integration tip's 297 — that branch is a different lineage.
- TypeScript: **0 errors**. Contracts `--check`: **0 drift**. Docs
  validator: **309 PASS**.
- UI tests: **323 passed, 0 failed** (incl. 3 new market tests).
- E2E `tests/e2e/swarmtank-market.spec.ts`: **3/3** (Estonia researched
  profile + saved-run label; France Custom assumptions + saved-run label;
  territory search).
- Regressions: `tests/e2e/larp-swarmtank.spec.ts` + `tests/e2e/swarmtank-arena.spec.ts`:
  **4/4** (scene 08 matrix and arena behaviour preserved; the panel change
  did not break them).
- API build: clean except the pre-existing `mcp.rs:715` warning.
- Live curl probe (debug API, port 1455): territories 262/CLDR 49,
  profiles EE/AE-DXB/US, EE researched with 5 evidence + 3 segments, FR
  researched false.

## Defects found and fixed in this checkpoint

1. **Missing third allowlist.** The new ops returned `INVALID_REQUEST`
   ("Use a supported AI command.") until they were added to the
   `ai_command` allowlist in `ai_discovery.rs` — the API allowlist and the
   `swarmtank_command` dispatcher alone are not enough. Found by a live
   curl probe.
2. **Market ops need no workspace.** `ai_command` refused the calls with
   "A workspaceId is required." until the three ops were treated like
   `providers-catalog` (static data, root fingerprint as scope).
3. **`customAssumptionsFrom` crash.** The filter
   `values[field]?.trim() !== ''` is true when the value is undefined, so
   empty fields passed the filter and `.map()` called `.trim()` on
   undefined — a `TypeError` that broke the panel as soon as any territory
   was selected (before the profile loaded). Fixed by normalising with
   `(values[field] ?? '').trim()`. Found from a browser page-error stack.
4. **Spec strict-mode collisions.** `getByLabel('Country or territory')`
   also matched the search box ("Search country or territory") — fixed with
   `{ exact: true }`. `getByText('Custom assumptions')` matched both the
   heading and the `<strong>` in the paragraph — fixed with
   `getByRole('heading', ...)`. Notices render at page level, not inside
   the panel — the generate/evaluate assertions use `page.getByText`.

## Known limitations

- AC-M4 registry note above (hardcoded id list in
  `researched_profile_ids()`).
- Font CMap over-count and direct-XObject refusal from the PDF extractor
  repair are unrelated and untouched.
- The three starter profiles are the only researched regions; every other
  territory is Custom assumptions until a profile is added. Worldwide
  dropdown coverage (262 territories), researched regional profiles (3),
  and real-world validation (calibration plan only, not performed) are three
  separate statuses — see the design §6.

## Proposed global-status updates (for Antigravity; not applied here)

None to AN-014 — SwarmTank v2 is new scope on top of the verified M9
milestone, and this checkpoint adds no new requirement row. When the full
v2 delivery lands, a new requirement (or a v2 note on AN-014) is a decision
for the owner and Antigravity, not this checkpoint.

## Windows acceptance checklist (Checkpoint 1)

1. Verify the branch tip and bundle hashes (see the delivery MANIFEST).
2. `cargo test --locked --workspace --lib` — expect 270 passed.
3. `npm run typecheck` (0), `node scripts/contracts.mjs --check` (0 drift),
   `node scripts/validate-docs.mjs` (309 PASS), `npm run test:ui` (323).
4. Build the API and the web app; run
   `tests/e2e/swarmtank-market.spec.ts` (3/3) and
   `tests/e2e/larp-swarmtank.spec.ts` + `tests/e2e/swarmtank-arena.spec.ts`
   (4/4) through the shared application-test lease.
5. In the browser: open SwarmTank in a project, search "Estonia", confirm
   the researched profile shows evidence with source + date, editable
   assumptions, and buyer segments; generate + evaluate and confirm the
   report reads `Estonia — researched profile v1`. Select France, confirm
   the Custom assumptions state, fill two fields, generate + evaluate, and
   confirm the report reads `France (FR) — Custom assumptions` with
   `2 custom assumption(s) recorded`.
6. Confirm no provider call is made at any step (the panel header already
   states generation/evaluation are deterministic backend operations).
7. Open a pre-market saved SwarmTank run and confirm it loads with the
   market shown `Unspecified`.
