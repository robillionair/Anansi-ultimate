// M12 Slice 1: unified product creation pipeline stage engine (TS mirror).
//
// Pure functions over artifact type strings, mirroring
// `crates/core/src/pipeline.rs` exactly: same stages, gates, evidence
// sets, and status rule. The navigator derives its input from the
// already-loaded `listProjectArtifacts` response, so inspecting stages
// performs zero server mutations and needs no model. Unknown artifact
// types are ignored, never errors.

export type PipelineStageId =
  | 'discover'
  | 'invent'
  | 'build'
  | 'simulate'
  | 'pressure_test'
  | 'refine'
  | 'launch';

export type PipelineStatus =
  | 'Complete'
  | 'Current / In Progress'
  | 'Available'
  | 'Awaiting Preceding Stage';

export interface PipelineStageState {
  id: PipelineStageId;
  label: string;
  status: PipelineStatus;
  /** Per-type evidence counts, only types actually present, in evidence order. */
  evidenceCounts: { type: string; count: number }[];
  /** Requirement sentence while the stage awaits its gate. */
  missing: string | null;
}

export interface PipelineDerivation {
  stages: PipelineStageState[];
  current: PipelineStageId | null;
}

const STAGE_DEFS: {
  id: PipelineStageId;
  label: string;
  evidence: string[];
  advanceable: boolean;
}[] = [
  { id: 'discover', label: 'Discover', evidence: ['idea'], advanceable: true },
  { id: 'invent', label: 'Invent', evidence: ['concept'], advanceable: true },
  {
    id: 'build',
    label: 'Build',
    evidence: ['specification', 'source-package', 'deliverable', 'source-review'],
    advanceable: true,
  },
  {
    id: 'simulate',
    label: 'Simulate',
    evidence: ['simulation', 'scenario'],
    advanceable: true,
  },
  {
    id: 'pressure_test',
    label: 'Pressure test',
    evidence: ['somnia-evaluation'],
    advanceable: true,
  },
  // Refine holds no evidence: GHOST-PROMPT sessions are in-memory only
  // and are never registered, so refine never auto-completes, is never
  // Current, and never blocks launch.
  { id: 'refine', label: 'Refine', evidence: [], advanceable: false },
  { id: 'launch', label: 'Launch', evidence: ['launch-material'], advanceable: true },
];

function gateMet(id: PipelineStageId, has: (t: string) => boolean): boolean {
  switch (id) {
    // Always open: §62 starts from a user idea, and GHOST-PROMPT is an
    // editor-scoped tool usable at any time.
    case 'discover':
    case 'invent':
    case 'refine':
      return true;
    // Forge, SwarmTank, and the SOMNIA QA bridge all work from a Forge concept.
    case 'build':
    case 'simulate':
    case 'pressure_test':
      return has('concept');
    // A completed simulation or evaluation artifact.
    case 'launch':
      return (
        has('simulation') ||
        has('scenario') ||
        has('somnia-evaluation') ||
        has('evaluation')
      );
  }
}

function missingSentence(id: PipelineStageId): string | null {
  switch (id) {
    case 'build':
    case 'simulate':
    case 'pressure_test':
      return 'Needs at least one Forge concept artifact.';
    case 'launch':
      return 'Needs a completed simulation or evaluation artifact.';
    default:
      return null;
  }
}

/** Derive the pipeline state from artifact type strings (duplicates count). */
export function derivePipeline(artifactTypes: string[]): PipelineDerivation {
  const has = (t: string) => artifactTypes.includes(t);
  const hasEvidence = STAGE_DEFS.map((s) => s.evidence.some((t) => has(t)));
  // Highest stage index with completing evidence, or -1.
  let furthest = -1;
  for (let i = 0; i < hasEvidence.length; i += 1) {
    if (hasEvidence[i]) furthest = i;
  }
  // Current: first advanceable stage after `furthest` whose gate is met.
  // Scanning after `furthest` (never from the start) keeps the
  // recommendation moving forward past work already done.
  let current: PipelineStageId | null = null;
  for (let i = 0; i < STAGE_DEFS.length; i += 1) {
    if (i > furthest) {
      const def = STAGE_DEFS[i];
      if (def.advanceable && gateMet(def.id, has)) {
        current = def.id;
        break;
      }
    }
  }
  const stages: PipelineStageState[] = STAGE_DEFS.map((def, i) => {
    let status: PipelineStatus;
    if (hasEvidence[i]) {
      // Evidence always beats a missing gate: a directly registered
      // simulation without a concept still shows Simulate Complete.
      status = 'Complete';
    } else if (current === def.id) {
      status = 'Current / In Progress';
    } else if (gateMet(def.id, has)) {
      status = 'Available';
    } else {
      status = 'Awaiting Preceding Stage';
    }
    const evidenceCounts = def.evidence
      .map((t) => ({ type: t, count: artifactTypes.filter((a) => a === t).length }))
      .filter((c) => c.count > 0);
    return {
      id: def.id,
      label: def.label,
      status,
      evidenceCounts,
      missing: status === 'Awaiting Preceding Stage' ? missingSentence(def.id) : null,
    };
  });
  return { stages, current };
}
