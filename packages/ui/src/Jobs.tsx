import {useEffect,useRef,useState} from 'react';
import {saveNativeExport,type Client,type Job,type MemoryMetadata,type ExportSelection,type JobAction} from '@anansi/contracts';

export function Jobs({client,workspaceId,native}:{client:Client;workspaceId:string;native:boolean}){
  const [jobs,setJobs]=useState<Job[]>([]);
  const [scope,setScope]=useState('workspace');
  const [ids,setIds]=useState<string[]>([]);
  const [choices,setChoices]=useState<MemoryMetadata[]>([]);
  const [choiceOffset,setChoiceOffset]=useState<number|null>(null);
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState('');
  const [notice,setNotice]=useState('');
  const [createdJob,setCreatedJob]=useState<string|null>(null);
  const [offset,setOffset]=useState(0);
  const [next,setNext]=useState<number|null>(null);
  const fetching=useRef(false);
  const choiceGeneration=useRef(0);
  async function loadChoices(offset=0){
    const generation=choiceGeneration.current;
    const result=await client.listMemories(workspaceId,{kind:scope==='project'?'project':undefined,offset,limit:100,includeArchived:true,includeExpired:true});
    if(generation!==choiceGeneration.current)return;
    setChoices(previous=>offset?[...previous,...result.memories]:result.memories);setChoiceOffset(result.nextOffset??null);
  }
  useEffect(()=>{choiceGeneration.current++;setIds([]);setChoices([]);setChoiceOffset(null);if(scope!=='workspace')void loadChoices().catch(e=>setError(e instanceof Error?e.message:'Selection unavailable. Retry.'));return()=>{choiceGeneration.current++;};},[scope,client,workspaceId]);
  async function refresh(){
    if(fetching.current)return;
    fetching.current=true;
    try{const result=await client.listJobs(workspaceId,offset);setJobs(result.jobs);setNext(result.nextOffset??null);}
    catch(e){setError(e instanceof Error?e.message:'Jobs unavailable. Retry.');}
    finally{fetching.current=false;}
  }
  useEffect(()=>{let stopped=false;void refresh();const timer=setInterval(()=>{if(!stopped)void refresh();},4000);return()=>{stopped=true;clearInterval(timer);};},[client,workspaceId,offset]);
  async function run(action:()=>Promise<void>){setBusy(true);setError('');setNotice('');try{await action();await refresh();}catch(e){setError(e instanceof Error?e.message:'Export unavailable. Retry.');}finally{setBusy(false);}}
  async function control(job:Job,action:JobAction){await client.controlJob(workspaceId,job.id,job.version,action);}
  async function download(job:Job){
    if(!job.artifactId||!job.artifactBytes||!job.artifactHash||job.artifactBytes>256*1024*1024)throw Error('Complete artifact missing or over its size limit. Refresh jobs.');
    if(native){const saved=await saveNativeExport(workspaceId,job.id,job.artifactId);setNotice(saved?'Complete export saved to your selected file.':'Save cancelled. The complete export remains in private storage.');return;}
    const bytes=new Uint8Array(job.artifactBytes);
    for(let offset=0;offset<bytes.length;){
      const chunk=await client.readArtifact(workspaceId,job.id,job.artifactId,offset);
      if(!chunk.bytes.length||chunk.bytes.length>64*1024||offset+chunk.bytes.length>bytes.length||chunk.bytes.some(b=>!Number.isInteger(b)||b<0||b>255))throw Error('Export integrity check failed. No download was published.');
      bytes.set(chunk.bytes,offset);offset+=chunk.bytes.length;
    }
    const hash=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))).map(b=>b.toString(16).padStart(2,'0')).join('');
    if(hash!==job.artifactHash)throw Error('Export integrity check failed. No download was published.');
    const url=URL.createObjectURL(new Blob([bytes],{type:'application/x-tar'}));
    const link=document.createElement('a');link.href=url;link.download=`anansi-${job.artifactId}.tar`;link.click();
    setTimeout(()=>URL.revokeObjectURL(url),60000);setNotice('Complete export verified and sent to your browser downloads.');
  }
  return <section className="card jobs" aria-labelledby="jobs-title" data-created-job-id={createdJob??undefined}><h2 id="jobs-title">Jobs and export</h2><p>Export your selected knowledge as Markdown and a JSON manifest in a portable TAR archive. All revisions and internal relationship history are included. Links to records outside the selection retain opaque references only.</p><p>Private content is plaintext in this export. Sealed content is refused. No network service or subscription is required. Limits: 10,000 records, 256 MiB per archive and 32 pending jobs. Backups and restore are planned separately.</p>
    {error&&<p role="alert" className="error">{error}</p>}{notice&&<p role="status">{notice}</p>}
    <form onSubmit={event=>{event.preventDefault();void run(async()=>{const selection:ExportSelection=scope==='workspace'?{kind:'workspace'}:scope==='project'?{kind:'project',id:ids[0]}:{kind:'records',ids};const result=await client.enqueueExport(workspaceId,selection);setCreatedJob(result.job.id);setOffset(0);setNotice('Export queued. Your latest request is marked below.');});}}>
      <label htmlFor="export-scope">Export selection</label><select id="export-scope" value={scope} onChange={e=>setScope(e.target.value)}><option value="workspace">Entire workspace</option><option value="project">Project and its current members</option><option value="records">Selected records</option></select>
      {scope==='project'&&<><label htmlFor="export-project">Project to export</label><select id="export-project" required value={ids[0]??''} onChange={e=>setIds([e.target.value])}><option value="">Choose a project</option>{choices.map(record=><option key={record.id} value={record.id}>{record.title}</option>)}</select></>}
      {scope==='records'&&<fieldset><legend>Records to export, including archived and expired items</legend>{choices.map(record=><label className="check-label" key={record.id}><input type="checkbox" checked={ids.includes(record.id)} onChange={e=>setIds(previous=>e.target.checked?[...previous,record.id]:previous.filter(id=>id!==record.id))}/>{record.title} ({record.kind})</label>)}</fieldset>}
      {scope!=='workspace'&&choiceOffset!==null&&<button type="button" disabled={busy} onClick={()=>void run(()=>loadChoices(choiceOffset))}>Load more choices</button>}
      <button disabled={busy||(scope!=='workspace'&&!ids.length)} type="submit">Create knowledge export</button>
    </form><div className="actions"><button disabled={busy} onClick={()=>void refresh()}>Refresh jobs</button><button disabled={busy||offset===0} onClick={()=>setOffset(Math.max(0,offset-50))}>Newer jobs</button><button disabled={busy||next===null} onClick={()=>setOffset(next!)}>Older jobs</button></div>
    <ul>{jobs.map(job=><li key={job.id} data-job-id={job.id}><h3>{job.selection.kind==='project'?'Project export':job.selection.kind==='records'?'Selected records export':'Workspace export'}</h3>{job.id===createdJob&&<p>Your latest export request.</p>}<p>Created {new Date(job.createdAt).toLocaleString()}</p><p>State: <strong>{job.state}</strong> · Records processed: {job.progress}</p><p>Checkpoint {job.checkpointVersion}: {job.checkpoint}. {job.log}</p><div className="actions">
      {['Queued','Running'].includes(job.state)&&<button disabled={busy} onClick={()=>void run(()=>control(job,'pause'))}>Pause export</button>}
      {job.state==='Paused'&&<button disabled={busy} onClick={()=>void run(()=>control(job,'resume'))}>Resume export</button>}
      {['Queued','Running','Paused'].includes(job.state)&&<button disabled={busy} onClick={()=>void run(()=>control(job,'cancel'))}>Cancel export</button>}
      {['Failed','Cancelled'].includes(job.state)&&<button disabled={busy} onClick={()=>void run(()=>control(job,'retry'))}>Retry export</button>}
      {job.state==='Completed'&&<button disabled={busy} onClick={()=>void run(()=>download(job))}>{native?'Save complete export':'Download complete export'}</button>}
    </div>{job.state==='Paused'&&<p>Resume explicitly. An interrupted snapshot stage is rebuilt safely; partial files are never downloadable.</p>}</li>)}</ul>{!jobs.length&&<p>No export jobs on this page.</p>}
  </section>;
}
