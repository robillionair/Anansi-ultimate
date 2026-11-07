# SOMNIA model comparison

Acceptance: explicitly review two bounded replay prompts, run both against one configured model through the application provider gates, surface the exact prompts, model replies and reported usage, and explicitly save a private project experiment. Local-only is the default. Explicit cloud authorization applies only to the reviewed text. A failed or mismatched dispatch must never produce a completed comparison. Exercise the browser workflow and a real local-model pair without replacing the offline SOMNIA evaluator.

## Windows evidence

The browser test verified zero dispatches during review, exactly two requests after approval, provider usage, and private artifact readback. A second browser run used a warmed local Qwen2.5 3B model on one frozen revision question. The initial cold-load attempt exceeded the provider timeout. The warmed run saved both replies and measured 425 versus 194 total tokens. Both answers were UNKNOWN; the expected answer was CODE-412. This establishes live application routing and persistence for one pair. The 48-question OpenRouter nightmare result remains standalone.

The proposed prompt is currently supplied and reviewed by the user. The existing offline consolidation proposals do not automatically generate model prompts or change live memory.

## OpenRouter reasoning fix

Acceptance: SOMNIA's exact-model OpenRouter comparison requests reasoning disabled in both arms and requires a provider that supports that option. The request field defaults off for existing callers. A wire-body regression covers the opt-in and default. Rerun the frozen Super VL file nightmare through the visible application and score only complete pairs.

Windows rerun: the rebuilt application completed all six original/proposed pairs in visible LARP scene 09 with Nemotron 3.5 Super VL (free). All 12 responses had finish reason stop. Exact-match scores were 11/48 original and 38/48 proposed. Reported total tokens were 64,215 original and 10,330 proposed, down 83.91%. The benchmark supplied both prepared prompts, including 36 synthetic attachment excerpts in the original histories. This measured in-app dispatch, reporting, and saving with the frozen benchmark inputs. It did not measure automatic proposal generation. The provider wire test, contract drift check, TypeScript, and web build passed.
