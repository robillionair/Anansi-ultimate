import { useEffect, useState } from 'react';
import type { Client, AiDiscoveryResponse, DiscoveredRuntime, ModelRecommendation } from '@anansi/contracts';

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const gb = bytes / (1024 * 1024 * 1024);
  if (gb >= 1) return `${gb.toFixed(1)} GB`;
  const mb = bytes / (1024 * 1024);
  return `${mb.toFixed(0)} MB`;
}

export function LocalModels({ client }: { client: Client }) {
  const [data, setData] = useState<AiDiscoveryResponse | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [customEndpoint, setCustomEndpoint] = useState('');
  const [customList, setCustomList] = useState<string[]>([]);

  async function loadDiscovery(endpoints?: string[]) {
    setBusy(true);
    setError('');
    try {
      const res = await client.aiDiscovery(endpoints && endpoints.length ? endpoints : undefined);
      setData(res);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unable to inspect local AI runtimes.');
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    void loadDiscovery(customList);
  }, [client]);

  function handleAddCustom(event: React.FormEvent) {
    event.preventDefault();
    if (!customEndpoint.trim()) return;
    const nextList = [...customList, customEndpoint.trim()];
    setCustomList(nextList);
    setCustomEndpoint('');
    void loadDiscovery(nextList);
  }

  return (
    <div className="local-models-view" aria-label="Local AI Models and Runtimes">
      <section className="card" aria-labelledby="hardware-profile-title">
        <div className="section-header">
          <h2 id="hardware-profile-title">System Capability & Hardware Profile</h2>
          <button
            className="secondary"
            disabled={busy}
            onClick={() => void loadDiscovery(customList)}
            type="button"
          >
            {busy ? 'Inspecting…' : 'Re-probe System'}
          </button>
        </div>

        {error && <p className="error" role="alert">{error}</p>}

        {data ? (
          <div className="hardware-grid">
            <div className="metric">
              <span className="metric-label">CPU Architecture:</span>
              <strong className="metric-value">{data.hardware.architecture} ({data.hardware.logicalCpuCores} logical cores)</strong>
            </div>
            <div className="metric">
              <span className="metric-label">System Memory (RAM):</span>
              <strong className="metric-value">
                {formatBytes(data.hardware.totalRamBytes)} total ({formatBytes(data.hardware.availableRamBytes)} free)
              </strong>
            </div>
            <div className="metric">
              <span className="metric-label">Storage Free Space:</span>
              <strong className="metric-value">{formatBytes(data.hardware.freeStorageBytes)} available</strong>
            </div>
            <div className="metric">
              <span className="metric-label">Graphics Acceleration:</span>
              <strong className="metric-value">
                {data.hardware.gpuName
                  ? `${data.hardware.gpuName}${data.hardware.vramBytes ? ` (${formatBytes(data.hardware.vramBytes)} VRAM)` : ''}`
                  : 'CPU Only / Integrated Graphics'}
              </strong>
            </div>
          </div>
        ) : (
          <p className="loading-state">Inspecting hardware capabilities and available memory…</p>
        )}
      </section>

      <section className="card" aria-labelledby="runtimes-title">
        <h2 id="runtimes-title">Detected Local AI Runtimes</h2>
        <p className="help">
          ANANSI connects exclusively to local loopback inference servers on your machine.
          No queries, embeddings, or prompts are sent to external cloud APIs.
        </p>

        {data && (
          <div className="runtime-cards" role="region" aria-label="Discovered Local Runtimes">
            {data.runtimes.map((rt: DiscoveredRuntime) => {
              const isAvailable = rt.status === 'available';
              return (
                <div key={rt.endpoint} className={`runtime-card ${rt.status}`}>
                  <div className="runtime-header">
                    <h3>
                      {rt.kind === 'ollama'
                        ? 'Ollama Local Engine'
                        : rt.kind === 'lmStudio'
                        ? 'LM Studio Server'
                        : 'Custom Loopback Server'}
                    </h3>
                    <span className={`status-badge ${rt.status}`} role="status">
                      {isAvailable ? 'Active' : rt.status === 'offline' ? 'Offline' : 'Unsupported'}
                    </span>
                  </div>
                  <p className="runtime-endpoint">Endpoint: <code>{rt.endpoint}</code></p>
                  {rt.version && <p className="runtime-version">Version: {rt.version}</p>}

                  {isAvailable ? (
                    <div className="runtime-models">
                      <h4>Installed Models ({rt.models.length}):</h4>
                      {rt.models.length > 0 ? (
                        <ul>
                          {rt.models.map((m: string) => (
                            <li key={m}><code>{m}</code></li>
                          ))}
                        </ul>
                      ) : (
                        <p className="help">No models currently loaded. Pull or download a model to begin.</p>
                      )}
                    </div>
                  ) : (
                    <p className="help">
                      {rt.status === 'offline'
                        ? 'Service is not running on this port. Start the application locally to enable inference.'
                        : 'The endpoint responded with an unsupported or unexpected contract.'}
                    </p>
                  )}
                </div>
              );
            })}
          </div>
        )}

        <form className="custom-endpoint-form" onSubmit={handleAddCustom}>
          <label htmlFor="custom-endpoint-input">Add Custom Loopback Endpoint</label>
          <div className="actions">
            <input
              id="custom-endpoint-input"
              type="text"
              placeholder="http://127.0.0.1:8080"
              value={customEndpoint}
              onChange={(e) => setCustomEndpoint(e.target.value)}
              pattern="^http:\/\/(127\.0\.0\.1|localhost|\[::1\]):[0-9]+(\/.*)?$"
              title="Must be a local loopback address (e.g. http://127.0.0.1:8080)"
            />
            <button type="submit" disabled={busy || !customEndpoint.trim()}>
              Add & Probe
            </button>
          </div>
          <p className="help">Only loopback endpoints (127.0.0.1 / localhost) are accepted for security.</p>
        </form>
      </section>

      <section className="card" aria-labelledby="recommendations-title">
        <h2 id="recommendations-title">Recommended Local Models for Your Hardware</h2>
        <p className="help">
          Model recommendations are scaled strictly to your system RAM and GPU VRAM to prevent system freeze or memory exhaustion.
          ANANSI requires explicit consent and never pulls models in the background.
        </p>

        {data && (
          <div className="recommendations-list">
            {data.recommendations.map((rec: ModelRecommendation) => (
              <div key={rec.modelTag} className={`rec-card ${rec.fitLevel}`}>
                <div className="rec-header">
                  <div>
                    <h3 className="rec-title"><code>{rec.modelTag}</code></h3>
                    <span className="rec-specs">
                      {rec.parameterSize} · {rec.quantization} · Min RAM: {formatBytes(rec.minimumRamBytes)}
                    </span>
                  </div>
                  <span className={`fit-badge ${rec.fitLevel}`}>
                    {rec.fitLevel === 'recommended'
                      ? 'Best Fit'
                      : rec.fitLevel === 'comfortable'
                      ? 'Comfortable'
                      : rec.fitLevel === 'tight'
                      ? 'Tight'
                      : 'Insufficient RAM'}
                  </span>
                </div>
                <p className="rec-desc">{rec.description}</p>
                <div className="rec-action-hint">
                  <span>To install via Ollama: </span>
                  <code>ollama run {rec.modelTag}</code>
                </div>
              </div>
            ))}
          </div>
        )}

        <div className="consent-callout">
          <strong>Explicit Governance & Consent:</strong>
          <p>
            Automatic background model downloads and unprompted cloud fallbacks are disabled by design.
            All model additions require deliberate user action in your preferred local runtime.
          </p>
        </div>
      </section>
    </div>
  );
}
