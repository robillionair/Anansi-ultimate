export type View = 'workspace'|'settings'|'connection'|'memory';
export const plannedGroups = [
  {name:'HOME',items:['Dashboard','Ask ANANSI']},
  {name:'KNOWLEDGE',items:['Search','Notes','Files','Big Brain']},
  {name:'CREATE',items:['Innovation Studio','My Ideas','Product Forge','My Products']},
  {name:'VALIDATE',items:['SwarmTank','SOMNIA','Experiments']},
  {name:'WORK',items:['Projects','Tasks','People']},
  {name:'SYSTEM',items:['AI Matrix','Local Models','Integrations','MCP','Backups']},
];
export function Navigation({view,hasWorkspace,onNavigate}:{view:View;hasWorkspace:boolean;onNavigate:(view:View)=>void}) {
  return <nav aria-label="Main navigation"><p className="nav-group">HOME</p><button aria-current={view==='workspace'?'page':undefined} onClick={()=>onNavigate('workspace')}>Workspace</button>
    <p className="nav-group">KNOWLEDGE</p><button disabled={!hasWorkspace} aria-current={view==='memory'?'page':undefined} onClick={()=>onNavigate('memory')}>Memory</button>
    <p className="nav-group">SYSTEM</p><button disabled={!hasWorkspace} aria-current={view==='settings'?'page':undefined} onClick={()=>onNavigate('settings')}>Settings</button><button aria-current={view==='connection'?'page':undefined} onClick={()=>onNavigate('connection')}>Connection</button>
    <details className="planned-tools"><summary>Explore planned tools</summary><p>These tools are not available yet.</p>{plannedGroups.map(group=><section key={group.name} aria-label={group.name+' planned tools'}><h3>{group.name}</h3><ul>{group.items.map(item=><li key={item}>{item} <span>Planned</span></li>)}</ul></section>)}<p>LEXI-COLLIDER will be part of the creative tools. GHOST-PROMPT will be an optional mode in supported editors, with separate consent.</p></details>
  </nav>;
}
