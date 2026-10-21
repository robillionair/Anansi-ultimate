# M5 provider verification record

Verified: **2026-10-20**, against official provider documentation (pages
opened on provider-owned domains) plus, for Ollama, the working
implementation already in this repository. This record covers the
*implemented* adapters in depth. It is a documentation check, not a
live-provider test: no provider was called, no credential exists on the
build machine, and real-provider verification remains pending for every
cloud provider.

## Implemented adapters

### OpenAI — Chat Completions (reference for the compatible family)

- Endpoint: `POST https://api.openai.com/v1/chat/completions`.
- Auth: `Authorization: Bearer <API key>` (OpenAI API Reference,
  developers.openai.com — "Use Bearer authentication (header
  Authorization)").
- Streaming: SSE `data:` lines terminated by `data: [DONE]`;
  `stream_options: {"include_usage": true}` returns a final usage chunk
  (documented Chat Completions streaming behaviour).
- Errors/rate limits: JSON `error.message`; HTTP 429 with `Retry-After`.
- User OAuth for inference: **not offered** for the API platform —
  API keys are the documented mechanism. (ChatGPT subscription access
  tokens documented for OpenAI's own tooling are a consumer mechanism,
  not an API OAuth flow; §15 forbids relying on undocumented consumer
  authentication, so the catalog records API key only.)
- Docs: https://platform.openai.com/docs/api-reference/chat ·
  https://developers.openai.com/api/reference/

### Anthropic — Messages API

- Endpoint: `POST https://api.anthropic.com/v1/messages`.
- Auth: `x-api-key: <API key>` header; `anthropic-version: 2023-06-01`
  header is required (currently the only supported version value).
- `max_tokens` is a **required** body field — the adapter supplies a
  bounded default (1024) when the caller sets none.
- System text is a top-level `system` field, not a system message.
- Streaming: SSE with named events (`message_start`,
  `content_block_delta` with `delta.text`, `message_delta` carrying
  usage + `stop_reason`, `message_stop`, `error`).
- Errors: `{"type":"error","error":{"type": "...", "message": "..."}}`;
  429 responses carry a `retry-after` header.
- User OAuth for inference: **not offered** — API keys (Console) are
  the documented mechanism.
- Docs: https://docs.claude.com/en/api/messages

### Google Gemini — generateContent API

- Endpoint (verified on ai.google.dev, opened 2026-10-20):
  `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`
  with the API key in the **`x-goog-api-key`** header — Google's
  current docs use the header form (the older `?key=` query form is
  legacy; this crate's endpoint policy forbids secrets in URLs, so the
  header form is the only one implemented).
- Streaming: `:streamGenerateContent` returns Server-Sent Events per
  the official API reference; the adapter requests the SSE form via
  `?alt=sse`. Verification nuance: `alt=sse` was confirmed on official
  pages for the Interactions streaming endpoint and in an official
  Apigee example for `streamGenerateContent`, but was not directly
  confirmed on an opened ai.google.dev reference page for
  `streamGenerateContent` itself — **partially verified**; the
  decoder is in any case proven against the SSE shape by the
  synthetic-server suite, and this is the first thing to re-check at
  real-provider integration.
- Body: `contents[]` with `role` `user`/`model` and `parts[].text`;
  system text goes in `systemInstruction`; usage returns as
  `usageMetadata.promptTokenCount` / `candidatesTokenCount`;
  finish as `candidates[0].finishReason`.
- Note recorded for the integrator: Google's get-started page now
  labels generateContent "Legacy" and recommends a newer Interactions
  API. The adapter targets generateContent because it is the
  documented, stable REST surface today; re-evaluate at integration.
- Vertex AI is a **different** surface (Google Cloud, ADC/service
  account OAuth2) and is catalogued separately as Deferred.
- Docs: https://ai.google.dev/gemini-api/docs/generate-content/get-started

### Ollama — local runtime

- Endpoint: `POST http://127.0.0.1:11434/api/chat` (loopback, no auth).
- Streaming: newline-delimited JSON; each line carries
  `message.content`; the final line has `done: true` with
  `prompt_eval_count` / `eval_count` usage counts and `done_reason`.
- Corroborated by docs.ollama.com (`/api/chat` request fields `model`,
  `messages`, `stream`, `options`) and by this repository's own working
  Ask implementation and its real-model acceptance evidence
  (M5-READINESS, 2026-10-20).
- Docs: https://docs.ollama.com/ · API reference in the Ollama repo
  (`docs/api.md`).

### Azure OpenAI — enterprise endpoint

- URL (verified on Microsoft Learn, opened via search 2026-10-20):
  `POST https://{resource}.openai.azure.com/openai/deployments/{deployment-id}/chat/completions?api-version={version}`;
  the request "model" is the **deployment name in the path**, not a
  body field.
- Auth: `api-key: <key>` header for API-key authentication (Microsoft
  Entra ID Bearer tokens are the documented alternative; this adapter
  implements the API-key form and says so).
- The GA data-plane specification version is **2024-10-21**, which is
  the adapter's default `api_version` (a field, so the integrator can
  pin the version their resource is verified against).
- Docs: https://learn.microsoft.com/en-us/azure/ai-services/openai/reference

## OpenAI-compatible family (one adapter, per-provider official bases)

Each of these providers officially documents an OpenAI-compatible Chat
Completions surface with Bearer API-key auth; the catalog records the
official base URL and docs page for each, checked 2026-10-20:
OpenRouter (`https://openrouter.ai/api/v1`), Mistral
(`https://api.mistral.ai/v1`), xAI (`https://api.x.ai/v1`), Moonshot/Kimi
(`https://api.moonshot.ai/v1`), NVIDIA NIM
(`https://integrate.api.nvidia.com/v1`), Groq
(`https://api.groq.com/openai/v1`), Cerebras
(`https://api.cerebras.ai/v1`), Cohere compatibility API
(`https://api.cohere.ai/compatibility/v1`), Together
(`https://api.together.xyz/v1`), Fireworks
(`https://api.fireworks.ai/inference/v1`), Perplexity
(`https://api.perplexity.ai`), Hugging Face router
(`https://router.huggingface.co/v1`), Z.AI
(`https://api.z.ai/api/paas/v4`), LM Studio local
(`http://127.0.0.1:1234/v1`). Per-provider quirks (extra headers, model
id formats) are configuration concerns at integration; none was
invented here. User OAuth for inference is not documented for these
API surfaces; all are API-key.

## Deferred / catalog-only (with reasons, not placeholders)

- **AWS Bedrock** — every request is AWS Signature Version 4 signed
  (or uses Bedrock API keys in limited regions); an unsigned
  "API-key adapter" would be a fabrication. Catalog: `Deferred`,
  auth `CloudIam`.
- **Google Vertex AI** — requires Google ADC / service-account OAuth2
  access tokens. Catalog: `Deferred`, auth `CloudIam`.
- **Cloudflare Workers AI** — OpenAI-compatible, but the endpoint
  embeds the account id
  (`https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/v1`),
  so it is configured as a custom endpoint. Catalog: `CatalogOnly`.

## Addendum from the full verification pass (2026-10-20)

A second, deeper pass over the official pages adds these nuances; none
changes an adapter behaviour or a catalog status:

- **Anthropic** also documents `Authorization: Bearer <token>` as an
  auth option, and its `POST /v1/oauth/token` endpoint is a **Workload
  Identity Federation** token exchange (machine/workload identity),
  not a user OAuth flow — reinforcing the conclusion below. Anthropic
  also documents a 529 `overloaded_error` (mapped by this crate's 5xx
  rule) and spend-cap 429s that may carry **no** `retry-after` (the
  retry policy already falls back to bounded exponential backoff).
- **Azure** additionally has a newer Foundry "v1" surface
  (`{endpoint}/openai/v1/chat/completions`, `api-version` optional).
  The adapter targets the classic GA data-plane surface (2024-10-21)
  recorded above; the v1 surface is a future integration option.
- **Cancellation** is the weakest-documented area across all
  providers: only Gemini documents a cancellation signal at all
  (HTTP 499 on client disconnect, on an Interactions-scoped page).
  Everywhere else, client disconnect is an assumption — which is why
  this crate defines cancellation as *closing the client socket* and
  proves exactly that against synthetic servers, claiming nothing
  about provider-side scheduler cancellation (the same honesty
  boundary M5-READINESS records for the existing Ask path).

## OAuth conclusion (§15)

For every provider surface implemented above, the officially
documented inference authentication at verification time is **API
keys** (plus cloud IAM for Bedrock/Vertex, and Entra ID for Azure).
No implemented surface documents user OAuth (Authorization Code + PKCE)
for inference. Accordingly the catalog contains **no** `OAuthUser`
entries — a unit test enforces this — while the crate implements the
PKCE S256 / state-validation contracts in `secrets.rs` so that a
provider which does officially offer OAuth can be integrated without
redesign. Claiming OAuth where only API keys exist would violate
§14/§15; claiming API keys are "OAuth" would be worse.
