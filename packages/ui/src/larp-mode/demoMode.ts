import type {Client} from '@anansi/contracts';
declare const __ANANSI_LARP_DEMO__:boolean;
export const LARP_DEMO_WORKSPACE='LARP Demo';
export function isLarpDemoBuild(){return typeof __ANANSI_LARP_DEMO__!=='undefined'&&__ANANSI_LARP_DEMO__;}
/** Demo startup reads the explicitly named sample only; never falls back to private workspaces. */
export async function loadLarpDemo(client:Pick<Client,'listWorkspaces'|'openWorkspace'|'listMemories'>){
  const list=await client.listWorkspaces();
  const sample=list.workspaces.find(w=>w.name===LARP_DEMO_WORKSPACE);
  if(!sample)throw Error('The LARP Demo sample workspace is missing. Re-run the demo setup.');
  const opened=await client.openWorkspace(sample.id);
  if(!opened.settings.onboardingComplete)throw Error('The LARP Demo sample setup is incomplete.');
  const snapshot=await client.listMemories(sample.id,{offset:0,kind:'memory'});
  return {opened,memories:snapshot.memories};
}
