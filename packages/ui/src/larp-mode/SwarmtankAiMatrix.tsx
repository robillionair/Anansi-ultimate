// ---------------------------------------------------------------------------
// SwarmTank v2 Checkpoint 3: the AI run matrix for LARP scene 08.
//
// The cinematic card matrix, now for an AI run. A timeline scrubber
// selects the round; each persona card shows what that agent did in the
// selected round and what evidence or prior event it cited. Two kinds
// of wiring are drawn and labelled separately:
//   - Decorative wiring (the grid cables): visual only, following card
//     positions, carrying no relationship data (as in the Quick matrix).
//   - Recorded interactions: a line from an agent to the agent (or
//     event) it cited in the selected round, taken from the saved run
//     doc. These are recorded, not decorative, and are labelled as such.
// Market and branch labels sit in the header. Reduced motion renders
// the static view (no animated signal dash).
// ---------------------------------------------------------------------------

import { useState } from 'react';
import type { AiAgent, SwarmtankAiRunDoc } from '@anansi/contracts';

const PAGE_SIZE = 12;
const COLUMNS = 4;

function actionLabel(action: string | null | undefined): string {
  switch (action) {
    case 'recommend':
      return 'Recommend';
    case 'reject':
      return 'Reject';
    case 'defer':
      return 'Defer';
    case 'request_evidence':
      return 'Request evidence';
    default:
      return 'No valid action';
  }
}

export function SwarmtankAiMatrix({
  doc,
  staticView,
}: {
  doc: SwarmtankAiRunDoc;
  staticView: boolean;
}) {
  const [page, setPage] = useState(0);
  const [selectedRound, setSelectedRound] = useState(doc.rounds.length);
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null);
  const round = doc.rounds.find(r => r.round === selectedRound) ?? doc.rounds[doc.rounds.length - 1];
  const first = page * PAGE_SIZE;
  const visible: AiAgent[] = doc.agents.slice(first, first + PAGE_SIZE);
  const actionByAgent = new Map(
    (round?.actions ?? []).map(a => [a.agentId, a]),
  );
  // Decorative grid wires (visual only), as in the Quick matrix.
  const links = visible.flatMap((_, i) =>
    [i + 1, i + COLUMNS]
      .filter(j => j < visible.length && (j - i === COLUMNS || i % COLUMNS !== COLUMNS - 1))
      .map(j => [i, j]),
  );
  // Recorded interactions: an agent citing another agent's prior
  // interaction in this round. Resolved to card positions on this page.
  const recorded: { from: number; to: number; ref: string }[] = [];
  if (round) {
    for (const action of round.actions) {
      const ref = action.priorInteractionRef;
      if (!ref) continue;
      const match = ref.match(/^round-(\d+)-(agent-\d+)$/);
      if (!match) continue;
      const fromIndex = visible.findIndex(a => a.id === action.agentId);
      const toIndex = visible.findIndex(a => a.id === match[2]);
      if (fromIndex >= 0 && toIndex >= 0 && fromIndex !== toIndex) {
        recorded.push({ from: fromIndex, to: toIndex, ref });
      }
    }
  }
  const cardPos = (index: number) => ({
    x: (index % COLUMNS) * 200 + 100,
    y: Math.floor(index / COLUMNS) * 140 + 70,
  });
  const selectedAction = selectedAgent ? actionByAgent.get(selectedAgent) : undefined;
  const selectedAgentDoc = selectedAgent
    ? doc.agents.find(a => a.id === selectedAgent)
    : undefined;

  return (
    <section className="larp-swarm-matrix" aria-label="AI persona matrix" data-static={staticView}>
      <header>
        <span>SWARM / AI DECISION MATRIX</span>
        <span>
          {doc.market?.territoryName ? `Market: ${doc.market.territoryName}. ` : ''}
          {doc.parentRunId ? `Branch of ${doc.parentRunId} from round ${doc.parentRound}. ` : ''}
          Conditional simulation result.
        </span>
        <div className="larp-matrix-pages">
          <button type="button" aria-label="Previous persona cards" disabled={page === 0} onClick={() => setPage(page - 1)}>‹</button>
          <span>{first + 1}–{first + visible.length} / {doc.agents.length}</span>
          <button type="button" aria-label="Next persona cards" disabled={first + PAGE_SIZE >= doc.agents.length} onClick={() => setPage(page + 1)}>›</button>
        </div>
      </header>
      <div>
        <label>
          Timeline — round{' '}
          <input
            aria-label="AI matrix timeline round"
            type="range"
            min={1}
            max={Math.max(1, doc.rounds.length)}
            value={selectedRound}
            onChange={event => setSelectedRound(Number(event.target.value))}
          />
          {round ? `Round ${round.round} of ${doc.rounds.length}${round.date ? ` — ${round.date}` : ''}` : ''}
        </label>
        <div>
          {doc.rounds.map(r => (
            <button
              key={r.round}
              type="button"
              aria-pressed={selectedRound === r.round}
              onClick={() => setSelectedRound(r.round)}
            >
              Round {r.round}
              {r.date ? ` (${r.date})` : ''}
            </button>
          ))}
        </div>
      </div>
      <div className="larp-matrix-field">
        <svg className="larp-matrix-wires" viewBox="0 0 800 420" preserveAspectRatio="none" aria-hidden="true">
          {links.map(([i, j]) => {
            const { x, y } = cardPos(i);
            const { x: endX, y: endY } = cardPos(j);
            const bend = (i % 2 === 0 ? 1 : -1) * (endX === x ? 102 : 65);
            const active = selectedAgent !== null && (visible[i].id === selectedAgent || visible[j].id === selectedAgent);
            const d = `M${x} ${y} C${x + bend} ${y + 68} ${endX + bend} ${endY - 68} ${endX} ${endY}`;
            return (
              <g key={`${i}-${j}`} data-active={active}>
                <path d={d} className="wire-halo" />
                <path d={d} className="wire-filament" />
                <path d={d} className="wire-signal" pathLength="100" />
              </g>
            );
          })}
          {recorded.map((link, index) => {
            const { x, y } = cardPos(link.from);
            const { x: endX, y: endY } = cardPos(link.to);
            return (
              <line
                key={`recorded-${index}`}
                x1={x}
                y1={y}
                x2={endX}
                y2={endY}
                className="wire-recorded"
                stroke="#f5c542"
                strokeWidth="3"
              />
            );
          })}
        </svg>
        <div className="larp-matrix-card-grid">
          {visible.map((agent, index) => {
            const action = actionByAgent.get(agent.id);
            return (
              <button
                key={agent.id}
                type="button"
                className="larp-matrix-card"
                data-decision={action?.action ?? 'failed'}
                aria-pressed={selectedAgent === agent.id}
                onClick={() => setSelectedAgent(selectedAgent === agent.id ? null : agent.id)}
              >
                <span className="larp-card-channel"><i aria-hidden="true" /> PERSONA {String(first + index + 1).padStart(2, '0')}</span>
                <strong>{agent.name}</strong>
                <small>{agent.role}</small>
                <span className="larp-card-state">
                  {action ? actionLabel(action.action) : 'No action'}
                  <span>
                    {action?.evidenceRef ? `Cited: ${action.evidenceRef}` : action?.priorInteractionRef ? `Prior: ${action.priorInteractionRef}` : 'No citation'}
                  </span>
                </span>
              </button>
            );
          })}
        </div>
      </div>
      {selectedAgentDoc && selectedAction ? (
        <section aria-label="Selected AI persona">
          <h5>
            {selectedAgentDoc.name} <span>Simulated persona.</span>
          </h5>
          <p>Role: {selectedAgentDoc.role}</p>
          <p>
            Round {round?.round}: {actionLabel(selectedAction.action)}.
            {selectedAction.rationale ? ` ${selectedAction.rationale}` : ''}
          </p>
          <p>
            Cited evidence: {selectedAction.evidenceRef ?? '—'}.
            Prior interaction: {selectedAction.priorInteractionRef ?? '—'}.
          </p>
          <p>
            {selectedAction.status === 'failed'
              ? `Failed: ${selectedAction.failure ?? ''}`
              : `Model: ${selectedAction.model ?? '—'}.`}
          </p>
          <p>
            <button type="button" onClick={() => setSelectedAgent(null)}>
              Close persona details
            </button>
          </p>
        </section>
      ) : null}
      <footer>
        SIMULATED PERSONAS{' '}
        <span>WIRING IS VISUAL ONLY</span>{' '}
        <span>RECORDED INTERACTIONS (GOLD LINES) ARE FROM THE SAVED RUN</span>
      </footer>
    </section>
  );
}
