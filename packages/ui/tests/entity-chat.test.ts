import test from 'node:test';import assert from 'node:assert/strict';
import type {Client} from '@anansi/contracts';
import {EntityChatRun,providerConversation,ENTITY_RESPONSE_LIMIT,type EntityChatState,type EntitySettings} from '../src/larp-mode/entityChatRun.ts';
import {entityActivity} from '../src/larp-mode/EntityAvatar.tsx';
const config:EntitySettings={transport:'provider',endpoint:'http://127.0.0.1:11434',model:'demo',provider:'ollama',cloudConsent:false};
const disclosure={provider:'ollama',model:'demo',local:true,costClass:'free',endpointHost:'127.0.0.1',routingMode:'chooseModel',routingReason:'selected',attempts:[],estimatedInputTokens:4,messageCount:1,segmentCount:1,redactionCount:0};
const started={runId:'owned',capability:'private-capability',disclosure};
const fake=(v:Record<string,unknown>)=>v as unknown as Client;
const instant=async()=>{};
function deferred<T>(){let resolve!:(value:T)=>void;const promise=new Promise<T>(r=>resolve=r);return {resolve,promise};}

test('entity does not dispatch on construction, empty prompts, or after disposal',async()=>{
 let calls=0;const run=new EntityChatRun(fake({providersStreamStart:async()=>{calls++;throw Error('unexpected');}}),'w',()=>{},instant);
 assert.equal(run.snapshot().phase,'idle');assert.equal(calls,0);await run.send(' ',config);assert.equal(calls,0);run.dispose();await run.send('hello',config);assert.equal(calls,0);
});
test('provider entity streams real events, preserves untrusted text and enforces free/private routing',async()=>{
 const states:EntityChatState[]=[];let request:any,options:any,polls=0;
 const run=new EntityChatRun(fake({providersStreamStart:async(_w:string,r:any,o:any)=>{request=r;options=o;return started;},providersStreamPoll:async()=>++polls===1?{events:[{kind:'token',text:'<script>literal</script>'}],done:false,disclosure}:{events:[{kind:'token',text:' complete'}],done:true,disclosure}}),'w',s=>states.push(s),instant);
 await run.send('Hello',config);
 assert.equal(run.snapshot().phase,'done');assert.equal(run.snapshot().turns[1].text,'<script>literal</script> complete');
 assert.ok(states.some(s=>s.phase==='waiting'));assert.ok(states.some(s=>s.phase==='streaming'));
 assert.equal(options.zeroCostLock,true);assert.deepEqual(options.chosen,['ollama','demo']);assert.equal(options.permitId,undefined);
 assert.equal(request.localOnly,true);assert.equal(request.explicitCloudAuthorization,false);assert.equal(request.messages[0].segments[0].privacy,'private');
});
test('cloud permission is explicit on each send and never selects paid authorization',async()=>{
 let request:any,options:any;const run=new EntityChatRun(fake({providersStreamStart:async(_w:string,r:any,o:any)=>{request=r;options=o;return started;},providersStreamPoll:async()=>({events:[],done:true,disclosure})}),'w',()=>{},instant);
 await run.send('typed text',{...config,provider:'openrouter',cloudConsent:true});assert.equal(request.explicitCloudAuthorization,true);assert.equal(request.localOnly,false);assert.equal(options.zeroCostLock,true);assert.equal(options.permitId,undefined);
});
test('late start after disposal is cancelled using its own capability and terminally drained',async()=>{
 const open=deferred<any>();let cancels=0,polls=0;
 const run=new EntityChatRun(fake({providersStreamStart:()=>open.promise,providersStreamCancel:async(w:string,id:string,cap:string)=>{assert.equal(w,'w');assert.equal(id,'owned');assert.equal(cap,'private-capability');cancels++;return {cancelled:true};},providersStreamPoll:async()=>{polls++;return {events:[{kind:'error',code:'Cancelled'}],done:true,disclosure};}}),'w',()=>{},instant);
 const sending=run.send('Hello',config);run.dispose();open.resolve(started);await sending;assert.equal(cancels,1);assert.equal(polls,1);assert.equal(run.snapshot().phase,'cancelled');
});
test('cancel is pending until the backend terminal event, not the UI click',async()=>{
 const gate=deferred<any>();let polls=0;const run=new EntityChatRun(fake({providersStreamStart:async()=>started,providersStreamPoll:async()=>{polls++;return gate.promise;},providersStreamCancel:async()=>({cancelled:true})}),'w',()=>{},instant);
 const sending=run.send('Hello',config);for(let i=0;i<10&&polls===0;i++)await Promise.resolve();assert.equal(polls,1);
 await run.cancel();assert.equal(run.snapshot().phase,'cancelling');gate.resolve({events:[{kind:'error',code:'Cancelled'}],done:true,disclosure});await sending;assert.equal(run.snapshot().phase,'cancelled');
});
test('completed answers arriving during a cancel race are reported as completed, not cancelled',async()=>{
 const gate=deferred<any>();let polls=0;const run=new EntityChatRun(fake({providersStreamStart:async()=>started,providersStreamPoll:async()=>{polls++;return gate.promise;},providersStreamCancel:async()=>({cancelled:true})}),'w',()=>{},instant);
 const sending=run.send('Hello',config);for(let i=0;i<10&&polls===0;i++)await Promise.resolve();await run.cancel();gate.resolve({events:[{kind:'token',text:'Finished already'}],done:true,disclosure});await sending;assert.equal(run.snapshot().phase,'done');
});
test('poll loss keeps an interrupted run resumable with its capability',async()=>{
 let fail=true;const run=new EntityChatRun(fake({providersStreamStart:async()=>started,providersStreamPoll:async()=>{if(fail)throw Error('offline');return {events:[{kind:'token',text:'Recovered'}],done:true,disclosure};}}),'w',()=>{},instant);
 await run.send('Hello',config);assert.equal(run.snapshot().phase,'interrupted');fail=false;await run.resume();assert.equal(run.snapshot().phase,'done');assert.equal(run.snapshot().turns[1].text,'Recovered');
});
test('local Ask preserves offline response and disclosure and releases its run',async()=>{
 let forgotten=0;const response={status:'offline',answer:'Local runtime offline.',citations:[],manifest:{estimatedTokens:10,compiledPrompt:'actual compiled context',modelTag:'demo',runtimeKind:'ollama',characterCount:23}};
 const run=new EntityChatRun(fake({startAsk:async()=>({id:'ask'}),pollAsk:async()=>({state:'completed',partialAnswer:'',response}),forgetAsk:async()=>{forgotten++;}}),'w',()=>{},instant);
 await run.send('question',{...config,transport:'workspace'});assert.equal(run.snapshot().phase,'offline');assert.equal(run.snapshot().ask?.manifest.compiledPrompt,'actual compiled context');assert.equal(forgotten,1);
});
test('history and displayed response are bounded without fabricating usage',async()=>{
 const messages=providerConversation(Array.from({length:20},(_,i)=>({role:i%2?'assistant' as const:'user' as const,text:'x'.repeat(2500)})));
 assert.ok(messages.length<=9);assert.ok(messages.flatMap(m=>m.segments).reduce((n,s)=>n+s.text.length,0)<=16000);
 const run=new EntityChatRun(fake({providersStreamStart:async()=>started,providersStreamPoll:async()=>({events:[{kind:'token',text:'x'.repeat(ENTITY_RESPONSE_LIMIT+10)}],done:true,disclosure})}),'w',()=>{},instant);
 for(let n=0;n<8;n++)await run.send('Hello',config);assert.ok(run.snapshot().turns.length<=12);assert.equal(run.snapshot().turns.at(-1)?.text.length,ENTITY_RESPONSE_LIMIT);assert.match(run.snapshot().error,/truncated/);
});
test('avatar activity follows real phases and distinguishes idle from streamed text',()=>{assert.ok(entityActivity('waiting')>entityActivity('idle'));assert.ok(entityActivity('streaming')>entityActivity('waiting'));assert.ok(entityActivity('offline')<entityActivity('idle'));});

test('switching from grounded Ask to API chat never forwards the prior memory-derived answer',async()=>{
 let wire:any;
 const run=new EntityChatRun(fake({startAsk:async()=>({id:'ask'}),pollAsk:async()=>({state:'completed',partialAnswer:'',response:{status:'answered',answer:'LOCAL_ONLY_MEMORY_CANARY',citations:[],manifest:{estimatedTokens:2,compiledPrompt:'source',modelTag:'demo',runtimeKind:'ollama',characterCount:6}}}),forgetAsk:async()=>({}),providersStreamStart:async(_w:string,r:any)=>{wire=r;return started;},providersStreamPoll:async()=>({events:[],done:true,disclosure})}),'w',()=>{},instant);
 await run.send('local question',{...config,transport:'workspace'});assert.ok(run.snapshot().turns.some(t=>t.text.includes('LOCAL_ONLY_MEMORY_CANARY')));
 await run.send('typed API question',{...config,cloudConsent:true});assert.ok(!JSON.stringify(wire).includes('LOCAL_ONLY_MEMORY_CANARY'));assert.equal(wire.messages.length,1);
});
