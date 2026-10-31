// M9 SwarmTank Slice 3: the instant local
// sensitivity engine mirrors the Rust evaluator
// (crates/core/src/swarmtank.rs) exactly — same
// factor weights, thresholds, override ladder,
// and integer-cents reason formatting. These
// tests mirror the Rust vectors where the fixture
// is shared, and cover the bounds the Rust suite
// does not (free tier, 10x price, zero features).

import test from 'node:test';
import assert from 'node:assert/strict';
import {
  computeScenarioComparison,
  defaultScenarioFor,
  evaluateScenarioPersona,
  evaluateScenarioPopulation,
  scenarioSignature,
  scenariosEqual,
} from '../src/swarmtankSensitivity';
import type {
  SwarmtankDecision,
  SwarmtankInput,
  SwarmtankPersona,
  SwarmtankScenario,
} from '@anansi/contracts';

function persona(overrides: Partial<SwarmtankPersona> & { id: string }): SwarmtankPersona {
  return {
    name: 'Test Persona',
    role: 'Small agency owner',
    companySize: '11-50',
    approximateBudget: 1000,
    needs: [],
    priorities: [],
    skepticism: 0.4,
    priceSensitivity: 0.6,
    purchaseConstraints: [],
    requiredTrustSignals: [],
    expectedImplementationEffort: 'Medium',
    synthetic: true,
    ...overrides,
  } as SwarmtankPersona;
}

function input(overrides: Partial<SwarmtankInput> = {}): SwarmtankInput {
  return {
    description: 'A scheduling tool.',
    pricing: '$49',
    priceAmount: 49,
    segment: 'small agencies',
    industry: 'services',
    features: [],
    valueProposition: 'Saves time.',
    ...overrides,
  } as SwarmtankInput;
}

function decision(
  personaId: string,
  name: 'Convert' | 'Confused' | 'Reject',
  objection: string | null,
): SwarmtankDecision {
  return {
    personaId,
    decision: name,
    reason: 'Score 0.70.',
    keyObjection: objection,
    perceivedValue: 0.7,
    willingnessToPay: 500,
    score: 0.7,
  };
}

function scenario(overrides: Partial<SwarmtankScenario> = {}): SwarmtankScenario {
  return {
    priceMonthly: null,
    activeFeatures: [],
    trustSignals: {
      soc2Security: false,
      freeTrialAvailable: false,
      slaGuarantee: false,
      moneyBackGuarantee: false,
    },
    setupDifficulty: 'moderate',
    ...overrides,
  } as SwarmtankScenario;
}

test('scenario veto fires only at high price sensitivity', () => {
  const freelancer = persona({
    id: 'persona-001',
    role: 'Cost-sensitive freelancer',
    approximateBudget: 500,
    priceSensitivity: 0.9,
    skepticism: 0.5,
    needs: ['Fast setup'],
  });
  const base = input({
    segment: 'freelancers',
    features: ['Fast setup dashboard'],
    valueProposition: 'Saves time for freelancers.',
  });
  const vetoed = evaluateScenarioPersona(
    freelancer,
    base,
    decision('persona-001', 'Convert', null),
    scenario({ priceMonthly: 600 }),
  );
  assert.equal(vetoed.decision, 'Reject');
  assert.equal(
    vetoed.keyObjection,
    'Price exceeds budget under high price sensitivity',
  );

  // Same over-budget price, moderate sensitivity: no veto.
  const owner = persona({
    id: 'persona-002',
    role: 'Small agency owner',
    approximateBudget: 100,
    priceSensitivity: 0.6,
    skepticism: 0.4,
    needs: ['Client reporting'],
  });
  const moderate = evaluateScenarioPersona(
    owner,
    input({
      segment: 'small agencies',
      features: ['Client reporting dashboard'],
      valueProposition: 'Agencies save time.',
    }),
    decision('persona-002', 'Reject', 'Price exceeds budget'),
    scenario({ priceMonthly: 500, activeFeatures: ['Client reporting dashboard'] }),
  );
  assert.equal(moderate.decision, 'Convert');
  assert.equal(moderate.keyObjection, null);
  assert.equal(moderate.score, 0.66);
});

test('free trial resolves freelancer price hesitation', () => {
  const freelancer = persona({
    id: 'persona-001',
    role: 'Cost-sensitive freelancer',
    approximateBudget: 300,
    priceSensitivity: 0.9,
    skepticism: 1.0,
    needs: ['Deep analytics'],
  });
  const base = input({
    segment: 'solo professionals',
    priceAmount: 250,
    features: ['Simple invoicing'],
    valueProposition: 'Get paid faster.',
  });
  const baseline = decision('persona-001', 'Confused', 'Price sensitivity');
  const withoutTrial = evaluateScenarioPersona(
    freelancer,
    base,
    baseline,
    scenario({ priceMonthly: 250, activeFeatures: ['Simple invoicing'] }),
  );
  assert.equal(withoutTrial.decision, 'Confused');
  assert.equal(withoutTrial.keyObjection, 'Price sensitivity');
  const withTrial = evaluateScenarioPersona(
    freelancer,
    base,
    baseline,
    scenario({
      priceMonthly: 250,
      activeFeatures: ['Simple invoicing'],
      trustSignals: {
        soc2Security: false,
        freeTrialAvailable: true,
        slaGuarantee: false,
        moneyBackGuarantee: false,
      },
    }),
  );
  assert.equal(withTrial.decision, 'Convert');
  assert.equal(withTrial.keyObjection, null);
});

test('dropping a covered need downgrades a convert with the exact objection', () => {
  const owner = persona({
    id: 'persona-001',
    role: 'Small agency owner',
    approximateBudget: 3000,
    needs: ['Client reporting'],
  });
  const base = input({ features: ['Client reporting'] });
  const dropped = evaluateScenarioPersona(
    owner,
    base,
    decision('persona-001', 'Convert', null),
    scenario({ activeFeatures: [] }),
  );
  assert.equal(dropped.decision, 'Confused');
  assert.equal(
    dropped.keyObjection,
    'Missing critical required feature: Client reporting',
  );
  const kept = evaluateScenarioPersona(
    owner,
    base,
    decision('persona-001', 'Convert', null),
    scenario({ activeFeatures: ['Client reporting'] }),
  );
  assert.equal(kept.decision, 'Convert');
});

test('complex setup downgrades low-effort converts only', () => {
  const freelancer = persona({
    id: 'persona-001',
    role: 'Cost-sensitive freelancer',
    approximateBudget: 5000,
    expectedImplementationEffort: 'Low',
    needs: ['Fast setup'],
  });
  const base = input({
    segment: 'freelancers',
    features: ['Fast setup dashboard'],
  });
  const complex = evaluateScenarioPersona(
    freelancer,
    base,
    decision('persona-001', 'Convert', null),
    scenario({ setupDifficulty: 'complex', activeFeatures: ['Fast setup dashboard'] }),
  );
  assert.equal(complex.decision, 'Confused');
  assert.equal(complex.keyObjection, 'High setup overhead exceeds team capacity');
  const moderate = evaluateScenarioPersona(
    freelancer,
    base,
    decision('persona-001', 'Convert', null),
    scenario({ setupDifficulty: 'moderate', activeFeatures: ['Fast setup dashboard'] }),
  );
  assert.equal(moderate.decision, 'Convert');
});

test('soc2 floor lifts a trust-confused enterprise persona', () => {
  const director = persona({
    id: 'persona-001',
    role: 'Enterprise finance director',
    approximateBudget: 50,
    priceSensitivity: 0.4,
    skepticism: 1.0,
    needs: ['Audit exports'],
    requiredTrustSignals: ['SOC 2 report'],
    purchaseConstraints: ['Security review required'],
  });
  const base = input({
    segment: 'enterprise finance',
    priceAmount: 100,
    features: ['Unrelated widget'],
    valueProposition: 'Finance teams close faster.',
  });
  const baseline = decision('persona-001', 'Confused', 'Missing trust signals');
  const withoutSoc2 = evaluateScenarioPersona(
    director,
    base,
    baseline,
    scenario({ priceMonthly: 100, activeFeatures: ['Unrelated widget'] }),
  );
  assert.equal(withoutSoc2.decision, 'Confused');
  assert.equal(withoutSoc2.score, 0.6);
  const withSoc2 = evaluateScenarioPersona(
    director,
    base,
    baseline,
    scenario({
      priceMonthly: 100,
      activeFeatures: ['Unrelated widget'],
      trustSignals: {
        soc2Security: true,
        freeTrialAvailable: false,
        slaGuarantee: false,
        moneyBackGuarantee: false,
      },
    }),
  );
  assert.equal(withSoc2.decision, 'Convert');
  assert.ok(withSoc2.reason.includes('trust 0.80'));
});

test('default scenario is idempotent: it reproduces its own evaluation', () => {
  const runInput = input({ features: ['Client reporting', 'Fast setup'] });
  const personas = [
    persona({
      id: 'persona-001',
      role: 'Cost-sensitive freelancer',
      approximateBudget: 5000,
      needs: ['Fast setup'],
    }),
    persona({
      id: 'persona-002',
      role: 'Small agency owner',
      approximateBudget: 100,
      skepticism: 0.4,
      needs: ['Client reporting'],
    }),
  ];
  // Under the default scenario (baseline price, all
  // features, no trust signals, moderate setup) the
  // override ladder cannot fire, so evaluation is
  // pure factor math — and feeding its output back
  // as the baseline reproduces it exactly.
  const dummyBaseline = personas.map(p => decision(p.id, 'Convert', null));
  const first = evaluateScenarioPopulation(
    personas,
    runInput,
    dummyBaseline,
    defaultScenarioFor({ input: runInput }),
  );
  assert.deepEqual(
    first.map(d => d.decision),
    ['Convert', 'Convert'],
  );
  const second = evaluateScenarioPopulation(
    personas,
    runInput,
    first,
    defaultScenarioFor({ input: runInput }),
  );
  assert.deepEqual(second, first);
  assert.ok(scenariosEqual(
    defaultScenarioFor({ input: runInput }),
    defaultScenarioFor({ input: runInput }),
  ));
  assert.ok(!scenariosEqual(
    defaultScenarioFor({ input: runInput }),
    scenario({ priceMonthly: 99 }),
  ));
  assert.equal(
    scenarioSignature(defaultScenarioFor({ input: runInput })),
    scenarioSignature(defaultScenarioFor({ input: runInput })),
  );
});

test('transition matrix counts migrations and objection deltas', () => {
  const baseline = [
    decision('persona-001', 'Convert', null),
    decision('persona-002', 'Confused', 'Price sensitivity'),
    decision('persona-003', 'Reject', 'Budget constraints'),
  ];
  const changed = [
    decision('persona-001', 'Confused', 'Price sensitivity'),
    decision('persona-002', 'Convert', null),
    decision('persona-003', 'Reject', 'Price sensitivity'),
  ];
  const comparison = computeScenarioComparison(baseline, changed, 49, 49);
  assert.deepEqual(comparison.convertedIds, ['persona-002']);
  assert.deepEqual(comparison.lostIds, ['persona-001']);
  assert.equal(comparison.unchangedCount, 1);
  assert.equal(comparison.transitions.length, 3);
  assert.equal(comparison.resolvedObjections.length, 1);
  assert.equal(comparison.resolvedObjections[0].item, 'Budget constraints');
  assert.equal(comparison.introducedObjections.length, 1);
  assert.equal(comparison.introducedObjections[0].item, 'Price sensitivity');
  assert.equal(comparison.introducedObjections[0].baselineCount, 1);
  assert.equal(comparison.introducedObjections[0].scenarioCount, 2);
  assert.equal(comparison.priceElasticity, null);
});

test('bounds: free tier never vetoes, 10x price vetoes, zero features drop fit', () => {
  const freelancer = persona({
    id: 'persona-001',
    role: 'Cost-sensitive freelancer',
    approximateBudget: 100,
    priceSensitivity: 0.9,
    skepticism: 0.5,
    needs: ['Fast setup'],
  });
  const base = input({
    segment: 'freelancers',
    features: ['Fast setup dashboard'],
  });
  const free = evaluateScenarioPersona(
    freelancer,
    base,
    decision('persona-001', 'Convert', null),
    scenario({ priceMonthly: 0, activeFeatures: ['Fast setup dashboard'] }),
  );
  assert.equal(free.decision, 'Convert');
  const tenX = evaluateScenarioPersona(
    freelancer,
    base,
    decision('persona-001', 'Convert', null),
    scenario({ priceMonthly: 490, activeFeatures: ['Fast setup dashboard'] }),
  );
  assert.equal(tenX.decision, 'Reject');
  assert.equal(
    tenX.keyObjection,
    'Price exceeds budget under high price sensitivity',
  );
  const noFeatures = evaluateScenarioPersona(
    freelancer,
    base,
    decision('persona-001', 'Convert', null),
    scenario({ activeFeatures: [] }),
  );
  // Baseline-covered need now uncovered: downgrade, not a veto.
  assert.equal(noFeatures.decision, 'Confused');
  assert.ok(
    (noFeatures.keyObjection ?? '').startsWith(
      'Missing critical required feature: ',
    ),
  );
});

test('population evaluation is deterministic and fast', () => {
  const personas = Array.from({ length: 100 }, (_, i) =>
    persona({
      id: `persona-${String(i + 1).padStart(3, '0')}`,
      role: i % 2 === 0 ? 'Small agency owner' : 'Cost-sensitive freelancer',
      approximateBudget: 100 + i * 50,
      needs: ['Fast setup'],
    }),
  );
  const base = input({
    segment: 'small agencies',
    features: ['Fast setup dashboard'],
  });
  const baseline = personas.map(p => decision(p.id, 'Convert', null));
  const scen = scenario({ priceMonthly: 99 });
  const first = evaluateScenarioPopulation(personas, base, baseline, scen);
  const started = performance.now();
  const second = evaluateScenarioPopulation(personas, base, baseline, scen);
  const elapsedMs = performance.now() - started;
  assert.deepEqual(second, first);
  assert.equal(first.length, 100);
  assert.ok(elapsedMs < 100, `100 personas took ${elapsedMs} ms`);
});
