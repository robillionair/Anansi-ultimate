import test from 'node:test';
import assert from 'node:assert/strict';
import { decodeReadiness, httpClient, nativeClient } from '../packages/contracts/src/client.ts';
const actual={applicationName:'ANANSI Ultimate',applicationVersion:'0.1.0',apiVersion:null,contractVersion:1,coreVersion:'0.1.0',operatingSystem:'windows',uptimeMs:5};
test('jobs use scoped HTTP commands and malformed artifacts are refused',async()=>{
  const original=globalThis.fetch;let path='',request:Record<string,unknown>={};
  try{
    globalThis.fetch=async(url,options)=>{path=String(url);request=JSON.parse(options!.body as string);return new Response(JSON.stringify({contractVersion:1,jobs:[],nextOffset:null}));};
    const client=httpClient('http://127.0.0.1:1451','a'.repeat(64));
    await client.listJobs('b'.repeat(32),50);assert.equal(path,'http://127.0.0.1:1451/v1/jobs/list');assert.deepEqual(request,{contractVersion:1,workspaceId:'b'.repeat(32),offset:50});
    await assert.rejects(client.readArtifact('b'.repeat(32),'c'.repeat(32),'d'.repeat(32),0),/incompatible response/);
  }finally{globalThis.fetch=original;}
});
test('lifecycle and note options remain bounded command data with sanitized conflicts',async()=>{
  const original=globalThis.fetch;let body:Record<string,unknown>={};
  try{
    globalThis.fetch=async(_url,options)=>{body=JSON.parse(options!.body as string);return new Response(JSON.stringify({code:'REVISION_CONFLICT',message:'private details'}),{status:409});};
    const client=httpClient('http://127.0.0.1:1451','a'.repeat(64));
    await assert.rejects(client.createMemory('a'.repeat(32),{title:'Note',body:'literal'},'note'),/draft was kept/);assert.equal(body.kind,'note');
    const lifecycle={state:'confirmed' as const,confirmed:true,successorId:null};
    await assert.rejects(client.reviseMemory('a'.repeat(32),'b'.repeat(32),'c'.repeat(32),{title:'Note',body:'literal'},false,{lifecycle}),/draft was kept/);assert.deepEqual(body.lifecycle,lifecycle);assert.equal(body.expectedRevision,'c'.repeat(32));
    await assert.rejects(client.reviseMemory('a'.repeat(32),'b'.repeat(32),'c'.repeat(32),{title:'Note',body:'literal'},false,{restoreRevision:'d'.repeat(32)}),/draft was kept/);assert.equal(body.restoreRevision,'d'.repeat(32));
    await assert.rejects(client.listMemories('a'.repeat(32),{kind:'note',includeArchived:true,includeExpired:true}),/draft was kept/);assert.equal(body.includeArchived,true);assert.equal(body.includeExpired,true);assert.ok(!('path' in body));
  }finally{globalThis.fetch=original;}
});
test('memory HTTP/native operations use scoped commands, validated DTOs and sanitized errors',async()=>{
  const original=globalThis.fetch;let address='';let body:Record<string,unknown>={};
  try{
    globalThis.fetch=async(url,options)=>{address=String(url);body=JSON.parse(options!.body as string);return new Response(JSON.stringify({contractVersion:1,memories:[],nextOffset:null}));};
    const client=httpClient('http://127.0.0.1:1451','a'.repeat(64));await client.listMemories('a'.repeat(32),{offset:50});assert.equal(new URL(address).pathname,'/v1/memories/list');assert.equal(body.workspaceId,'a'.repeat(32));assert.equal(body.offset,50);assert.ok(!('path' in body));
    globalThis.fetch=async()=>new Response(JSON.stringify({contractVersion:1,memory:{id:'fake'},body:'text'}));await assert.rejects(client.readMemory('a'.repeat(32),'b'.repeat(32)),/incompatible/);
    for(const [code,message] of [['REVISION_CONFLICT','draft was kept'],['VAULT_UNAVAILABLE','future encrypted vault'],['CONTENT_INTEGRITY','integrity verification']]){globalThis.fetch=async()=>new Response(JSON.stringify({code,message:'private path must not appear'}),{status:409});await assert.rejects(client.createMemory('a'.repeat(32),{title:'X',body:'text'}),new RegExp(message));}
  }finally{globalThis.fetch=original;}
  const window=Object.getOwnPropertyDescriptor(globalThis,'window');let command='',args:unknown;
  Object.defineProperty(globalThis,'window',{configurable:true,value:{__TAURI_INTERNALS__:{invoke:async(c:string,a:unknown)=>{command=c;args=a;return {contractVersion:1,memories:[],nextOffset:null};}}}});
  try{await nativeClient().listMemories('a'.repeat(32));assert.equal(command,'memory_command');assert.deepEqual(args,{command:'list',request:{contractVersion:1,workspaceId:'a'.repeat(32)}});}finally{if(window)Object.defineProperty(globalThis,'window',window);else Reflect.deleteProperty(globalThis,'window');}
});
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
  const details={contractVersion:1,schemaVersion:2,workspace:{id:'1'.repeat(32),name:'Synthetic'},settings:{reducedMotion:true,intent:'explore',onboardingComplete:false}};
  try{
    globalThis.fetch=async(url,options)=>{calls.push({url:String(url),body:JSON.parse(options!.body as string)});return new Response(JSON.stringify(String(url).endsWith('/list')?{contractVersion:1,workspaces:[details.workspace]}:details));};
    const api=httpClient('http://127.0.0.1:1451','a'.repeat(64));await api.listWorkspaces();await api.createWorkspace('Synthetic');await api.openWorkspace(details.workspace.id);assert.deepEqual(await api.updateSettings(details.workspace.id,true),details);
    assert.deepEqual(calls.map(call=>new URL(call.url).pathname),['/v1/workspaces/list','/v1/workspaces/create','/v1/workspaces/open','/v1/workspaces/settings']);
    assert.deepEqual(calls[3].body,{contractVersion:1,id:details.workspace.id,reducedMotion:true});assert.ok(calls.every(call=>!('path' in call.body)));
    await api.updateSettings(details.workspace.id,true,{intent:'invent_product',onboardingComplete:true});
    assert.deepEqual(calls[4].body,{contractVersion:1,id:details.workspace.id,reducedMotion:true,intent:'invent_product',onboardingComplete:true});
    globalThis.fetch=async()=>new Response(JSON.stringify({...details,settings:{...details.settings,intent:'invalid'}}));await assert.rejects(api.openWorkspace(details.workspace.id),/incompatible/);
    globalThis.fetch=async()=>new Response(JSON.stringify({...details,settings:{reducedMotion:'true'}}));await assert.rejects(api.openWorkspace(details.workspace.id),/incompatible/);
    globalThis.fetch=async()=>new Response(JSON.stringify({code:'CORRUPT_STORAGE',message:'private path must not be echoed'}),{status:409});await assert.rejects(api.listWorkspaces(),/Keep the original files/);
  }finally{globalThis.fetch=original;}
});
test('relationship HTTP/native operations use scoped routes, validated DTOs and reject paths',async()=>{
  const original=globalThis.fetch;let address='';let body:Record<string,unknown>={};
  const relDetails={contractVersion:1,relationship:{id:'r'.repeat(32),workspaceId:'w'.repeat(32),sourceId:'s'.repeat(32),targetId:'t'.repeat(32),predicate:'relates_to',createdAt:'2026-10-17T21:00:00.000Z',updatedAt:'2026-10-17T21:00:00.000Z',version:1,archived:false,provenance:'user_authored',metadata:{}}};
  try{
    globalThis.fetch=async(url,options)=>{address=String(url);body=JSON.parse(options!.body as string);return new Response(JSON.stringify(url.toString().endsWith('/list')?{contractVersion:1,relationships:[relDetails.relationship]}:relDetails));};
    const client=httpClient('http://127.0.0.1:1451','a'.repeat(64));
    await client.createRelationship('w'.repeat(32),{sourceId:'s'.repeat(32),targetId:'t'.repeat(32),predicate:'relates_to'});
    assert.equal(new URL(address).pathname,'/v1/relationships/create');
    assert.equal(body.workspaceId,'w'.repeat(32));
    assert.deepEqual(body.draft,{sourceId:'s'.repeat(32),targetId:'t'.repeat(32),predicate:'relates_to'});
    await client.listRelationships('w'.repeat(32),{recordId:'s'.repeat(32)});
    assert.equal(new URL(address).pathname,'/v1/relationships/list');
    await client.deleteRelationship('w'.repeat(32),'r'.repeat(32),1);
    assert.equal(new URL(address).pathname,'/v1/relationships/delete');
    assert.equal(body.expectedVersion,1);
    await client.restoreRelationship('w'.repeat(32),'r'.repeat(32),2);
    assert.equal(new URL(address).pathname,'/v1/relationships/restore');
    assert.equal(body.expectedVersion,2);
    await client.listRelationships('w'.repeat(32),{includeHistory:true});
    assert.equal(body.includeHistory,true);
  }finally{globalThis.fetch=original;}
  const window=Object.getOwnPropertyDescriptor(globalThis,'window');let command='',args:unknown;
  Object.defineProperty(globalThis,'window',{configurable:true,value:{__TAURI_INTERNALS__:{invoke:async(c:string,a:unknown)=>{command=c;args=a;return relDetails;}}}});
  try{
    await nativeClient().createRelationship('w'.repeat(32),{sourceId:'s'.repeat(32),targetId:'t'.repeat(32),predicate:'relates_to'});
    assert.equal(command,'relationship_command');
    assert.deepEqual(args,{command:'create',request:{contractVersion:1,workspaceId:'w'.repeat(32),draft:{sourceId:'s'.repeat(32),targetId:'t'.repeat(32),predicate:'relates_to'}}});
  }finally{if(window)Object.defineProperty(globalThis,'window',window);else Reflect.deleteProperty(globalThis,'window');}
});
