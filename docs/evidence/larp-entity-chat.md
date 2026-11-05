# LARP Entity chat and Desktop demo — 2026-11-05

Source: `codex/larp-entity-chat` in isolated `E:/Anansi/worktrees/larp-entity`. Merge `043060316` preserves cinematic/interactivity `414f75fd2` and verified MCP4 `b6c6f39ea`; design `fcf3c1bba` preceded entity implementation `900a661f6`. Slow-renderer acceptance `de16a2960` preceded resolution adaptation `3923d2ca8`. The other agent's `review-m8-overnight` worktree was not edited or switched.

## Implemented workflow

Seventh observatory scene: reference-inspired cyan/gold portrait on an animated Three.js relief, with ornamental orbits and actual waiting/streaming/cancelling/error/offline states. Existing authenticated Client APIs handle local workspace-grounded Ask or configured provider streams. Typed provider conversation is bounded and private, Zero-Cost Lock remains enabled, cloud consent resets after each Send and connection change, and connection changes clear history. No keys or session capabilities appear in the UI. Responses render as text, including hostile script strings.

Cancellation keeps watching the backend; interrupted watches retain the capability for Resume/Cancel. Disposal cancels late starts and drains the owned run where connectivity permits; a 10-second disposed-watch bound is not a guarantee of remote cleanup during total network loss. Synthetic provider acceptance proves connection closure and zero reservations after explicit Cancel and scene change. Local Ask uses its existing backend run semantics; this slice does not revise backend cancellation guarantees.

The face is an artwork-based relief with stream-driven motion, not an anatomical character rig, speech lip-sync, cognition, emotion or a view into internal reasoning. No audio support added. [Artwork, built-in image tool and exact generation prompt](larp-entity-artwork.md). [Desktop controls and owner guide](../handoffs/larp-entity-desktop-guide.md).

## Verification

- Final UI suite: **314/314** (including input-overhead display); earlier focused controller/compositor subset: **24/24**. TypeScript, generated-contract drift, documentation consistency, web build and diff checks passed. Existing web chunk-size warning remains.
- Entity browser workflow: **2/2** on the initial corrected synthetic harness and **2/2** in the combined run, covering real local/API transport, escaped output, disclosure, no mount traffic, transcript isolation, cancellation socket closure/reservation settlement, scene cleanup, reduced motion, pre-network model refusal and offline/WebGL fallback. These are synthetic workflow checks, not real-model quality or provider certification.
- Post-adaptation original all-six and navigation browser workflows: **3/3**, unchanged assertions and timeouts, 3.2 minutes. They verify live graph refresh, actual Ask telemetry, instrument hover/resize, 2D fallbacks and camera orbit/zoom/pan/keyboard/reset without click-through.
- Explicit demo-profile auto-start: **1/1**, 28.2 seconds, including a private-workspace canary that was not auto-opened. Demo opens only the exact sample workspace.
- Final post-adaptation entity rerun: **2/2**, 2.3 minutes. Browser assertions and deadlines retained.
- Native smoke on `3923d2ca8`: embedded `http://tauri.localhost/` app rendered the sample workspace's eight records, scene switch to the animated entity and composer. An owned synthetic loopback endpoint received exactly one native `/api/generate` request; UI was observed waiting and then completed with the exact labelled synthetic response and disclosure control. App closed with exit 0, endpoint closed and shared test lease released. This verifies transport/UI plumbing, not real-model quality. The final telemetry-only repair is `548f3b36e`; delivery build and final visual smoke are recorded in the local delivery record.

## Failures retained

The first entity browser attempt was 0/2: its fake used Ollama message-shaped chunks for the local custom endpoint, which actually speaks OpenAI SSE, so the view correctly waited rather than displaying nonexistent partial text. The second failure was a project-selection setup timeout before the entity workflow. Correcting the fixture protocol (no app parser change or weakened assertion) yielded 2/2.

The initial combined regression was 3/5 and its idle rerun 1/3: scene switching repeatedly exceeded the existing 8-second click bound. The bounded measured-resolution adaptation keeps all passes, data, cameras and labels while reducing effects resolution only for slow renders, to a floor of half linear resolution. Original tests subsequently passed 3/3; their failed logs are retained.

The first native package compiled but treated an absolute `E:/...` frontendDist as a URL; the smoke exposed external file navigation instead of the embedded app. The corrected build uses a relative filesystem path, embeds the assets, and keeps the existing CSP. The incorrect package was not promoted as accepted.

Native chat exposed a clipped negative input-reduction percentage: the genuine compiled prompt was larger than the short typed question. `548f3b36e` preserves the values but shows input overhead as a multiplier, with the typed-question baseline and estimated badge. This is not full-context savings or a benchmark claim. SSR regression covers overhead, positive reduction and missing data.

## Local delivery and limits

Final native build: `ANANSI-Demo-548f3b36e.exe`, 32,436,736 bytes, SHA-256 `643c2264099dd0e16b67e979f0d1d105031e092fd15e76b18717af33d63238bf`; custom-protocol build finished successfully in 1m05s. Final native smoke observed automatic sample startup, wheel zoom, drag orbit, Reset View, scene 7's loaded portrait and usable composer. Actual native screenshot retained as `E:/Anansi/reviews/larp-entity-2026-11-05/entity-native-delivered.png`. The leased final smoke closed with exit 0, no inference (synthetic server hit count 0), endpoint closed and lease released. The preceding native chat smoke (one labelled synthetic reply) is scoped separately above. The owner's ordinary demo is subsequently launched outside the test lease for interactive use. The shared cached executable was restored to the exact pre-build copy only after verifying that its hash still matched this task's output.

Desktop shortcut: `C:/Users/robin/Desktop/ANANSI Ultimate Demo.lnk`. Versioned executable, launch script and readme live under `E:/Anansi/previews/larp-cinematic`; root is `E:/Anansi/previews/larp-cinematic/data`, eight labelled sample memories and seven sample relationships. Existing preview/data roots remain separate. The default build remains ordinary startup; only the explicit demo build auto-opens sample data.

Logs and actual rendered screenshots are retained under `E:/Anansi/previews/larp-cinematic` and `E:/Anansi/reviews/larp-entity-2026-11-05` (idle, waiting, streaming and reduced-motion images inspected). No Rust/domain/schema changes in this feature; native build recompiles the already-verified merged backend. No real/paid/cloud inference, provider credentials, downloads of models, pushes, CI, installers or generated-product execution. Standing M5/security/release and full-product acceptance gates remain open.
