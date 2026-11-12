# ANANSI v1 pilot RC — focused Windows review, 2026-11-12

Verdict: PASS for package integration, build prerequisites, TypeScript/web/API builds, and the synthetic browser pilot journey. NOT installer or native acceptance.

Runtime/source: `8931d34b06faf16806a089a587ebd58e6b26f9a5` (`muse/v1-pilot-release-candidate`). Incoming ZIP SHA-256 `7d2a4bcf9d36b975fc2d6bddbe231f1da5cb30b40e7512e2d4ba6a9d0331e39a`. All 16 MANIFEST hashes verified. Full-history bundle verifies and fetched tip matches exactly. Both published baseline `dff9bb16688f3af075c395ad8fd5b179af9c3a9d` and accepted reviewer `8453f649e385472e86055bd8f8a55abdad8c009f` are ancestors. There are zero differences in crates/, apps/, packages/ against accepted reviewer 8453f649.

## Windows results on the delivered source

- TypeScript --noEmit: exit 0.
- Web build: exit 0; existing bundle-size warning retained.
- API debug build --locked: exit 0, 3m03s.
- Native doctor: Windows prerequisites detected; no missing prerequisites. This is discovery, not native launch/installer acceptance.
- Documentation validator: PASS, 380 Markdown files before this review.
- Documentation and hygiene tests: 27/27.
- Unmodified `tests/e2e/pilot-journey.spec.ts`: **1/1**, 2.4 minutes, under the shared application-test lease, synthetic data on E:. Fresh onboarding, project, provider/browser credential refusal, TXT/MD/PDF import and invalid-file errors, search, offline/answered Ask with citations, cancellation, API restart, backup export/import, fresh-ID restore, and reopened search all passed. Zero external browser requests asserted. Model responses came from the test's local fake; this is not model-quality evidence.

The previous 416/416 Windows library aggregate remains scoped to accepted reviewer 8453f649; it was not rerun or relabelled for this candidate. App code is unchanged. No broad UI or provider-suite rerun is claimed.

One reviewer command initially pointed Vite at the root node_modules directory and failed with MODULE_NOT_FOUND; corrected to the existing apps/web dependency directory before building. No product code, assertions, or timeouts were changed.

## Remaining gates

No installer built or installed during this review. Clean-machine/VM installation, upgrade, uninstall/data preservation, SmartScreen observation, native credential-store/CredUI, native file-dialog behavior, concurrent ingestion probe, and standing workflow-specific Windows/native items remain outstanding as listed in the delivery handoff. The candidate is an integrated source checkpoint, not a customer-approved installer. OAuth stays deferred/disabled. No owner data, model calls, credentials, generated execution, promotion, publication, push, or remote CI used.

Next assignment: qualify a version-identified isolated Windows installer using the existing release tooling; native manual checks must report observed results, never browser-test substitutes. Keep the running preview and existing owner storage untouched.
