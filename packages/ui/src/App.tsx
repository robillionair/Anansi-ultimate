import { useEffect, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { httpClient, nativeClient, type Client, type Readiness } from '@anansi/contracts';

export function App() {
  const [native] = useState(() => isTauri());
  const [client, setClient] = useState<Client | null>(() => native ? nativeClient() : null);
  const [readiness, setReadiness] = useState<Readiness | null>(null);
  const [endpoint, setEndpoint] = useState('http://127.0.0.1:1451');
  const [token, setToken] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  async function refresh(current = client) {
    if (!current) return;
    setBusy(true); setError('');
    try { setReadiness(await current.readiness()); }
    catch (e) { setReadiness(null); setError(e instanceof Error ? e.message : 'Connection failed. Try again.'); }
    finally { setBusy(false); }
  }
  useEffect(() => { if (native) void refresh(client); }, []);
  return <div className="shell">
    <aside><a className="brand" href="#main"><span className="mark">A</span> ANANSI<span className="edition">ULTIMATE</span></a><div className="side-note">A local foundation<br/>for connected thinking.</div><p className="edition-label">{native ? 'Windows desktop' : 'Browser edition'}<br/>Development · M1.2</p></aside>
    <main id="main"><header><span className="eyebrow">YOUR LOCAL CORE</span><span className="privacy">● Local connection</span></header><h1>Start with a connection.</h1><p className="intro">Check that your application can reach ANANSI’s shared core.</p>
      <section className="card" aria-labelledby="connection-title"><div className="card-heading"><div><h2 id="connection-title">Core connection</h2><p>{native ? 'Using the desktop’s direct native command.' : 'Pair this browser with the API running on your computer.'}</p></div><span className={'badge '+(readiness ? 'ready' : '')} role="status">{busy ? 'Checking…' : readiness ? 'Connected' : 'Not connected'}</span></div>
        {!native && !readiness && <form onSubmit={e => {e.preventDefault(); try { const next = httpClient(endpoint, token); setClient(next); setToken(''); void refresh(next); } catch (e) { setError((e as Error).message); }}}>
          <label htmlFor="endpoint">Local API address</label><input id="endpoint" value={endpoint} onChange={e => setEndpoint(e.target.value)} autoComplete="off" spellCheck={false}/>
          <label htmlFor="token">Session token</label><input id="token" type="password" value={token} onChange={e => setToken(e.target.value)} autoComplete="off" spellCheck={false} aria-describedby="pairing-help"/>
          <p className="help" id="pairing-help">Start <code>pnpm dev:api</code>. Open the private pairing file shown in its terminal and copy its token here. The token stays in this tab’s memory and is cleared when you reload or forget the session.</p><button disabled={busy} type="submit">Connect to core <span aria-hidden="true">↗</span></button>
        </form>}
        {error && <p className="error" role="alert">{error}</p>}
        {readiness && <dl><div><dt>Application</dt><dd>{readiness.applicationName}</dd></div><div><dt>Core version</dt><dd>{readiness.coreVersion}</dd></div><div><dt>Operating system</dt><dd>{readiness.operatingSystem}</dd></div><div><dt>Core uptime</dt><dd>{readiness.uptimeMs} ms</dd></div><div><dt>Contract version</dt><dd>{readiness.contractVersion}</dd></div><div><dt>Application version</dt><dd>{readiness.applicationVersion}</dd></div>{readiness.apiVersion && <div><dt>API version</dt><dd>{readiness.apiVersion}</dd></div>}</dl>}
        {(native || readiness || client) && <div className="actions"><button disabled={busy} onClick={() => void refresh()}>Check again</button>{!native && <button className="secondary" disabled={busy} onClick={() => {setClient(null);setReadiness(null);setToken('');setError('');}}>Forget session</button>}</div>}
      </section><footer><strong>Foundation in progress</strong><p>This check verifies the shared core and its connection. Workspaces, memories and AI tools are not available yet.</p></footer>
    </main></div>;
}
