import { useEffect, useRef, useState } from 'react';
import type { Client, ProviderDispatchResponse } from '@anansi/contracts';

const MAX_PROMPT_CHARS = 32_768;
const MAX_OUTPUT_TOKENS = 384;

type ReviewedPair = {
  provider: string;
  model: string;
  baseline: string;
  candidate: string;
  cloud: boolean;
};
type CompletedPair = {
  input: ReviewedPair;
  baseline: ProviderDispatchResponse;
  candidate: ProviderDispatchResponse;
};

export type SomniaModelActivity = {
  phase: 'idle' | 'running' | 'complete' | 'error';
  baselineTokens?: number;
  candidateTokens?: number;
};

function totalTokens(response: ProviderDispatchResponse) {
  const usage = response.response.usage ?? response.disclosure.usage;
  return usage ? usage.inputTokens + usage.outputTokens : undefined;
}

function usageText(response: ProviderDispatchResponse) {
  const usage = response.response.usage ?? response.disclosure.usage;
  return usage
    ? `${usage.inputTokens} input + ${usage.outputTokens} output = ${usage.inputTokens + usage.outputTokens} total tokens`
    : 'Token usage unavailable';
}

export function SomniaModelCompare({ client, workspaceId, projectId, onSaved, busy, onActivity }: {
  client: Client;
  workspaceId: string;
  projectId: string;
  onSaved: () => Promise<void>;
  busy: boolean;
  onActivity?: (activity: SomniaModelActivity) => void;
}) {
  const [input, setInput] = useState<ReviewedPair>({
    provider: 'ollama', model: '', baseline: '', candidate: '', cloud: false,
  });
  const [reviewed, setReviewed] = useState<ReviewedPair | null>(null);
  const [result, setResult] = useState<CompletedPair | null>(null);
  const [running, setRunning] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState('');
  const [error, setError] = useState('');
  const mounted = useRef(false);
  const cancelledRef = useRef(false);
  const savingRef = useRef(false);

  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);

  useEffect(() => {
    onActivity?.({
      phase: running ? 'running' : result ? 'complete' : error ? 'error' : 'idle',
      baselineTokens: running ? undefined : result ? totalTokens(result.baseline) : undefined,
      candidateTokens: running ? undefined : result ? totalTokens(result.candidate) : undefined,
    });
  }, [error, onActivity, result, running]);

  function change(patch: Partial<ReviewedPair>) {
    setInput(previous => ({ ...previous, ...patch }));
    setReviewed(null);
  }

  function cancel() {
    if (!running) return;
    cancelledRef.current = true;
    setRunning(false);
    setError('Model comparison cancelled.');
  }

  async function compare(snapshot: ReviewedPair) {
    if (running || saving || busy) return;
    cancelledRef.current = false;
    setRunning(true);
    setReviewed(null);
    setResult(null);
    setSaved('');
    setError('');
    async function dispatch(text: string) {
      const result = await client.providersDispatch(workspaceId, {
        model: snapshot.model,
        localOnly: !snapshot.cloud,
        explicitCloudAuthorization: snapshot.cloud,
        maxOutputTokens: MAX_OUTPUT_TOKENS,
        temperature: 0,
        ...(snapshot.provider === "openrouter" ? { disableOpenRouterReasoning: true } : {}),
        messages: [{ role: 'user', segments: [{ text, privacy: 'private' }] }],
      }, { mode: 'chooseModel', chosen: [snapshot.provider, snapshot.model], zeroCostLock: true });
      if (result.disclosure.provider !== snapshot.provider || result.disclosure.model !== snapshot.model ||
          result.response.model !== snapshot.model) {
        throw Error('The provider reported a different model. This pair cannot be compared.');
      }
      return result;
    }
    try {
      const baseline = await dispatch(snapshot.baseline);
      if (!mounted.current || cancelledRef.current) return;
      const candidate = await dispatch(snapshot.candidate);
      if (mounted.current && !cancelledRef.current) setResult({ input: snapshot, baseline, candidate });
    } catch (e) {
      if (mounted.current && !cancelledRef.current) setError(e instanceof Error ? e.message : 'Model comparison failed.');
    } finally {
      if (mounted.current && !cancelledRef.current) setRunning(false);
    }
  }

  async function save(completed: CompletedPair) {
    if (saving || savingRef.current || saved || busy) return;
    if (!mounted.current) return;
    savingRef.current = true;
    setSaving(true);
    setError('');
    try {
      const artifact = await client.registerProjectArtifact(workspaceId, projectId, {
        type: 'experiment',
        title: 'SOMNIA model comparison',
        privacy: 'private',
        body: JSON.stringify({ format: 'anansi-somnia-model-comparison', version: 1, ...completed }, null, 2),
      });
      if (mounted.current) setSaved(artifact.artifact.artifactId);
      await onSaved();
    } catch (e) {
      if (mounted.current) setError(e instanceof Error ? e.message : 'Could not save the comparison.');
    } finally {
      savingRef.current = false;
      if (mounted.current) setSaving(false);
    }
  }

  return <section aria-label="SOMNIA model comparison">
    <h4>Compare with model</h4>
    <p>Paste the original replay prompt and the proposed version. This sends two real requests to the same configured model with Zero-Cost Lock enabled.</p>
    <form onSubmit={event => {
      event.preventDefault();
      if (busy || running || saving || !input.provider.trim() || !input.model.trim() ||
          !input.baseline.trim() || !input.candidate.trim()) return;
      setError('');
      setReviewed({ ...input, provider: input.provider.trim(), model: input.model.trim() });
    }}>
      <fieldset disabled={busy || running || saving}>
        <label>Provider ID <input required maxLength={128} value={input.provider} onChange={e => change({ provider: e.target.value })} /></label>
        <label>Model ID <input required maxLength={256} value={input.model} onChange={e => change({ model: e.target.value })} /></label>
        <label>Original replay prompt <textarea required rows={4} maxLength={MAX_PROMPT_CHARS} value={input.baseline} onChange={e => change({ baseline: e.target.value })} /></label>
        <label>Proposed replay prompt <textarea required rows={4} maxLength={MAX_PROMPT_CHARS} value={input.candidate} onChange={e => change({ candidate: e.target.value })} /></label>
        <label><input type="checkbox" checked={input.cloud} onChange={e => change({ cloud: e.target.checked })} /> Authorize cloud transmission of these two prompts</label>
        <button type="submit">Review model comparison</button>
      </fieldset>
    </form>
    {reviewed && <section aria-label="Reviewed model prompts">
      <p>{reviewed.provider} / {reviewed.model} · {reviewed.cloud ? 'Cloud transmission authorized for the text below' : 'Local only'}</p>
      <details open><summary>Original prompt</summary><pre style={{ whiteSpace: 'pre-wrap' }}>{reviewed.baseline}</pre></details>
      <details open><summary>Proposed prompt</summary><pre style={{ whiteSpace: 'pre-wrap' }}>{reviewed.candidate}</pre></details>
      <button type="button" disabled={busy || running || saving} onClick={() => void compare(reviewed)}>Run approved model comparison</button>
    </section>}
    {running && <div>
      <p role="status">Running model comparison…</p>
      <button type="button" onClick={cancel}>Cancel model comparison</button>
    </div>}
    {error && <p role="alert">{error}</p>}
    {result && <section aria-label="Model comparison results">
      <h5>Model comparison results</h5>
      {(['baseline', 'candidate'] as const).map(arm => <article key={arm} aria-label={arm === 'baseline' ? 'Original result' : 'Proposed result'}>
        <h6>{arm === 'baseline' ? 'Original' : 'Proposed'}</h6>
        <p>{result[arm].disclosure.provider} / {result[arm].response.model} · {usageText(result[arm])}</p>
        <pre style={{ whiteSpace: 'pre-wrap' }}>{result[arm].response.text}</pre>
        <details><summary>Prompt and dispatch disclosure</summary>
          <pre style={{ whiteSpace: 'pre-wrap' }}>{result.input[arm]}</pre>
          <pre style={{ whiteSpace: 'pre-wrap' }}>{JSON.stringify(result[arm].disclosure, null, 2)}</pre>
        </details>
      </article>)}
      <button type="button" disabled={busy || running || saving || Boolean(saved)} onClick={() => void save(result)}>Save model comparison to project</button>
      {saved && <p role="status">Saved private model comparison to project artifacts.</p>}
    </section>}
  </section>;
}
