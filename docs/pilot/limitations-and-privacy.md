# ANANSI v1 pilot — limitations and privacy sheet

Read this before putting anything real into ANANSI.
Every statement here was checked against the current
code and recorded evidence for the v1 pilot. If the
app behaves differently from what is written here,
that is a defect — report it with the
[bug-report template](bug-report-template.md).

Short version: ANANSI keeps your data on your
computer in plaintext. There is no encryption, no
signed release, and no production certification in
this pilot. AI features send only what you approve,
to a provider you configured, and show you what was
sent. API keys live only in the operating system
credential store. ChatGPT sign-in and generated-code
execution are not available.

## 1. Storage: plaintext, on this computer

- Workspaces, memories, notes, projects, documents,
  history, and settings are stored as plaintext on
  this computer. On Windows the default storage
  root is `%LOCALAPPDATA%\ANANSI\workspaces`. A
  different root applies only when the app was
  launched with one (for example, an isolated test
  root); never edit the files inside the storage
  folder by hand.
- Local file permissions are the only access
  protection. This is not an encrypted vault.
  Anyone who can read the files on the computer
  can read the workspace contents. Disk encryption
  and application lock are different things and
  are not provided by ANANSI.
- Privacy labels (Private, Standard, Public) are
  classifications, not protection: they do not
  encrypt files, do not grant sharing consent, and
  do not publish anything. "Public (not published)"
  means exactly that.
- Sealed records fail closed everywhere: Sealed
  content is excluded from search, Ask evidence,
  exports, and backups, and Sealed import is
  refused ("Sealed — vault unavailable"). The
  encrypted vault itself does not exist in this
  pilot; the isolated vault core is not integrated
  into the application. Do not rely on Sealed as
  a working privacy control — it is a refusal, not
  a store.
- There are no automatic backups. Scheduled
  backups are off by default, run only while the
  desktop app or local API is open, and require
  your explicit plaintext consent. A backup on the
  same disk cannot protect against losing that
  disk.
- Backups and exports are plaintext snapshots
  (TAR archives). Treat backup and export files as
  data-bearing: store them accordingly and never
  share them as if they were encrypted. Sealed
  records and credentials are refused from
  backups and exports.
- Onboarding makes no AI or cloud requests.
  Workspace names and preferences stay on this
  computer. Continuing through onboarding does not
  authorize future cloud sharing, model downloads,
  or recording of drafts; each of those requires
  a separate choice.
- The browser client uses a paired local service
  on the same computer. Pairing is kept in the
  tab's memory and forgotten on reload. Never
  share the pairing file or session token.

## 2. What is sent to AI providers

- Nothing is sent to any provider until you
  configure one and authorize a specific use.
  Title and full-body search send nothing to an
  AI provider. Hybrid search sends nothing to an
  external AI provider (it uses a local embedding
  model you name).
- Ask ANANSI sends only to a local loopback
  runtime (Ollama / LM Studio) on your computer,
  never to a cloud provider from that view. It
  compiles at most 4,000 characters of workspace
  evidence into the prompt.
- Other AI workflows (idea generation, opportunity
  discovery, Product Forge, LEXI-COLLIDER, and
  similar) send their inputs and only the context
  you explicitly select or approve to the provider
  you configured. Each workflow shows what will be
  sent before sending, and a "View what ANANSI
  sent" disclosure afterwards naming the provider,
  model, endpoint, cost class, and the exact
  compiled prompt or inputs. Check the disclosure
  before trusting a result.
- The Zero-Cost Lock is on by default. Paid
  candidates are refused until verified free or
  covered by a paid permit you issue. A new model
  starts cost-unknown and is treated as paid, so
  the lock blocks it until verified. ANANSI never
  assumes an unverified model is free. Paid use
  always needs your explicit permission and a
  budget.
- Model answers are generated from the evidence
  shown in their citations. Citations do not
  guarantee correctness. Check cited sources
  before acting on an answer. No model-answer
  quality, token-efficiency, or benchmark claim
  is made for this pilot.
- ANANSI never downloads a model. Local runtimes
  and their models are installed and managed by
  you, outside ANANSI. Discovery only inspects
  hardware and probes loopback endpoints you
  request.
- No telemetry claim is made for the whole
  product. The Ask view's local-runtime statement
  ("Zero telemetry, zero cloud data transfer")
  describes that view's loopback path only; any
  provider workflow you authorize does send its
  approved inputs to that provider, as disclosed.

## 3. Credentials: OS store only, never in the app

- API keys are entered only through the trusted
  native Windows credential prompt (CredUI) in the
  desktop app, reached from AI Providers >
  Connect AI > Credential setup > Set credential
  (native prompt). The key moves from the prompt
  into the operating system credential store and
  is never typed into, or sent through, the app
  page, never crosses an HTTP/IPC message, and is
  never written to ANANSI's files, logs, or
  configuration.
- Configuration holds only the credential
  reference (the keyring account label, 1–128
  characters). The secret is resolved inside the
  backend at dispatch time, from the provider
  keyring service. Provider configuration files
  live under the storage root's `provider-config/`
  folder and contain references and settings only.
- In a paired browser session, credential setup
  is refused with a direction to use the desktop
  app. No key entry exists in the browser. If any
  browser screen asks you to type a key, stop and
  report it.
- Cancelling the native prompt stores nothing and
  leaves no partial credential.
- Removing a credential is guarded: a credential
  in use by a saved connection cannot be removed
  until the connection is saved under a different
  credential. Keys stored during a setup that was
  never saved appear under Unused credentials
  (labels only — provider and reference, no
  secret), where Remove unused key deletes the
  key from the OS store.
- Verification status (`connectionVerified`) is
  bound to the saved connection: provider,
  endpoint, credential reference, and endpoint
  security flags. Changing the endpoint or
  removing the credential clears verification.
  Models are not part of the binding.
- Never put an API key, pairing token, or
  password into a bug report, screenshot, chat
  message, or document stored in ANANSI. Support
  never needs your key.

## 4. Unsigned build, no commit in the binary

- The installer is an unsigned development build
  (signature status `NotSigned` is expected).
  SmartScreen and reputation warnings are
  expected; record them, do not bypass them with
  fake signing. Signed releases are later work.
- The installer is per-user (`currentUser`),
  English-only, x64-only, NSIS-only, and embeds
  the WebView2 bootstrapper. Downgrades are
  blocked. A machine without WebView2 needs
  internet access for the runtime install.
- No git commit hash is embedded in the built
  binary. The running app shows version `0.1.0`
  and contract version `1`, which cannot
  distinguish one pilot build from another. The
  exact build identity lives only in
  `packaging.json` (with the installer) and
  `release-manifest.json` (with the isolated
  build): source commit, clean/dirty state,
  installer and application hashes, config hash,
  toolchain versions, and unsigned status. Keep
  `packaging.json` with the installer, and cite
  the full identity in every bug report.
- This pilot has no external security audit, no
  production certification, and no installer
  qualification claim yet. Native credential
  store, native file dialog, clean-machine
  install/upgrade/uninstall, and live-provider
  verification are open gates; see the
  [release decision sheet](release-decision-sheet.md).
  The app footer states: "This development preview
  is not production-certified."

## 5. Supported formats and actual limits

Document import (Documents view):

| Format | Raw file limit | Extracted text limit | Notes |
| --- | --- | --- | --- |
| `.txt` | 65,536 bytes | 1–8,000 characters | Strict UTF-8; one leading BOM stripped and recorded; empty/whitespace-only refused |
| `.md` | 65,536 bytes | 1–8,000 characters | Same rules as `.txt` |
| `.pdf` | 1,048,576 bytes (1 MiB) | 1–32,000 characters | Text layer only, extracted server-side; scanned/image-only PDFs need OCR (not in pilot) |
| `.docx` | 1,048,576 bytes (1 MiB) | 1–32,000 characters | Text-bearing DOCX, extracted server-side, with heading provenance |

- Only those four formats are supported. All other
  formats are refused with a message naming the
  extension. File names must be plain basenames
  (no path separators, at most 255 characters);
  no filesystem path is read, sent, or stored.
- Uploads are chunked (64 KiB parts), resumable,
  and cancellable, with at most 8 concurrent
  ingestion uploads per workspace.
- Other actual limits: Ask questions up to 500
  characters with a 4,000-character evidence
  budget; memory/note text up to 256 KiB; title
  search up to 8 words / 256 bytes; full-body
  search up to 16 terms or phrases / 512 bytes;
  searches bounded to 10,000 eligible records;
  exports and backups bounded to 10,000 records
  and 256 MiB per archive, with at most 32 pending
  jobs; saved backup files up to 256 MiB;
  workspace names 1–64 characters.
- Single-user local deployment. No multi-machine
  sync, no hosted multi-user deployment, no macOS
  build in this pilot.

## 6. Explicitly unavailable

- **ChatGPT sign-in ("Continue with ChatGPT").**
  Deferred and disabled for the first pilot. No
  sign-in button is offered in any normal build;
  the prototype is dev-gated only, and
  "open-source partner" eligibility is unverified.
  Do not test for it as a pilot feature. If a
  control with that name appears, report it.
- **Generated-code execution.** Product Forge
  generates source packages for review only.
  ANANSI never builds, tests, previews, executes,
  or deploys generated code; there is no execution
  sandbox. Exported source is inert text: review
  it before any use, and never run it with real
  credentials or important data. The only
  prototype template is a static web prototype.
- **2.0 work (planning only).** ANANSI Corporate,
  Nanzi, and Enterprise Workspaces are plans, not
  features. No code, no claims, no pilot
  availability. Likewise: OCR/transcription,
  broad document connectors, background ingestion,
  the AI Matrix workflow, Big Brain deliberation,
  signed installers/updates, and macOS are not in
  this pilot.
- **Encrypted vault / Sealed storage.** Not
  available, as stated in section 1. Sealed is a
  refusal everywhere, not a working store.

## 7. What the pilot does not prove

- Passing the pilot checklist does not certify
  security, performance, accessibility breadth,
  or production readiness. It shows the named
  workflows behave as documented on synthetic
  data, on the named build.
- Linux evidence never closes a Windows item, and
  browser evidence never closes a native desktop
  or installer item. The
  [tester checklist](tester-checklist.md) keeps
  those sections separate for exactly this reason.
- Synthetic test results (including any persona,
  simulation, or comparison output) are comparison
  material, not real-world validation. Persona and
  simulation outcomes are not market evidence.
