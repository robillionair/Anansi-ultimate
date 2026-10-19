import { useState } from 'react';
import { nativeVaultCommand } from '@anansi/contracts';

export function Vault({workspaceId}:{workspaceId:string}) {
  const [jobId,setJobId]=useState(''),[artifactId,setArtifactId]=useState(''),[sha256,setSha256]=useState('');
  const [busy,setBusy]=useState(false),[notice,setNotice]=useState(''),[error,setError]=useState('');
  async function run(label:string,action:()=>Promise<unknown>){setBusy(true);setError('');setNotice('');try{await action();setNotice(label);}catch(e){setError(e instanceof Error?e.message:'Vault operation failed.');}finally{setBusy(false);}}
  return <section className="card" aria-labelledby="vault-title" aria-busy={busy}>
    <h2 id="vault-title">Sealed vault</h2>
    <p>Credentials and recovery acknowledgement use trusted Windows dialogs. Keys and session handles never enter this page.</p>
    {error&&<p className="error" role="alert">{error}</p>}{notice&&<p className="notice" role="status">{notice}</p>}
    <div className="actions"><button disabled={busy} onClick={()=>void run('Recovery acknowledged. Select the verified backup below to seal this workspace.',()=>nativeVaultCommand('setup',{workspaceId}))}>Set up vault</button><button disabled={busy} onClick={()=>void run('Vault unlocked for this desktop process.',()=>nativeVaultCommand('unlock',{workspaceId}))}>Unlock</button><button disabled={busy} className="secondary" onClick={()=>void run('Vault locked and the desktop session was revoked.',()=>nativeVaultCommand('lock',{workspaceId}))}>Lock</button><button disabled={busy} className="secondary" onClick={()=>void run('Authenticated vault read completed.',()=>nativeVaultCommand('read',{workspaceId}))}>Verify authorized read</button></div>
    <details><summary>Seal using a verified plaintext backup</summary><p>The retained backup remains plaintext by design. Sealing excludes active plaintext search entries but cannot securely erase historical copies.</p><label htmlFor="vault-job">Backup job ID</label><input id="vault-job" value={jobId} onChange={e=>setJobId(e.target.value)} autoComplete="off"/><label htmlFor="vault-artifact">Artifact ID</label><input id="vault-artifact" value={artifactId} onChange={e=>setArtifactId(e.target.value)} autoComplete="off"/><label htmlFor="vault-hash">Backup SHA-256</label><input id="vault-hash" value={sha256} onChange={e=>setSha256(e.target.value)} autoComplete="off" spellCheck={false}/><button disabled={busy||!jobId||!artifactId||!/^[a-f0-9]{64}$/.test(sha256)} onClick={()=>void run('Workspace sealed. Unlock it to read encrypted records.',()=>nativeVaultCommand('seal',{workspaceId,jobId,artifactId,sha256}))}>Seal workspace</button></details>
  </section>;
}
