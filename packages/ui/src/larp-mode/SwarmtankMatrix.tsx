import { useState } from 'react';
import type { SwarmtankDecision, SwarmtankPersona } from '@anansi/contracts';

const PAGE_SIZE = 12;
const COLUMNS = 4;

export function SwarmtankMatrix({ personas, decisions, selected, onSelect, staticView }: {
  personas: SwarmtankPersona[];
  decisions: SwarmtankDecision[];
  selected: string | null;
  onSelect: (id: string) => void;
  staticView: boolean;
}) {
  const [page, setPage] = useState(0);
  const first = page * PAGE_SIZE;
  const visible = personas.slice(first, first + PAGE_SIZE);
  const byPersona = new Map(decisions.map(decision => [decision.personaId, decision]));
  const links = visible.flatMap((_, i) => [i + 1, i + COLUMNS].filter(j =>
    j < visible.length && (j - i === COLUMNS || i % COLUMNS !== COLUMNS - 1),
  ).map(j => [i, j]));
  return (
    <section className="larp-swarm-matrix" aria-label="Persona matrix" data-static={staticView}>
      <header>
        <span>SWARM / DECISION MATRIX</span>
        <div className="larp-matrix-pages">
          <button type="button" aria-label="Previous persona cards" disabled={page === 0} onClick={() => setPage(page - 1)}>‹</button>
          <span>{first + 1}–{first + visible.length} / {personas.length}</span>
          <button type="button" aria-label="Next persona cards" disabled={first + PAGE_SIZE >= personas.length} onClick={() => setPage(page + 1)}>›</button>
        </div>
      </header>
      <div className="larp-matrix-field">
        <svg className="larp-matrix-wires" viewBox="0 0 800 420" preserveAspectRatio="none" aria-hidden="true">
          {links.map(([i, j]) => {
            const x = i % COLUMNS * 200 + 100;
            const y = Math.floor(i / COLUMNS) * 140 + 70;
            const endX = j % COLUMNS * 200 + 100;
            const endY = Math.floor(j / COLUMNS) * 140 + 70;
            const bend = (i % 2 === 0 ? 1 : -1) * (endX === x ? 102 : 65);
            const active = selected !== null && (visible[i].id === selected || visible[j].id === selected);
            const d = `M${x} ${y} C${x + bend} ${y + 68} ${endX + bend} ${endY - 68} ${endX} ${endY}`;
            return <g key={`${i}-${j}`} data-active={active}>
              <path d={d} className="wire-halo" />
              <path d={d} className="wire-filament" />
              <path d={d} className="wire-signal" pathLength="100" />
            </g>;
          })}
        </svg>
        <div className="larp-matrix-card-grid">
          {visible.map((persona, index) => {
            const decision = byPersona.get(persona.id);
            if (!decision) return null;
            return <button key={persona.id} type="button" className="larp-matrix-card" data-decision={decision.decision} aria-pressed={selected === persona.id} onClick={() => onSelect(persona.id)}>
              <span className="larp-card-channel"><i aria-hidden="true" /> PERSONA {String(first + index + 1).padStart(2, '0')}</span>
              <strong>{persona.name}</strong>
              <small>{persona.role}</small>
              <span className="larp-card-state">{decision.decision}<span>VALUE {decision.perceivedValue.toFixed(2)}</span></span>
            </button>;
          })}
        </div>
      </div>
      <footer>SIMULATED PERSONAS <span>WIRING IS VISUAL ONLY</span></footer>
    </section>
  );
}
