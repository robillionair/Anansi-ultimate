// Shared §27 fixture: the SAME values as
// `collider_artifacts::artifact_tests` in Rust.
// This is a helper, not a test file: it is imported
// by the artifact and export suites.
import type { ArtifactSourceDoc } from '../src/colliderArtifacts';

export const fixtureDoc: ArtifactSourceDoc = {
  beamA: 'Beam A text about kitchens.',
  beamB: 'Beam B text about orbitals.',
  provider: 'test-provider',
  model: 'test-model',
  result: {
    sourceUnderstanding: 'Both beams describe bounded work.',
    conceptualPrimitives: ['Bounded work', 'Shared heat', 'Timed orbit'],
    crossDomainMappings: [
      'Bounded work maps to timed orbit',
      'Shared heat maps to bounded work',
    ],
    combinations: [
      { name: 'Orbit kitchen', description: 'A kitchen timed like an orbit.' },
      { name: 'Heat timer', description: 'Heat applied on a schedule.' },
    ],
    feasibility: {
      hypotheses: ['Timers help cooks', 'Orbits explain timing'],
      dependencies: ['A clock'],
      unknownCosts: [],
      risks: ['Burnt food'],
      testability: 'Cook one timed meal.',
    },
    productization: {
      conceptBrief: 'Brief text.',
      architecture: 'Architecture text.',
      roadmap: 'Roadmap text.',
      testPlan: 'Test plan text.',
      prototypeProposal: 'Prototype text.',
    },
  },
};
