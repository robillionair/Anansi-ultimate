import {useEffect,useRef,useState,type FormEvent} from 'react';
import type {Client} from '@anansi/contracts';
import {EntityChatRun,type EntityChatState,type EntitySettings} from './entityChatRun.ts';
const empty:EntityChatState={phase:'idle',turns:[],error:'',ask:null,disclosure:null,startedAt:0};
export function EntityChat({client,workspaceId,active,onState}:{client:Client;workspaceId:string;active:boolean;onState:(state:EntityChatState)=>void}){
 const [state,setState]=useState(empty),[prompt,setPrompt]=useState('');
 const [settings,setSettings]=useState<EntitySettings>({transport:'workspace',endpoint:'http://127.0.0.1:11434',model:'qwen2.5:3b',provider:'ollama',cloudConsent:false});
 const run=useRef<EntityChatRun|null>(null),updates=useRef(onState),scroll=useRef<HTMLDivElement>(null);updates.current=onState;
 useEffect(()=>{const controller=new EntityChatRun(client,workspaceId,next=>{setState(next);updates.current(next);});run.current=controller;setState(controller.snapshot());updates.current(controller.snapshot());return()=>{controller.dispose();if(run.current===controller)run.current=null;};},[client,workspaceId]);
 useEffect(()=>{if(!active&&run.current)void run.current.cancel();},[active]);
 useEffect(()=>{scroll.current?.scrollTo({top:scroll.current.scrollHeight});},[state.turns]);
 const busy=['waiting','streaming','cancelling'].includes(state.phase),retained=state.phase==='interrupted';
 const change=(key:keyof EntitySettings,value:string|boolean)=>{if(key!=='cloudConsent')run.current?.clear();setSettings(old=>({...old,...(key!=='cloudConsent'?{cloudConsent:false}:{}),[key]:value}));};
 const submit=(event:FormEvent)=>{event.preventDefault();if(!prompt.trim()||busy||retained)return;const text=prompt;setPrompt('');const approvedSettings={...settings};setSettings(old=>({...old,cloudConsent:false}));void run.current?.send(text,approvedSettings);};
 return <section className="entity-chat" aria-label="Chat with ANANSI">
  <header><div><span>CONVERSATION CHANNEL</span><h3>Ask ANANSI</h3></div><b className={'entity-phase phase-'+state.phase} role="status">{state.phase==='waiting'?'Waiting for model':state.phase==='streaming'?'Receiving reply':state.phase==='cancelling'?'Cancelling…':state.phase==='idle'?'Ready':state.phase}</b></header>
  <p className="entity-chat-boundary">{settings.transport==='workspace'?'Grounded in this workspace. Each question is independently compiled by Ask ANANSI.':'Configured API / model. Only recent typed conversation is sent; memories are not automatically included.'}</p>
  <details className="entity-connection"><summary>Model connection</summary><p>Changing the connection starts a fresh transcript. Workspace-grounded answers are never carried into API chat.</p><fieldset disabled={busy||retained}>
   <label>Chat connection<select aria-label="Entity chat connection" value={settings.transport} onChange={e=>change('transport',e.target.value)}><option value="workspace">Workspace Ask · local runtime</option><option value="provider">Configured provider / API</option></select></label>
   {settings.transport==='workspace'?<label>Local endpoint<input aria-label="Entity local endpoint" value={settings.endpoint} onChange={e=>change('endpoint',e.target.value)} autoComplete="off"/></label>:<label>Configured provider ID<input aria-label="Entity provider" value={settings.provider} onChange={e=>change('provider',e.target.value)} maxLength={100}/></label>}
   <label>Model<input aria-label="Entity model" value={settings.model} onChange={e=>change('model',e.target.value)} maxLength={128} autoComplete="off"/></label>
   {settings.transport==='provider'&&<><label className="entity-consent"><input type="checkbox" checked={settings.cloudConsent} onChange={e=>change('cloudConsent',e.target.checked)}/>Allow this typed conversation to be sent to a configured cloud provider on Send.</label><p>Zero-Cost Lock stays on. Configure models and credentials in AI Providers; no keys are entered here. Recent turns only, bounded to 16,000 characters.</p></>}
  </fieldset></details>
  <div ref={scroll} className="entity-transcript" role="log" aria-label="Entity conversation" aria-live="polite" aria-relevant="additions text">
   {!state.turns.length&&<div className="entity-chat-welcome"><span>01 / OPEN A CHANNEL</span><p>Your workspace, in conversation.</p><small>Choose a model connection, then ask a question. Opening this view starts no inference.</small></div>}
   {state.turns.map((turn,index)=><article key={index} className={'entity-turn entity-turn-'+turn.role}><span>{turn.role==='user'?'YOU':'ANANSI'}</span><p>{turn.text||((busy&&index===state.turns.length-1)?'Awaiting response…':'No reply received.')}</p></article>)}
  </div>
  {state.error&&<p role="alert" className="entity-chat-error">{state.error}</p>}
  {retained&&<button type="button" onClick={()=>void run.current?.resume()}>Resume watching backend run</button>}
  {(state.ask||state.disclosure)&&<details className="entity-disclosure"><summary>View what ANANSI sent</summary>{state.ask?<><p>{state.ask.manifest.modelTag} · {state.ask.manifest.runtimeKind} · {state.ask.manifest.characterCount} prompt characters</p><p>Estimated input tokens: {state.ask.manifest.estimatedTokens}</p><pre>{state.ask.manifest.compiledPrompt}</pre>{state.ask.citations.map(c=><p key={c.index}>[{c.index}] {c.title} · {c.snippet}</p>)}</>:state.disclosure&&<><p>{state.disclosure.provider} / {state.disclosure.model}</p><p>{state.disclosure.endpointHost} · {state.disclosure.local?'Local':'Cloud'} · {state.disclosure.costClass}</p><p>{state.disclosure.routingReason}</p>{state.disclosure.usage&&<p>Actual tokens: {state.disclosure.usage.inputTokens} in / {state.disclosure.usage.outputTokens} out</p>}</>}</details>}
  <form onSubmit={submit} className="entity-composer"><label htmlFor="entity-prompt">Your message</label><textarea id="entity-prompt" aria-label="Message ANANSI" value={prompt} onChange={e=>setPrompt(e.target.value)} maxLength={settings.transport==='workspace'?500:3000} disabled={busy||retained} rows={3} placeholder={settings.transport==='workspace'?'Ask about your workspace…':'Write to the configured model…'}/><div>{busy||retained?<button type="button" aria-label="Cancel ANANSI reply" disabled={state.phase==='cancelling'} onClick={()=>void run.current?.cancel()}>Cancel reply</button>:<button type="submit" disabled={!prompt.trim()||!settings.model.trim()}>Send message <span>↗</span></button>}<button type="button" className="entity-clear" disabled={busy||retained} onClick={()=>run.current?.clear()}>Clear chat</button></div></form>
  <small className="entity-chat-note">Session-only transcript · Switching scenes cancels an active reply · Avatar activity is not internal reasoning</small>
 </section>;
}
