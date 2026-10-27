# M7 preview discovery correction

Baseline: Windows native preview `bf6f1fd9c`.
Scope: make the existing project-scoped ideas/Forge workflow discoverable and
correct obsolete onboarding claims. No new engine, execution capability, consent,
credential flow, schema or provider is introduced.

Acceptance criteria before implementation:

1. A named Ideas & Product Forge navigation action opens the existing Projects
   view only after workspace onboarding; no AI request starts on navigation.
2. Product Forge is no longer labelled wholly unavailable under planned tools.
3. Welcome/AI setup explains existing local-model/provider screens, optional
   configuration and manual source review; generated execution stays unavailable.
4. Privacy copy distinguishes onboarding from optional inference and vault setup;
   continuing grants no cloud sharing, downloads or recording consent.
5. Existing keyboard onboarding/reload/responsive browser tests plus UI/contracts,
   TypeScript and web build pass, with a browser assertion exercising the shortcut.
6. Existing personal data and previews remain intact. No generated code runs,
   real credentials, downloads, paid inference, remote CI or pushes.

This is M7 acceptance polish, not approval of the deferred execution environment.
Results follow after checks; do not infer certification from wording changes.

## Verification record

UI/contracts 60/60, TypeScript, generated-contract drift, web build and docs
validation passed. The contract generator retains its existing ignored-format
warnings; generated files were unchanged. Browser onboarding 3/3 passed, including
the new shortcut, keyboard/focus, narrow layout, privacy review gating, skip-AI,
durable intent/motion preferences, invalid-name recovery and unfinished reload.
The privacy wording received one final clarification afterwards (inputs and
selected context); UI tests/typecheck/contracts/web build passed on that text.

Windows embedded desktop build passed, normal `custom-protocol`, no test-provider
or CDP feature. SHA-256:
`d8d1ed66795c0f3dc2e9c85442311348529293ffd1cf51cc7f13a7c216665c96`.
The previous idle, integrator-launched executable was closed before rebuild and
preserved under `E:\Anansi\build-cache\rollback-m7-native-bf6f1fd9c`.
Personal root and old M7 process/data were not changed by the build.

Additional browser regression run while the native build was active:
revision passed (restart and fresh restore included); deliverable failed waiting
8 seconds for its approval button after selecting the saved specification.
No timeout/assertion was changed. Original diagnostic preserved outside source:
`E:\Anansi\deliveries\m7-preview-discovery-evidence\initial-deliverable-diagnostic.md`.
An idle deliverable-only rerun is recorded separately below; the first failure
is retained rather than silently counted as a pass.

Idle deliverable-only rerun passed 1/1 in 3.0 minutes with the exact original
8-second action timeout and 240-second test limit, no code/assertion changes.
It covered specification approval, documentation generation/edit/save/export,
version switching and Cancel/reopen, API restart and fresh backup/restore.
Thus the initial failure did not reproduce in the idle run; resource contention
is a plausible contributor, not an established root cause. No retry was hidden
and no backend behavior was changed to obtain the pass.

Rebuilt personal desktop launched successfully with the existing separate root.
Actual native welcome inspection confirmed the CREATE shortcut (disabled until a
workspace is ready), updated idea/source-review descriptions and generic
Development preview label. It was left open for the owner. Shortcut activation
after onboarding is browser-verified; this native check does not claim a new
privacy-consent/onboarding interaction. Prior full synthetic native Forge evidence
remains in the baseline record. No new native acceptance of real inference or
credentials is claimed. Source, tests and guide are recoverable on
`codex/m7-preview-discovery`; implementation checkpoint `a67487d8a`.
