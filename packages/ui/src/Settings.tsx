import { useEffect, useState } from 'react';
import type { Intent, WorkspaceDetails } from '@anansi/contracts';
import { IntentChoice, PrivacyReview } from './Onboarding';
export function Settings({workspace,busy,onSave}:{workspace:WorkspaceDetails;busy:boolean;onSave:(reducedMotion:boolean,intent:Intent)=>void}) {
  const [intent,setIntent]=useState(workspace.settings.intent);
  useEffect(()=>setIntent(workspace.settings.intent),[workspace.settings.intent]);
  return <section className="card" aria-labelledby="settings-title"><h2 id="settings-title">Workspace settings</h2><p>Preferences for <strong>{workspace.workspace.name}</strong>. Changes stay on this computer.</p>
    <form onSubmit={event=>{event.preventDefault();onSave(workspace.settings.reducedMotion,intent);}}><IntentChoice value={intent} onChange={setIntent}/><p>This is a starting point, not a permanent mode. All available workspace actions stay accessible.</p><button disabled={busy||intent===workspace.settings.intent} type="submit">Save starting point</button></form>
    <form onSubmit={event=>{event.preventDefault();onSave(!workspace.settings.reducedMotion,workspace.settings.intent);}}><h3>Motion</h3><p>Reduced motion: <strong>{workspace.settings.reducedMotion?'On':'Off'}</strong></p><p>Suppress animations and smooth scrolling while this workspace is open. Your system’s reduced-motion preference is also respected.</p><button disabled={busy} type="submit">{workspace.settings.reducedMotion?'Turn reduced motion off':'Turn reduced motion on'}</button></form>
    <details><summary>Review privacy limits</summary><PrivacyReview/></details>
  </section>;
}
