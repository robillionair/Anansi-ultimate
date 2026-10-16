import { useEffect, useRef, useState } from 'react';
import type { Intent, WorkspaceDetails } from '@anansi/contracts';

export const intents: {id:Intent;label:string;description:string}[] = [
  {id:'second_memory',label:'Build my second memory',description:'Start with a place for your knowledge. Memory tools arrive in a later update.'},
  {id:'invent_product',label:'Invent a product',description:'Prepare your workspace. Innovation tools are still in development.'},
  {id:'build_application',label:'Build an application',description:'Prepare your workspace. Product Forge is still in development.'},
  {id:'connect_ai',label:'Connect AI models',description:'Start locally. Model and provider connections are not available yet.'},
  {id:'explore',label:'Explore ANANSI',description:'Get to know your local workspace and settings.'},
];
export function IntentChoice({value,onChange}:{value:Intent;onChange:(intent:Intent)=>void}) {
  return <fieldset><legend>What would you like to do?</legend><div className="intent-options">{intents.map(intent=><label className="intent-option" key={intent.id}><input type="radio" name="intent" value={intent.id} checked={value===intent.id} onChange={()=>onChange(intent.id)}/><span><strong>{intent.label}</strong><small>{intent.description}</small></span></label>)}</div></fieldset>;
}
export function PrivacyReview() {
  return <div className="privacy-review"><h3>Your privacy, plainly</h3><ul>
    <li>Workspace names and preferences stay on this computer. This edition makes no AI or cloud requests.</li>
    <li>The browser uses your paired local service. Pairing is kept in this tab’s memory and is forgotten on reload.</li>
    <li>Local file permissions protect access; this is not an encrypted vault. Application lock and disk encryption are different.</li>
    <li>Memory privacy classes (Sealed, Private, Standard, Public), an encrypted vault and automatic data backups are not available yet.</li>
    <li>Continuing does not authorize future cloud sharing, model downloads or recording of drafts. Those require separate choices.</li>
  </ul></div>;
}
export function Onboarding({workspace,intent,busy,onFinish}:{workspace:WorkspaceDetails;intent:Intent;busy:boolean;onFinish:()=>void}) {
  const [reviewed,setReviewed]=useState(false);
  const [step,setStep]=useState<'privacy'|'ai'>('privacy');
  const title=useRef<HTMLHeadingElement>(null);
  useEffect(()=>{title.current?.focus();},[step]);
  const choice=intents.find(choice=>choice.id===intent)!;
  return <section className="card" aria-labelledby="onboarding-title" aria-busy={busy}><p className="eyebrow">{step==='privacy'?'STEP 3 OF 4':'STEP 4 OF 4'}</p><h2 id="onboarding-title" ref={title} tabIndex={-1}>{step==='privacy'?'Review your privacy':'Choose AI setup'}</h2>
    <p><strong>{workspace.workspace.name}</strong> is stored locally. Your starting point: {choice.label}. You can change it in Settings.</p>
    {busy&&<p>Saving your workspace preferences…</p>}
    {step==='privacy'?<><PrivacyReview/><label className="check-label"><input type="checkbox" checked={reviewed} onChange={event=>setReviewed(event.target.checked)}/>I have reviewed these privacy limits</label><button disabled={!reviewed||busy} onClick={()=>setStep('ai')}>Continue to AI setup</button></>:<>
      <p>No AI setup is required to use your workspace.</p><ul><li>Local hardware detection and model recommendations are not available yet.</li><li>Cloud provider connections are not available yet. No account or API key is needed.</li></ul>
      <p>Next: enter your workspace and adjust your preferences.</p><div className="actions"><button disabled={busy} onClick={onFinish}>{busy?'Saving…':'Skip AI setup and enter ANANSI'}</button><button className="secondary" disabled={busy} onClick={()=>setStep('privacy')}>Back to privacy</button></div>
    </>}
  </section>;
}
