import {useEffect,useState} from 'react';
import type {Client,Job,SnapshotPreview,BackupSchedule} from '@anansi/contracts';

export function Backups({client,workspaceId}:{client:Client;workspaceId:string}){
  const [jobs,setJobs]=useState<Job[]>([]),[schedule,setSchedule]=useState<BackupSchedule|null>(null);
  const [consent,setConsent]=useState(false),[application,setApplication]=useState(false),[confirm,setConfirm]=useState(false);
  const [interval,setInterval]=useState(86400),[enabled,setEnabled]=useState(false),[retention,setRetention]=useState(false),[destination,setDestination]=useState(false);
  const [preview,setPreview]=useState<SnapshotPreview|null>(null),[selected,setSelected]=useState<Job|null>(null),[name,setName]=useState('');
  const [busy,setBusy]=useState(false),[error,setError]=useState(''),[notice,setNotice]=useState('');
  const [offset,setOffset]=useState(0),[next,setNext]=useState<number|null>(null);
  const [created,setCreated]=useState('');
  const [importFile,setImportFile]=useState<File|null>(null);
  async function refresh(){const list=await client.listJobs(workspaceId,offset);setJobs(list.jobs.filter(j=>j.selection.kind==='backup'));setNext(list.nextOffset??null);}
  async function run(action:()=>Promise<void>){setBusy(true);setError('');setNotice('');try{await action();}catch(e){setError(e instanceof Error?e.message:'Backup operation failed. Originals were preserved.');}finally{setBusy(false);}}
  useEffect(()=>{void run(async()=>{await refresh();const saved=await client.readBackupSchedule(workspaceId);setSchedule(saved);setEnabled(saved.enabled);setInterval(saved.intervalSeconds);});},[client,workspaceId,offset]);
  return <section aria-labelledby="backups-title" className="card backups" aria-busy={busy} data-created-backup-id={created}>
    <h2 id="backups-title">Back up and recover</h2><p>Snapshots stay in private managed storage on this computer. Use Jobs and export to save a complete backup file elsewhere. A backup on the same disk cannot protect against losing that disk.</p>
    <p>These backups contain plaintext records and history. Sealed records and credentials are refused. Project repositories and product files have separate backup scopes.</p>
    {error&&<p role="alert" className="error">{error}</p>}{notice&&<p role="status" className="notice">{notice}</p>}
    <label className="check-label"><input type="checkbox" checked={consent} onChange={e=>setConsent(e.target.checked)}/>I consent to a plaintext copy of the selected workspace and its history in private managed storage.</label>
    <label className="check-label"><input type="checkbox" checked={application} onChange={e=>setApplication(e.target.checked)}/>Application data backup: include all workspaces, settings, history and completed job artifacts.</label>
    <button disabled={busy||!consent} onClick={()=>void run(async()=>{const result=await client.enqueueBackup(workspaceId,application,true);setCreated(result.job.id);setOffset(0);await refresh();setNotice('Backup queued. Refresh to check progress.');})}>Create backup</button>
    <h3>Recover a saved backup file</h3><p>Select a previously saved TAR backup. Import verifies every member before it becomes available for preview. Use this from a new local workspace after recovering or replacing your computer.</p>
    <label>Saved backup file<input type="file" accept=".tar,application/x-tar" disabled={busy} onChange={e=>setImportFile(e.target.files?.[0]??null)}/></label>
    <button disabled={busy||!consent||!importFile} onClick={()=>void run(async()=>{if(!importFile||importFile.size>256*1024*1024)throw Error('Choose a backup up to 256 MiB.');const job=await client.importBackup(workspaceId,new Uint8Array(await importFile.arrayBuffer()),true);setCreated(job.id);await refresh();setNotice('Saved backup verified. Preview it before restoring.');})}>Import saved backup</button>
    <h3>Scheduled workspace backups</h3><p>Runs only while the desktop or local API is open. Missed intervals are reported; reopening queues one current snapshot. All versions are retained, with no automatic deletion.</p>
    {schedule&&<p>Saved schedule: {schedule.enabled?'Enabled':'Off'} · Missed runs: {schedule.missedRuns} · Next due: {schedule.nextDue?new Date(schedule.nextDue*1000).toLocaleString():'None'}</p>}
    <label className="check-label"><input type="checkbox" checked={enabled} onChange={e=>setEnabled(e.target.checked)}/>Enable scheduled workspace backups</label>
    <label>Backup interval<select value={interval} onChange={e=>setInterval(Number(e.target.value))}><option value={3600}>Hourly</option><option value={86400}>Daily</option><option value={604800}>Weekly</option></select></label>
    <label className="check-label"><input type="checkbox" checked={destination} onChange={e=>setDestination(e.target.checked)}/>Use private managed storage on this computer.</label>
    <label className="check-label"><input type="checkbox" checked={retention} onChange={e=>setRetention(e.target.checked)}/>Keep all versions without automatic pruning.</label>
    <button disabled={busy||enabled&&(!consent||!destination||!retention)} onClick={()=>void run(async()=>{setSchedule(await client.setBackupSchedule(workspaceId,enabled,interval,consent,retention,destination));setNotice('Backup schedule saved.');})}>Save backup schedule</button>
    <h3>Your backup jobs</h3><button className="secondary" disabled={busy} onClick={()=>void run(refresh)}>Refresh backups</button>
    <ul>{jobs.map(job=><li key={job.id} data-backup-job={job.id}><p>{job.selection.kind==='backup'&&job.selection.application?'Application data backup':'Workspace backup'} · <strong>{job.state}</strong> · {new Date(job.createdAt).toLocaleString()}</p><p>{job.log}</p>
      {job.state==='Completed'&&<button disabled={busy} onClick={()=>void run(async()=>{const p=await client.previewBackup(workspaceId,job.id,job.artifactId!);setPreview(p);setSelected(job);setConfirm(false);setName('');setNotice('Backup integrity verified. Review the preview before restoring.');})}>Preview restore</button>}
      <p>Pause, resume, cancel, retry and save are available in Jobs and export.</p>
    </li>)}</ul>
    <div className="actions"><button disabled={busy||offset===0} onClick={()=>setOffset(Math.max(0,offset-50))}>Newer backup jobs</button><button disabled={busy||next===null} onClick={()=>setOffset(next!)}>Older backup jobs</button></div>
    {preview&&selected&&<section aria-label="Restore preview"><h3>Verified restore preview</h3><p>{preview.workspaceName} · Schema {preview.schemaVersion} · {preview.workspaceCount} workspace(s), {preview.records} records, {preview.revisions} revisions, {preview.relationships} relationships · {preview.totalBytes} bytes</p><p>{preview.privacy}</p><p>SHA-256: <code>{preview.sha256}</code></p>{preview.conflicts.map(c=><p key={c}>{c}</p>)}
      <label>Restored workspace name<input value={name} onChange={e=>setName(e.target.value)} maxLength={64}/></label>
      <label className="check-label"><input type="checkbox" checked={confirm} onChange={e=>setConfirm(e.target.checked)}/>Restore into fresh workspace IDs and preserve all original workspaces.</label>
      <button disabled={busy||!confirm||!name} onClick={()=>void run(async()=>{const restored=await client.restoreBackup(workspaceId,selected.id,selected.artifactId!,name,preview.sha256,true);setNotice('Restore complete: '+restored.workspaces.map(w=>w.name).join(', ')+'. Reopen it from Workspace.');setPreview(null);setSelected(null);})}>Restore verified backup</button>
    </section>}
  </section>;
}
