export type View = 'workspace'|'settings'|'connection'|'memory'|'notes'|'projects'|'tasks'|'people'|'records'|'jobs'|'backups'|'search'|'models';
export const plannedGroups = [
  {name:'HOME',items:['Dashboard','Ask ANANSI']},
  {name:'KNOWLEDGE',items:['Files','Big Brain']},
  {name:'CREATE',items:['Innovation Studio','My Ideas','Product Forge','My Products']},
  {name:'VALIDATE',items:['SwarmTank','SOMNIA','Experiments']},
  {name:'WORK',items:['Team Workspace','Automations']},
  {name:'SYSTEM',items:['AI Matrix','Integrations','MCP']},
];
export function Navigation({view,hasWorkspace,onNavigate}:{view:View;hasWorkspace:boolean;onNavigate:(view:View)=>void}) {
  return <nav aria-label="Main navigation">
    <p className="nav-group">HOME</p>
    <button aria-current={view==='workspace'?'page':undefined} onClick={()=>onNavigate('workspace')}>Workspace</button>
    <p className="nav-group">WORK</p>
    <button disabled={!hasWorkspace} aria-current={view==='projects'?'page':undefined} onClick={()=>onNavigate('projects')}>Projects</button>
    <button disabled={!hasWorkspace} aria-current={view==='tasks'?'page':undefined} onClick={()=>onNavigate('tasks')}>Tasks</button>
    <button disabled={!hasWorkspace} aria-current={view==='people'?'page':undefined} onClick={()=>onNavigate('people')}>People</button>
    <p className="nav-group">KNOWLEDGE</p>
    <button disabled={!hasWorkspace} aria-current={view==='search'?'page':undefined} onClick={()=>onNavigate('search')}>Search</button>
    <button disabled={!hasWorkspace} aria-current={view==='memory'?'page':undefined} onClick={()=>onNavigate('memory')}>Memory</button>
    <button disabled={!hasWorkspace} aria-current={view==='notes'?'page':undefined} onClick={()=>onNavigate('notes')}>Notes</button>
    <button disabled={!hasWorkspace} aria-current={view==='records'?'page':undefined} onClick={()=>onNavigate('records')}>Records</button>
    <p className="nav-group">SYSTEM</p>
    <button disabled={!hasWorkspace} aria-current={view==='jobs'?'page':undefined} onClick={()=>onNavigate('jobs')}>Jobs and export</button>
    <button disabled={!hasWorkspace} aria-current={view==='backups'?'page':undefined} onClick={()=>onNavigate('backups')}>Backups</button>
    <button disabled={!hasWorkspace} aria-current={view==='models'?'page':undefined} onClick={()=>onNavigate('models')}>Local Models</button>
    <button disabled={!hasWorkspace} aria-current={view==='settings'?'page':undefined} onClick={()=>onNavigate('settings')}>Settings</button>
    <button aria-current={view==='connection'?'page':undefined} onClick={()=>onNavigate('connection')}>Connection</button>
    <details className="planned-tools"><summary>Explore planned tools</summary><p>These tools are not available yet.</p>{plannedGroups.map(group=><section key={group.name} aria-label={group.name+' planned tools'}><h3>{group.name}</h3><ul>{group.items.map(item=><li key={item}>{item} <span>Planned</span></li>)}</ul></section>)}<p>LEXI-COLLIDER will be part of the creative tools. GHOST-PROMPT will be an optional mode in supported editors, with separate consent.</p></details>
  </nav>;
}
