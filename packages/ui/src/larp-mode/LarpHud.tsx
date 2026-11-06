import type {Client} from '@anansi/contracts';
import { LarpSwarmtank } from './LarpSwarmtank.tsx';
import {EntityAvatar} from './EntityAvatar.tsx';
import {EntityChat} from './EntityChat.tsx';
import type {EntityChatState} from './entityChatRun.ts';
import { useEffect, useState, useRef } from 'react';
import { toggleLarpMode } from './store.ts';
import { telemetryStream, type TelemetryEvent } from './telemetry.ts';
import { MemoryMatrix, latestMemoryNodes } from './MemoryMatrix.tsx';
import { TokenDeltaStream } from './TokenDeltaStream.tsx';
import { CoherenceWaveform } from './CoherenceWaveform.tsx';
import { TelemetryReadout } from './TelemetryReadout.tsx';
import { View1MemoryVault } from './views/View1MemoryVault.tsx';
import { View2NeuralFunnel } from './views/View2NeuralFunnel.tsx';
import {latestCitedIds} from './views/forceLayout.ts';
import { View3AttentionSpider } from './views/View3AttentionSpider.tsx';
import { View4QuantDrift } from './views/View4QuantDrift.tsx';
import { View5AttentionHeatmap } from './views/View5AttentionHeatmap.tsx';
import { View6ColliderField } from './views/View6ColliderField.tsx';

const MODES = [
  ['SECOND BRAIN', 'Memory constellation', '01 / SPATIAL MEMORY'],
  ['NEURAL FUNNEL', 'Context compaction', '02 / SIGNAL COMPRESSION'],
  ['ATTENTION SPIDER', 'Memory topology', '03 / RELATIONSHIP GRAPH'],
  ['QUANT DRIFT', 'Signal observatory', '04 / COHERENCE ANALYSIS'],
  ['TOKEN ALIGNMENT', 'Context correspondence', '05 / WORD ALIGNMENT'],
  ['ORBITAL COLLIDER', 'Concept resonance field', '06 / COLLISION OBSERVATORY'],
  ['ANANSI ENTITY', 'Conversation interface', '07 / ENTITY INTERFACE'],
  ['SWARMTANK', 'Synthetic buyer arena', '08 / SCENARIO LAB'],
];

export function LarpHud({ stream = telemetryStream, initialView = 1, onRefreshMemory, demo=false, client, workspaceId }: { stream?: typeof telemetryStream; initialView?: number; onRefreshMemory?:()=>Promise<void>;demo?:boolean;client?:Client;workspaceId?:string }) {
  const [events, setEvents] = useState<TelemetryEvent[]>(() => stream.snapshot());
  const [view, setView] = useState(initialView);
  const [entity,setEntity]=useState<EntityChatState>({phase:'idle',turns:[],error:'',ask:null,disclosure:null,startedAt:0});
  const [refreshing,setRefreshing]=useState(false);
  const [refreshError,setRefreshError]=useState('');
  const rootRef=useRef<HTMLDivElement>(null);
  useEffect(()=>{rootRef.current?.querySelector<HTMLButtonElement>('.larp-view-button-on')?.focus();},[]);
  useEffect(() => stream.subscribe(() => setEvents(stream.snapshot())), [stream]);
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      const target = event.target;
      if (target instanceof HTMLElement && (['INPUT','TEXTAREA','SELECT'].includes(target.tagName) || target.isContentEditable)) return;
      if (['1','2','3','4','5','6','7','8'].includes(event.key)) setView(Number(event.key));
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);
  const active = MODES[view-1] ?? MODES[0];
  const nodes = latestMemoryNodes(events);
  const memoryIdentity=JSON.stringify(nodes);
  const graphIdentity=memoryIdentity+JSON.stringify([...latestCitedIds(events)]);
  return <div ref={rootRef} className="larp-hud" role="complementary" aria-label="LARP telemetry frame" data-view={view}>
    <div className="larp-rail larp-rail-top">
      <span className="larp-brand"><b>ANANSI</b><i/> SECOND BRAIN <span className="larp-version">{demo?'DEMO / SAMPLE DATA':'/ LOCAL INTELLIGENCE SYSTEM'}</span></span>
      <div className="larp-top-actions">{onRefreshMemory&&<button type="button" aria-label="Refresh LARP memory" disabled={refreshing} onClick={async()=>{setRefreshing(true);setRefreshError('');try{await onRefreshMemory();}catch{setRefreshError('Memory refresh failed. Try again.');}finally{setRefreshing(false);}}}>{refreshing?'REFRESHING…':'REFRESH MEMORY'}</button>}<span className="larp-hud-status"><i/> [ LARP: ACTIVE ]</span><button type="button" aria-label="Exit LARP mode" onClick={toggleLarpMode}>EXIT <span>↗</span></button></div>
    </div>
    <div className="larp-rail larp-rail-left">
      <div className="larp-rail-caption">01 / SIGNAL INTELLIGENCE</div>
      <TelemetryReadout events={events}/>
      <div className="larp-archive-heading">MEMORY INDEX <span>{nodes.length} RECORDS</span></div>
      <ul className="larp-record-list">{nodes.slice(0,12).map((node,i)=><li key={node.id}><span>{String(i+1).padStart(2,'0')}</span><b>{node.title}</b><small>{node.kind}</small></li>)}</ul>
      {nodes.length===0 && <p className="larp-empty">Open a workspace memory to observe its snapshot. No records are synthesized.</p>}
      <div className="larp-local-seal">◈ <span>PRIVATE BY ARCHITECTURE<br/><b>LOCAL TELEMETRY ONLY</b></span></div>
    </div>
    <div className="larp-center"><section className="larp-centerpiece larp-centerpiece-view" aria-labelledby="larp-center-heading">
      <div className="larp-scene-caption"><span>{active[2]}</span><span>REALTIME RENDER / OBSERVATORY</span></div>
      <div className="larp-scene-title"><div><span>ANANSI OBSERVATORY</span><h3 id="larp-center-heading">{active[0]}</h3></div><span className="larp-scene-coordinate">{active[1]}<br/>LOCAL / IN-MEMORY</span></div>
      {view===1 ? <View1MemoryVault key={memoryIdentity} stream={stream}/> : view===2 ? <View2NeuralFunnel stream={stream}/> : view===3 ? <View3AttentionSpider key={graphIdentity} stream={stream}/> : view===4 ? <View4QuantDrift stream={stream}/> : view===5 ? <View5AttentionHeatmap stream={stream}/> : view===6?<View6ColliderField stream={stream}/>:view===7?<EntityAvatar phase={entity.phase} characters={entity.turns.at(-1)?.text.length??0}/>:<LarpSwarmtank key={workspaceId} client={client} workspaceId={workspaceId}/>}
      {[1,3,6].includes(view)&&<div className="larp-camera-tools"><span>DRAG ROTATE · SCROLL ZOOM · RIGHT-DRAG PAN</span><button type="button" aria-label="Reset scene camera" onClick={event=>event.currentTarget.closest('.larp-centerpiece')?.querySelector('canvas[data-interactive="true"]')?.dispatchEvent(new Event('larp-camera-reset'))}>RESET VIEW</button></div>}
      {refreshError&&<p className="larp-refresh-error" role="alert">{refreshError}</p>}
      <div className="larp-scene-footnote"><span>{view===8?'LOCAL SIMULATION · SYNTHETIC PERSONAS':'PARTICLES / LAYOUT ARE ORNAMENTAL · VALUES COME FROM TELEMETRY'}</span><span>SCENE {String(view).padStart(2,'0')} / {String(MODES.length).padStart(2,'0')}</span></div>
    </section></div>
    <div className="larp-rail larp-rail-right">
      <div className="larp-rail-caption">{view===7?'07 / CONVERSATION CHANNEL':'02 / CONTEXT ALLOCATION'}</div>
      {view!==7&&<><MemoryMatrix nodes={nodes}/><CoherenceWaveform events={events}/><TokenDeltaStream events={events}/></>}
      {client&&workspaceId?<div className="entity-chat-host" hidden={view!==7}><EntityChat key={workspaceId} client={client} workspaceId={workspaceId} active={view===7} onState={setEntity}/></div>:view===7&&<p className="larp-empty">Open a workspace to chat with ANANSI.</p>}
      <div className="larp-terminal-note" hidden={view===7}><span>BOUNDARY STATUS</span><p>Events stay in this process.<br/>Scenes do not start provider traffic. Chat sends only on your command.<br/>Unavailable metrics remain unreported.</p></div>
    </div>
    <nav className="larp-mode-dock" aria-label="LARP centerpiece view">{MODES.map((mode,i)=><button key={i} type="button" className={view===i+1?'larp-view-button larp-view-button-on':'larp-view-button'} aria-pressed={view===i+1} aria-label={`LARP view ${i+1}`} onClick={()=>setView(i+1)}><span className="larp-mode-index">0{i+1}</span><span><b>{mode[0]}</b><small>{mode[1]}</small></span><i/></button>)}</nav>
    <div className="larp-rail larp-rail-bottom"><span>ANANSI // OBSERVATORY</span><span>{view===8?'PROJECT RUNS · NO PROVIDER TRAFFIC':view===7?'EXPLICIT SEND ONLY · EXISTING PRIVACY / COST GATES':'IN-MEMORY ONLY · NO PROVIDER TRAFFIC'}</span><span>CTRL + SHIFT + L <b>EXIT</b></span></div>
  </div>;
}
