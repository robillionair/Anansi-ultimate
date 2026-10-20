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
  const [streamState, setStreamState] = useState<'idle' | 'streaming' | 'cancelling' | 'done' | 'cancelled' | 'error'>('idle');
  const [streamNotice, setStreamNotice] = useState('');
  const [runRetained, setRunRetained] = useState(false);
  const [disclosure, setDisclosure] = useState<DisclosureRecord | null>(null);
  const [dispatchError, setDispatchError] = useState('');
  const activeRun = useRef<{ runId: string; capability: string } | null>(null);
  // Set when the backend's terminal event for the active run was
  // an error; decides the final state once `done` confirms the
  // run has actually ended.
  const terminalError = useRef<string | null>(null);

  // Usage
  const [usage, setUsage] = useState<ProviderUsageResponse | null>(null);

  // Credential setup (native prompt + keyring, host-side)
  const [credPresent, setCredPresent] = useState<boolean | null>(null);
  const [credMessage, setCredMessage] = useState('');
  const [credError, setCredError] = useState('');
  const [credBusy, setCredBusy] = useState(false);

  // Browser-session access (host approval queue)
  const [access, setAccess] = useState<import('@anansi/contracts').ProviderSessionAccessResponse | null>(null);
  const [accessNote, setAccessNote] = useState('');
  const [accessBusy, setAccessBusy] = useState(false);

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
    void refreshAccess();
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [client, workspaceId]);

  // Leaving the screen (navigation, workspace switch) must not
  // leak a live run: ask the backend to cancel it. Best effort —
  // the run also expires server-side — but the capability is
  // never carried into another view.
  useEffect(() => {
    return () => {
      const run = activeRun.current;
      if (run) {
        activeRun.current = null;
        void client.providersStreamCancel(workspaceId, run.runId, run.capability).catch(() => {});
      }
    };
  }, [client, workspaceId]);

  async function refreshUsage() {
    try {
      setUsage(await client.providersUsage(workspaceId));
    } catch {
      setUsage(null);
    }
  }

  async function refreshCredStatus() {
    if (!credentialRef.trim()) {
      setCredPresent(null);
      return;
    }
    try {
      const status = await client.providersCredentialStatus(workspaceId, providerId, credentialRef.trim());
      setCredPresent(status.present);
    } catch {
      setCredPresent(null);
    }
  }

  async function handleCredentialSet() {
    setCredBusy(true);
    setCredError('');
    setCredMessage('');
    try {
      await client.providersCredentialSet(workspaceId, providerId, credentialRef.trim());
      setCredPresent(true);
      setCredMessage('Stored in the OS credential store under this reference.');
    } catch (e) {
      setCredError(e instanceof Error ? e.message : 'Credential setup failed.');
    } finally {
      setCredBusy(false);
    }
  }

  async function handleCredentialRemove() {
    setCredBusy(true);
    setCredError('');
    setCredMessage('');
    try {
      await client.providersCredentialRemove(workspaceId, providerId, credentialRef.trim());
      setCredPresent(false);
      setCredMessage('Credential removed from the OS credential store.');
    } catch (e) {
      setCredError(e instanceof Error ? e.message : 'Credential removal failed.');
    } finally {
      setCredBusy(false);
    }
  }

  async function refreshAccess() {
    try {
      setAccess(await client.providersSessionAccess(workspaceId));
      setAccessNote('');
    } catch (e) {
      setAccess(null);
      setAccessNote(
        e instanceof Error ? e.message : 'Session access is managed by the host.',
      );
    }
  }

  async function handleApproveSession(session: string) {
    setAccessBusy(true);
    try {
      await client.providersGrantSession(workspaceId, session, 1800);
      await refreshAccess();
    } catch (e) {
      setAccessNote(e instanceof Error ? e.message : 'Approval failed.');
    } finally {
      setAccessBusy(false);
    }
  }

  async function handleRevokeSession(session: string) {
    setAccessBusy(true);
    try {
      await client.providersRevokeSession(workspaceId, session);
      await refreshAccess();
    } catch (e) {
      setAccessNote(e instanceof Error ? e.message : 'Revocation failed.');
    } finally {
      setAccessBusy(false);
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
        terminalError.current = null;
        setDisclosure(started.disclosure);
        setStreamState('streaming');
        await watchRun(started.runId, started.capability, '');
      }
    } catch (e) {
      setStreamState('error');
      setDispatchError(e instanceof Error ? e.message : 'Dispatch failed.');
    } finally {
      await refreshUsage();
    }
  }

  /**
   * Poll the active run until the BACKEND confirms it has ended
   * (`done`). Cancellation is never declared locally: clicking
   * Cancel only moves to 'cancelling'; the terminal state comes
   * from the run's own terminal event. Transient poll failures
   * are retried with a visible notice; if watching ultimately
   * fails, the capability is retained and the user can resume —
   * the run is never abandoned silently.
   */
  async function watchRun(runId: string, capability: string, initialText: string) {
    let text = initialText;
    let failures = 0;
    for (;;) {
      let poll;
      try {
        poll = await client.providersStreamPoll(workspaceId, runId, capability);
        failures = 0;
        setStreamNotice('');
      } catch (e) {
        failures += 1;
        if (failures > 5) {
          setRunRetained(true);
          setStreamState('error');
          setDispatchError(
            'Lost contact while watching the stream: ' +
              (e instanceof Error ? e.message : 'unknown error') +
              ' The run may still be active on the backend — resume watching or cancel it.',
          );
          return;
        }
        setStreamNotice('Connection interrupted; still watching the run…');
        await new Promise((r) => setTimeout(r, 400));
        continue;
      }
      for (const ev of poll.events) {
        if (ev.kind === 'token' && ev.text) {
          text += ev.text;
          setOutput(text);
        } else if (ev.kind === 'error') {
          terminalError.current = ev.code ?? 'Error';
          if (ev.code !== 'Cancelled') {
            setDispatchError(`${ev.code ?? 'Error'}: ${ev.message ?? 'The stream failed.'}`);
          }
        }
      }
      setDisclosure(poll.disclosure);
      if (poll.done) {
        const terminal = terminalError.current;
        activeRun.current = null;
        setRunRetained(false);
        terminalError.current = null;
        setStreamState(terminal === 'Cancelled' ? 'cancelled' : terminal ? 'error' : 'done');
        await refreshUsage();
        return;
      }
      await new Promise((r) => setTimeout(r, 120));
    }
  }

  async function handleCancel() {
    const run = activeRun.current;
    if (!run || streamState !== 'streaming') return;
    // Enter 'cancelling' — NOT 'cancelled'. The terminal state
    // arrives only when the watch loop sees the backend confirm
    // the run ended.
    setStreamState('cancelling');
    try {
      await client.providersStreamCancel(workspaceId, run.runId, run.capability);
    } catch (e) {
      // The cancellation could not be delivered: say so, keep
      // the capability, keep polling (the watch loop is still
      // running), and let the user retry.
      setDispatchError(
        'Cancellation could not be delivered: ' +
          (e instanceof Error ? e.message : 'unknown error') +
          ' The stream is still being watched; try Cancel again.',
      );
      setStreamState('streaming');
    }
  }

  async function handleResumeWatch() {
    const run = activeRun.current;
    if (!run) return;
    setDispatchError('');
    setStreamState('streaming');
    await watchRun(run.runId, run.capability, output);
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

      <section className="card" aria-labelledby="providers-credential-title">
        <h2 id="providers-credential-title">Credential setup</h2>
        <p>
          Approved setup for <strong>{providerId}</strong> under the reference{' '}
          <strong>{credentialRef.trim() || '(set a reference in the configure form)'}</strong>.
          Setting a credential opens the trusted native prompt on this computer: the key moves
          from the prompt into the operating system credential store and is never typed into, or
          sent through, this page. In a paired browser this is an owner operation and is refused —
          set credentials in the desktop app.
        </p>
        <div className="actions">
          <button type="button" disabled={credBusy || !credentialRef.trim()} onClick={() => void handleCredentialSet()}>
            Set credential (native prompt)
          </button>
          <button type="button" className="secondary" disabled={credBusy || !credentialRef.trim()} onClick={() => void handleCredentialRemove()}>
            Remove credential
          </button>
          <button type="button" className="secondary" disabled={credBusy || !credentialRef.trim()} onClick={() => void refreshCredStatus()}>
            Check status
          </button>
        </div>
        {credPresent !== null && (
          <p role="status">Credential for this reference: {credPresent ? 'present in the OS store' : 'not present'}.</p>
        )}
        {credMessage && <p role="status">{credMessage}</p>}
        {credError && <p className="error" role="alert" aria-label="Credential error">{credError}</p>}
      </section>

      <section className="card" aria-labelledby="providers-dispatch-title">
        <h2 id="providers-dispatch-title">Dispatch</h2>
        <form onSubmit={handleDispatch}>
          <fieldset disabled={streamState === 'streaming' || streamState === 'cancelling'}>
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
            <button type="submit" disabled={streamState === 'streaming' || streamState === 'cancelling'}>Send</button>
            {streamState === 'streaming' && (
              <button type="button" className="secondary" onClick={() => void handleCancel()}>
                Cancel stream
              </button>
            )}
            {streamState === 'cancelling' && (
              <button type="button" className="secondary" disabled>
                Cancelling…
              </button>
            )}
            {streamState === 'error' && runRetained && (
              <button type="button" className="secondary" onClick={() => void handleResumeWatch()}>
                Resume watching this run
              </button>
            )}
          </div>
          {streamState === 'cancelling' && (
            <p role="status">Cancelling — waiting for the backend to confirm the stream has ended.</p>
          )}
          {streamNotice && <p role="status">{streamNotice}</p>}
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

      <section className="card" aria-labelledby="providers-access-title">
        <h2 id="providers-access-title">Browser session access</h2>
        <p>
          Owner operations (configure, authorize, credentials) are refused for paired browser
          sessions unless the host approves them here. Approval is per workspace, names the
          exact session, and expires. This panel works in the desktop app (the host); a browser
          cannot approve itself.
        </p>
        {accessNote && <p className="help">{accessNote}</p>}
        {access && (
          <>
            <h3>Pending approval requests</h3>
            {access.pending.length === 0 ? (
              <p>No browser session is waiting for approval.</p>
            ) : (
              <ul>
                {access.pending.map((p) => (
                  <li key={p.session}>
                    Session <code>{p.session}</code> — {p.attempts} refused attempt
                    {p.attempts === 1 ? '' : 's'}, last{' '}
                    {new Date(p.lastSeenMs).toLocaleTimeString()}{' '}
                    <button type="button" className="secondary" disabled={accessBusy} onClick={() => void handleApproveSession(p.session)}>
                      Approve for 30 minutes
                    </button>
                  </li>
                ))}
              </ul>
            )}
            <h3>Active grants</h3>
            {access.grants.length === 0 ? (
              <p>No active grants.</p>
            ) : (
              <ul>
                {access.grants.map((g) => (
                  <li key={g.session}>
                    Session <code>{g.session}</code> — expires{' '}
                    {new Date(g.expiresAtMs).toLocaleTimeString()}{' '}
                    <button type="button" className="secondary" disabled={accessBusy} onClick={() => void handleRevokeSession(g.session)}>
                      Revoke
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </>
        )}
        <button type="button" className="secondary" disabled={accessBusy} onClick={() => void refreshAccess()}>
          Refresh session access
        </button>
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
