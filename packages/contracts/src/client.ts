import { validReadiness, validError, validWorkspaceDetails, validWorkspaceList, validMemoryDetails, validMemoryList, validRelationshipDetails, validRelationshipList, validJobDetails, validJobList, validArtifactChunk } from './validators.mjs';
import { invoke } from '@tauri-apps/api/core';
import {validSnapshotPreview,validRestoreResult,validBackupSchedule,validUploadStatus} from './validators.mjs';
import type {SnapshotPreview,RestoreResult,BackupSchedule} from './generated';
export type {SnapshotPreview,RestoreResult,BackupSchedule} from './generated';
import type { Readiness, WorkspaceDetails, WorkspaceList, Intent, MemoryDraft, MemoryDetails, MemoryList, MemoryMetadata, Privacy, LifecycleChange, RelationshipDraft, RelationshipRecord, RelationshipDetails, RelationshipList, Job, JobDetails, JobList, ArtifactChunk, ExportSelection, JobAction } from './generated';
export type { Readiness, WorkspaceDetails, WorkspaceList, Intent, MemoryDraft, MemoryDetails, MemoryList, MemoryMetadata, Privacy, LifecycleChange, RelationshipDraft, RelationshipRecord, RelationshipDetails, RelationshipList, Job, JobDetails, JobList, ArtifactChunk, ExportSelection, JobAction } from './generated';
export interface Client {
  importBackup(workspaceId:string,bytes:Uint8Array,plaintextConsent:boolean):Promise<Job>;
  enqueueBackup(workspaceId:string,application:boolean,plaintextConsent:boolean):Promise<JobDetails>;
  previewBackup(workspaceId:string,jobId:string,artifactId:string):Promise<SnapshotPreview>;
  restoreBackup(workspaceId:string,jobId:string,artifactId:string,name:string,expectedHash:string,confirm:boolean):Promise<RestoreResult>;
  readBackupSchedule(workspaceId:string):Promise<BackupSchedule>;
  setBackupSchedule(workspaceId:string,enabled:boolean,intervalSeconds:number,plaintextConsent:boolean,keepAllVersions:boolean,managedDestination:boolean):Promise<BackupSchedule>;
  enqueueExport(workspaceId:string,selection:ExportSelection):Promise<JobDetails>;
  listJobs(workspaceId:string,offset?:number):Promise<JobList>;
  controlJob(workspaceId:string,id:string,expectedVersion:number,action:JobAction):Promise<JobDetails>;
  readArtifact(workspaceId:string,jobId:string,artifactId:string,offset:number):Promise<ArtifactChunk>;
  listMemories(workspaceId:string, options?:{id?:string;offset?:number;limit?:number;kind?:string;query?:string;includeArchived?:boolean;includeExpired?:boolean}): Promise<MemoryList>;
  readMemory(workspaceId:string,id:string,revisionId?:string): Promise<MemoryDetails>;
  createMemory(workspaceId:string,draft:MemoryDraft,kind?:string): Promise<MemoryDetails>;
  reviseMemory(workspaceId:string,id:string,expectedRevision:string,draft:MemoryDraft,confirmPrivacyChange?:boolean,options?:{lifecycle?:LifecycleChange;restoreArchive?:boolean;restoreRevision?:string}): Promise<MemoryDetails>;
  createRelationship(workspaceId:string,draft:RelationshipDraft): Promise<RelationshipDetails>;
  deleteRelationship(workspaceId:string,id:string,expectedVersion:number): Promise<RelationshipDetails>;
  restoreRelationship(workspaceId:string,id:string,expectedVersion:number): Promise<RelationshipDetails>;
  listRelationships(workspaceId:string,options?:{recordId?:string;predicate?:string;includeHistory?:boolean}): Promise<RelationshipList>;
  readiness(): Promise<Readiness>;
  listWorkspaces(): Promise<WorkspaceList>;
  createWorkspace(name: string): Promise<WorkspaceDetails>;
  openWorkspace(id: string): Promise<WorkspaceDetails>;
  updateSettings(id: string, reducedMotion: boolean, preferences?: {intent?:Intent;onboardingComplete?:boolean}): Promise<WorkspaceDetails>;
}
function decode<T extends {contractVersion:number}>(value:unknown, validate:(value:unknown)=>value is T):T {
  if(!validate(value)||value.contractVersion!==1)throw new Error('The core returned an incompatible response. Update both applications.');
  return value;
}
export function decodeReadiness(value:unknown):Readiness {return decode(value,validReadiness);}
function failure(value:unknown,operation=''):Error {
  if(validError(value)){
    if(operation.startsWith('jobs/')&&value.code==='REVISION_CONFLICT')return new Error('This job changed or its lease expired. Refresh jobs and try again.');
    switch(value.code){
      case 'NOT_FOUND': return new Error('The requested item was not found in this workspace. Try again after refreshing.');
      case 'INVALID_REQUEST': return new Error('Check the selection, resource limits and current job state, then refresh.');
      case 'INVALID_MEMORY': return new Error('Check the fields, lifecycle action, expiry, successor and privacy confirmation. Use a title up to 200 characters, text up to 256 KiB, UTC dates and confidence from 0 to 1.');
      case 'INVALID_SEARCH': return new Error('Use one to eight title words, up to 256 UTF-8 bytes. If the workspace exceeds 10,000 eligible records or 32 MiB of metadata, narrow the record type. History search is unavailable; no partial results were returned.');
      case 'MEMORY_NOT_FOUND': return new Error('Memory not found in this workspace. Refresh the list.');
      case 'REVISION_CONFLICT': return new Error('This memory changed. Your draft was kept. Reopen the latest revision before saving again.');
      case 'CONTENT_INTEGRITY': return new Error('Memory content failed integrity verification. Preserve the storage root and restore a verified copy.');
      case 'VAULT_UNAVAILABLE': return new Error('Sealed memories require the future encrypted vault. No plaintext fallback was used.');
      case 'UNAUTHORIZED': return new Error('Pairing was rejected. Enter the current session token.');
      case 'FORBIDDEN': return new Error('This browser address is not allowed. Open http://127.0.0.1:1420.');
      case 'UNSUPPORTED_CONTRACT': return new Error('The core uses a different contract version. Update both applications.');
      case 'INVALID_WORKSPACE': return new Error('Use a valid workspace name or choose an existing workspace from the list.');
      case 'WORKSPACE_EXISTS': return new Error('A workspace with that name already exists. Open it from the list or use a different name.');
      case 'WORKSPACE_NOT_FOUND': return new Error('Workspace not found. Refresh the list and choose an existing workspace.');
      case 'STORAGE_UNAVAILABLE': return new Error('Local storage is unavailable. Check the configured folder and its permissions, then retry.');
      case 'UNSUPPORTED_SCHEMA': return new Error('Storage uses an unsupported schema version. Keep the original files and use a compatible ANANSI version.');
      case 'CORRUPT_STORAGE': return new Error('Workspace storage cannot be read. Keep the original files and restore a verified backup.');
    }
  }
  return new Error('The core could not complete the request. Try again.');
}
type Call=(operation:string,request:Record<string,unknown>)=>Promise<unknown>;
function client(call:Call):Client {
  return {
    importBackup:async(workspaceId,bytes,plaintextConsent)=>{
      if(bytes.length>256*1024*1024||bytes.length<1024)throw Error('Choose a backup up to 256 MiB.');
      const sha256=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',Uint8Array.from(bytes)))).map(b=>b.toString(16).padStart(2,'0')).join('');
      const opened=decode(await call('jobs/upload-begin',{workspaceId,bytes:bytes.length,sha256,plaintextConsent}),validUploadStatus);
      let result=opened;
      for(let offset=0;offset<bytes.length;offset+=65536){const end=Math.min(offset+65536,bytes.length);result=decode(await call('jobs/upload-part',{workspaceId,id:opened.id,offset,bytes:Array.from(bytes.subarray(offset,end)),finish:end===bytes.length}),validUploadStatus);}
      if(!result.job)throw Error('Backup import did not publish a verified job.');return result.job;
    },
    enqueueBackup:async(workspaceId,application,plaintextConsent)=>decode(await call('jobs/backup',{workspaceId,application,plaintextConsent}),validJobDetails),
    previewBackup:async(workspaceId,jobId,artifactId)=>decode(await call('jobs/preview',{workspaceId,jobId,artifactId}),validSnapshotPreview),
    restoreBackup:async(workspaceId,jobId,artifactId,name,expectedHash,confirm)=>decode(await call('jobs/restore',{workspaceId,jobId,artifactId,name,expectedHash,confirm}),validRestoreResult),
    readBackupSchedule:async(workspaceId)=>decode(await call('jobs/schedule-read',{workspaceId}),validBackupSchedule),
    setBackupSchedule:async(workspaceId,enabled,intervalSeconds,plaintextConsent,keepAllVersions,managedDestination)=>decode(await call('jobs/schedule',{workspaceId,enabled,intervalSeconds,plaintextConsent,keepAllVersions,managedDestination}),validBackupSchedule),
    enqueueExport:async(workspaceId,selection)=>decode(await call('jobs/export',{workspaceId,selection}),validJobDetails),
    listJobs:async(workspaceId,offset=0)=>decode(await call('jobs/list',{workspaceId,offset}),validJobList),
    controlJob:async(workspaceId,id,expectedVersion,action)=>decode(await call('jobs/control',{workspaceId,id,expectedVersion,action}),validJobDetails),
    readArtifact:async(workspaceId,jobId,artifactId,offset)=>decode(await call('jobs/artifact',{workspaceId,jobId,artifactId,offset}),validArtifactChunk),
    listMemories:async(workspaceId,options={})=>decode(await call('memories/list',{workspaceId,...options}),validMemoryList),
    readMemory:async(workspaceId,id,revisionId)=>decode(await call('memories/read',{workspaceId,id,revisionId}),validMemoryDetails),
    createMemory:async(workspaceId,draft,kind)=>decode(await call('memories/create',{workspaceId,draft,kind}),validMemoryDetails),
    reviseMemory:async(workspaceId,id,expectedRevision,draft,confirmPrivacyChange=false,options={})=>decode(await call('memories/revise',{workspaceId,id,expectedRevision,draft,confirmPrivacyChange,...options}),validMemoryDetails),
    createRelationship:async(workspaceId,draft)=>decode(await call('relationships/create',{workspaceId,draft}),validRelationshipDetails),
    deleteRelationship:async(workspaceId,id,expectedVersion)=>decode(await call('relationships/delete',{workspaceId,id,expectedVersion}),validRelationshipDetails),
    restoreRelationship:async(workspaceId,id,expectedVersion)=>decode(await call('relationships/restore',{workspaceId,id,expectedVersion}),validRelationshipDetails),
    listRelationships:async(workspaceId,options={})=>decode(await call('relationships/list',{workspaceId,...options}),validRelationshipList),
    readiness:async()=>decodeReadiness(await call('readiness',{})),
    listWorkspaces:async()=>decode(await call('list',{}),validWorkspaceList),
    createWorkspace:async name=>decode(await call('create',{name}),validWorkspaceDetails),
    openWorkspace:async id=>decode(await call('open',{id}),validWorkspaceDetails),
    updateSettings:async(id,reducedMotion,preferences={})=>decode(await call('settings',{id,reducedMotion,...preferences}),validWorkspaceDetails),
  };
}
export function httpClient(endpoint:string,token:string):Client {
  let url:URL;
  try{url=new URL(endpoint);}catch{throw new Error('Enter a local API address such as http://127.0.0.1:1451.');}
  if(url.protocol!=='http:'||url.hostname!=='127.0.0.1'||!url.port||url.username||url.password||url.pathname!=='/'||url.search||url.hash)throw new Error('Use an HTTP address on 127.0.0.1 with a port and no path.');
  if(!/^[a-f0-9]{64}$/.test(token))throw new Error('Enter the 64-character token from the current private pairing file.');
  return client(async(operation,request)=>{
    let response:Response;
    const path = operation.startsWith('jobs/') || operation.startsWith('memories/') || operation.startsWith('relationships/')
      ? operation
      : operation === 'readiness'
      ? 'readiness'
      : 'workspaces/' + operation;
    try{response=await fetch(`${url.origin}/v1/${path}`,{method:'POST',credentials:'omit',cache:'no-store',redirect:'error',headers:{Authorization:`Bearer ${token}`,'Content-Type':'application/json'},body:JSON.stringify({contractVersion:1,...request}),signal:AbortSignal.timeout(operation.startsWith('jobs/')?120000:15000)});}
    catch{throw new Error('Local core unavailable. Start the API, then retry or pair with its new session.');}
    let value:unknown;try{value=await response.json();}catch{throw new Error('The core returned an unreadable response. Try again.');}
    if(!response.ok)throw failure(value,operation);return value;
  });
}
export function nativeClient():Client {
  return client(async(operation,request)=>{
    try{
      if(operation.startsWith('jobs/'))return await invoke('job_command',{command:operation.slice(5),request:{contractVersion:1,...request}});
      if(operation.startsWith('memories/'))return await invoke('memory_command',{command:operation.slice(9),request:{contractVersion:1,...request}});
      if(operation.startsWith('relationships/'))return await invoke('relationship_command',{command:operation.slice(14),request:{contractVersion:1,...request}});
      if(operation==='readiness')return await invoke('get_readiness',{request:{contractVersion:1}});
      return await invoke('workspace_command',{command:operation,request:{contractVersion:1,...request}});
    }
    catch(error){throw failure(error,operation);}
  });
}

export async function saveNativeExport(workspaceId:string,jobId:string,artifactId:string):Promise<boolean>{
  try {const saved=await invoke('save_export',{request:{contractVersion:1,workspaceId,jobId,artifactId,offset:0}});if(typeof saved!=='boolean')throw Error();return saved;}catch(error){throw failure(error);}
}
