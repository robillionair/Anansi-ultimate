# ANANSI v1 — Easier AI provider onboarding

Status · API-key setup and its connection-verification repair are implemented; [setup evidence](../docs/handoffs/connect-ai-api-key-setup.md) and [verification repair](../docs/handoffs/connect-ai-verification-repair.md) record the scope. The [ChatGPT prototype](../docs/handoffs/connect-ai-chatgpt-signin.md) and lifecycle repairs are implemented behind a disabled dev gate, with real eligibility and native credential-store acceptance still open. The requirements below preserve the original slice design. The pilot can use local models and API-key connections while sign-in is evaluated.

## Slice 1 · Add an AI provider

Put one **Connect AI** entry in onboarding and Settings. Let users skip it and use local workflows. For the first slice, reuse the existing adapters and provider catalog. Present OpenAI, OpenRouter, Anthropic, Gemini, and local Ollama as choices. Show provider-specific help, use the catalog endpoint where available, and let advanced users enter a custom endpoint separately.

1. Select the provider. Display its connection type, destination, and a link to the provider's own key setup instructions. Generate an opaque workspace-scoped credential reference in the host. Users should only see the provider name and connection status.
2. Choose **Add API key**. Use the existing desktop-owned native credential prompt and OS store. Browser sessions can see connection status and ask the desktop owner to finish setup. API keys stay outside the web form, DTOs, exports, and diagnostic logs.
3. Select a model from available catalog information or enter its exact ID. Show free, paid, or unverified cost status. A credential stored in the keyring is one setup step. A verified model connection requires separate evidence. Keep Zero-Cost Lock on by default and make any network check explicit. An optional inference test uses a fixed synthetic prompt and follows normal provider authorization and cost checks.
4. Save the connection without replacing other configured providers or resetting existing usage limits. The pre-slice form submitted a single-provider replacement configuration. The delivered backend-owned, atomic add/update operation retains the existing owner, storage, and policy checks. Preserve the behavior of full replacement callers. Keep edit, disconnect, and model changes available after restart.

### Acceptance

- Through Windows, add two providers in sequence and verify the first provider and existing budget survive the second save and a process restart. Test cancel and failed save, including recovery from partial credential setup. Verify that a paired browser cannot enter a key or approve its own setup.
- Use loopback test servers for missing key, invalid key, unavailable endpoint, model mismatch, and successful disclosure. Check exact hit counts for preflight refusals. Keep live provider verification and native prompt acceptance separately recorded.

## Slice 2 · Continue with ChatGPT

OpenAI documents separate permissions for account identity and eligible plan usage. ANANSI's first proposed flow is a **provider connection** from **Connect AI**. Local ANANSI workspaces remain available offline. If eligible, users authorize plan usage in the system browser and return to the app. OpenAI documents an open-source/local-client route and a separate interest process for paid or remotely hosted apps. Confirm ANANSI's distribution eligibility before offering the sign-in button as available. [OpenAI sign-in quickstart](https://developers.openai.com/siwc/quickstart) · [ChatGPT plan usage](https://developers.openai.com/siwc/token-sharing-open-source)

Use the official authorization-code flow with PKCE, state, nonce, and a registered loopback callback where the documented flow requires it. Validate issuer, token, and scopes in the desktop host. Use protected OS credential storage, scoped access and refresh tokens, and clear disconnect and expired-session behavior. Browser access to the shared local backend must respect host authorization. ChatGPT plan usage and OpenAI project-key billing are separate connection choices. Show scope and usage eligibility accurately and preserve disclosure and privacy checks.

For ordinary answers, evaluate the documented Responses API path. For an actual Codex workflow, evaluate the documented Codex app-server path with the access token from the same supported sign-in. Treat Codex tools and execution capabilities as a separate design and approval boundary. [Codex app-server integration](https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server)

### Acceptance

- Synthetic OAuth coverage for success, wrong state, nonce and issuer, denied consent, insufficient scopes, token expiry, refresh and revocation. Ensure credentials and callback codes are excluded from UI state, logs, and exports.
- Verify one approved synthetic-data turn using an eligible real account before release. Show the selected connection type, destination, and result. Test that a failed, declined, or ineligible sign-in leaves the local and API-key routes available. Use official scopes and credentials issued to ANANSI. Any Codex action beyond model inference requires its own scope and acceptance.
