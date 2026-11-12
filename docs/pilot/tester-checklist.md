# ANANSI v1 pilot — tester checklist

Use synthetic data only. Do not use real secrets,
passwords, or irreplaceable data. Work through one
section at a time, in order, and record the result
of every item. A skipped item is NOT TESTED with a
reason — never leave it blank, and never mark it
passed from another section's evidence.

Browser, native desktop, and installer checks are
separate sections because their evidence does not
transfer: a browser pass never closes a native or
installer item, and a Linux or mock result never
closes a Windows item.

Before you start, fill in the build identity from
the `packaging.json` file that came with your
installer (or `release-manifest.json` for an
isolated build). The app version `0.1.0` alone is
never sufficient identification.

- Tester:
- Date:
- Edition(s) tested:
- Installer file name:
- `packaging.json` `gitCommit`:
- Installer SHA-256:
- Application SHA-256:
- Windows version/build:
- Local runtime (if any): [none / Ollama + model
  tag / LM Studio + model]

## Synthetic fixtures

Create these files before Section A. Use the exact
codewords so search and Ask results are checkable.
Reviewers with the source tree should use the
deterministic set from the qualification checklist
instead (step D, `scripts/qualification-fixtures.mjs`);
its contents use the same codewords.

| File | Contents (plain text) |
| --- | --- |
| `field-notes.txt` | `quokka pilot stores records in SQLite with FTS5. bramblewick.` (under 65,536 bytes) |
| `guide.md` | `# Pilot guide` then one line: `bramblewick notes for the quokka pilot.` |
| `unsupported.exe` | Any small file renamed to `.exe` (content does not matter; it must be refused by extension) |
| `oversized.txt` | Any text repeated past 65,536 bytes (for example, 70,000 bytes) |

If a text-layer PDF or DOCX fixture is supplied,
use it in B-06/N-03 as well; otherwise record those
rows NOT TESTED with reason "no fixture supplied".

## How to record results

- PASS: the expected outcome happened exactly as
  written, and the listed evidence was captured.
- FAIL: anything else. File a bug report with the
  [bug-report template](bug-report-template.md)
  and write the report reference in Notes.
- NOT TESTED: the item could not be run. Write the
  reason in Notes (for example, "no local runtime",
  "held by RAM preflight", "no fixture supplied").

Evidence means: the verbatim message shown, the
screenshot path (synthetic data only), the job
state/checkpoint, or the hash recorded — whatever
the item lists. Do not record keys, tokens, or
private paths.

## Section A — Browser checks (paired local API)

Run the local API on the same computer, open the
browser client, and pair on the Core connection
screen (Local API address + Session token, then
Connect to core). If you cannot start the local
service, mark the whole section NOT TESTED with
the reason; do not substitute desktop evidence.

| ID | Steps | Expected outcome | Evidence to capture | Result / Notes |
| --- | --- | --- | --- | --- |
| A-01 | Pair with the session token; open Connection. | Connected. Core connection panel shows Application, Core version, Operating system, Core uptime, Contract version, Application version. | Panel values (versions only). | NOT TESTED / |
| A-02 | Create a workspace through all four onboarding steps; try Continue to AI setup before ticking the privacy checkbox. | Step order: starting point, create/reopen, privacy review, AI setup. Continue to AI setup is disabled until "I have reviewed these privacy limits" is ticked. Skip AI setup and enter ANANSI lands on the Dashboard. Privacy text states plaintext storage and no encrypted vault. | Screenshot of the privacy review; note the disabled-button observation. | NOT TESTED / |
| A-03 | Open Memory and Documents in the fresh workspace; create a project (New project, Project title, Description & objectives, Save project) and a note (New note, Note title, Note text, Save note). | Empty states read "No saved memories on this page." and "No imported documents yet." Project and note save and appear in their lists with revision numbers. | Project title, note title, revision numbers shown. | NOT TESTED / |
| A-04 | Open Local Models; choose Re-probe System. | Hardware profile shows CPU, RAM, storage, graphics. Runtime badges read Active, Offline, or Unsupported with the documented help text. Record which case you observed; Offline with no runtime installed is a PASS for this item, not a failure. | Runtime names, badge states, models listed (if any). | NOT TESTED / |
| A-05 | Open AI Providers; start Connect AI; reach Credential setup; choose Generate reference. | Credential setup is refused in the browser with a message directing you to the desktop app. No key entry is possible. Continue without a credential proceeds. | Verbatim refusal message. | NOT TESTED / |
| A-06 | In Documents, try `unsupported.exe`, then `oversized.txt`, then import `field-notes.txt` and `guide.md` (Review import, Import document). If supplied, also import the PDF/DOCX fixtures. | EXE refused with "EXE files are not supported yet. ...". Oversized TXT refused with "The file must be at most 65536 bytes." Supported files reach Completed and appear under Imported documents. | Verbatim refusal messages; job states; imported file names. | NOT TESTED / |
| A-07 | In Search, run Title only and Full-body search for `quokka`; then search `nonexistentcodeword` in Full-body mode. | Title mode finds titles containing the word (if any). Full-body search (Search knowledge) finds `field-notes.txt`. The nonsense word returns "No matching records. Try different words or filters." | Result counts and titles; verbatim empty message. | NOT TESTED / |
| A-08 | In Ask ANANSI, use a dead loopback endpoint (a port with nothing listening) and ask "What does the quokka pilot store records in?" Open View What ANANSI Sent (Disclosure). | OFFLINE banner with "Local AI runtime is offline. Grounded context compiled below." Citations name `field-notes.txt`. The manifest shows runtime, model, characters, estimated tokens, routing reason, sources, and the compiled prompt. No answer is invented. | Banner text; first citation title; manifest fields present. | NOT TESTED / |
| A-09 | Only if a local runtime with a model is available: ask the same question with the real endpoint and full model tag. | ANSWERED banner; answer text; citations present. | Answer text (synthetic); citation titles. If no runtime, NOT TESTED with reason. | NOT TESTED / |
| A-10 | Only if a generation runs long enough: choose Cancel generation while Ask is busy. | Ask Question becomes enabled again; no answer banner appears for the cancelled run. If generation finishes too fast to cancel, record NOT TESTED with reason "completed before cancel". | Observation + timing note. | NOT TESTED / |
| A-11 | In Jobs and export, create a knowledge export (Entire workspace); Pause export, Resume export, then let it Complete; choose Download complete export. Create a second export and Cancel export. | Pause/Resume/Cancel behave as labeled. Completed export downloads with "Complete export verified and sent to your browser downloads." Cancelled job shows Cancelled and offers Retry export; no partial download is offered. | Job states in order; download notice text. | NOT TESTED / |
| A-12 | In Backups, tick the plaintext consent and choose Create backup; save the Completed artifact from Jobs and export; try importing an invalid `.tar` file, then import the real saved file; Preview restore; Restore verified backup into a fresh name; reopen the restored workspace and search `quokka`. | Create backup is disabled until consent is ticked. Invalid archive is refused with a visible error; originals preserved. Preview shows schema, counts, bytes, privacy, SHA-256, conflicts. Restore creates a fresh workspace ID (different from the original). Search works in the restored workspace. | Preview counts + SHA-256; original vs restored workspace IDs; search result. | NOT TESTED / |
| A-13 | Reload the browser tab, re-pair, reopen the workspace. | Pairing was forgotten on reload (token must be entered again). Workspace, project, documents, and search results persist. | Observation. | NOT TESTED / |
| A-14 | In AI Providers, look for any "Continue with ChatGPT" control. | None appears anywhere in the view. | Observation (present/absent). If present, FAIL and file a bug report. | NOT TESTED / |

## Section B — Native desktop checks

Run the installed desktop app with a fresh isolated
storage root and synthetic data only. Steps B-02
to B-04 follow the installer qualification
checklist steps I–J; that checklist is authoritative
for commands and fixture paths — this section
records the customer-pilot result.

| ID | Steps | Expected outcome | Evidence to capture | Result / Notes |
| --- | --- | --- | --- | --- |
| B-01 | Launch the installed app; complete onboarding (Section A, item A-02 steps); create one workspace with a synthetic name. | Same four steps and privacy gate as the browser. First-launch screens and empty states recorded. Readiness values read from the Core connection panel. | Screens observed; panel values. | NOT TESTED / |
| B-02 | Providers > Connect AI > choose a provider > Credential setup. Choose Generate reference; cancel the native prompt; then Set credential (native prompt) with an obviously synthetic key (for example, `synthetic-qualification-key-001`); restart the app; remove the credential; check Unused credentials. | A native Windows credential prompt appears (not a web form). Cancel survives with no partial credential. The synthetic value appears nowhere in the storage root or logs afterwards (grep for it). After restart the connection remains configured. Guarded removal refuses a credential still in use; an unused key can be removed under Unused credentials. | Per-step verbatim observations; grep result (absent). Never record a real key. | NOT TESTED / |
| B-03 | In Documents, use the file picker for each fixture: TXT, MD, PDF, DOCX, Unicode-named file, cancel, unsupported, oversized, empty. | Record for each file whether the picker presented is the native Windows dialog or the in-app control, the outcome, and the exact message shown. Supported files import; unsupported/oversized/empty are refused with messages naming the problem. Cancel imports nothing with no error. Do not assume native-dialog behavior: record what you observe. | Per-file dialog behavior + verbatim message. | NOT TESTED / |
| B-04 | Start two imports of the same PDF in immediate succession, starting the second before the first completes. Record both outcomes. Restart and confirm documents and search survive. | Both outcomes recorded exactly as observed (completed / error + message). This is an open probe: any outcome is recorded, none is assumed. | Both outcomes verbatim; post-restart check. | NOT TESTED / |
| B-05 | In Jobs and export, complete an export and choose Save complete export. | Record whether a native save dialog appears or the in-app download path is used, and where the file lands. The file is hash-verified before publishing. | Dialog behavior; file location shape (no user name); notice text. | NOT TESTED / |
| B-06 | Restart the app; run backup, save, import, preview, fresh restore, reopen (same steps as A-12) in the desktop app. | Same expected outcomes as A-12, observed natively. | Preview counts + SHA-256; workspace IDs; search result. | NOT TESTED / |
| B-07 | In Product Forge (Ideas & Product Forge), only if a provider is configured: open a generated source package's inspection view. | Files are shown as inert text ("ANANSI does not run or preview generated code"). Build, Tests, and Preview report Unavailable with the §32 sandbox explanation. No execution occurs. If no provider, NOT TESTED with reason. | Verbatim unavailable messages. | NOT TESTED / |

## Section C — Installer checks (reviewer-executed)

These items run only on a clean machine or VM, only
by the Windows reviewer, and only after the RAM
preflight passes (free RAM at least 4 GiB). They
are listed here so the pilot decision sheet can
cite them; customer testers do not run them unless
instructed. Commands, preconditions, and evidence
lists are in the
[installer qualification checklist](../handoffs/v1-pilot-installer-checklist.md);
step letters below refer to that checklist.

| ID | Checklist step | Expected outcome | Evidence to capture | Result / Notes |
| --- | --- | --- | --- | --- |
| C-01 | A. Delivery verification | Fetched tip equals the MANIFEST tip; clean tree; ancestry holds; product-code diff empty. | Tip + status output. | NOT TESTED / Held: RAM preflight (see decision sheet). |
| C-02 | B. Preflight and toolchain | Doctor + preflight exit 0; preflight prints VERDICT: PASS. | Toolchain block; preflight output. | NOT TESTED / Held: free RAM 3.07 GiB < 4 GiB at last review. |
| C-03 | C. Isolated build + identity | Build + verify PASS; identity cross-checks equal (installer hash, payload hash, commit, dirty=false, unsigned). | `release-manifest.json`, `packaging.json`, build log. | NOT TESTED / Blocked by C-02. |
| C-04 | D. Deterministic fixtures | Generate + verify PASS for 9 fixtures. | Fixture MANIFEST. | NOT TESTED / (Tooling-only run passed on Windows 2026-11-12; re-run with the candidate build.) |
| C-05 | E. Installer qualification | Script PASS line verbatim (install + UI workflow + uninstall + preserved data). | `installer-verification.json` + PASS line. | NOT TESTED / Blocked by C-03. |
| C-06 | F. Native candidate acceptance (+ G optional guarded launch) | All 7 checks pass; screenshot captured. | Full output + screenshot path. | NOT TESTED / Blocked by C-03. |
| C-07 | H. Fresh install + first launch | SmartScreen observation recorded verbatim; default per-user install completes; first launch works. | SmartScreen text + clicks; install notes. | NOT TESTED / Blocked by C-03. |
| C-08 | K. Upgrade preserving data | Same-installer reinstall preserves synthetic data and registration. (Same-version reinstall; a true version upgrade cannot be tested until the version increments.) | Pre/post data check. | NOT TESTED / Blocked by C-03. |
| C-09 | M. Uninstall preserving storage | App, uninstaller, and registration gone; storage directory byte-identical. | Before/after hash lists. | NOT TESTED / Blocked by C-03. |

## Section D — Behaviors that must hold in every edition

Run each item in every edition you test, and record
the edition with the result.

| ID | Steps | Expected outcome | Evidence to capture | Result / Notes |
| --- | --- | --- | --- | --- |
| D-01 | Try to select Sealed privacy for a memory and for a document import. | Sealed is disabled ("Sealed — vault unavailable") in both places. | Observation per view + edition. | NOT TESTED / |
| D-02 | In Backups, try Create backup without ticking the plaintext consent. | The button is disabled until consent is ticked. | Observation + edition. | NOT TESTED / |
| D-03 | Only if a provider is configured: attempt an AI task with a new cost-unknown model while the Zero-Cost Lock is on. | The task is refused (unknown treated as paid) until the model is verified free or a paid permit is issued. If no provider, NOT TESTED with reason. | Verbatim refusal + edition. | NOT TESTED / |
| D-04 | If any Queued job shows a recovery notice with Retry recovery: follow the troubleshooting recovery steps and record the outcome. | The notice message, attempts count, and Retry recovery behavior match the troubleshooting guide. If no notice appears during testing, NOT TESTED with reason "no recovery notice observed" — do not break storage to force one. | Message + attempts + outcome + edition. | NOT TESTED / |

## Result summary

Count the items per section after testing. A section
with any FAIL is not passed. A section with NOT
TESTED items is incomplete; the reasons decide
whether the pilot can proceed (see the
[release decision sheet](release-decision-sheet.md)).

| Section | PASS | FAIL | NOT TESTED |
| --- | --- | --- | --- |
| A — Browser | 0 | 0 | 14 |
| B — Native desktop | 0 | 0 | 7 |
| C — Installer | 0 | 0 | 9 |
| D — Every edition | 0 | 0 | 4 |
| Total | 0 | 0 | 34 |

Tester sign-off (name, date, build identity as
above). Attach the completed checklist and any bug
report references to the pilot record.
