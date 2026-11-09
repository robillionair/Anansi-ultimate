import { useEffect, useRef, useState } from 'react';
import type {
  Client,
  CompletionRequest,
  DisclosureRecord,
  ProviderCatalogEntry,
  ProviderStatusOrphanedCredential,
  ProviderStatusResponse,
  ProviderUsageResponse,
  RuntimeProvider,
} from '@anansi/contracts';

/**
 * AI Providers — the M5 configuration, credential-reference,
 * authorization, dispatch and disclosure workflow (§§14–19),
 * plus the guided Connect AI flow: choose a provider, confirm
 * the endpoint, set the credential in the native prompt, choose
 * a model, review and save — one provider at a time, without
 * touching the providers or budget already configured.
 *
 * Trust model, made visible rather than hidden: configuration and
 * paid authorization are OWNER operations (the desktop host, or a
 * browser session the host has granted — the backend enforces
 * this and this screen surfaces its refusals). Secrets are never
 * typed here: the host generates a credential REFERENCE; the
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

/** The model rows a fresh Connect AI flow starts with: a new model
 * is cost-unknown until verified (M5-D5) — never free by default. */
export function defaultFlowModels(): ModelRow[] {
  return [{ id: '', cost: 'unknown' }];
}

/**
 * Unused credentials: keys stored during a setup that was never
 * saved. Each entry is a label only (provider + reference) — no
 * secret — with a removal action. Rendered only when the list is
 * non-empty; kept separate so it can be unit-tested statically.
 */
export function UnusedCredentials({
  orphaned,
  busy,
  note,
  onRemove,
}: {
  orphaned: ProviderStatusOrphanedCredential[];
  busy: string;
  note: string;
  onRemove: (entry: ProviderStatusOrphanedCredential) => void;
}) {
  if (orphaned.length === 0) return null;
  return (
    <div>
      <h3>Unused credentials</h3>
      <p>
        Keys stored during a setup that was never saved. They are not connected to any
        provider — removing one deletes it from the operating system credential store.
        A credential a saved connection uses never appears here.
      </p>
      <ul>
        {orphaned.map((o) => (
          <li key={o.reference}>
            {o.provider} — {o.reference}{' '}
            <button type="button" className="secondary" disabled={busy === o.reference} onClick={() => onRemove(o)}>
              Remove unused key
            </button>
          </li>
        ))}
      </ul>
      {note && <p role="status">{note}</p>}
    </div>
  );
}

export function Providers({ client, workspaceId }: { client: Client; workspaceId: string }) {
  const [catalog, setCatalog] = useState<ProviderCatalogEntry[]>([]);
  const [catalogError, setCatalogError] = useState('');

  // Selected provider for dispatch / authorization (set by the
  // catalog picker and by a successful Connect AI save).
  const [providerId, setProviderId] = useState('ollama');

  // Connected providers (backend-owned status: credential stored
  // vs verified in this session, plus the surviving budget).
  const [connected, setConnected] = useState<ProviderStatusResponse | null>(null);
  // Unused-credential removal (orphaned setup keys).
  const [orphanBusy, setOrphanBusy] = useState('');
  const [orphanNote, setOrphanNote] = useState('');

  // Guided Connect AI flow (steps 2–5; step 1 is the catalog).
  const [flowStep, setFlowStep] = useState(0);
  const [flowProviderId, setFlowProviderId] = useState('');
  const [flowBaseUrl, setFlowBaseUrl] = useState('');
  const [flowCustomEndpoint, setFlowCustomEndpoint] = useState(false);
  const [flowAllowInsecure, setFlowAllowInsecure] = useState(false);
  const [flowReference, setFlowReference] = useState('');
  const [flowCredStored, setFlowCredStored] = useState<boolean | null>(null);
  const [flowModels, setFlowModels] = useState<ModelRow[]>([{ id: '', cost: 'unknown' }]);
  const [flowError, setFlowError] = useState('');
  const [flowNotice, setFlowNotice] = useState('');
  const [flowBusy, setFlowBusy] = useState(false);
  const [flowDone, setFlowDone] = useState(false);

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
    void refreshConnected();
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

  async function refreshConnected() {
    try {
      // No owner check on this op: a paired browser may see the
      // setup status and is directed to the desktop for keys.
      setConnected(await client.providersStatus(workspaceId));
    } catch {
      setConnected(null);
    }
  }

  // --- Guided Connect AI flow -------------------------------------

  const flowEntry = catalog.find((c) => c.id === flowProviderId);
  const flowNeedsKey = flowProviderId !== '' && flowProviderId !== 'ollama' && flowProviderId !== 'lmstudio';
  const flowLocality = deriveLocality(flowBaseUrl);

  function startFlow(entry: ProviderCatalogEntry) {
    setFlowProviderId(entry.id);
    setFlowBaseUrl(entry.baseUrl ?? '');
    setFlowCustomEndpoint(false);
    setFlowAllowInsecure(false);
    setFlowReference('');
    setFlowCredStored(null);
    setFlowModels(defaultFlowModels());
    setFlowError('');
    setFlowNotice('');
    setFlowDone(false);
    setFlowStep(2);
  }

  function pickProvider(entry: ProviderCatalogEntry) {
    setProviderId(entry.id);
    startFlow(entry);
  }

  function isOwnerRefusal(e: unknown) {
    const message = e instanceof Error ? e.message : '';
    return /owner authorization|Forbidden/i.test(message);
  }

  // Whether the draft's reference is used by a saved connection:
  // such a credential is protected and can never be removed from
  // the draft (the backend refuses it too).
  const flowReferenceIsReferenced =
    flowReference.trim() !== '' &&
    (connected?.providers.some((p) => p.credentialReference === flowReference.trim()) ?? false);

  function resetFlow() {
    setFlowProviderId('');
    setFlowBaseUrl('');
    setFlowCustomEndpoint(false);
    setFlowAllowInsecure(false);
    setFlowReference('');
    setFlowCredStored(null);
    setFlowModels(defaultFlowModels());
    setFlowError('');
    setFlowDone(false);
    setFlowStep(0);
  }

  // Discard the draft (AC-V6/V9): leave setup and, when this
  // draft stored a key no saved connection uses, remove that
  // unused key through the guarded op. If the key became
  // referenced meanwhile (an overlapping setup saved it first)
  // the backend refuses and the key is kept; if this is a paired
  // browser the key stays stored and listed under Unused
  // credentials for the desktop to remove. The draft always
  // resets either way.
  async function handleDiscardDraft() {
    setFlowBusy(true);
    setFlowError('');
    const reference = flowReference.trim();
    const stored = flowCredStored === true && reference !== '';
    try {
      if (stored) {
        try {
          await client.providersCredentialRemove(workspaceId, flowProviderId, reference);
          setFlowNotice('Draft discarded. The unused key was removed from the OS credential store.');
        } catch (e) {
          if (isOwnerRefusal(e)) {
            setFlowNotice(
              'Draft discarded. The key is still stored — remove it in the desktop app, or find it under Unused credentials.',
            );
          } else if (/in use/i.test(e instanceof Error ? e.message : '')) {
            setFlowNotice('Draft discarded. The key is now in use by a saved connection, so it was kept.');
          } else {
            setFlowNotice('Draft discarded. The key could not be removed — find it under Unused credentials.');
          }
        }
      } else {
        setFlowNotice('Draft discarded.');
      }
      await refreshConnected();
    } finally {
      resetFlow();
      setFlowBusy(false);
    }
  }

  // Remove one orphaned (unused) credential from the Connected
  // providers list.
  async function handleRemoveOrphan(entry: ProviderStatusOrphanedCredential) {
    setOrphanBusy(entry.reference);
    setOrphanNote('');
    try {
      await client.providersCredentialRemove(workspaceId, entry.provider, entry.reference);
      setOrphanNote(`Removed the unused ${entry.provider} key (${entry.reference}).`);
      await refreshConnected();
    } catch (e) {
      setOrphanNote(
        isOwnerRefusal(e)
          ? 'Remove unused keys in the desktop app — browsers cannot manage API keys.'
          : e instanceof Error
            ? e.message
            : 'Credential removal failed.',
      );
    } finally {
      setOrphanBusy('');
    }
  }

  async function handleGenerateReference() {
    setFlowBusy(true);
    setFlowError('');
    try {
      const issued = await client.providersCredentialReference(workspaceId, flowProviderId);
      setFlowReference(issued.reference);
      setFlowCredStored(false);
      setFlowNotice('Reference created on the host. The key itself is entered in the native prompt, never here.');
    } catch (e) {
      setFlowError(
        isOwnerRefusal(e)
          ? 'This needs the desktop app: browsers cannot create credential references. Ask the host to open AI Providers on the desktop.'
          : e instanceof Error ? e.message : 'Could not create a credential reference.',
      );
    } finally {
      setFlowBusy(false);
    }
  }

  async function handleFlowCredentialSet() {
    setFlowBusy(true);
    setFlowError('');
    setFlowNotice('');
    try {
      await client.providersCredentialSet(workspaceId, flowProviderId, flowReference.trim());
      setFlowCredStored(true);
      setFlowNotice('Stored in the OS credential store under this reference.');
    } catch (e) {
      const message = e instanceof Error ? e.message : 'Credential setup failed.';
      setFlowError(
        isOwnerRefusal(e)
          ? 'Set this credential in the desktop app — browsers cannot enter API keys.'
          : /cancelled/i.test(message)
            ? 'Credential entry was cancelled. Nothing was changed — try again.'
            : message,
      );
    } finally {
      setFlowBusy(false);
    }
  }

  async function handleFlowCredentialRemove() {
    setFlowBusy(true);
    setFlowError('');
    setFlowNotice('');
    try {
      await client.providersCredentialRemove(workspaceId, flowProviderId, flowReference.trim());
      setFlowCredStored(false);
      setFlowNotice('Credential removed from the OS credential store.');
    } catch (e) {
      setFlowError(
        isOwnerRefusal(e)
          ? 'Remove this credential in the desktop app — browsers cannot manage API keys.'
          : e instanceof Error ? e.message : 'Credential removal failed.',
      );
    } finally {
      setFlowBusy(false);
    }
  }

  async function handleFlowCredentialStatus() {
    setFlowBusy(true);
    setFlowError('');
    try {
      const status = await client.providersCredentialStatus(workspaceId, flowProviderId, flowReference.trim());
      setFlowCredStored(status.present);
    } catch (e) {
      setFlowError(
        isOwnerRefusal(e)
          ? 'Check this credential in the desktop app.'
          : e instanceof Error ? e.message : 'Credential status check failed.',
      );
    } finally {
      setFlowBusy(false);
    }
  }

  function flowProviderEntry(): RuntimeProvider {
    return {
      config: {
        provider: flowProviderId,
        baseUrl: flowBaseUrl.trim(),
        credential: flowReference.trim()
          ? { provider: flowProviderId, reference: flowReference.trim() }
          : null,
        enabled: true,
        customEndpoint: flowCustomEndpoint,
        allowInsecureCustom: flowAllowInsecure,
      },
      locality: flowLocality,
      models: flowModels
        .filter((m) => m.id.trim())
        .map((m) => ({
          id: m.id.trim(),
          provider: flowProviderId,
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
    };
  }

  async function handleFlowSave() {
    setFlowBusy(true);
    setFlowError('');
    try {
      const saved = await client.providersConfigureAdd(workspaceId, flowProviderEntry());
      setProviderId(flowProviderId);
      const first = flowModels.find((m) => m.id.trim());
      if (first) setModel(first.id.trim());
      setFlowDone(true);
      setFlowNotice(
        `Saved ${flowProviderId} (${saved.updated ? 'updated' : 'added'}). ` +
          'Your other providers and budget are unchanged.',
      );
      await refreshConnected();
      await refreshUsage();
    } catch (e) {
      setFlowError(
        (e instanceof Error ? e.message : 'Saving failed.') +
          ' Your existing connections are still usable — review the step and try again.',
      );
    } finally {
      setFlowBusy(false);
    }
  }

  function flowCanContinue() {
    if (flowStep === 2) return flowBaseUrl.trim().length > 0;
    if (flowStep === 3) return !flowNeedsKey || flowReference.trim().length > 0;
    if (flowStep === 4) return flowModels.some((m) => m.id.trim());
    return true;
  }

  async function refreshUsage() {
    try {
      setUsage(await client.providersUsage(workspaceId));
    } catch {
      setUsage(null);
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

  const selectedEntry = catalog.find((c) => c.id === providerId);
  const flowSelectedEntry = catalog.find((c) => c.id === flowProviderId);
  const guidedProviders = ['ollama', 'openai', 'openrouter', 'anthropic', 'gemini'];

  return (
    <div className="providers-view" aria-label="AI Providers">
      <section className="card" aria-labelledby="providers-catalog-title">
        <h2 id="providers-catalog-title">Step 1 — Choose a provider</h2>
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
                {guidedProviders.includes(entry.id) ? ' · Connect AI' : ''}
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

      <section className="card" aria-labelledby="providers-connect-title">
        <h2 id="providers-connect-title">Connect AI</h2>
        {flowStep === 0 && (
          <p>
            Pick a provider above to start. Connecting adds one provider at a time and never
            replaces the providers or budget you already configured.
          </p>
        )}
        {flowStep > 0 && flowSelectedEntry && (
          <>
            <p role="status">
              Step {flowStep} of 5 — {flowSelectedEntry.displayName}
              {flowDone ? ' — connected.' : ''}
            </p>
            {flowStep === 2 && (
              <div>
                <h3>Step 2 — Endpoint</h3>
                <label htmlFor="connect-base-url">Base URL</label>
                <input id="connect-base-url" value={flowBaseUrl} onChange={(e) => setFlowBaseUrl(e.target.value)} required autoComplete="off" spellCheck={false} disabled={flowBusy} />
                <p className="help">
                  Locality is derived from the endpoint by the backend: this URL is{' '}
                  <strong>{flowLocality}</strong>. A configuration claiming otherwise is rejected.
                </p>
                <details>
                  <summary>Advanced settings — custom endpoint</summary>
                  <label htmlFor="connect-custom">
                    <input id="connect-custom" type="checkbox" checked={flowCustomEndpoint} onChange={(e) => setFlowCustomEndpoint(e.target.checked)} disabled={flowBusy} />{' '}
                    Custom endpoint (a non-catalog base URL)
                  </label>
                  <label htmlFor="connect-insecure">
                    <input id="connect-insecure" type="checkbox" checked={flowAllowInsecure} onChange={(e) => setFlowAllowInsecure(e.target.checked)} disabled={flowBusy || !flowCustomEndpoint} />{' '}
                    Allow a plaintext (http) custom endpoint — only for an internal gateway, and never with a credential
                  </label>
                </details>
                <div className="actions">
                  <button type="button" className="secondary" disabled={flowBusy} onClick={() => setFlowStep(0)}>Back to providers</button>
                  <button type="button" disabled={flowBusy || !flowCanContinue()} onClick={() => setFlowStep(flowNeedsKey ? 3 : 4)}>Continue</button>
                  <button type="button" className="secondary" disabled={flowBusy} onClick={() => void handleDiscardDraft()}>Discard draft</button>
                </div>
              </div>
            )}
            {flowStep === 3 && (
              <div>
                <h3 id="providers-credential-title">Credential setup</h3>
                <p>
                  Approved setup for <strong>{flowProviderId}</strong>. Setting a credential opens
                  the trusted native prompt on this computer: the key moves from the prompt into the
                  operating system credential store and is never typed into, or sent through, this
                  page. In a paired browser this is an owner operation and is refused — set
                  credentials in the desktop app.
                </p>
                <label htmlFor="connect-credential">Credential reference (OS keyring account)</label>
                <input id="connect-credential" value={flowReference} readOnly autoComplete="off" spellCheck={false} placeholder="Generate a reference first" />
                <p className="help">
                  The reference is generated on the host; ANANSI stores only the reference and
                  resolves the secret inside the backend at dispatch time.
                </p>
                <div className="actions">
                  <button type="button" className="secondary" disabled={flowBusy} onClick={() => void handleGenerateReference()}>
                    Generate reference
                  </button>
                  <button type="button" disabled={flowBusy || !flowReference.trim()} onClick={() => void handleFlowCredentialSet()}>
                    Set credential (native prompt)
                  </button>
                  <button type="button" className="secondary" disabled={flowBusy || !flowReference.trim() || flowReferenceIsReferenced} onClick={() => void handleFlowCredentialRemove()}>
                    Remove credential
                  </button>
                  <button type="button" className="secondary" disabled={flowBusy || !flowReference.trim()} onClick={() => void handleFlowCredentialStatus()}>
                    Check status
                  </button>
                </div>
                {flowReferenceIsReferenced && (
                  <p className="help">
                    This credential is in use by a saved connection — removing it would break that
                    connection. Save the connection under a different credential first.
                  </p>
                )}
                {flowCredStored !== null && (
                  <p role="status">Credential for this reference: {flowCredStored ? 'present in the OS store' : 'not present'}.</p>
                )}
                <div className="actions">
                  <button type="button" className="secondary" disabled={flowBusy} onClick={() => setFlowStep(2)}>Back</button>
                  <button type="button" disabled={flowBusy || !flowCanContinue()} onClick={() => setFlowStep(4)}>Continue</button>
                  <button type="button" className="secondary" disabled={flowBusy} onClick={() => setFlowStep(4)}>Continue without a credential</button>
                  <button type="button" className="secondary" disabled={flowBusy} onClick={() => void handleDiscardDraft()}>Discard draft</button>
                </div>
              </div>
            )}
            {flowStep === 4 && (
              <div>
                <h3>Step 4 — Choose a model</h3>
                <p className="help">
                  Enter the model id exactly as the provider names it. A new model is cost{' '}
                  <strong>unknown</strong> until verified — unknown is treated as paid, so
                  Zero-Cost Lock blocks it until it is verified free or a paid permit is issued.
                  ANANSI never assumes an unverified model is free.
                </p>
                {flowModels.map((row, i) => (
                  <div key={i} className="model-row">
                    <label htmlFor={`connect-model-${i}`}>Model</label>
                    <input
                      id={`connect-model-${i}`}
                      value={row.id}
                      onChange={(e) =>
                        setFlowModels(flowModels.map((m, j) => (j === i ? { ...m, id: e.target.value } : m)))
                      }
                      autoComplete="off"
                      disabled={flowBusy}
                    />
                    <label htmlFor={`connect-model-cost-${i}`}>Cost class</label>
                    <select
                      id={`connect-model-cost-${i}`}
                      value={row.cost}
                      disabled={flowBusy}
                      onChange={(e) =>
                        setFlowModels(
                          flowModels.map((m, j) =>
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
                  disabled={flowBusy}
                  onClick={() => setFlowModels([...flowModels, { id: '', cost: 'unknown' }])}
                >
                  Add model
                </button>
                <div className="actions">
                  <button type="button" className="secondary" disabled={flowBusy} onClick={() => setFlowStep(flowNeedsKey ? 3 : 2)}>Back</button>
                  <button type="button" disabled={flowBusy || !flowCanContinue()} onClick={() => setFlowStep(5)}>Review</button>
                  <button type="button" className="secondary" disabled={flowBusy} onClick={() => void handleDiscardDraft()}>Discard draft</button>
                </div>
              </div>
            )}
            {flowStep === 5 && (
              <div>
                <h3>Step 5 — Review and save</h3>
                <dl>
                  <div><dt>Provider</dt><dd>{flowSelectedEntry.displayName} ({flowProviderId})</dd></div>
                  <div><dt>Endpoint</dt><dd>{flowBaseUrl} ({flowLocality})</dd></div>
                  <div><dt>Credential</dt><dd>{flowReference.trim() ? `${flowReference.trim()} — ${flowCredStored ? 'stored in the OS store' : 'not stored yet'}` : 'none (keyless)'}</dd></div>
                  <div><dt>Models</dt><dd>{flowModels.filter((m) => m.id.trim()).map((m) => `${m.id.trim()} (${m.cost})`).join(', ')}</dd></div>
                </dl>
                <p className="help">
                  Saving adds or updates this one provider. Your existing budget and other
                  providers are unchanged, and setup makes no inference calls — the connection is
                  verified on your first authorized use.
                </p>
                <div className="actions">
                  <button type="button" className="secondary" disabled={flowBusy} onClick={() => setFlowStep(4)}>Back</button>
                  <button type="button" disabled={flowBusy || flowDone} onClick={() => void handleFlowSave()}>Save connection</button>
                  <button type="button" className="secondary" disabled={flowBusy} onClick={() => void handleDiscardDraft()}>Discard draft</button>
                  {flowDone && (
                    <button type="button" className="secondary" onClick={() => { setFlowStep(0); setFlowDone(false); }}>
                      Connect another provider
                    </button>
                  )}
                </div>
              </div>
            )}
          </>
        )}
        {flowError && <p className="error" role="alert" aria-label="Connect AI error">{flowError}</p>}
        {flowNotice && <p role="status">{flowNotice}</p>}
      </section>

      <section className="card" aria-labelledby="providers-connected-title">
        <h2 id="providers-connected-title">Connected providers</h2>
        <p>
          What this workspace has configured. Credential stored means a key is in the operating
          system store; connection verified means a dispatch has succeeded for that exact saved
          connection — its endpoint and credential — in this app session. Changing the endpoint
          or replacing the key clears it until the new connection succeeds. A fresh setup is
          always unverified until its first authorized use.
        </p>
        {!connected ? (
          <p>No provider status available.</p>
        ) : connected.providers.length === 0 ? (
          <p>No providers connected yet.</p>
        ) : (
          <ul>
            {connected.providers.map((p) => (
              <li key={p.provider}>
                <strong>{p.displayName}</strong> — {p.baseUrlHost} ({p.locality})
                {p.enabled ? '' : ' · disabled'}
                <br />
                Models: {p.models.map((m) => `${m.id} (${m.cost})`).join(', ')}
                <br />
                Credential: {p.credentialReference ? `${p.credentialReference} — ${p.credentialStored ? 'stored' : 'not stored'}` : 'none (keyless)'}
                <br />
                Connection: {p.connectionVerified ? 'verified in this session' : 'not yet verified'}
              </li>
            ))}
          </ul>
        )}
        {connected && (
          <p className="help">
            Budget: {connected.budget.maxRequests != null ? `max ${connected.budget.maxRequests} requests` : 'no request cap'}
            {connected.budget.maxTotalTokens != null ? `, max ${connected.budget.maxTotalTokens} tokens` : ', no token cap'}
            {connected.budget.maxEstimatedCostMicroUsd != null ? `, cost cap ${connected.budget.maxEstimatedCostMicroUsd} µUSD` : ''}.
          </p>
        )}
        {connected && (
          <UnusedCredentials
            orphaned={connected.orphanedCredentials}
            busy={orphanBusy}
            note={orphanNote}
            onRemove={(entry) => void handleRemoveOrphan(entry)}
          />
        )}
        <button type="button" className="secondary" onClick={() => void refreshConnected()}>Refresh connected providers</button>
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
export function DisclosurePanel({
  disclosure,
  headingId = 'providers-disclosure-title',
}: {
  disclosure: DisclosureRecord;
  /** R3: an instance-scoped heading id; other panels keep the legacy id. */
  headingId?: string;
}) {
  return (
    <section className="card disclosure" aria-labelledby={headingId}>
      <h2 id={headingId}>View what ANANSI sent</h2>
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
