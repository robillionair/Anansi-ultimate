# ANANSI v1 pilot — bug-report template

Copy everything below the line, fill it in, and send
it to the pilot organizer. Use synthetic data only.
Never include API keys, pairing tokens, passwords,
or private documents — in the steps, the logs, or
screenshots. If a step needs a key, write "[API key
entered in the native prompt]" and nothing more.

The app version `0.1.0` alone cannot identify your
build. Copy the identity values from the
`packaging.json` file that came with your installer
(or from `release-manifest.json` for an isolated
build). If you cannot find `packaging.json`, say so
explicitly instead of guessing.

---

## Bug report — ANANSI v1 pilot

### Build identity (required)

- Installer file name:
- `packaging.json` `gitCommit` (40 hex characters):
- `packaging.json` `dirtySource` (must be `false`
  for a pilot build):
- Installer SHA-256 (`installerSha256`):
- Application SHA-256 (`applicationSha256`):
- Config SHA-256 (`configSha256`):
- Kind (expected `unsigned-development-nsis`):
- App version shown in the app (expected `0.1.0`):
- Contract version shown in the app (expected `1`):
- Core version / API version from the Core
  connection panel (Connection view):

### Environment

- Edition: [desktop installer / browser + local API]
- Windows version and build (for example, from
  Settings > System > About):
- Computer: [clean test machine/VM / personal
  machine] (do not include your user name or full
  profile path; write paths with the user folder
  replaced by `<user>`)
- Local runtime, if relevant: [none / Ollama
  version + model tag / LM Studio version + model]
- Provider, if relevant: [provider + model id only;
  never the key or endpoint credentials]
- Storage: [default storage root / isolated test
  root] (give the shape of the path only, for
  example `<LOCALAPPDATA>\ANANSI\workspaces` or
  `E:\Anansi\previews\...`)

### Severity

Pick one. Severity describes impact on the pilot,
not how annoyed you are.

- Blocker: pilot workflow cannot complete, data
  loss or corruption, a credential or private
  record is exposed, or the app cannot start.
- Major: a pilot workflow fails or gives wrong
  results, but a workaround exists using documented
  behavior.
- Minor: a message, label, or empty state is wrong
  or confusing, but the workflow completes.
- Question: behavior is unclear from the
  documentation; you are unsure whether it is a
  defect.

Severity:

### Summary

One sentence: what happened, in which view, while
doing what.

### Reproduction steps

Numbered, exact, starting from a known state
(for example, "fresh workspace, no provider").
Use the exact button and field labels. Include the
synthetic data used (file names, sizes, search
words, question text). A report without steps that
someone else can follow cannot be fixed.

1.
2.
3.

Reproduces every time? [yes / sometimes / once]

### Expected behavior

What the user guide, limitations sheet, or
troubleshooting guide says should happen. Quote or
name the section.

### Actual behavior

What actually happened. Quote error messages
verbatim, including the full text shown in the app.
Note the job state, checkpoint line, and job log
for job problems; the status banner for Ask
problems; and the preview counts for restore
problems.

### Sanitized evidence

Attach or paste only evidence with secrets removed.
Check every line before sending.

- Screenshot(s) of the error or wrong state, with
  synthetic data only. Cover or crop anything
  private. Never screenshot the native credential
  prompt with a key visible, the pairing file, or
  a session token.
- Exact on-screen messages (copy the text; do not
  paraphrase):
- Job checkpoint/log lines, if relevant (copy from
  Jobs and export; these contain no secrets):
- Readiness values from the Core connection panel,
  if relevant:
- Installer/SmartScreen observation, if relevant
  (exact prompt text and what you clicked):
- What you already tried from the troubleshooting
  guide, and what changed:

### Privacy check (confirm before sending)

- [ ] No API key, pairing token, password, or
      credential reference secret is included.
- [ ] No private document content is included;
      all file contents described are synthetic.
- [ ] No full user profile path is included
      (user folder replaced by `<user>`).
- [ ] Screenshots show synthetic data only.
- [ ] The build identity above comes from
      `packaging.json` / `release-manifest.json`,
      not from memory.

### Additional notes

Anything else that might matter: what changed just
before the problem, whether a restart changes it,
whether it happens in both desktop and browser (if
you tested both — say so explicitly if you did
not).
