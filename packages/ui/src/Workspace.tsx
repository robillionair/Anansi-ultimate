import { Jobs } from './Jobs';
import { Backups } from './Backups';
import { Notes } from './Notes';
import { Memory } from './Memory';
import { Search } from './Search';
import { Projects } from './Projects';
import { Tasks } from './Tasks';
import { People } from './People';
import { KnowledgeRecords } from './KnowledgeRecords';
import { useEffect, useRef, useState, type ReactNode } from 'react';
import type { Client, Intent, WorkspaceDetails, WorkspaceList } from '@anansi/contracts';
import { Navigation, type View } from './Navigation';
import { IntentChoice, Onboarding, intents } from './Onboarding';
import { Settings } from './Settings';

export function Workspace({client,native,connection,connected,busy:connecting}:{client:Client|null;native:boolean;connection:ReactNode;connected:boolean;busy:boolean}) {
  const [view,setView]=useState<View>('workspace');
  const [list,setList]=useState<WorkspaceList['workspaces']>([]);
  const [workspace,setWorkspace]=useState<WorkspaceDetails|null>(null);
  const [intent,setIntent]=useState<Intent>('explore');
  const [choosing,setChoosing]=useState(false);
  const [name,setName]=useState('');
  const [selected,setSelected]=useState('');
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState('');
  const [notice,setNotice]=useState('');
  const title=useRef<HTMLHeadingElement>(null);
  const [focusRevision,setFocusRevision]=useState(0);
  function navigate(next:View){setView(next);setNotice('');setError('');setFocusRevision(value=>value+1);}
  useEffect(()=>{title.current?.focus();},[focusRevision]);
  async function refresh(){if(client)setList((await client.listWorkspaces()).workspaces);}
  async function run(action:()=>Promise<void>){setBusy(true);setError('');setNotice('');try{await action();}catch(error){setError(error instanceof Error?error.message:'Workspace unavailable. Retry the operation.');}finally{setBusy(false);}}
  useEffect(()=>{setWorkspace(null);setList([]);setSelected('');setChoosing(false);setError('');setNotice('');if(client)void run(refresh);},[client]);
  useEffect(()=>{document.documentElement.dataset.reducedMotion=String(workspace?.settings.reducedMotion??false);return()=>{delete document.documentElement.dataset.reducedMotion;};},[workspace?.settings.reducedMotion]);
  const ready=workspace?.settings.onboardingComplete??false;
  const heading=view==='search'?'Search your workspace':view==='backups'?'Your backups':view==='jobs'?'Your jobs and exports':view==='notes'?'Your notes':view==='memory'?'Your memories':view==='projects'?'Your projects':view==='tasks'?'Your tasks':view==='people'?'Your contacts':view==='records'?'Knowledge records':view==='connection'?'Your local connection':view==='settings'?'Make it yours':ready?workspace!.workspace.name:'Welcome to ANANSI';
  return <div className="shell"><a className="skip-link" href="#main">Skip to main content</a><aside><a className="brand" href="#main"><span className="mark">A</span> ANANSI<span className="edition">ULTIMATE</span></a><Navigation view={view} hasWorkspace={ready} onNavigate={navigate}/><p className="edition-label">{native?'Windows desktop':'Browser edition'}<br/>Development · M3.1</p></aside>
    <main id="main" tabIndex={-1}><header><span className="eyebrow">YOUR PRIVATE STARTING POINT</span><span className={'badge '+(connected?'ready':'')} role="status">{connecting?'Checking…':connected?'Connected':'Not connected'}</span></header><h1 ref={title} tabIndex={-1}>{heading}</h1><p className="intro">{view==='connection'?'Check your connection or pair this browser with your computer.':ready?'Your workspace is ready. Save a memory, manage tasks or change your preferences.':'Your memory, ideas, AI, and products—all in one private workspace.'}</p>
    {view==='connection'||!client?<>{!client&&view!=='connection'&&<p>Connect locally to create or reopen a workspace. No cloud account is needed.</p>}{connection}</>:<>
      {error&&<p className="error" role="alert">{error}</p>}{notice&&<p className="notice" role="status">{notice}</p>}
      {view==='backups'&&workspace&&ready?<Backups key={workspace.workspace.id} client={client} workspaceId={workspace.workspace.id}/>:view==='jobs'&&workspace&&ready?<Jobs key={workspace.workspace.id} client={client} workspaceId={workspace.workspace.id} native={native}/>:view==='notes'&&workspace&&ready?<Notes key={workspace.workspace.id} client={client} workspaceId={workspace.workspace.id}/>:
       view==='memory'&&workspace&&ready?<Memory key={workspace.workspace.id} client={client} workspaceId={workspace.workspace.id}/>:
       view==='search'&&workspace&&ready?<Search key={workspace.workspace.id} client={client} workspaceId={workspace.workspace.id}/>:
       view==='projects'&&workspace&&ready?<Projects key={workspace.workspace.id} client={client} workspaceId={workspace.workspace.id}/>:
       view==='tasks'&&workspace&&ready?<Tasks key={workspace.workspace.id} client={client} workspaceId={workspace.workspace.id}/>:
       view==='people'&&workspace&&ready?<People key={workspace.workspace.id} client={client} workspaceId={workspace.workspace.id}/>:
       view==='records'&&workspace&&ready?<KnowledgeRecords key={workspace.workspace.id} client={client} workspaceId={workspace.workspace.id}/>:
       view==='settings'&&workspace&&ready?<Settings workspace={workspace} busy={busy} onSave={(motion,intent)=>void run(async()=>{setWorkspace(await client.updateSettings(workspace.workspace.id,motion,{intent}));setNotice('Workspace preferences saved.');})}/>:
       workspace?(ready?<section className="card" aria-labelledby="workspace-title"><h2 id="workspace-title">Your local workspace</h2><p className="workspace-id">Workspace ID: <code>{workspace.workspace.id}</code></p><p>Starting point: <strong>{intents.find(choice=>choice.id===workspace.settings.intent)!.label}</strong></p><p>Saved: your workspace name, starting point and motion preference. You can manage projects, tasks, contacts, notes and memories. AI tools are not available yet.</p><div className="actions"><button onClick={()=>navigate('settings')}>Open settings</button><button className="secondary" disabled={busy} onClick={()=>{setWorkspace(null);setChoosing(true);void run(refresh);}}>Close workspace</button></div></section>:<Onboarding key={workspace.workspace.id} workspace={workspace} intent={workspace.settings.intent} busy={busy} onFinish={()=>void run(async()=>{setWorkspace(await client.updateSettings(workspace.workspace.id,workspace.settings.reducedMotion,{onboardingComplete:true}));navigate('workspace');})}/>):<section className="card" aria-labelledby="welcome-title" aria-busy={busy}>
        {!choosing?<><p className="eyebrow">STEP 1 OF 4</p><h2 id="welcome-title">Choose your starting point</h2><IntentChoice value={intent} onChange={setIntent}/><p>You can change this later. Your choice does not start an AI task.</p><button disabled={busy} onClick={()=>{setChoosing(true);setFocusRevision(value=>value+1);}}>Continue to workspace</button></>:<><p className="eyebrow">STEP 2 OF 4</p><h2 id="welcome-title">Create or reopen a workspace</h2><p>Your starting point: {intents.find(choice=>choice.id===intent)!.label}.</p><form onSubmit={event=>{event.preventDefault();void run(async()=>{const created=await client.createWorkspace(name);setWorkspace(created);setName('');const updated=await client.updateSettings(created.workspace.id,false,{intent});setWorkspace(updated);});}}><label htmlFor="workspace-name">New workspace name</label><input id="workspace-name" value={name} onChange={event=>setName(event.target.value)} required maxLength={64} autoComplete="off" aria-describedby="name-help"/><p id="name-help" className="help">Use 1–64 ASCII letters, numbers, spaces, underscores or hyphens. Start with a letter or number.</p><button disabled={busy||!name} type="submit">Create workspace</button></form><button className="text-button" disabled={busy} onClick={()=>setChoosing(false)}>Change starting point</button></>}
        <form onSubmit={event=>{event.preventDefault();void run(async()=>{const opened=await client.openWorkspace(selected);setWorkspace(opened);setIntent(opened.settings.intent);});}}><label htmlFor="workspace-choice">Existing workspace</label><select id="workspace-choice" value={selected} onChange={event=>setSelected(event.target.value)} disabled={busy||!list.length}><option value="">Choose a workspace</option>{list.map(workspace=><option key={workspace.id} value={workspace.id}>{workspace.name}</option>)}</select>{!list.length&&<p className="help">No local workspaces yet. Create one to get started.</p>}<div className="actions"><button disabled={busy||!selected} type="submit">Open workspace</button><button disabled={busy} type="button" className="secondary" onClick={()=>void run(refresh)}>Refresh workspaces</button></div></form>
      </section>}
    </>}
    <footer><strong>Local foundation · In development</strong><p>Workspace settings, notes, memory lifecycle, projects, tasks, entities and relationships work today. Workspace backups and restore preview are available. AI and the encrypted vault remain in development.</p></footer></main></div>;
}
