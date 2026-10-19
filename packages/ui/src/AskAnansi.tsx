import { useState } from 'react';
import type { Client, AskResponse, GroundedCitation } from '@anansi/contracts';

export function AskAnansi({ client, workspaceId }: { client: Client; workspaceId: string }) {
  const [prompt, setPrompt] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [result, setResult] = useState<AskResponse | null>(null);
  const [showManifest, setShowManifest] = useState(false);

  async function handleSubmit(event: React.FormEvent) {
    event.preventDefault();
    const trimmed = prompt.trim();
    if (!trimmed) return;

    setBusy(true);
    setError('');
    setResult(null);
    setShowManifest(false);

    try {
      const res = await client.askAnansi(workspaceId, trimmed);
      setResult(res);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unable to complete Ask ANANSI query.');
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="ask-anansi-view" aria-label="Ask ANANSI Local Assistant">
      <section className="card" aria-labelledby="ask-anansi-title" aria-busy={busy}>
        <h2 id="ask-anansi-title">Ask ANANSI</h2>
        <p>
          Ask questions grounded strictly in your saved workspace memories, notes, and records.
          Inference runs 100% locally on your machine via loopback runtime (Ollama / LM Studio).
          Zero telemetry, zero cloud data transfer.
        </p>

        <form onSubmit={handleSubmit}>
          <fieldset disabled={busy}>
            <label htmlFor="ask-prompt">Your Question</label>
            <input
              id="ask-prompt"
              type="text"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="e.g. What did we decide about search and offline support?"
              maxLength={500}
              required
              autoComplete="off"
            />
            <div className="actions">
              <button type="submit" disabled={busy || !prompt.trim()}>
                {busy ? 'Grounding & Thinking…' : 'Ask Question'}
              </button>
            </div>
          </fieldset>
        </form>

        {busy && <p role="status" className="loading-state">Compiling context from workspace and querying local AI…</p>}
        {error && <p role="alert" className="error">{error}</p>}

        {result && (
          <div className="ask-result-section" aria-label="Answer and Citations">
            <div className={`status-banner status-${result.status}`}>
              <span className="badge">{result.status.toUpperCase()}</span>
              {result.status === 'insufficientEvidence' && (
                <span className="status-desc">No relevant memories found in this workspace.</span>
              )}
              {result.status === 'offline' && (
                <span className="status-desc">Local AI runtime is offline. Grounded context compiled below.</span>
              )}
              {result.status === 'answered' && (
                <span className="status-desc">Answered using verified workspace citations.</span>
              )}
            </div>

            <div className="answer-box" aria-label="Generated Answer">
              <h3>Answer</h3>
              <p className="answer-text">{result.answer}</p>
            </div>

            {result.citations.length > 0 && (
              <div className="citations-box" aria-label="Grounded Citations">
                <h3>Grounded Citations ({result.citations.length})</h3>
                <ol className="citation-list">
                  {result.citations.map((c: GroundedCitation) => (
                    <li key={c.index} className="citation-item">
                      <div className="citation-header">
                        <strong className="citation-title">[{c.index}] {c.title}</strong>
                        <span className="citation-meta">
                          {c.kind} · {c.sectionHeading ?? 'General'}{c.lineNumber ? ` (line ${c.lineNumber})` : ''}
                        </span>
                      </div>
                      {c.snippet && <blockquote className="citation-snippet">{c.snippet}</blockquote>}
                    </li>
                  ))}
                </ol>
              </div>
            )}

            <div className="manifest-box" aria-label="Data Disclosure">
              <button
                type="button"
                className="secondary manifest-toggle"
                onClick={() => setShowManifest(!showManifest)}
              >
                {showManifest ? 'Hide What ANANSI Sent' : 'View What ANANSI Sent (Disclosure)'}
              </button>

              {showManifest && (
                <div className="manifest-details">
                  <h4>Context Disclosure Manifest</h4>
                  <ul className="manifest-metrics">
                    <li><strong>Runtime:</strong> {result.manifest.runtimeKind} ({result.manifest.endpoint})</li>
                    <li><strong>Model:</strong> {result.manifest.modelTag}</li>
                    <li><strong>Characters:</strong> {result.manifest.characterCount}</li>
                    <li><strong>Estimated Tokens:</strong> {result.manifest.estimatedTokens}</li>
                    <li><strong>Routing Reason:</strong> {result.manifest.routingReason}</li>
                    <li><strong>Sources:</strong> {result.manifest.sourceMemories.map((s) => s.title).join(', ') || 'None'}</li>
                  </ul>
                  <label htmlFor="compiled-prompt-preview">Compiled Prompt Sent to Local Model:</label>
                  <pre id="compiled-prompt-preview" className="compiled-prompt-text">
                    {result.manifest.compiledPrompt || '(No prompt compiled)'}
                  </pre>
                </div>
              )}
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
