import type { TelemetryEvent } from './telemetry.ts';
import { latestMemoryNodes } from './MemoryMatrix.tsx';

export function summarizeTelemetry(events: TelemetryEvent[]) {
  const delta = [...events].reverse().find(e => e.kind === 'token-delta');
  const drift = [...events].reverse().find(e => e.kind === 'drift');
  const reduction = (raw: number, compiled: number) => raw > 0 ? (1 - compiled / raw) * 100 : null;
  return {
    input: delta?.kind === 'token-delta' ? reduction(delta.rawInputTokens, delta.compiledInputTokens) : null,
    output: delta?.kind === 'token-delta' ? reduction(delta.rawOutputTokens, delta.compiledOutputTokens) : null,
    latency: delta?.kind === 'token-delta' ? delta.latencyMs : null,
    estimated: delta?.kind === 'token-delta' ? !!delta.estimated : false,
    drift: drift?.kind === 'drift' ? drift.score : null,
    nodes: latestMemoryNodes(events),
    compilations: events.filter(e => e.kind === 'compilation').length,
  };
}

export function TelemetryReadout({ events }: { events: TelemetryEvent[] }) {
  const summary = summarizeTelemetry(events);
  const percent = (n: number | null) => n === null ? '—' : `${n.toFixed(1)}%`;
  const overhead=summary.input!==null&&summary.input<0;
  const ratio=1-(summary.input??0)/100;
  const inputValue=overhead?`${ratio>=1000?ratio.toExponential(1):ratio.toFixed(1)}×`:percent(summary.input);
  return <section className="larp-readout" aria-label="Live telemetry summary">
    <div className="larp-eyebrow">CONTEXT ECONOMY <span>{summary.input === null ? 'WAITING' : summary.estimated ? 'ESTIMATED' : 'OBSERVED'}</span></div>
    <div className="larp-hero-metric"><span>{overhead?'INPUT OVERHEAD':'INPUT REDUCTION'}</span><strong style={overhead?{fontSize:'clamp(26px,3.3vw,44px)'}:undefined}>{inputValue}</strong>{summary.input!==null&&<small style={{fontSize:7,color:'#897a67'}}>Compiled prompt vs typed question</small>}</div>
    <div className="larp-metric-pair"><div><span>OUTPUT DELTA</span><b>{percent(summary.output)}</b></div><div><span>LAST LATENCY</span><b>{summary.latency === null ? '—' : `${summary.latency.toFixed(0)} ms`}</b></div></div>
    <div className="larp-instrument"><span>CONTEXT DRIFT</span><b>{summary.drift === null ? 'NOT REPORTED' : summary.drift.toFixed(3)}</b><div className="larp-meter-track"><i style={{ width: summary.drift === null ? '0%' : `${summary.drift * 100}%` }}/></div></div>
    <dl className="larp-key-values"><div><dt>MEMORY RECORDS</dt><dd>{summary.nodes.length}</dd></div><div><dt>COMPILE EVENTS</dt><dd>{summary.compilations}</dd></div><div><dt>PROVIDER COST</dt><dd>NOT REPORTED</dd></div><div><dt>TELEMETRY</dt><dd>{events.length ? 'LOCAL / LIVE' : 'AWAITING EVENTS'}</dd></div></dl>
  </section>;
}
