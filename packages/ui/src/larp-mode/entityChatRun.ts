import type {Client,AskResponse,CompletionRequest,DisclosureRecord} from '@anansi/contracts';
import {reportAskRun} from './telemetryProducers.ts';
export type EntityPhase='idle'|'waiting'|'streaming'|'cancelling'|'done'|'cancelled'|'offline'|'error'|'interrupted';
export interface EntityTurn{role:'user'|'assistant';text:string;}
export interface EntitySettings{transport:'workspace'|'provider';endpoint:string;model:string;provider:string;cloudConsent:boolean;}
export interface EntityChatState{phase:EntityPhase;turns:EntityTurn[];error:string;ask:AskResponse|null;disclosure:DisclosureRecord|null;startedAt:number;}
export const ENTITY_RESPONSE_LIMIT=24000;
export function providerConversation(turns:EntityTurn[]):CompletionRequest['messages']{
 let remaining=16000;const selected:EntityTurn[]=[];
 for(const turn of [...turns].reverse()){if(!turn.text)continue;if(turn.text.length>remaining)break;selected.push(turn);remaining-=turn.text.length;if(selected.length===9)break;}
 return selected.reverse().map(turn=>({role:turn.role,segments:[{text:turn.text,privacy:'private' as const,redacted:false}]}));
}
type Active={kind:'workspace';id:string}|{kind:'provider';id:string;capability:string};
export class EntityChatRun{
 private value:EntityChatState={phase:'idle',turns:[],error:'',ask:null,disclosure:null,startedAt:0};
 private active:Active|null=null;private disposed=false;private disposedAt=0;private cancelRequested=false;private watching=false;private starting=false;private connection='';
 constructor(private client:Client,private workspace:string,private notify:(state:EntityChatState)=>void,private wait=(ms:number)=>new Promise<void>(r=>setTimeout(r,ms))){}
 snapshot(){return {...this.value,turns:this.value.turns.map(t=>({...t}))};}
 private update(patch:Partial<EntityChatState>){this.value={...this.value,...patch};if(!this.disposed)this.notify(this.snapshot());}
 private answer(text:string){const turns=[...this.value.turns];const last=turns[turns.length-1];if(last?.role==='assistant')turns[turns.length-1]={role:'assistant',text:text.slice(0,ENTITY_RESPONSE_LIMIT)};this.update({turns,...(text.length>ENTITY_RESPONSE_LIMIT?{error:'Response display truncated at the 24,000-character limit.'}:{})});}
 async send(raw:string,settings:EntitySettings){
  if(this.disposed||this.active||this.starting)return;
  const prompt=raw.trim(),limit=settings.transport==='workspace'?500:3000;
  if(!prompt||prompt.length>limit){this.update({phase:'error',error:`Enter 1–${limit} characters.`});return;}
  const connection=JSON.stringify([settings.transport,settings.provider,settings.model,settings.endpoint]);
  if(this.connection!==connection){this.value={...this.value,turns:[]};this.connection=connection;}
  this.cancelRequested=false;this.starting=true;
  const conversation=[...this.value.turns,{role:'user' as const,text:prompt}];
  this.update({phase:'waiting',turns:[...conversation.slice(-11),{role:'assistant',text:''}],error:'',ask:null,disclosure:null,startedAt:Date.now()});
  try{
   if(settings.transport==='workspace'){
    const opened=await this.client.startAsk(this.workspace,prompt,{runtimeEndpoint:settings.endpoint.trim(),modelTag:settings.model.trim()});
    this.active={kind:'workspace',id:opened.id};
   }else{
    const opened=await this.client.providersStreamStart(this.workspace,{model:settings.model.trim(),messages:providerConversation(conversation),maxOutputTokens:1024,temperature:.4,explicitCloudAuthorization:settings.cloudConsent,localOnly:!settings.cloudConsent},{mode:'chooseModel',chosen:[settings.provider.trim(),settings.model.trim()],zeroCostLock:true});
    this.active={kind:'provider',id:opened.runId,capability:opened.capability};this.update({disclosure:opened.disclosure});
   }
   if(this.cancelRequested||this.disposed)await this.requestCancel();
   await this.watch(prompt);
  }catch(error){if(!this.active)this.update({phase:'error',error:error instanceof Error?error.message:'Unable to start the request.'});else this.update({phase:'interrupted',error:'Connection interrupted. The backend run may still be active. Resume watching or cancel it.'});}
  finally{this.starting=false;}
 }
 private async requestCancel(){
  const run=this.active;if(!run)return;
  try{if(run.kind==='workspace')await this.client.cancelAsk(this.workspace,run.id);else await this.client.providersStreamCancel(this.workspace,run.id,run.capability);}
  catch{this.update({error:'Cancel delivery failed; still watching the backend run.'});}
 }
 async cancel(){this.cancelRequested=true;if(this.starting||this.active)this.update({phase:'cancelling'});await this.requestCancel();if(this.active&&!this.watching)await this.watch('');}
 async resume(){if(this.active&&!this.watching){this.update({phase:this.cancelRequested?'cancelling':'waiting',error:''});await this.watch('');}}
 private async watch(prompt:string){
  if(this.watching)return;this.watching=true;let failures=0,text=this.value.turns.at(-1)?.text??'',terminalError='';const disposeDeadline=()=>this.disposed&&Date.now()-this.disposedAt>10000;
  try{while(this.active){
   const run=this.active;
   try{
    if(run.kind==='workspace'){
     const poll=await this.client.pollAsk(this.workspace,run.id);failures=0;
     text=poll.partialAnswer??'';this.answer(text);
     if(poll.state==='cancelled'){this.update({phase:'cancelled'});await this.client.forgetAsk(this.workspace,run.id).catch(()=>{});this.active=null;break;}
     if(poll.state==='completed'){
      if(poll.error){this.update({phase:'error',error:poll.error.message});}
      else if(poll.response){this.answer(poll.response.answer);this.update({ask:poll.response,phase:poll.response.status==='offline'?'offline':poll.response.status==='error'?'error':'done'});if(prompt)reportAskRun(prompt,poll.response,Date.now()-this.value.startedAt);}
      else this.update({phase:'error',error:'The backend returned no response.'});
      await this.client.forgetAsk(this.workspace,run.id).catch(()=>{});this.active=null;break;
     }
    }else{
     const poll=await this.client.providersStreamPoll(this.workspace,run.id,run.capability);failures=0;
     for(const event of poll.events){if(event.kind==='token'&&event.text)text+=event.text;if(event.kind==='error'){terminalError=event.code??'Error';if(terminalError!=='Cancelled')this.update({error:event.message??'The provider returned an error.'});}}
     this.answer(text);this.update({disclosure:poll.disclosure});
     if(poll.done){this.update({phase:terminalError==='Cancelled'?'cancelled':terminalError?'error':'done'});this.active=null;break;}
    }
    if(text.length>ENTITY_RESPONSE_LIMIT&&!this.cancelRequested){this.update({error:'Response display limit reached; stopping the request.'});await this.cancel();}
    if(!this.cancelRequested&&text)this.update({phase:'streaming'});
    if(this.disposed&&!this.cancelRequested){this.cancelRequested=true;await this.requestCancel();}
    if(disposeDeadline())break;
   }catch{if(++failures>5){this.update({phase:'interrupted',error:'Connection lost. The run may still be active; resume watching or cancel.'});break;}}
   await this.wait(failures?350:100);
  }}finally{this.watching=false;}
 }
 clear(){if(!this.active&&!this.starting){this.connection='';this.update({phase:'idle',turns:[],error:'',ask:null,disclosure:null,startedAt:0});}}
 dispose(){if(this.disposed)return;this.disposed=true;this.disposedAt=Date.now();this.cancelRequested=true;void this.requestCancel();if(this.active&&!this.watching)void this.watch('');}
}
