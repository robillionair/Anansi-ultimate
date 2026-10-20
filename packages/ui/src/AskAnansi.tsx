import { useEffect, useRef, useState } from 'react';
import type { Client, AskResponse, GroundedCitation } from '@anansi/contracts';

export function AskAnansi({ client, workspaceId }: { client: Client; workspaceId: string }) {
  const [prompt, setPrompt] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [result, setResult] = useState<AskResponse | null>(null);
  const [showManifest, setShowManifest] = useState(false);
  const [model,setModel]=useState('qwen2.5:0.5b');
  const [embeddingModel,setEmbeddingModel]=useState('all-minilm:22m');
  const [endpoint,setEndpoint]=useState('http://127.0.0.1:11434');
  const [partial,setPartial]=useState('');
  const active=useRef<string|null>(null);
  const cancelled=useRef(false);
  useEffect(()=>()=>{cancelled.current=true;const id=active.current;if(id)void client.forgetAsk(workspaceId,id).catch(()=>{});},[client,workspaceId]);

  async function handleSubmit(event: React.FormEvent) {
    event.preventDefault();
    const trimmed = prompt.trim();
    if (!trimmed) return;

    setBusy(true);
    setError('');
    setResult(null);
    setShowManifest(false);
    setPartial('');cancelled.current=false;

    try {
      const opened=await client.startAsk(workspaceId,trimmed,{
        runtimeEndpoint:endpoint.trim(),
        modelTag:model.trim(),
        ...(embeddingModel.trim()?{embeddingModel:embeddingModel.trim()}:{}),
      });
      active.current=opened.id;
      if(cancelled.current){await client.cancelAsk(workspaceId,opened.id);return;}
      for(;;){
        const run=await client.pollAsk(workspaceId,opened.id);
        if(cancelled.current||run.state==='cancelled')break;
        setPartial(run.partialAnswer);
        if(run.state==='completed'){
          if(run.error)throw Error(run.error.message);
          setResult(run.response??null);break;
        }
        await new Promise(resolve=>setTimeout(resolve,50));
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unable to complete Ask ANANSI query.');
    } finally {
      const id=active.current;active.current=null;if(id)await client.forgetAsk(workspaceId,id).catch(()=>{});
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
            <label htmlFor="ask-model">Installed Ollama model</label><input id="ask-model" value={model} onChange={event=>setModel(event.target.value)} required autoComplete="off"/>
            <label htmlFor="ask-embedding-model">Embedding model for semantic grounding</label><input id="ask-embedding-model" value={embeddingModel} onChange={event=>setEmbeddingModel(event.target.value)} autoComplete="off"/>
            <label htmlFor="ask-endpoint">Local inference endpoint</label><input id="ask-endpoint" value={endpoint} onChange={event=>setEndpoint(event.target.value)} required autoComplete="off"/>
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
        {busy&&<button type="button" onClick={()=>{cancelled.current=true;setPartial('');setError('Cancelling local generation…');const id=active.current;if(id)void client.cancelAsk(workspaceId,id).catch(error=>setError(error.message));}}>Cancel generation</button>}
        {busy&&partial&&<section aria-label="Streaming answer"><h3>Generating answer</h3><p>{partial}</p></section>}

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
                <span className="status-desc">Generated from workspace evidence. Check the cited sources; citations do not guarantee correctness.</span>
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
