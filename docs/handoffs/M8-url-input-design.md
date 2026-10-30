# M8 URL text input — bounded design and acceptance criteria

Baseline: Windows-verified review tip `144acc715`
(`codex/review-m8-document-input-repair2`, runtime
`0c62533c`). This slice adds bounded URL text input for
either Collider beam through the existing beam/document/
context and M5 infrastructure. It does not reopen the
closed document findings DI-01–DI-03, does not change the
document, memory-context, or saved-project behavior, and
does not touch Antigravity's release tooling.

Authority: MASTER_SPEC §24 (per-beam "Paste URL"),
§25 (extract page content "through an authorized
retrieval method"; on failure, request pasted information
or proceed with only the provided description; never
fabricate unseen website content), and the owner
assignment for this slice.

## User workflow

1. In an existing project's Collider, type a URL for Beam
   A or Beam B.
2. Press the separate Fetch action for that beam.
3. Review the exact fetched text and its source
   provenance (submitted URL, final URL, content kind,
   extraction policy, byte/char counts, hashes) as inert
   text.
4. Press Collide Ideas explicitly; only then does the
   reviewed text reach a provider, as the beam's existing
   private data segment.
5. Save explicitly; reopen after restart; back up and
   restore into a fresh workspace with the same
   generation-time text and provenance; follow an
   approved Forge transfer.

Fetching never starts inference, never saves an artifact,
never registers a memory or document, and never sends the
URL or fetched text anywhere except back to the reviewing
user in the same panel.

## Backend-owned fetch service (new `crates/core/src/url_fetch.rs`)

The browser never fetches a URL itself and never receives
a filesystem or generic network capability. The UI calls
one new backend operation, `collider-fetch-url` (API
allowlist + `ai_command` arm, same pairing-token auth as
every `ai/` op), which requires the workspace project to
exist and be writable (the same `require_project_writable`
gate `collider-start` uses) and then runs the bounded
fetch below. New core dependencies, all already pinned in
`Cargo.lock` at exactly these versions: `url =2.5.8`,
`rustls =0.23.46` (ring/std/tls12, mirroring
`providers/src/tls.rs`), `rustls-pki-types =1.15.1`,
`webpki-roots =1.0.10`. No new provider client, no async
runtime in core: the transport is synchronous
`std::net::TcpStream` + rustls, mirroring the providers
crate's transport style.

### Destination validation (every hop, before connecting)

- Parse with the `url` crate. Scheme must be `http` or
  `https`. Any username or password in the URL refuses.
  Host must be present. Fragment is dropped (never sent).
- Port: the production policy allows only the scheme's
  default port (80/443). A test policy may allow any port
  for loopback synthetic servers (see below).
- IP-literal hosts are classified directly. DNS hosts are
  resolved with the system resolver; a host is refused
  when it has no addresses, when any resolved address is
  not public, or when its name is `localhost` or ends in
  `.localhost`, `.local`, `.internal`, or `.home.arpa`.
  Blocked classes: unspecified, loopback, private
  (RFC 1918 + ULA), link-local, multicast, reserved,
  documentation, benchmarking, and CGNAT ranges, each
  classified explicitly (including IPv4-mapped IPv6
  addresses, which are classified by their embedded IPv4
  address). The connection goes to a validated resolved
  address —
  never to a re-resolved name — with `Host` set to the
  original host and, for HTTPS, TLS SNI set to the
  original host and certificate/hostname validation left
  at rustls/WebPKI defaults (no danger APIs, no custom
  verifier, no bypass flag). This is what prevents
  DNS-rebinding bypasses: the address that was validated
  is the address that is connected to.
- Redirects: at most 5, only 301/302/303/307/308 with a
  `Location` header, relative targets resolved against
  the current URL, and every redirect target re-validated
  by the full rules above before connecting. A redirect
  to a non-HTTP(S) scheme, a credentialed URL, a private
  address, or a non-default port refuses without
  connecting to it.
- Request: `GET` over HTTP/1.1 with `Host`,
  `User-Agent: ANANSI-url-fetch/1`,
  `Accept: text/plain, text/html`, `Accept-Encoding:
  identity`, `Connection: close`. No cookies, no
  authorization headers, no referer, no browser-session
  forwarding, no JavaScript execution, no external
  resource loading (images, stylesheets, scripts are
  never requested — HTML extraction reads the single
  response body only).

### Bounds

- Connect 5s, per-socket-operation 10s, whole fetch
  (all hops) 20s. Header block at most 32 KiB.
- Downloaded body at most 1,048,576 bytes
  (Content-Length larger than that refuses before
  reading; chunked bodies are cut at the cap).
  `Content-Encoding` other than `identity` refuses.
- Response must be status 200 (after redirect
  handling). `Content-Type` must be `text/plain` or
  `text/html`; any other type refuses. A declared
  charset other than UTF-8 (or absent) refuses.
- Extracted text must be 1–8,000 characters and
  non-whitespace, matching the existing beam bound.
  Oversized extracted text refuses visibly — never
  silently truncated — and the user falls back to
  pasting text, per §25.

### Extraction policies (documented, deterministic)

- `plain`: strict UTF-8 decode (fatal on invalid
  sequences, no replacement characters); one leading BOM
  stripped and recorded; no other normalization.
- `html-conservative`: strict UTF-8 decode, then remove
  `<head>`, `<script>`, and `<style>` sections entirely;
  block-level tags (`p`, `div`, `section`, `article`,
  `h1`–`h6`, `li`, `ul`, `ol`, `blockquote`, `pre`, `tr`,
  `br`, `hr`) become newlines; all other tags are
  stripped; HTML entities are decoded (`&amp; &lt;
  &gt; &quot; &apos;` plus decimal/hex numeric
  references; unknown entities are left verbatim);
  runs of more than two newlines collapse to two; the
  result is trimmed of leading/trailing blank lines.
  Fetched content is untrusted data throughout: it is
  never executed, never treated as instructions, and
  reaches the provider only as the beam's private data
  segment, exactly like typed or document text.

### Fetch result and provenance (honest distinctions)

`ColliderUrlFetchResponse`: `submittedUrl` (user-entered,
trimmed, verbatim), `finalUrl` (serialized URL of the
final hop, fragment removed), `contentKind` (`text` |
`html`), `extraction` (`plain` | `html-conservative`),
`fetchedBytes`, `fetchedHash` (SHA-256 of the raw
response body bytes), `textHash` (SHA-256 of the
extracted text bytes), `textChars`, `bom`, `text`.

`ColliderUrlSource` (the saved provenance record) holds
the same fields except `text`. Its honesty contract,
enforced by `validate_url_source` and stated in the UI:

- `textHash` IS re-verifiable: it must equal the
  SHA-256 of the submitted beam text, and
  `textChars` must equal the beam's char count. A
  forged or stale (edited-beam) record refuses at
  `collider-start` before provider traffic, and again
  at restore and transfer through
  `validate_experiment_doc`, exactly like document
  provenance.
- `fetchedHash` is fetch-time evidence only: it proves
  what the fetch service returned at fetch time, but it
  cannot be re-verified later without refetching, and
  the backend never refetches at start, restore, or
  transfer. It is format-checked (64 lowercase hex) but
  never trusted as a beam-text anchor.
- `submittedUrl`/`finalUrl` are shape-checked at
  start/restore/transfer (HTTP(S), no credentials, host
  present, length at most 2048 chars), but a saved
  record is not re-resolved or re-fetched: the saved
  beam text is the generation-time truth. The port is
  deliberately not restricted here: it is a fetch-policy
  matter (the production fetch policy allows only
  default ports), and a saved record is never fetched
  or connected to, so restricting the record would
  refuse records the fetch service legitimately
  produced under a wider test policy.

`ColliderStartRequest` gains optional `urlA`/`urlB`
(serde defaults), `ColliderExperimentDoc` gains optional
`urlA`/`urlB` (`skip_serializing_if = none`),
`ColliderRun` carries them to save, and the save `Claim`
boxes them like document sources (large-enum-variant
lint). Old experiments without URL fields parse, restore,
and transfer unchanged; new experiments without URL
sources serialize identically to before. No schema
change.

## UI contract (new `packages/ui/src/urlFetch.ts`, `Collider.tsx`)

- Per beam: a URL text input, a separate Fetch button,
  and a Cancel button shown while a fetch is pending.
  Fetching is never automatic (no fetch on typing,
  pasting, or focusing).
- A per-beam `UrlFetchSession` with the document
  session's generation discipline: a newer selection,
  edit, source switch, clear, rejection, or cancel
  invalidates older pending fetches, and a stale outcome
  — success or failure — changes neither beam, error,
  nor loading state. Cancel invalidates the session and
  clears loading; the in-flight backend fetch runs to
  its bounded end (at most the 20s total budget) but
  saves nothing and its outcome is dropped on arrival.
  This is the cancellation semantics: client-side
  invalidation plus a bounded backend, not a
  server-side abort — stated here so it is not
  oversold.
- Attribution rules mirror documents: a fetched beam
  carries its URL source; editing the beam clears it;
  picking a document clears the URL source and fetching
  a URL clears the document source; clearing a source
  clears the beam text and attribution. Only the current
  attribution is sent on Collide.
- Review line (inert React-escaped text): submitted URL,
  final URL when different, kind word (Plain text /
  Web page), extraction note for HTML, fetched bytes,
  text chars, text hash truncated to 16 chars with full
  submitted/final URLs and both hashes in the `title`
  attribute, BOM note. A fetch failure shows a visible
  error and leaves the previous beam and attribution
  untouched (retry = Fetch again). Submit is disabled
  while either beam's fetch or document load is pending.
- Saved view: URL provenance renders beside the verbatim
  beams through a strict parser (`parseColliderUrlSource`)
  that rejects malformed records, like the document
  parser.
- Contracts: `ColliderUrlFetchRequest`/`Response` and
  `ColliderUrlSource` are generated from the Rust DTOs
  (`scripts/contracts.mjs`), re-exported through
  `packages/contracts/src/client.ts` with a new
  `fetchProjectColliderUrl` client method, following the
  document-slice pattern. The API allowlist gains
  `/v1/ai/collider-fetch-url`.

## Testing (deterministic; no real external sites)

- Rust unit tests (`url_fetch_tests`): address
  classification (loopback, private, link-local,
  reserved, documentation, multicast, unspecified,
  public), URL shape refusals (credentials, non-HTTP(S),
  missing host, internal suffixes, non-default port),
  HTML extraction cases (script/style/head stripped,
  entities, block newlines, nested tags), plain-text BOM,
  redirect-target validation as a pure function.
- Rust integration tests (synthetic `std::TcpListener`
  servers; HTTPS via the providers-style rcgen harness):
  plain-text fetch exact text/hashes/final URL; HTML
  extraction; same-host and cross-host redirect chains;
  redirect to a private address refused without
  connecting (server hit counts prove it); direct
  private/loopback/link-local/credentialed URLs refused
  with zero server hits; oversized body, wrong
  content-type, invalid UTF-8, and non-200 refusals;
  TLS validation (wrong-name certificate refuses;
  test-CA certificate succeeds).
- Collider integration tests: URL start saves exact
  provenance and wire text (URL/hashes absent from the
  raw provider wire; prompt-like fetched text stays
  data); forged/stale URL metadata refuses before
  traffic with zero provider hits; a URL-backed
  experiment transfers and survives backup/restore.
- Node UI tests: fetch-session races (stale success and
  failure dropped after a newer fetch, edit, clear, and
  cancel), provenance parser accept/reject, saved-view
  rendering.
- Canonical browser spec
  (`tests/e2e/collider-url-input.spec.ts`): synthetic
  loopback content server + fake provider; URL entry →
  Fetch → review + provenance with zero provider hits;
  fetch race (slow first fetch, fast second wins);
  source switches (URL ↔ document, edit clears);
  rejection (invalid URL, oversized page, private
  redirect) leaving the beam intact; cancel of a slow
  fetch with the late response dropped; Collide → result
  + disclosure; save; API-read body byte-exact
  including `urlA`; reopen + restart +
  backup/fresh-restore with identical content and
  provenance.
- Test-only loopback allowance: the real API process
  cannot reach a synthetic server under the production
  policy, so the browser spec spawns the API with
  `ANANSI_URL_FETCH_ALLOW_LOOPBACK=1`. The flag allows
  loopback destinations only (never private, link-local,
  or reserved addresses), is off by default, is read by
  the fetch handler, and is documented as test-only.
  Strict-policy behavior is covered by the Rust tests,
  which construct policies directly and never rely on
  the flag. The flag is not a TLS bypass and does not
  weaken certificate validation.

## Acceptance criteria (all must pass, sequential, Linux)

1. Design committed before code (this document).
2. Focused Rust: new URL fetch + collider URL tests
   pass; existing collider/transfer/forge_restore/
   backup_restore/backup_artifacts/opportunities_restore/
   entities_roundtrip/lib suites unchanged in result.
3. Node UI/contracts: all pass including new URL
   session/parser tests.
4. Browser: new canonical URL spec passes; prior
   document probes (DI-01/DI-02, rejected-selection),
   canonical document spec, and the affected
   collider/transfer/forge/backup battery pass.
5. Strict Clippy (workspace, all targets, warnings
   denied), fmt, contracts `--check`, TypeScript, docs
   validation, API/web builds pass.
6. Broadened once: full core `--no-fail-fast
   -- --test-threads=1` and providers+API; only the 4
   known live-Ollama `semantic_workflow` environment
   failures may fail.
7. Delivery: checksummed ZIP with the full-history
   bundle, MANIFEST (baseline, tip, scope, evidence,
   Windows checklist), design, and evidence logs;
   independently scratch-verified (ZIP hash, bundle
   hash, exact tip, ancestry, credentials sweep).
   No pushes, master merges, CI, preview promotion,
   generated-code execution, or benchmark campaign.

## Explicitly out of scope

PDF, archives, OCR, transcription, arbitrary downloads,
JavaScript execution, external-resource loading, cookies
or session forwarding, paid services, real external
sites in tests, native file-picker or desktop changes,
M9 work. URL fetching is one beam-input method among
manual text, document input, memory context, and
saved-project context — all of which keep working.
Remaining gates after this slice (stated, not waived):
Windows verification of this slice (including any native
behavior), Windows verification of Slice 4, M7 native
acceptance + preview promotion, M5 production gates,
§32 sandbox, 3D visualization.
