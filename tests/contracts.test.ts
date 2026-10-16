import test from 'node:test';
import assert from 'node:assert/strict';
import { decodeReadiness, httpClient, nativeClient } from '../packages/contracts/src/client.ts';
const actual={applicationName:'ANANSI Ultimate',applicationVersion:'0.1.0',apiVersion:null,contractVersion:1,coreVersion:'0.1.0',operatingSystem:'windows',uptimeMs:5};
test('runtime schema accepts a readiness DTO and rejects malformed and unknown fields',()=>{
  assert.deepEqual(decodeReadiness(actual),actual);
  for(const value of [{...actual,uptimeMs:'5'},{...actual,secret:'unknown'},{...actual,contractVersion:2},null,{}])assert.throws(()=>decodeReadiness(value));
});
test('static validators run without dynamic code evaluation',()=>{
  const original=globalThis.Function;
  globalThis.Function=(()=>{throw Error('Dynamic evaluation disabled');}) as unknown as FunctionConstructor;
  try{assert.deepEqual(decodeReadiness(actual),actual);}finally{globalThis.Function=original;}
});
test('browser refuses credentials in addresses and non-loopback endpoints',()=>{
  for(const endpoint of ['https://example.org','http://localhost:1451','http://127.0.0.1:1451/?token=secret','http://user:secret@127.0.0.1:1451','http://127.0.0.1:1451/path'])assert.throws(()=>httpClient(endpoint,'a'.repeat(64)));
  assert.throws(()=>httpClient('http://127.0.0.1:1451','short'));
});
test('unavailable core returns an actionable sanitized error',async()=>{
  const original=globalThis.fetch;globalThis.fetch=async()=>{throw Error('sensitive transport details');};
  try{await assert.rejects(httpClient('http://127.0.0.1:1451','a'.repeat(64)).readiness(),/Local core unavailable/);}finally{globalThis.fetch=original;}
});
test('HTTP error and malformed success responses fail safely',async()=>{
  const original=globalThis.fetch;
  try{
    globalThis.fetch=async()=>new Response(JSON.stringify({code:'UNAUTHORIZED',message:'do not echo remote text'}),{status:401});
    await assert.rejects(httpClient('http://127.0.0.1:1451','a'.repeat(64)).readiness(),/Pairing was rejected/);
    globalThis.fetch=async()=>new Response(JSON.stringify({...actual,extra:true}));
    await assert.rejects(httpClient('http://127.0.0.1:1451','a'.repeat(64)).readiness(),/incompatible/);
  }finally{globalThis.fetch=original;}
});
test('native transport validates DTOs and sanitizes rejected or unavailable IPC',async()=>{
  const original=Object.getOwnPropertyDescriptor(globalThis,'window');
  const internal={invoke:async(_command:string,_args:unknown):Promise<unknown>=>actual};
  Object.defineProperty(globalThis,'window',{configurable:true,value:{__TAURI_INTERNALS__:internal}});
  try{
    assert.deepEqual(await nativeClient().readiness(),actual);
    internal.invoke=async()=>{throw {code:'NOT_FOUND',message:'untrusted details'};};
    await assert.rejects(nativeClient().readiness(),/Try again/);
    internal.invoke=async()=>({...actual,unknown:true});
    await assert.rejects(nativeClient().readiness(),/incompatible/);
  }finally{if(original)Object.defineProperty(globalThis,'window',original);else Reflect.deleteProperty(globalThis,'window');}
});
test('workspace HTTP contracts validate settings and never send paths',async()=>{
  const original=globalThis.fetch;const calls:{url:string,body:Record<string,unknown>}[]=[];
  const details={contractVersion:1,schemaVersion:1,workspace:{id:'1'.repeat(32),name:'Synthetic'},settings:{reducedMotion:true}};
  try{
    globalThis.fetch=async(url,options)=>{calls.push({url:String(url),body:JSON.parse(options!.body as string)});return new Response(JSON.stringify(String(url).endsWith('/list')?{contractVersion:1,workspaces:[details.workspace]}:details));};
    const api=httpClient('http://127.0.0.1:1451','a'.repeat(64));await api.listWorkspaces();await api.createWorkspace('Synthetic');await api.openWorkspace(details.workspace.id);assert.deepEqual(await api.updateSettings(details.workspace.id,true),details);
    assert.deepEqual(calls.map(call=>new URL(call.url).pathname),['/v1/workspaces/list','/v1/workspaces/create','/v1/workspaces/open','/v1/workspaces/settings']);
    assert.deepEqual(calls[3].body,{contractVersion:1,id:details.workspace.id,reducedMotion:true});assert.ok(calls.every(call=>!('path' in call.body)));
    globalThis.fetch=async()=>new Response(JSON.stringify({...details,settings:{reducedMotion:'true'}}));await assert.rejects(api.openWorkspace(details.workspace.id),/incompatible/);
    globalThis.fetch=async()=>new Response(JSON.stringify({code:'CORRUPT_STORAGE',message:'private path must not be echoed'}),{status:409});await assert.rejects(api.listWorkspaces(),/Keep the original files/);
  }finally{globalThis.fetch=original;}
});
