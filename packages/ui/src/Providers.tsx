import { useEffect, useRef, useState } from 'react';
import type {
  Client,
  CompletionRequest,
  DisclosureRecord,
  ProviderCatalogEntry,
  ProviderUsageResponse,
  RuntimeConfig,
} from '@anansi/contracts';

/**
 * AI Providers — the M5 configuration, credential-reference,
 * authorization, dispatch and disclosure workflow (§§14–19).
 *
 * Trust model, made visible rather than hidden: configuration and
 * paid authorization are OWNER operations (the desktop host, or a
 * browser session the host has granted — the backend enforces
 * this and this screen surfaces its refusals). Secrets are never
 * typed here: the form stores only a credential REFERENCE; the
 * secret itself lives in the OS keyring (§15).
 */

const ROUTING_MODES = [
  'auto',
  'autoFree',
  'private',
  'fast',
  'deep',
  'coding',
  'vision',
  'chooseModel',
] as const;

/** Mirrors the backend's locality derivation (endpoint validation):
 * loopback hosts are Local, everything else is Cloud. The backend
 * re-derives it and rejects a mismatch — this only pre-fills the
 * form honestly. */
export function deriveLocality(baseUrl: string): 'local' | 'cloud' {
  try {
    const host = new URL(baseUrl).hostname.toLowerCase();
    if (host === 'localhost' || host === '::1' || host === '[::1]' || host.startsWith('127.')) return 'local';
    return 'cloud';
  } catch {
    return 'cloud';
  }
}

interface ModelRow {
  id: string;
  cost: 'free' | 'paid' | 'unknown';
}

interface PermitState {
  id: string;
  expiresAtMs: number;
  provider: string;
  model: string;
}

export function Providers({ client, workspaceId }: { client: Client; workspaceId: string }) {
  const [catalog, setCatalog] = useState<ProviderCatalogEntry[]>([]);
  const [catalogError, setCatalogError] = useState('');

  // Configure form
  const [providerId, setProviderId] = useState('ollama');
  const [baseUrl, setBaseUrl] = useState('http://127.0.0.1:11434');
  const [credentialRef, setCredentialRef] = useState('');
  const [models, setModels] = useState<ModelRow[]>([{ id: 'llama3.1', cost: 'free' }]);
  const [maxRequests, setMaxRequests] = useState('');
  const [maxTotalTokens, setMaxTotalTokens] = useState('');
  const [configured, setConfigured] = useState<RuntimeConfig | null>(null);
  const [configureError, setConfigureError] = useState('');
  const [configureBusy, setConfigureBusy] = useState(false);

  // Authorize
  const [permit, setPermit] = useState<PermitState | null>(null);
  const [permitCap, setPermitCap] = useState('100000');
  const [authorizeError, setAuthorizeError] = useState('');
  const [authorizeBusy, setAuthorizeBusy] = useState(false);

  // Dispatch
  const [model, setModel] = useState('llama3.1');
  const [mode, setMode] = useState<string>('auto');
  const [zeroCostLock, setZeroCostLock] = useState(true);
  const [streaming, setStreaming] = useState(true);
  const [prompt, setPrompt] = useState('');
  const [output, setOutput] = useState('');
  const [streamState, setStreamState] = useState<'idle' | 'streaming' | 'done' | 'cancelled' | 'error'>('idle');
  const [disclosure, setDisclosure] = useState<DisclosureRecord | null>(null);
  const [dispatchError, setDispatchError] = useState('');
  const activeRun = useRef<{ runId: string; capability: string } | null>(null);
  const cancelWanted = useRef(false);

  // Usage
  const [usage, setUsage] = useState<ProviderUsageResponse | null>(null);

  useEffect(() => {
    let live = true;
    client
      .providersCatalog()
      .then((c) => {
        if (live) setCatalog(c.providers);
      })
      .catch((e) => {
        if (live) setCatalogError(e instanceof Error ? e.message : 'Catalog unavailable.');
      });
    void refreshUsage();
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [client, workspaceId]);

  async function refreshUsage() {
    try {
      setUsage(await client.providersUsage(workspaceId));
    } catch {
      setUsage(null);
    }
  }

  function pickProvider(entry: ProviderCatalogEntry) {
    setProviderId(entry.id);
    if (entry.baseUrl) setBaseUrl(entry.baseUrl);
  }

  async function handleConfigure(event: React.FormEvent) {
    event.preventDefault();
    setConfigureBusy(true);
    setConfigureError('');
    try {
      const budget: RuntimeConfig['budget'] = {};
      if (maxRequests.trim()) budget.maxRequests = Number(maxRequests);
      if (maxTotalTokens.trim()) budget.maxTotalTokens = Number(maxTotalTokens);
      const config: RuntimeConfig = {
        providers: [
          {
            config: {
              provider: providerId,
              baseUrl: baseUrl.trim(),
              credential: credentialRef.trim()
                ? { provider: providerId, reference: credentialRef.trim() }
                : null,
              enabled: true,
            },
            locality: deriveLocality(baseUrl),
            models: models
              .filter((m) => m.id.trim())
              .map((m) => ({
                id: m.id.trim(),
                provider: providerId,
                capabilities: {
                  chat: true,
                  streaming: true,
                  vision: false,
                  coding: true,
                  deepReasoning: false,
                  embeddings: false,
                },
                contextWindow: 8192,
                cost: m.cost,
                speed: 'balanced' as const,
              })),
          },
        ],
        ...(Object.keys(budget).length ? { budget } : {}),
      };
      await client.providersConfigure(workspaceId, config);
      setConfigured(config);
      const first = config.providers[0]?.models[0];
      if (first) setModel(first.id);
      setPermit(null);
      await refreshUsage();
    } catch (e) {
      setConfigureError(e instanceof Error ? e.message : 'Configuration failed.');
    } finally {
      setConfigureBusy(false);
    }
  }

  async function handleAuthorize() {
    setAuthorizeBusy(true);
    setAuthorizeError('');
    try {
      const issued = await client.providersAuthorize(
        workspaceId,
        providerId,
        model,
        Number(permitCap) || 100000,
      );
      setPermit({
        id: issued.permitId,
        expiresAtMs: issued.expiresAtMs,
        provider: providerId,
        model,
      });
    } catch (e) {
      setAuthorizeError(e instanceof Error ? e.message : 'Authorization failed.');
    } finally {
      setAuthorizeBusy(false);
    }
  }

  function buildRequest(): CompletionRequest {
    return {
      model,
      messages: [
        {
          role: 'user',
          segments: [{ text: prompt.trim(), privacy: 'standard' }],
        },
      ],
    };
  }

  function dispatchOptions() {
    return {
      mode,
      zeroCostLock,
      ...(mode === 'chooseModel' ? { chosen: [providerId, model] as [string, string] } : {}),
      ...(permit && permit.model === model ? { permitId: permit.id } : {}),
    };
  }

  async function handleDispatch(event: React.FormEvent) {
    event.preventDefault();
    if (!prompt.trim()) return;
    setDispatchError('');
    setOutput('');
    setDisclosure(null);
    try {
      if (!streaming) {
        setStreamState('streaming');
        const out = await client.providersDispatch(workspaceId, buildRequest(), dispatchOptions());
        setOutput(out.response.text);
        setDisclosure(out.disclosure);
        setStreamState('done');
      } else {
        const started = await client.providersStreamStart(workspaceId, buildRequest(), dispatchOptions());
        activeRun.current = { runId: started.runId, capability: started.capability };
        cancelWanted.current = false;
        setDisclosure(started.disclosure);
        setStreamState('streaming');
        let text = '';
        for (;;) {
          if (cancelWanted.current) break;
          const poll = await client.providersStreamPoll(workspaceId, started.runId, started.capability);
          for (const ev of poll.events) {
            if (ev.kind === 'token' && ev.text) {
              text += ev.text;
              setOutput(text);
            } else if (ev.kind === 'error') {
              setDisclosure(poll.disclosure);
              setStreamState(ev.code === 'Cancelled' ? 'cancelled' : 'error');
              if (ev.code !== 'Cancelled') {
                setDispatchError(`${ev.code ?? 'Error'}: ${ev.message ?? 'The stream failed.'}`);
              }
              activeRun.current = null;
              await refreshUsage();
              return;
            }
          }
          setDisclosure(poll.disclosure);
          if (poll.done) {
            setStreamState('done');
            break;
          }
          await new Promise((r) => setTimeout(r, 120));
        }
        activeRun.current = null;
        // The cancel button exits the loop via cancelWanted
        // (possibly before the terminal Cancelled event is
        // polled): the state machine must still land on
        // 'cancelled', not hang in 'streaming'.
        if (cancelWanted.current) setStreamState('cancelled');
      }
    } catch (e) {
      setStreamState('error');
      setDispatchError(e instanceof Error ? e.message : 'Dispatch failed.');
    } finally {
      await refreshUsage();
    }
  }

  async function handleCancel() {
    const run = activeRun.current;
    if (!run) return;
    cancelWanted.current = true;
    try {
      await client.providersStreamCancel(workspaceId, run.runId, run.capability);
    } catch {
      // The stream may already be finished; the poll loop will
      // surface the terminal state.
    }
  }

  const locality = deriveLocality(baseUrl);
  const selectedEntry = catalog.find((c) => c.id === providerId);

  return (
    <div className="providers-view" aria-label="AI Providers">
      <section className="card" aria-labelledby="providers-catalog-title">
        <h2 id="providers-catalog-title">Provider catalog</h2>
        <p>
          Built-in provider connections (MASTER_SPEC §14). Configuration and paid authorization are
          owner operations: in the desktop app you are the owner; a browser session must first be
          granted by the host. Secrets are never typed here — only a credential reference; the
          secret itself lives in the operating system keyring (§15).
        </p>
        {catalogError && <p className="error" role="alert" aria-label="Catalog error">{catalogError}</p>}
        <ul className="provider-catalog">
          {catalog.map((entry) => (
            <li key={entry.id}>
              <button
                type="button"
                className="secondary"
                aria-pressed={providerId === entry.id}
                onClick={() => pickProvider(entry)}
              >
                {entry.displayName}
              </button>{' '}
              <span>
                {entry.locality} · {entry.status}
                {entry.id === providerId ? ' · selected' : ''}
              </span>
            </li>
          ))}
        </ul>
        {selectedEntry?.docsUrl && (
          <p className="help">
            Provider documentation: <a href={selectedEntry.docsUrl} target="_blank" rel="noreferrer">{selectedEntry.docsUrl}</a>
          </p>
        )}
      </section>

      <section className="card" aria-labelledby="providers-configure-title" aria-busy={configureBusy}>
        <h2 id="providers-configure-title">Configure provider</h2>
        <form onSubmit={handleConfigure}>
          <fieldset disabled={configureBusy}>
            <label htmlFor="provider-id">Provider</label>
            <input id="provider-id" value={providerId} onChange={(e) => setProviderId(e.target.value)} required autoComplete="off" />
            <label htmlFor="provider-base-url">Base URL</label>
            <input id="provider-base-url" value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} required autoComplete="off" spellCheck={false} />
            <p className="help">
              Locality is derived from the endpoint by the backend: this URL is{' '}
              <strong>{locality}</strong>. A configuration claiming otherwise is rejected.
            </p>
            <label htmlFor="provider-credential">Credential reference (OS keyring account)</label>
            <input id="provider-credential" value={credentialRef} onChange={(e) => setCredentialRef(e.target.value)} autoComplete="off" spellCheck={false} placeholder="Leave empty for keyless local providers" />
            <p className="help">
              Approved credential setup: paste the API key into the OS keyring under this account
              name using the operating system's credential prompt; ANANSI stores only the
              reference and resolves the secret inside the backend at dispatch time.
            </p>
            {models.map((row, i) => (
              <div key={i} className="model-row">
                <label htmlFor={`provider-model-${i}`}>Model</label>
                <input
                  id={`provider-model-${i}`}
                  value={row.id}
                  onChange={(e) =>
                    setModels(models.map((m, j) => (j === i ? { ...m, id: e.target.value } : m)))
                  }
                  autoComplete="off"
                />
                <label htmlFor={`provider-model-cost-${i}`}>Cost class</label>
                <select
                  id={`provider-model-cost-${i}`}
                  value={row.cost}
                  onChange={(e) =>
                    setModels(
                      models.map((m, j) =>
                        j === i ? { ...m, cost: e.target.value as ModelRow['cost'] } : m,
                      ),
                    )
                  }
                >
                  <option value="free">free</option>
                  <option value="paid">paid</option>
                  <option value="unknown">unknown</option>
                </select>
              </div>
            ))}
            <button
              type="button"
              className="secondary"
              onClick={() => setModels([...models, { id: '', cost: 'free' }])}
            >
              Add model
            </button>
            <label htmlFor="provider-budget-requests">Budget — max requests (optional)</label>
            <input id="provider-budget-requests" value={maxRequests} onChange={(e) => setMaxRequests(e.target.value)} inputMode="numeric" autoComplete="off" />
            <label htmlFor="provider-budget-tokens">Budget — max total tokens (optional)</label>
            <input id="provider-budget-tokens" value={maxTotalTokens} onChange={(e) => setMaxTotalTokens(e.target.value)} inputMode="numeric" autoComplete="off" />
          </fieldset>
          {configureError && <p className="error" role="alert" aria-label="Configuration error">{configureError}</p>}
          {configured && !configureError && (
            <p role="status">Configured. The setup is saved for this workspace and survives restarts.</p>
          )}
          <button type="submit" disabled={configureBusy}>Save configuration</button>
        </form>
      </section>

      <section className="card" aria-labelledby="providers-dispatch-title">
        <h2 id="providers-dispatch-title">Dispatch</h2>
        <form onSubmit={handleDispatch}>
          <fieldset disabled={streamState === 'streaming'}>
            <label htmlFor="dispatch-model">Model</label>
            <input id="dispatch-model" value={model} onChange={(e) => setModel(e.target.value)} required autoComplete="off" />
            <label htmlFor="dispatch-mode">Routing mode</label>
            <select id="dispatch-mode" value={mode} onChange={(e) => setMode(e.target.value)}>
              {ROUTING_MODES.map((m) => (
                <option key={m} value={m}>{m}</option>
              ))}
            </select>
            <label htmlFor="dispatch-lock">
              <input id="dispatch-lock" type="checkbox" checked={zeroCostLock} onChange={(e) => setZeroCostLock(e.target.checked)} />{' '}
              Zero-Cost Lock (never spend: paid candidates are refused)
            </label>
            <label htmlFor="dispatch-stream">
              <input id="dispatch-stream" type="checkbox" checked={streaming} onChange={(e) => setStreaming(e.target.checked)} />{' '}
              Stream the response
            </label>
            <label htmlFor="dispatch-prompt">Prompt</label>
            <input id="dispatch-prompt" value={prompt} onChange={(e) => setPrompt(e.target.value)} required autoComplete="off" placeholder="Ask the configured model…" />
          </fieldset>
          <div className="actions">
            <button type="submit" disabled={streamState === 'streaming'}>Send</button>
            {streamState === 'streaming' && (
              <button type="button" className="secondary" onClick={() => void handleCancel()}>
                Cancel stream
              </button>
            )}
          </div>
        </form>
        <div className="authorize-row">
          <label htmlFor="permit-cap">Paid authorization — token cap</label>
          <input id="permit-cap" value={permitCap} onChange={(e) => setPermitCap(e.target.value)} inputMode="numeric" autoComplete="off" />
          <button type="button" className="secondary" disabled={authorizeBusy} onClick={() => void handleAuthorize()}>
            Authorize paid dispatch for this model
          </button>
          {permit && (
            <p role="status">
              Permit {permit.id.slice(0, 18)}… for {permit.provider}/{permit.model}, expires{' '}
              {new Date(permit.expiresAtMs).toLocaleTimeString()}. Single-use; issued permits are
              bound to this session.
            </p>
          )}
          {authorizeError && <p className="error" role="alert" aria-label="Authorization error">{authorizeError}</p>}
        </div>
        {dispatchError && <p className="error" role="alert" aria-label="Dispatch error">{dispatchError}</p>}
        {streamState === 'cancelled' && <p role="status">Stream cancelled.</p>}
        {output && (
          <div className="provider-output" aria-label="Model response">
            <h3>Response</h3>
            <p style={{ whiteSpace: 'pre-wrap' }}>{output}</p>
          </div>
        )}
      </section>

      {disclosure && <DisclosurePanel disclosure={disclosure} />}
      <section className="card" aria-labelledby="providers-usage-title">
        <h2 id="providers-usage-title">Usage</h2>
        {usage ? (
          <dl>
            <div><dt>Total requests (this session of the app)</dt><dd>{usage.totalRequests}</dd></div>
            <div><dt>Reserved requests / tokens</dt><dd>{usage.reservedRequests} / {usage.reservedTokens}</dd></div>
            {usage.providers.map((p) => (
              <div key={p.provider}>
                <dt>{p.provider}</dt>
                <dd>{p.requests} requests · {p.inputTokens} in / {p.outputTokens} out</dd>
              </div>
            ))}
          </dl>
        ) : (
          <p>No usage recorded yet.</p>
        )}
        <button type="button" className="secondary" onClick={() => void refreshUsage()}>Refresh usage</button>
      </section>
    </div>
  );
}

/** The §19 pre/post-transmission disclosure: exactly what the
 * backend recorded about a dispatch — provider, model, endpoint,
 * locality, cost class, routing decision, content shape,
 * redactions, usage and every attempt. */
export function DisclosurePanel({ disclosure }: { disclosure: DisclosureRecord }) {
  return (
    <section className="card disclosure" aria-labelledby="providers-disclosure-title">
      <h2 id="providers-disclosure-title">View what ANANSI sent</h2>
      <dl>
        <div><dt>Provider</dt><dd>{disclosure.provider}</dd></div>
        <div><dt>Model</dt><dd>{disclosure.model}</dd></div>
        <div><dt>Endpoint host</dt><dd>{disclosure.endpointHost}</dd></div>
        <div><dt>Local</dt><dd>{disclosure.local ? 'yes' : 'no'}</dd></div>
        <div><dt>Cost class</dt><dd>{disclosure.costClass}</dd></div>
        <div><dt>Routing</dt><dd>{disclosure.routingMode} — {disclosure.routingReason}</dd></div>
        <div><dt>Messages / segments</dt><dd>{disclosure.messageCount} / {disclosure.segmentCount}</dd></div>
        <div><dt>Redactions</dt><dd>{disclosure.redactionCount}</dd></div>
        <div><dt>Estimated input tokens</dt><dd>{disclosure.estimatedInputTokens}</dd></div>
        {disclosure.usage && (
          <div><dt>Actual usage</dt><dd>{disclosure.usage.inputTokens} in / {disclosure.usage.outputTokens} out</dd></div>
        )}
      </dl>
      {disclosure.attempts.length > 0 && (
        <>
          <h3>Attempts</h3>
          <ul>
            {disclosure.attempts.map((a, i) => (
              <li key={i}>{a.provider}/{a.model}: {a.outcome}</li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
}
