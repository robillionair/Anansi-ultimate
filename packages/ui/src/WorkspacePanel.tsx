import { useEffect, useState } from 'react';
import type { Client, WorkspaceDetails, WorkspaceList } from '@anansi/contracts';

export function WorkspacePanel({client}:{client:Client}) {
  const [list,setList]=useState<WorkspaceList['workspaces']>([]);
  const [workspace,setWorkspace]=useState<WorkspaceDetails|null>(null);
  const [name,setName]=useState('');
  const [selected,setSelected]=useState('');
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState('');
  const [notice,setNotice]=useState('');
  async function run(action:()=>Promise<void>){
    setBusy(true);setError('');setNotice('');
    try{await action();}catch(error){setError(error instanceof Error?error.message:'Workspace unavailable. Retry the operation.');}
    finally{setBusy(false);}
  }
  async function refresh(){const result=await client.listWorkspaces();setList(result.workspaces);}
  useEffect(()=>{void run(refresh);},[client]);
  useEffect(()=>{
    document.documentElement.dataset.reducedMotion=String(workspace?.settings.reducedMotion??false);
    return()=>{delete document.documentElement.dataset.reducedMotion;};
  },[workspace?.settings.reducedMotion]);
  return <section className="card workspace-card" aria-labelledby="workspace-title" aria-busy={busy}>
    <h2 id="workspace-title">Your local workspace</h2>
    <p>Create a workspace or reopen one on this computer. Work is stored locally.</p>
    {error&&<p className="error" role="alert">{error}</p>}
    {notice&&<p className="notice" role="status">{notice}</p>}
    {workspace?<>
      <h3>{workspace.workspace.name}</h3><p className="workspace-id">Workspace ID: <code>{workspace.workspace.id}</code></p>
      <form onSubmit={event=>{event.preventDefault();void run(async()=>{const next=await client.updateSettings(workspace.workspace.id,!workspace.settings.reducedMotion);setWorkspace(next);setNotice('Reduced motion saved to this workspace.');});}}>
        <p>Reduced motion: <strong>{workspace.settings.reducedMotion?'On':'Off'}</strong></p>
        <p className="help">When enabled, motion and smooth scrolling are suppressed in both editions whenever this workspace is open.</p>
        <button type="submit" disabled={busy}>{workspace.settings.reducedMotion?'Turn reduced motion off':'Turn reduced motion on'}</button>
      </form>
      <div className="actions"><button className="secondary" disabled={busy} onClick={()=>{setWorkspace(null);void run(refresh);}}>Close workspace</button></div>
    </>:<>
      <form onSubmit={event=>{event.preventDefault();void run(async()=>{const next=await client.createWorkspace(name);setWorkspace(next);setName('');await refresh();});}}>
        <label htmlFor="workspace-name">New workspace name</label><input id="workspace-name" value={name} onChange={event=>setName(event.target.value)} required maxLength={64} autoComplete="off" aria-describedby="workspace-name-help"/>
        <p id="workspace-name-help" className="help">Use letters, numbers, spaces, underscores or hyphens; start with a letter or number.</p><button disabled={busy||!name} type="submit">Create workspace</button>
      </form>
      <form onSubmit={event=>{event.preventDefault();void run(async()=>setWorkspace(await client.openWorkspace(selected)));}}>
        <label htmlFor="workspace-choice">Existing workspace</label><select id="workspace-choice" value={selected} onChange={event=>setSelected(event.target.value)} disabled={busy||!list.length}><option value="">Choose a workspace</option>{list.map(workspace=><option key={workspace.id} value={workspace.id}>{workspace.name}</option>)}</select>
        {!list.length&&<p className="help">No local workspaces yet.</p>}
        <div className="actions"><button type="submit" disabled={busy||!selected}>Open workspace</button><button className="secondary" type="button" disabled={busy} onClick={()=>void run(refresh)}>Refresh workspaces</button></div>
      </form>
    </>}
    <p className="help storage-note">Workspace names and preferences are available here. Memories and AI tools arrive in later tasks.</p>
  </section>;
}
