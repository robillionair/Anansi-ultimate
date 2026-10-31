/**
 * M9 SwarmTank Slice 2: the arena section of the
 * SwarmTank report. The 3D canvas is the visual
 * view; the complete 2D table in the report view
 * below is the accessible alternative and is always
 * rendered, in every state. Selection is shared:
 * picking an agent marks its table row, and a table
 * row's "Select in arena" button selects the agent.
 *
 * Reduced motion: the checkbox starts from the
 * synchronous probe (system media query OR workspace
 * dataset) — the §26 reviewer lesson: never
 * useState(false) + effect, which flickered on mount
 * and raced unmounts in high-DPI screenshot tests.
 * WebGL failure is tracked per run identity, so a
 * run switch retries WebGL instead of inheriting a
 * stale failure (the §26 per-identity lesson).
 */

import { useMemo, useState } from 'react';
import type {
  SwarmtankDecision,
  SwarmtankPersona,
  SwarmtankRunDoc,
} from '@anansi/contracts';
import { isReducedMotion } from './swarmtankThreeScene';
import { layoutSwarmtankArena } from './swarmtankArenaLayout';
import { SwarmtankThreeView } from './SwarmtankThreeView';

export function SwarmtankArena({
  run,
  runKey,
  effectiveDecisions,
  layoutKey,
  selectedPersonaId,
  onSelectPersona,
}: {
  run: SwarmtankRunDoc;
  runKey: string;
  /** Decisions the arena currently displays (baseline or scenario). */
  effectiveDecisions: SwarmtankDecision[];
  /** Changes when the effective layout changes (run or scenario). */
  layoutKey: string;
  selectedPersonaId: string | null;
  onSelectPersona: (id: string | null) => void;
}) {
  // Synchronous initial value: system query OR
  // workspace dataset setting. See the module note.
  const [reduceMotion, setReduceMotion] = useState(() => isReducedMotion());
  const [failedRunKey, setFailedRunKey] = useState<string | null>(null);
  const layout = useMemo(
    () => layoutSwarmtankArena(run.personas, effectiveDecisions),
    [run, effectiveDecisions],
  );
  const webglFailed = failedRunKey === runKey;

  const selectedPersona: SwarmtankPersona | null =
    run.personas.find(persona => persona.id === selectedPersonaId) ?? null;
  const selectedDecision: SwarmtankDecision | null =
    effectiveDecisions.find(decision => decision.personaId === selectedPersonaId) ??
    null;

  const converts = layout.agents.filter(agent => agent.decision === 'Convert').length;
  const confused = layout.agents.filter(agent => agent.decision === 'Confused').length;
  const rejects = layout.agents.filter(agent => agent.decision === 'Reject').length;

  return (
    <section aria-label="SwarmTank 3D arena">
      <h4>SwarmTank 3D arena</h4>
      <p>
        Simulated persona agents around the product pedestal: {converts}{' '}
        Convert (green, near the pedestal, facing it), {confused} Confused
        (amber, orbiting the perimeter), {rejects} Reject (red, outer
        perimeter, facing away). Simulated personas, not real customers.
      </p>
      <p>
        <label>
          <input
            type="checkbox"
            aria-label="Reduce motion (static arena view)"
            checked={reduceMotion}
            onChange={event => setReduceMotion(event.target.checked)}
          />{' '}
          Reduce motion (static arena view)
        </label>
      </p>
      {webglFailed ? (
        <p>
          3D arena unavailable — the complete table below carries the same
          information.
        </p>
      ) : (
        <SwarmtankThreeView
          layout={layout}
          layoutKey={layoutKey}
          runKey={runKey}
          selectedPersonaId={selectedPersonaId}
          onSelect={onSelectPersona}
          reducedMotion={reduceMotion}
          onUnavailable={() => setFailedRunKey(runKey)}
        />
      )}
      {reduceMotion && !webglFailed ? (
        <p>Motion reduced — static view.</p>
      ) : null}
      {selectedPersona && selectedDecision ? (
        <section aria-label="Selected persona">
          <h5>
            {selectedPersona.name} <span>Simulated persona.</span>
          </h5>
          <p>Role: {selectedPersona.role}</p>
          <p>Approximate budget: {selectedPersona.approximateBudget}</p>
          <p>Decision: {selectedDecision.decision}</p>
          <p>
            Key objection:{' '}
            {selectedDecision.keyObjection ?? 'None — converted.'}
          </p>
          <p>Perceived value: {selectedDecision.perceivedValue}</p>
          <p>
            Willingness to pay: {selectedDecision.willingnessToPay}
          </p>
          <p>
            <button onClick={() => onSelectPersona(null)}>
              Close persona details
            </button>
          </p>
        </section>
      ) : null}
    </section>
  );
}
