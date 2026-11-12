# ANANSI v1 pilot — troubleshooting guide

Use this guide with the
[first-time-user guide](user-guide.md) and the
[limitations and privacy sheet](limitations-and-privacy.md).
Section numbers in parentheses refer to the user
guide. Every label, message, and behavior below is
taken from the current app; if the app behaves
differently from what is written here, that is a
defect — report it with the
[bug-report template](bug-report-template.md).

General first steps for any problem:

1. Read the exact on-screen message and write it
   down verbatim, including the view you were in
   (Dashboard, Documents, Jobs and export, and so
   on).
2. Note what you expected, what actually happened,
   and the last button you pressed.
3. Do not paste API keys, pairing tokens, or
   private document contents into a report or a
   screenshot. Synthetic data only.
4. Do not delete your storage folder, edit files
   inside it, or copy a live database file while
   ANANSI is running. See "Safe rollback" below.

## No provider configured

Symptom: an AI workflow says no provider is
available, or AI Providers shows "No providers
connected yet." / "No provider status available."

What it means: no AI setup is required to use a
workspace, and a fresh workspace has no provider.
This is expected, not a defect.

What to do:

1. If you do not need AI, continue without it:
   notes, search, documents, jobs, and backups
   all work with no provider.
2. For local AI, follow user guide section 4a:
   start Ollama or LM Studio first, then open
   Local Models and choose Re-probe System.
3. For cloud AI in the desktop app, follow user
   guide section 4b (Connect AI). In a browser
   session, credential setup is refused by design;
   use the desktop app.
4. After setup, check that the Zero-Cost Lock
   state is what you intend. A new model starts
   cost-unknown and is treated as paid, so the
   lock blocks it until it is verified free or a
   paid permit is issued. That refusal is correct
   behavior.

## Unreachable local runtime (Ask shows OFFLINE)

Symptom: Ask ANANSI shows the OFFLINE banner:
"Local AI runtime is offline. Grounded context
compiled below." Citations are still shown, but
there is no generated answer.

What it means: ANANSI compiled the evidence but
could not reach the local runtime. It does not
invent an answer. This is correct behavior.

What to do:

1. Check that Ollama or LM Studio is actually
   running on this computer.
2. In Ask ANANSI, check the Local inference
   endpoint. The usual Ollama endpoint is
   `http://127.0.0.1:11434`. The endpoint must be a
   loopback address on this computer.
3. Check the Installed Ollama model field: enter
   the full model tag exactly as the runtime
   lists it, including the `:tag` suffix. A bare
   name without its tag may not match.
4. Open Local Models and choose Re-probe System.
   The runtime badge should read Active and list
   Installed Models. Offline means "Service is
   not running on this port. Start the application
   locally to enable inference." Unsupported
   means "The endpoint responded with an
   unsupported or unexpected contract."
5. Ask the question again. If the runtime is
   Active in Local Models but Ask still shows
   OFFLINE, copy the exact endpoint and model tag
   you used (no keys involved) and report it.

If Ask shows INSUFFICIENTEVIDENCE ("No relevant
memories found in this workspace."), the runtime
is not the problem: no relevant material was
found. Add or import relevant material, confirm
it is searchable in Search first, then ask again
with a narrower question.

## Invalid or oversized documents

Symptom: importing a file is refused with an
error message, or the import never starts.

Refusals happen before upload for text files, and
the message names the problem. Common messages
and what to do:

- "EXE files are not supported yet. Only .txt,
  .md, text-layer .pdf and .docx files can be
  imported in this slice. Other formats are
  tracked as follow-up work." — The format is
  not supported. The suffix in the message is
  your file's extension (for example, `EXE`,
  `PNG`, `ZIP`). Convert the content to `.txt`
  or `.md`, or use a supported file.
- "The file must be at most 65536 bytes." — A
  text or Markdown file over 65,536 bytes.
  Split it into smaller files, each under the
  limit.
- "The file must be 1 to 1048576 bytes." — A PDF
  or DOCX file that is empty or over 1 MiB.
  Use a smaller file.
- "The file's text must be at most 8000
  characters." — A text or Markdown file whose
  decoded text exceeds 8,000 characters even
  though the file fits the byte limit. Split it.
- "The file is not valid UTF-8 text." — The
  text file is not strict UTF-8. Re-save it as
  UTF-8 without replacement characters and try
  again. A single leading UTF-8 BOM is allowed:
  it is stripped and recorded.
- "The file is empty or contains only
  whitespace." — The file has no importable
  text. Add content and try again.
- "The file name must be a plain file name, not
  a path." — The file name contains a path
  separator or is over 255 characters. Rename
  the file to a plain name and try again.
- A PDF imports but yields no text, or far less
  text than expected. Scanned or image-only PDFs
  have no text layer; they need OCR, which is a
  separate slice and is not in this pilot. Use a
  text-layer PDF, or copy the text into a `.txt`
  file.
- "Sealed — vault unavailable" (disabled
  privacy option). Sealed import is refused
  until an encrypted vault path exists. There is
  no encrypted vault in this pilot; import as
  Private, Standard, or Public (not published),
  knowing the data is stored as plaintext.

If a file within the limits for its format is
refused with a different message, record the
exact message, the file extension, and the byte
count shown in Review import, and report it. Do
not attach the file itself if it contains
anything private; use a synthetic file that
reproduces the refusal.

## Queued or parked imports

Symptom: an import job stays in Queued, or
Import progress shows a state that does not
advance.

What to do:

1. Choose Refresh import (Documents) or Refresh
   jobs (Jobs and export). Job state updates on
   refresh; the view does not live-update.
2. Read the Checkpoint line and the job log
   shown under the job. They state where the job
   stopped.
3. Check whether a recovery notice is shown. If
   one is, follow "Recovery" below; do not just
   wait.
4. If the job is Paused, it says "Uploaded X of
   Y bytes". Resume it explicitly: in Jobs and
   export, choose Resume export. Paused jobs do
   not resume on their own.
5. If the upload was interrupted (for example,
   the browser was closed), the job appears
   under Interrupted uploads in Documents. Pick
   the same file again, then choose Resume
   upload. Completed parts are not re-sent.
6. Imports run as persistent jobs and are
   bounded: at most 8 concurrent ingestion
   uploads per workspace, and at most 32 pending
   jobs overall. If you started many imports at
   once, later ones wait in Queued until earlier
   ones finish. Let them finish, or cancel the
   ones you do not need.
7. Restarting the app does not lose a job: jobs
   are durable. After a restart, reopen the
   workspace and refresh Jobs and export. An
   in-flight job resumes or parks truthfully; it
   never silently reports Completed.

A job that stays Queued with no recovery notice,
no Paused state, and no other jobs ahead of it
after a restart and a refresh is worth reporting,
with the checkpoint line and job log copied
verbatim.

## Cancellation

Symptom: you cancelled something and want to know
what actually stopped.

What the app guarantees: cancelling publishes
nothing and leaves no phantom Completed state.

- **Ask generation.** Choose Cancel generation
  while Ask ANANSI is busy. The Ask Question
  button becomes enabled again and no answer
  banner appears for the cancelled run. A
  partial streamed answer is discarded.
- **Document import.** Choose Cancel import in
  Import progress, or Cancel export in Jobs and
  export while the job is Queued, Running, or
  Paused. The notice reads "Import cancelled. No
  document was published." (or, for an upload in
  progress, "...and the upload staging was
  removed."). No document record is created.
- **Export/backup job.** Choose Cancel export in
  Jobs and export. A cancelled job can later be
  retried with Retry export, which starts it
  again; retry does not resurrect partial output.
  Partial files are never downloadable: only a
  Completed job offers Save/Download complete
  export, and the download is hash-verified
  before it is published.
- **Credential prompt.** Cancelling the native
  Windows credential prompt stores nothing. The
  Connect AI flow reports "Credential entry was
  cancelled." and no partial credential remains.
  You can retry Generate reference / Set
  credential (native prompt), or Discard draft
  to leave the setup.
- **File picker.** Cancelling the file picker
  imports nothing, shows no error, and leaves the
  app stable. If cancelling produces an error
  message, record it verbatim and report it.

If a Cancelled state appears while backend work
is visibly still running (for example, progress
keeps advancing after cancellation), record the
job id, state, checkpoint, and times, and report
it as a defect. Do not work around it by deleting
storage files.

## Recovery (Retry recovery)

Symptom: a Queued job shows a red recovery notice
with an "Attempts used" count and a Retry
recovery button.

What it means: the job's storage root exhausted
claim recovery — the worker tried to claim the
job, storage failed, and automatic retries
stopped rather than spinning forever. The notice
carries no filesystem detail and no record
contents. The button appears only while the job
is still Queued.

The two messages, verbatim:

- Storage unavailable (3 attempts used):
  "Recovery stopped because storage was
  unavailable. Check the storage folder and
  permissions, then choose Retry recovery."
- Storage could not be read (1 attempt used):
  "Recovery stopped because storage could not
  be read. Keep the original files and restore a
  verified backup, then choose Retry recovery."

What to do:

1. For the storage-unavailable message: check
   that the storage folder exists and is
   writable, and that no other copy of ANANSI is
   using the same storage root. Then choose
   Retry recovery. Retry recovery is a
   deliberate re-arm: it clears the exhausted
   state for that root and lets the worker try
   the job again.
2. For the storage-could-not-be-read message:
   keep the original files untouched. Restore a
   verified backup into a fresh workspace (user
   guide section 8) before choosing Retry
   recovery. Do not delete, move, or "repair"
   the storage files by hand.
3. If Retry recovery leads to the same notice
   again, stop retrying. Record the exact
   message, the attempts count, the job's
   checkpoint line, and whether a restart
   changes anything, and report it. The
   exhausted state lives in the running process,
   so a restart clears it; if the notice returns
   immediately after a restart, that itself is
   useful evidence.
4. The same notice appears in Jobs and export
   and in the Documents Import progress section,
   with the same message and count. If the two
   views disagree, report that too.

## Failed restore

Symptom: Import saved backup is refused, Preview
restore shows unexpected counts, or Restore
verified backup fails.

What to do:

1. **Import refused.** Import verifies every
   member of the `.tar` file before it becomes
   available for preview. An invalid archive, an
   unknown schema, a hash-invalid member, or a
   backup containing Sealed content fails closed:
   nothing is imported and originals are
   preserved. Check that the file is the complete
   `.tar` saved from Jobs and export (up to
   256 MiB), that it was not edited, renamed
   mid-download, or truncated, and try the import
   again from the intact file. Keep the original
   file; do not try to unpack or fix the archive
   by hand.
2. **Preview counts look wrong.** The Verified
   restore preview shows the workspace name,
   schema version, workspace/record/revision/
   relationship counts, total bytes, privacy
   summary, SHA-256, and conflicts. Compare the
   counts with the backup job's record before
   restoring. If they do not match what you
   backed up, do not restore: record the preview
   values and the backup job's details and
   report it.
3. **Restore fails.** Restore creates fresh
   workspace IDs and preserves all original
   workspaces; a failed restore leaves the
   originals untouched. Note the exact error
   message and whether the restored workspace
   name you entered was accepted. Restoring the
   same backup artifact a second time into the
   same workspace is refused as a conflict rather
   than duplicating data; use a fresh workspace
   name if you need a second restored copy.
4. **After restore, something is missing.** Reopen
   the restored workspace from Workspace
   (Existing workspace) and check Documents and
   Search there — not in the original workspace.
   Restored schedules start disabled; that is
   expected. If records present in the preview
   counts are absent after reopen, record the
   preview counts, the restored workspace name,
   and what is missing, and report it.
5. **Never downgrade storage by hand.** Never
   open a newer-schema workspace with an older
   build expecting it to work, never copy a live
   database file alone, and never edit the
   storage files directly. The app refuses
   foreign schemas; that refusal is correct.

## Safe rollback

Rollback means returning to the previous build
and the pre-upgrade data. Never build a rollback
from a dirty or mixed state.

Before any upgrade test:

1. Take a backup (user guide section 8) and save
   the backup file outside the app. Keep it until
   the upgrade test is done.

To roll back the app:

1. Uninstall the candidate from Windows Settings
   > Apps (or the bundled uninstaller).
2. Confirm the user storage root survived:
   uninstall must not delete user data. Check
   that the storage folder still exists with its
   contents before reinstalling anything.
3. Reinstall the previous build from its own
   installer and `packaging.json` identity.
   Record both builds' commits and installer
   hashes.
4. Reopen the workspace and verify documents,
   search, and projects are intact.

To roll back the data:

1. If the storage root was opened by a newer
   build, do not force the older build onto it.
   Restore from the pre-upgrade backup file
   instead (user guide section 8), into fresh
   workspace IDs.
2. If the storage root was never opened by the
   newer build, reopening it with the previous
   build is safe; verify before continuing.

Downgrades are blocked by the installer
(`allowDowngrades: false`). A downgrade attempt
that is refused rather than silently applied is
correct behavior; record the refusal message.

## Problems not covered here

If the app shows a blank screen, crashes, or
behaves in a way no section above describes:

1. Write down the exact steps, the last screen
   you saw, and any message shown.
2. Restart the app once and check whether the
   problem repeats with the same steps. Note
   whether it repeats.
3. Check the Core connection panel (Connection)
   for the Application/Core/Contract versions.
4. Report it with the
   [bug-report template](bug-report-template.md),
   including your build identity from
   `packaging.json`. Do not include keys, tokens,
   or private content.
