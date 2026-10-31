/**
 * M9 SwarmTank Slice 2: the arena layout, pure.
 *
 * This module imports nothing from Three.js and touches
 * no DOM: node tests and the Playwright spec can use
 * it without a GL context. The scene builder consumes
 * exactly this output, so the layout a test computes
 * is the layout the canvas renders.
 *
 * Amphitheater plan (MASTER_SPEC §38): a central
 * pedestal, concentric rings around it, one agent per
 * persona. Convert agents sit closest and face the
 * pedestal; Confused agents orbit the perimeter
 * tangentially with hesitation markers; Reject agents
 * sit outermost and face away. Everything is a
 * deterministic function of run order — no RNG, no
 * clock — so the same run always lays out identically.
 */

import type {
  SwarmtankDecision,
  SwarmtankPersona,
} from '@anansi/contracts';

export type ArenaDecision = 'Convert' | 'Confused' | 'Reject';

export type ArenaAgent = {
  personaId: string;
  decision: ArenaDecision;
  /** Zero-based ring within this decision group. */
  ring: number;
  radius: number;
  angle: number;
  x: number;
  z: number;
  /** Unit facing vector in XZ. */
  facingX: number;
  facingZ: number;
};

export type ArenaLayout = {
  agents: ArenaAgent[];
  /** Every ring radius actually used, ascending. */
  ringRadii: number[];
};

const BASE_RADII: Record<ArenaDecision, number> = {
  Convert: 60,
  Confused: 120,
  Reject: 180,
};
const RING_SPACING = 26;
const AGENT_SPACING = 14;
const RING_STAGGER = 0.35;

function isArenaDecision(value: string): value is ArenaDecision {
  return value === 'Convert' || value === 'Confused' || value === 'Reject';
}

function facingFor(
  decision: ArenaDecision,
  angle: number,
): { facingX: number; facingZ: number } {
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  if (decision === 'Convert') return { facingX: -cos, facingZ: -sin };
  if (decision === 'Reject') return { facingX: cos, facingZ: sin };
  // Confused: tangential, the orbit direction.
  return { facingX: -sin, facingZ: cos };
}

/**
 * Lay out one agent per persona, in run order within
 * each decision group. Personas without a decision
 * (should not happen for a validated run) are skipped.
 */
export function layoutSwarmtankArena(
  personas: SwarmtankPersona[],
  decisions: SwarmtankDecision[],
): ArenaLayout {
  const decisionByPersona = new Map(
    decisions.map(decision => [decision.personaId, decision.decision]),
  );
  const agents: ArenaAgent[] = [];
  const ringRadii = new Set<number>();
  const groups: Record<ArenaDecision, SwarmtankPersona[]> = {
    Convert: [],
    Confused: [],
    Reject: [],
  };
  for (const persona of personas) {
    const decision = decisionByPersona.get(persona.id);
    if (decision && isArenaDecision(decision)) groups[decision].push(persona);
  }
  (Object.keys(groups) as ArenaDecision[]).forEach(decision => {
    const group = groups[decision];
    const base = BASE_RADII[decision];
    let ring = 0;
    let placedInRing = 0;
    let radius = base;
    let capacity = Math.max(1, Math.floor((2 * Math.PI * radius) / AGENT_SPACING));
    for (const persona of group) {
      if (placedInRing >= capacity) {
        ring += 1;
        placedInRing = 0;
        radius = base + ring * RING_SPACING;
        capacity = Math.max(
          1,
          Math.floor((2 * Math.PI * radius) / AGENT_SPACING),
        );
      }
      const angle =
        (placedInRing / capacity) * Math.PI * 2 + ring * RING_STAGGER;
      const facing = facingFor(decision, angle);
      agents.push({
        personaId: persona.id,
        decision,
        ring,
        radius,
        angle,
        x: radius * Math.cos(angle),
        z: radius * Math.sin(angle),
        facingX: facing.facingX,
        facingZ: facing.facingZ,
      });
      ringRadii.add(radius);
      placedInRing += 1;
    }
  });
  return {
    agents,
    ringRadii: [...ringRadii].sort((a, b) => a - b),
  };
}
