// M12 Slice 3: guided lifecycle walk derivation.
//
// Pure functions over artifact type strings, built on the Slice 1 pipeline
// engine (`lifecycle.ts`). The walk is the six-step guided path through
// the pipeline: Discover, Define, Simulate, Pressure-Test, Refine, Launch.
// Define spans two pipeline stages (Invent + Build): a concept alone is
// not a defined product — the step completes only when a concept AND a
// specification are both registered. Refine never completes: GHOST-PROMPT
// sessions are in-memory only and are never registered, exactly like the
// pipeline's refine stage.
//
// "Continue walk" targets the first non-Complete step among the five
// completable steps. Refine is deliberately excluded from Continue
// targeting: it never completes, so including it would pin Continue on
// Refine forever and block the Launch step. Refine keeps its own button,
// and the completion note says so plainly.
import {
  derivePipeline,
  missingSentence,
  type PipelineStageId,
  type PipelineStatus,
} from './lifecycle.ts';

export type WalkStepId =
  | 'discover'
  | 'define'
  | 'simulate'
  | 'pressure_test'
  | 'refine'
  | 'launch';

export interface WalkStep {
  id: WalkStepId;
  label: string;
  instruction: string;
  status: PipelineStatus;
  /** Requirement sentence while the step awaits its gate. */
  missing: string | null;
  /** Anchor within the project view, or null when a callback button is used. */
  anchor: string | null;
  /** Which callback the step's button uses, or null for anchor steps. */
  action: 'ask' | 'launch' | null;
}

export interface WalkDerivation {
  steps: WalkStep[];
  /** First non-Complete step among the five completable steps, or null. */
  continueStep: WalkStepId | null;
  /** All five completable steps Complete (Refine excluded by design). */
  complete: boolean;
}

const COMPLETABLE: WalkStepId[] = [
  'discover',
  'define',
  'simulate',
  'pressure_test',
  'launch',
];

function stepOf(
  id: WalkStepId,
  label: string,
  instruction: string,
  status: PipelineStatus,
  missing: string | null,
  anchor: string | null,
  action: 'ask' | 'launch' | null,
): WalkStep {
  return { id, label, instruction, status, missing, anchor, action };
}

/** Derive the guided walk state from artifact type strings (duplicates count). */
export function deriveWalk(artifactTypes: string[]): WalkDerivation {
  const { stages, current } = derivePipeline(artifactTypes);
  const stage = (id: PipelineStageId) => stages.find((s) => s.id === id)!;

  const discover = stage('discover');
  const invent = stage('invent');
  const build = stage('build');
  const simulate = stage('simulate');
  const pressureTest = stage('pressure_test');
  const refine = stage('refine');
  const launch = stage('launch');

  let defineStatus: PipelineStatus;
  let defineMissing: string | null = null;
  if (invent.status === 'Complete' && build.status === 'Complete') {
    defineStatus = 'Complete';
  } else if (current === 'invent' || current === 'build') {
    defineStatus = 'Current / In Progress';
  } else if (invent.status === 'Complete') {
    // A concept exists but the walk has moved past Build (for example a
    // simulation is already registered): drafting the spec is still open.
    defineStatus = 'Available';
  } else {
    defineStatus = 'Awaiting Preceding Stage';
    // Build's own `missing` is null when Build itself is Complete (the
    // spec-without-concept edge), so take the sentence from the engine
    // rather than the stage state.
    defineMissing = missingSentence('build');
  }

  const steps: WalkStep[] = [
    stepOf(
      'discover',
      'Discover',
      'Generate an opportunity set from your sources, then save it as an artifact.',
      discover.status,
      discover.missing,
      '#project-opportunities-heading',
      null,
    ),
    stepOf(
      'define',
      'Define',
      'Create a concept in LEXI-COLLIDER, then draft its specification and plan in Product Forge.',
      defineStatus,
      defineMissing,
      invent.status === 'Complete'
        ? '#project-artifacts-heading'
        : '#project-collider-heading',
      null,
    ),
    stepOf(
      'simulate',
      'Simulate',
      'Run a SwarmTank simulation across 20 personas and save the run.',
      simulate.status,
      simulate.missing,
      '#project-swarmtank-heading',
      null,
    ),
    stepOf(
      'pressure_test',
      'Pressure-Test',
      'Run a SOMNIA adversarial evaluation over nightmare scenarios and save it.',
      pressureTest.status,
      pressureTest.missing,
      '#project-somnia-heading',
      null,
    ),
    stepOf(
      'refine',
      'Refine',
      'Open Ask ANANSI, enable Draft Exploration, revise, review fragments, then wipe the session.',
      refine.status,
      refine.missing,
      null,
      'ask',
    ),
    stepOf(
      'launch',
      'Launch',
      'Open Launch Studio, generate the four-channel copy, download the bundle, and register it as a launch-material artifact.',
      launch.status,
      launch.missing,
      null,
      'launch',
    ),
  ];

  const continueStep =
    COMPLETABLE.find((id) => steps.find((s) => s.id === id)!.status !== 'Complete') ?? null;
  const complete = continueStep === null;
  return { steps, continueStep, complete };
}
