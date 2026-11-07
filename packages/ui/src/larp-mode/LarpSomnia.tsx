import { SomniaFilamentField } from './SomniaFilamentField';
import { useCallback, useEffect, useRef, useState } from 'react';
import type { ArtifactRecord, Client } from '@anansi/contracts';
import { SomniaPanel, type SomniaActivity } from '../Somnia';
import type { SomniaModelActivity } from '../SomniaModelCompare';

const instruments = ['Replay', 'Findings', 'Model'] as const;
const particles = Array.from({ length: 170 }, (_, index) => {
  const angle = index * 2.399963;
  const radius = 72 + (index * 47 % 300);
  return {
    x: 480 + Math.cos(angle) * radius,
    y: 302 + Math.sin(angle) * radius * .62,
    size: index % 17 === 0 ? 1.9 : .65,
    opacity: .14 + (index % 9) / 18,
  };
});
const traces = Array.from({ length: 18 }, (_, i) => {
  const side = i < 9 ? -1 : 1;
  const y = 125 + (i % 9) * 43;
  return `M ${480 + side * 388} ${y} C ${480 + side * 255} ${y - 35}, ${480 + side * 190} ${300 + (y - 300) * .38}, ${480 + side * 67} 300`;
});

function Chamber({ activity, model }: { activity: SomniaActivity | null; model: SomniaModelActivity | null }) {
  const [instrument, setInstrument] = useState<typeof instruments[number]>('Replay');
  const [paused, setPaused] = useState(false);
  const [fieldUnavailable, setFieldUnavailable] = useState(false);
  const onUnavailable = useCallback(() => setFieldUnavailable(true), []);
  const metrics = activity?.metrics;
  const replayStatus = activity?.phase === 'complete' ? 'EVALUATION COMPLETE' :
    activity?.phase === 'review' ? 'REVIEW READY' : 'STANDBY';
  const modelStatus = model?.phase === 'error' ? 'MODEL ERROR' :
    model?.phase === 'complete' ? 'COMPARISON COMPLETE' : replayStatus;
  const phase = model?.phase === 'running' ? 'MODEL REPLAY' :
    activity?.phase === 'running' ? 'EVALUATING' :
    instrument === 'Model' || replayStatus === 'STANDBY' ? modelStatus : replayStatus;
  const active = phase === 'MODEL REPLAY' || phase === 'EVALUATING';

  return <section className="larp-somnia-chamber" aria-label="SOMNIA chamber" data-active={active} data-paused={paused}>
    <div className="somnia-chamber-caption"><span>09 / REPLAY CHAMBER</span><span aria-live="polite">{phase}</span></div>
    {!fieldUnavailable && <SomniaFilamentField active={active} paused={paused} onUnavailable={onUnavailable}/>}
    {fieldUnavailable && <svg className="somnia-optics" viewBox="0 0 960 600" aria-hidden="true">
      <defs>
        <radialGradient id="somnia-chamber-light"><stop stopColor="#6945ba" stopOpacity=".62"/><stop offset=".4" stopColor="#39275e" stopOpacity=".33"/><stop offset="1" stopColor="#05050f" stopOpacity="0"/></radialGradient>
        <radialGradient id="somnia-core-light"><stop stopColor="#f0ffff"/><stop offset=".2" stopColor="#88f3ee"/><stop offset=".55" stopColor="#8953f0" stopOpacity=".8"/><stop offset="1" stopColor="#7b49de" stopOpacity="0"/></radialGradient>
        <linearGradient id="somnia-chamber-wire"><stop stopColor="#c1a1ff" stopOpacity=".08"/><stop offset=".5" stopColor="#b986ff"/><stop offset="1" stopColor="#b1fff9" stopOpacity=".35"/></linearGradient>
        <pattern id="somnia-grid" width="40" height="40" patternUnits="userSpaceOnUse"><path d="M40 0H0V40" fill="none" stroke="#9480b7" strokeOpacity=".11"/></pattern>
        <filter id="somnia-soft"><feGaussianBlur stdDeviation="5"/></filter>
      </defs>
      <rect width="960" height="600" fill="url(#somnia-grid)"/>
      <ellipse cx="480" cy="302" rx="458" ry="294" fill="url(#somnia-chamber-light)"/>
      <g className="somnia-starfield" fill="#dac9ff">{particles.map((p,i)=><circle key={i} cx={p.x} cy={p.y} r={p.size} opacity={p.opacity}/>)}</g>
      <g fill="none" stroke="url(#somnia-chamber-wire)">
        {traces.map(d=><path key={d} d={d} opacity=".30"/>)}
        {traces.filter((_,i)=>i%3===0).map(d=><path className="somnia-trace" key={d} d={d} pathLength="100" strokeWidth="2"/>)}
      </g>
      <g fill="none" stroke="#b6a0f2" opacity=".55">
        <ellipse cx="480" cy="302" rx="293" ry="160" transform="rotate(-19 480 302)" strokeDasharray="1 6"/>
        <ellipse cx="480" cy="302" rx="274" ry="119" transform="rotate(22 480 302)" opacity=".4"/>
        <ellipse cx="480" cy="302" rx="188" ry="174" strokeDasharray="19 5 2 8" opacity=".52"/>
        <circle cx="480" cy="302" r="144" strokeDasharray="2 6"/>
        <circle cx="480" cy="302" r="129" className="somnia-orbit" strokeDasharray="94 15 30 76 150 47" strokeWidth="2"/>
      </g>
      <g className="somnia-core">
        <circle cx="480" cy="302" r="120" fill="url(#somnia-core-light)" opacity=".30" filter="url(#somnia-soft)"/>
        <circle cx="480" cy="302" r="66" fill="url(#somnia-core-light)" opacity=".8"/>
        <path d="M480 209 541 302 480 395 419 302Z M480 209V395 M419 302H541" stroke="#b8fff4" fill="#b0a0ff" fillOpacity=".08" strokeWidth="1.5"/>
        <circle cx="480" cy="302" r="6" fill="#e7ffff"/>
      </g>
    </svg>}
    <div className="somnia-chamber-readout" role="region" aria-label={instrument + ' instrument'}>
      <span>◈ {instrument.toUpperCase()} / INSTRUMENT</span>
      {instrument === 'Replay' && <p>{metrics ? `${metrics.scenariosTotal} scenarios · ${metrics.completed} completed` : activity?.scenarioCount ? `${activity.scenarioCount} scenarios in review` : 'Awaiting replay input'}</p>}
      {instrument === 'Findings' && <p>{metrics ? `${metrics.refused} refused · ${metrics.aborted} aborted · ${metrics.violationsByCategory.reduce((n,x)=>n+x.count,0)} recorded violations` : 'No findings from this session'}</p>}
      {instrument === 'Model' && <p>{model?.phase === 'complete' ? `${model.baselineTokens ?? 'Unavailable'} original · ${model.candidateTokens ?? 'Unavailable'} proposed tokens` : model?.phase === 'running' ? 'Approved comparison in progress' : model?.phase === 'error' ? 'Comparison failed · see controls' : 'Awaiting an approved comparison'}</p>}
    </div>
    <div className="somnia-instrument-bank" aria-label="Replay instrument bank">
      <div><span>01 / SCENARIO REPLAY</span><strong>{metrics?.scenariosTotal ?? activity?.scenarioCount ?? '—'}</strong><small>{metrics ? metrics.completed + ' completed' : 'Awaiting evaluation'}</small></div>
      <div><span>02 / CONSTRAINT AUDIT</span><strong>{metrics ? metrics.violationsByCategory.reduce((n,x)=>n+x.count,0) : '—'}</strong><small>{metrics ? metrics.refused + ' refused · ' + metrics.aborted + ' aborted' : 'No recorded outcome'}</small></div>
      <div><span>03 / MODEL USAGE</span><strong>{model?.phase === 'complete' ? model.candidateTokens ?? '—' : '—'}</strong><small>{model?.phase === 'complete' ? (model.baselineTokens ?? 'Unavailable') + ' original tokens' : 'Awaiting model usage'}</small></div>
    </div>
    <div className="somnia-chamber-controls">
      <div aria-label="SOMNIA instruments">{instruments.map(name=><button key={name} type="button" aria-pressed={instrument===name} onClick={()=>setInstrument(name)}>{name}</button>)}</div>
      {!fieldUnavailable && <button type="button" onClick={e=>e.currentTarget.closest('section')?.querySelector('canvas')?.dispatchEvent(new Event('larp-camera-reset'))}>Reset view</button>}
      <button type="button" aria-pressed={paused} onClick={()=>setPaused(!paused)}>{paused?'Resume motion':'Pause motion'}</button>
    </div>
  </section>;
}

export function LarpSomnia({ client, workspaceId }: { client?: Client; workspaceId?: string }) {
  const [projects, setProjects] = useState<{ id: string; title: string }[] | null>(null);
  const [project, setProject] = useState('');
  const [error, setError] = useState('');
  useEffect(() => {
    if (!client || !workspaceId) return;
    let current = true;
    void client.listMemories(workspaceId, { kind: 'project' }).then(result => {
      if (current) setProjects(result.memories);
    }).catch(() => { if (current) setError('Could not load projects. Reopen SOMNIA to retry.'); });
    return () => { current = false; };
  }, [client, workspaceId]);

  const picker = <div className="larp-somnia-project">
    <label htmlFor="larp-somnia-project">Project</label>
    <select id="larp-somnia-project" value={project} onChange={e=>setProject(e.target.value)}>
      <option value="">Choose a project</option>
      {projects?.map(item=><option key={item.id} value={item.id}>{item.title}</option>)}
    </select>
    {error && <p role="alert">{error}</p>}
    {client && workspaceId && projects === null && !error && <p role="status">Loading projects…</p>}
    {projects?.length === 0 && <p>Create a project in Projects to begin.</p>}
    {(!client || !workspaceId) && <p>Open a workspace to run a replay.</p>}
  </div>;
  if (!client || !workspaceId || !project) return <div className="larp-swarmtank larp-somnia">
    <Chamber activity={null} model={null}/>
    <div className="larp-swarm-console" aria-label="SOMNIA controls">{picker}</div>
  </div>;
  return <ProjectChamber key={project} client={client} workspaceId={workspaceId} projectId={project} picker={picker}/>;
}

function ProjectChamber({ client, workspaceId, projectId, picker }: {
  client: Client; workspaceId: string; projectId: string; picker: React.ReactNode;
}) {
  const [artifacts, setArtifacts] = useState<ArtifactRecord[] | null>(null);
  const [activity, setActivity] = useState<SomniaActivity | null>(null);
  const [model, setModel] = useState<SomniaModelActivity | null>(null);
  const [notice, setNotice] = useState<{ text: string; error: boolean } | null>(null);
  const current = useRef(false);
  async function refresh() {
    const result = await client.listProjectArtifacts(workspaceId, projectId);
    if (current.current) setArtifacts(result.artifacts);
  }
  useEffect(() => {
    current.current = true;
    void refresh().catch(() => { if (current.current) setNotice({ text: 'Could not load project artifacts.', error: true }); });
    return () => { current.current = false; };
  }, [client, workspaceId, projectId]);
  return <div className="larp-swarmtank larp-somnia">
    <Chamber activity={activity} model={model}/>
    <div className="larp-swarm-console" aria-label="SOMNIA controls">
      <header><span>09 / NIGHT SHIFT</span><h2>SOMNIA controls</h2><p>Review, replay and compare.</p></header>
      {picker}
      {notice && <p role={notice.error?'alert':'status'}>{notice.text}</p>}
      {artifacts ? <SomniaPanel client={client} workspaceId={workspaceId} projectId={projectId} artifacts={artifacts}
        onSaved={refresh} notify={(text,error)=>setNotice({text,error})} busy={false}
        onActivity={setActivity} onModelActivity={setModel}/> : !notice && <p role="status">Loading evaluations…</p>}
    </div>
  </div>;
}
