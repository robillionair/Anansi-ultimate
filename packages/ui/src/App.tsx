import { useEffect, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { httpClient, nativeClient, type Client, type Readiness } from '@anansi/contracts';
import { Workspace } from './Workspace';

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
  const connection=<section className="card" aria-labelledby="connection-title"><h2 id="connection-title">Core connection</h2><p>{native ? 'Connected directly to this computer.' : 'Pair this browser with the local service running on your computer.'}</p>
    {!native && !readiness && <form onSubmit={e => {e.preventDefault(); try { const next = httpClient(endpoint, token); setClient(next); setToken(''); void refresh(next); } catch (e) { setError((e as Error).message); }}}>
      <label htmlFor="endpoint">Local API address</label><input id="endpoint" value={endpoint} onChange={e => setEndpoint(e.target.value)} autoComplete="off" spellCheck={false}/>
      <label htmlFor="token">Session token</label><input id="token" type="password" value={token} onChange={e => setToken(e.target.value)} autoComplete="off" spellCheck={false} aria-describedby="pairing-help"/>
      <p className="help" id="pairing-help">Open the private pairing file provided by the local service and copy its session token here. It stays in this tab’s memory and is forgotten on reload. See the repository’s development guide for starting the service.</p><button disabled={busy} type="submit">Connect to core</button>
    </form>}
    {error && <p className="error" role="alert">{error}</p>}
    {readiness && <><p>Connected. Choose Workspace to create or reopen your local workspace.</p><dl><div><dt>Application</dt><dd>{readiness.applicationName}</dd></div><div><dt>Core version</dt><dd>{readiness.coreVersion}</dd></div><div><dt>Operating system</dt><dd>{readiness.operatingSystem}</dd></div><div><dt>Core uptime</dt><dd>{readiness.uptimeMs} ms</dd></div><div><dt>Contract version</dt><dd>{readiness.contractVersion}</dd></div><div><dt>Application version</dt><dd>{readiness.applicationVersion}</dd></div>{readiness.apiVersion && <div><dt>API version</dt><dd>{readiness.apiVersion}</dd></div>}</dl></>}
    {(native || readiness || client) && <div className="actions"><button disabled={busy} onClick={() => void refresh()}>Check again</button>{!native && <button className="secondary" disabled={busy} onClick={() => {setClient(null);setReadiness(null);setToken('');setError('');}}>Forget session</button>}</div>}
  </section>;
  return <Workspace client={readiness?client:null} native={native} connection={connection} connected={!!readiness} busy={busy}/>;
}
