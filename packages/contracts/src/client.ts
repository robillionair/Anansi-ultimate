import { validReadiness, validError, validWorkspaceDetails, validWorkspaceList, validMemoryDetails, validMemoryList } from './validators.mjs';
import { invoke } from '@tauri-apps/api/core';
import type { Readiness, WorkspaceDetails, WorkspaceList, Intent, MemoryDraft, MemoryDetails, MemoryList, MemoryMetadata, Privacy } from './generated';
export type { Readiness, WorkspaceDetails, WorkspaceList, Intent, MemoryDraft, MemoryDetails, MemoryList, MemoryMetadata, Privacy } from './generated';
export interface Client {
  listMemories(workspaceId:string, options?:{id?:string;offset?:number;limit?:number}): Promise<MemoryList>;
  readMemory(workspaceId:string,id:string,revisionId?:string): Promise<MemoryDetails>;
  createMemory(workspaceId:string,draft:MemoryDraft): Promise<MemoryDetails>;
  reviseMemory(workspaceId:string,id:string,expectedRevision:string,draft:MemoryDraft,confirmPrivacyChange?:boolean): Promise<MemoryDetails>;
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
function failure(value:unknown):Error {
  if(validError(value)){
    switch(value.code){
      case 'INVALID_MEMORY': return new Error('Check the memory fields and privacy confirmation. Use a title up to 200 characters, text up to 256 KiB, UTC dates and confidence from 0 to 1.');
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
    listMemories:async(workspaceId,options={})=>decode(await call('memories/list',{workspaceId,...options}),validMemoryList),
    readMemory:async(workspaceId,id,revisionId)=>decode(await call('memories/read',{workspaceId,id,revisionId}),validMemoryDetails),
    createMemory:async(workspaceId,draft)=>decode(await call('memories/create',{workspaceId,draft}),validMemoryDetails),
    reviseMemory:async(workspaceId,id,expectedRevision,draft,confirmPrivacyChange=false)=>decode(await call('memories/revise',{workspaceId,id,expectedRevision,draft,confirmPrivacyChange}),validMemoryDetails),
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
    try{response=await fetch(`${url.origin}/v1/${operation.startsWith('memories/')?operation:operation==='readiness'?'readiness':'workspaces/'+operation}`,{method:'POST',credentials:'omit',cache:'no-store',redirect:'error',headers:{Authorization:`Bearer ${token}`,'Content-Type':'application/json'},body:JSON.stringify({contractVersion:1,...request}),signal:AbortSignal.timeout(15000)});}
    catch{throw new Error('Local core unavailable. Start the API, then retry or pair with its new session.');}
    let value:unknown;try{value=await response.json();}catch{throw new Error('The core returned an unreadable response. Try again.');}
    if(!response.ok)throw failure(value);return value;
  });
}
export function nativeClient():Client {
  return client(async(operation,request)=>{
    try{return operation.startsWith('memories/')?await invoke('memory_command',{command:operation.slice(9),request:{contractVersion:1,...request}}):operation==='readiness'?await invoke('get_readiness',{request:{contractVersion:1}}):await invoke('workspace_command',{command:operation,request:{contractVersion:1,...request}});}
    catch(error){throw failure(error);}
  });
}
