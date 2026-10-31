/**
 * M9 SwarmTank Slice 3: the instant local
 * sensitivity engine — the TypeScript mirror of the
 * Rust scenario evaluator in
 * crates/core/src/swarmtank.rs. Pure arithmetic,
 * no DOM, no network, no randomness: 100 personas
 * re-evaluate in far under 5 ms by construction.
 *
 * Mirror discipline (design §2.4): every formula,
 * every operation order, and every rounding step
 * matches the Rust side. Factors and scores use
 * IEEE doubles in both languages, so identical
 * operation order gives identical results. Reason
 * strings use integer-cents formatting in both
 * mirrors — never toFixed(2), which rounds halves
 * differently from Rust {:.2}. The archetype table
 * below (role -> effort class, authority, segment
 * keywords) is copied from the Rust ARCHETYPES
 * table and must be updated if that table changes.
 */

import type {
  SwarmtankDecision,
  SwarmtankInput,
  SwarmtankPersona,
  SwarmtankScenario,
} from '@anansi/contracts';

export type ScenarioDecision = SwarmtankDecision;

export type ScenarioTransition = {
  from: string;
  to: string;
  count: number;
};

export type ScenarioObjectionDelta = {
  item: string;
  baselineCount: number;
  scenarioCount: number;
};

export type ScenarioMetrics = {
  total: number;
  converts: number;
  confused: number;
  rejects: number;
  conversionRate: number;
  averageWillingnessToPay: number;
  averagePerceivedValue: number;
};

export type ScenarioComparison = {
  transitions: ScenarioTransition[];
  convertedIds: string[];
  lostIds: string[];
  unchangedCount: number;
  resolvedObjections: ScenarioObjectionDelta[];
  introducedObjections: ScenarioObjectionDelta[];
  baseline: ScenarioMetrics;
  scenario: ScenarioMetrics;
  priceElasticity: number | null;
};

type ArchetypeRow = {
  segmentKeywords: string[];
  authority: number;
  effortClass: number;
};

const ARCHETYPES: Record<string, ArchetypeRow> = {
  'Cost-sensitive freelancer': {
    segmentKeywords: ['freelanc'],
    authority: 0.4,
    effortClass: 0.0,
  },
  'Enterprise finance director': {
    segmentKeywords: ['enterprise', 'finance'],
    authority: 0.7,
    effortClass: 1.0,
  },
  'Technical founder': {
    segmentKeywords: ['startup', 'founder', 'technical'],
    authority: 0.9,
    effortClass: 0.0,
  },
  'Small agency owner': {
    segmentKeywords: ['agency', 'agencies'],
    authority: 0.9,
    effortClass: 0.5,
  },
  'Corporate procurement manager': {
    segmentKeywords: ['corporate', 'procurement'],
    authority: 0.7,
    effortClass: 1.0,
  },
  'Privacy-conscious consultant': {
    segmentKeywords: ['privacy', 'consult'],
    authority: 0.4,
    effortClass: 0.0,
  },
  'Overwhelmed operations manager': {
    segmentKeywords: ['operations', 'ops'],
    authority: 0.7,
    effortClass: 0.5,
  },
  'Startup product manager': {
    segmentKeywords: ['startup', 'product'],
    authority: 0.7,
    effortClass: 0.5,
  },
  'Nonprofit program lead': {
    segmentKeywords: ['nonprofit', 'charity'],
    authority: 0.7,
    effortClass: 0.0,
  },
  'E-commerce store owner': {
    segmentKeywords: ['ecommerce', 'e-commerce', 'retail', 'store'],
    authority: 0.9,
    effortClass: 0.0,
  },
  'IT administrator': {
    segmentKeywords: ['it', 'infrastructure'],
    authority: 0.4,
    effortClass: 1.0,
  },
  'Marketing lead': {
    segmentKeywords: ['marketing'],
    authority: 0.7,
    effortClass: 0.5,
  },
};

function round2(value: number): number {
  return Math.round(value * 100) / 100;
}

function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}

/** Integer-cents formatting, mirroring Rust fmt2. */
export function fmt2(value: number): string {
  const cents = Math.sign(value) * Math.round(Math.abs(value) * 100);
  const sign = cents < 0 ? '-' : '';
  const abs = Math.abs(cents);
  return `${sign}${Math.floor(abs / 100)}.${String(abs % 100).padStart(2, '0')}`;
}

function contains(haystack: string, needle: string): boolean {
  return haystack.toLowerCase().includes(needle.toLowerCase());
}

function archetypeFor(persona: SwarmtankPersona): ArchetypeRow | null {
  return ARCHETYPES[persona.role] ?? null;
}

function fitFactor(
  persona: SwarmtankPersona,
  input: SwarmtankInput,
): number {
  const archetype = archetypeFor(persona);
  if (!archetype) return 0.4;
  let fit = 0.4;
  if (archetype.segmentKeywords.some(keyword => contains(input.segment ?? '', keyword))) {
    fit += 0.3;
  }
  const haystack = `${(input.features ?? []).join(' ')} ${input.valueProposition ?? ''}`;
  if (persona.needs.length > 0) {
    const hits = persona.needs.filter(need =>
      need
        .split(/\s+/)
        .some(word => [...word].length > 3 && contains(haystack, word)),
    ).length;
    fit += 0.3 * (hits / persona.needs.length);
  }
  return clamp(fit, 0, 1);
}

function clarityFactor(input: SwarmtankInput): number {
  const present = [
    (input.description ?? '').trim() !== '',
    (input.pricing ?? '').trim() !== '',
    (input.valueProposition ?? '').trim() !== '',
    (input.features ?? []).length > 0,
  ].filter(Boolean).length;
  switch (present) {
    case 4:
      return 0.9;
    case 3:
      return 0.7;
    case 2:
      return 0.5;
    case 1:
      return 0.3;
    default:
      return 0.1;
  }
}

type Factors = {
  fit: number;
  budget: number;
  price: number;
  trust: number;
  switching: number;
  clarity: number;
  authority: number;
  score: number;
};

function evaluateFactors(
  persona: SwarmtankPersona,
  input: SwarmtankInput,
): Factors {
  const fit = fitFactor(persona, input);
  let budget: number;
  let price: number;
  const statedPrice = input.priceAmount ?? null;
  if (statedPrice !== null && statedPrice > 0) {
    const amount = statedPrice;
    if (persona.approximateBudget >= amount) {
      budget = 1.0;
    } else if (persona.approximateBudget <= 0) {
      budget = 0.0;
    } else {
      budget = persona.approximateBudget / amount;
    }
    const denom = Math.max(amount, persona.approximateBudget);
    price =
      denom > 0
        ? clamp(1.0 - persona.priceSensitivity * (amount / denom), 0, 1)
        : 0.5;
  } else {
    budget = 1.0;
    price = 0.5;
  }
  const trust = 0.75 - 0.5 * persona.skepticism;
  const archetype = archetypeFor(persona);
  const effortClass = archetype ? archetype.effortClass : 0.5;
  const switching = 1.0 - effortClass * 0.5;
  const clarity = clarityFactor(input);
  const authority = archetype ? archetype.authority : 0.5;
  const score =
    0.25 * fit +
    0.2 * budget +
    0.15 * price +
    0.1 * trust +
    0.1 * switching +
    0.1 * clarity +
    0.1 * authority;
  return { fit, budget, price, trust, switching, clarity, authority, score };
}

function objectionFor(factor: string): string {
  switch (factor) {
    case 'budget':
      return 'Budget constraints';
    case 'price':
      return 'Price sensitivity';
    case 'trust':
      return 'Missing trust signals';
    case 'switching':
      return 'Implementation effort';
    case 'clarity':
      return 'Unclear value proposition';
    case 'authority':
      return 'Lacks purchasing authority';
    default:
      return 'Weak product-customer fit';
  }
}

function scenarioReason(f: Factors): string {
  return (
    `Score ${fmt2(f.score)}: fit ${fmt2(f.fit)}, budget ${fmt2(f.budget)}, ` +
    `price ${fmt2(f.price)}, trust ${fmt2(f.trust)}, switching ${fmt2(f.switching)}, ` +
    `clarity ${fmt2(f.clarity)}, authority ${fmt2(f.authority)}.`
  );
}

export function coveredNeeds(
  persona: SwarmtankPersona,
  features: string[],
  valueProposition: string,
): string[] {
  const haystack = `${features.join(' ')} ${valueProposition}`;
  return persona.needs.filter(need =>
    need
      .split(/\s+/)
      .some(word => [...word].length > 3 && contains(haystack, word)),
  );
}

function hasSecuritySignal(persona: SwarmtankPersona): boolean {
  return [...persona.requiredTrustSignals, ...persona.purchaseConstraints].some(
    signal =>
      contains(signal, 'soc') ||
      contains(signal, 'security') ||
      contains(signal, 'compliance'),
  );
}

function decisionRank(decision: string): number {
  if (decision === 'Convert') return 2;
  if (decision === 'Confused') return 1;
  return 0;
}

export function evaluateScenarioPersona(
  persona: SwarmtankPersona,
  baselineInput: SwarmtankInput,
  baselineDecision: SwarmtankDecision,
  scenario: SwarmtankScenario,
): SwarmtankDecision {
  const scenarioInput: SwarmtankInput = {
    ...baselineInput,
    priceAmount:
      scenario.priceMonthly !== null && scenario.priceMonthly !== undefined
        ? scenario.priceMonthly
        : baselineInput.priceAmount,
    features: scenario.activeFeatures ?? [],
  };
  const trustSignals = scenario.trustSignals ?? {
    soc2Security: false,
    freeTrialAvailable: false,
    slaGuarantee: false,
    moneyBackGuarantee: false,
  };
  const setupDifficulty = scenario.setupDifficulty ?? 'moderate';
  const f = evaluateFactors(persona, scenarioInput);
  // Trust deltas.
  const effectiveSkepticism = clamp(
    persona.skepticism - (trustSignals.freeTrialAvailable ? 0.2 : 0),
    0,
    1,
  );
  f.trust = 0.75 - 0.5 * effectiveSkepticism;
  if (trustSignals.soc2Security && hasSecuritySignal(persona)) {
    f.trust = Math.max(f.trust, 0.8);
  }
  if (trustSignals.slaGuarantee) {
    f.trust = Math.min(1, f.trust + 0.05);
  }
  if (trustSignals.moneyBackGuarantee) {
    f.price = Math.min(1, f.price + 0.1);
  }
  // Setup-effort override.
  if (setupDifficulty === 'effortless') {
    f.switching = 1.0;
  } else if (setupDifficulty === 'complex') {
    f.switching = Math.min(f.switching, 0.5);
  }
  f.score =
    0.25 * f.fit +
    0.2 * f.budget +
    0.15 * f.price +
    0.1 * f.trust +
    0.1 * f.switching +
    0.1 * f.clarity +
    0.1 * f.authority;
  let decision =
    f.score >= 0.65 ? 'Convert' : f.score >= 0.4 ? 'Confused' : 'Reject';
  let keyObjection: string | null;
  if (decision === 'Convert') {
    keyObjection = null;
  } else {
    // First minimal factor wins (Rust min_by semantics).
    const ranked: Array<[string, number]> = [
      ['fit', f.fit],
      ['budget', f.budget],
      ['price', f.price],
      ['trust', f.trust],
      ['switching', f.switching],
      ['clarity', f.clarity],
      ['authority', f.authority],
    ];
    let lowest = ranked[0];
    for (const entry of ranked) {
      if (entry[1] < lowest[1]) lowest = entry;
    }
    keyObjection = objectionFor(lowest[0]);
  }
  // Override ladder, in order.
  const scenarioPrice = scenarioInput.priceAmount ?? null;
  const veto =
    scenarioPrice !== null &&
    scenarioPrice > 0 &&
    scenarioPrice > persona.approximateBudget &&
    persona.priceSensitivity >= 0.7;
  if (veto) {
    decision = 'Reject';
    keyObjection = 'Price exceeds budget under high price sensitivity';
  } else {
    if (baselineDecision.decision === 'Convert' && decision === 'Convert') {
      const baselineCovered = coveredNeeds(
        persona,
        baselineInput.features ?? [],
        baselineInput.valueProposition ?? '',
      );
      const scenarioCovered = coveredNeeds(
        persona,
        scenario.activeFeatures ?? [],
        baselineInput.valueProposition ?? '',
      );
      if (baselineCovered.length > 0 && scenarioCovered.length === 0) {
        const first =
          baselineCovered.find(need => !scenarioCovered.includes(need)) ??
          baselineCovered[0];
        decision = 'Confused';
        keyObjection = `Missing critical required feature: ${first}`;
      } else if (
        setupDifficulty === 'complex' &&
        persona.expectedImplementationEffort === 'Low'
      ) {
        decision = 'Confused';
        keyObjection = 'High setup overhead exceeds team capacity';
      }
    }
    if (
      baselineDecision.decision === 'Confused' &&
      decision === 'Confused' &&
      trustSignals.freeTrialAvailable &&
      (persona.role === 'Cost-sensitive freelancer' ||
        persona.role === 'Small agency owner') &&
      (baselineDecision.keyObjection === 'Price sensitivity' ||
        baselineDecision.keyObjection === 'Missing trust signals')
    ) {
      decision = 'Convert';
      keyObjection = null;
    }
  }
  const perceivedValue = round2(0.6 * f.fit + 0.4 * f.score);
  const willingnessToPay = clamp(
    Math.round(
      persona.approximateBudget *
        perceivedValue *
        (1.0 - 0.5 * persona.priceSensitivity),
    ),
    0,
    persona.approximateBudget,
  );
  return {
    personaId: persona.id,
    decision,
    reason: scenarioReason(f),
    keyObjection,
    perceivedValue,
    willingnessToPay,
    score: round2(f.score),
  };
}

export function evaluateScenarioPopulation(
  personas: SwarmtankPersona[],
  baselineInput: SwarmtankInput,
  baselineDecisions: SwarmtankDecision[],
  scenario: SwarmtankScenario,
): SwarmtankDecision[] {
  const out: SwarmtankDecision[] = [];
  personas.forEach((persona, index) => {
    const baseline = baselineDecisions[index];
    if (baseline && baseline.personaId === persona.id) {
      out.push(
        evaluateScenarioPersona(persona, baselineInput, baseline, scenario),
      );
    }
  });
  return out;
}

function scenarioMetrics(decisions: SwarmtankDecision[]): ScenarioMetrics {
  const total = decisions.length;
  const converts = decisions.filter(d => d.decision === 'Convert').length;
  const confused = decisions.filter(d => d.decision === 'Confused').length;
  const rejects = decisions.filter(d => d.decision === 'Reject').length;
  return {
    total,
    converts,
    confused,
    rejects,
    conversionRate:
      total > 0 ? round2((100 * converts) / total) : 0,
    averageWillingnessToPay:
      total > 0
        ? round2(
            decisions.reduce((sum, d) => sum + d.willingnessToPay, 0) / total,
          )
        : 0,
    averagePerceivedValue:
      total > 0
        ? round2(
            decisions.reduce((sum, d) => sum + d.perceivedValue, 0) / total,
          )
        : 0,
  };
}

export function computeScenarioComparison(
  baselineDecisions: SwarmtankDecision[],
  scenarioDecisions: SwarmtankDecision[],
  baselinePriceInput: number | null | undefined,
  scenarioPriceInput: number | null | undefined,
): ScenarioComparison {
  const baselinePrice = baselinePriceInput ?? null;
  const scenarioPrice = scenarioPriceInput ?? null;
  const baseline = scenarioMetrics(baselineDecisions);
  const scenario = scenarioMetrics(scenarioDecisions);
  const counts: ScenarioTransition[] = [];
  const convertedIds: string[] = [];
  const lostIds: string[] = [];
  let unchangedCount = 0;
  baselineDecisions.forEach((base, index) => {
    const scen = scenarioDecisions[index];
    if (!scen) return;
    const entry = counts.find(e => e.from === base.decision && e.to === scen.decision);
    if (entry) {
      entry.count += 1;
    } else {
      counts.push({ from: base.decision, to: scen.decision, count: 1 });
    }
    const rankDiff = decisionRank(scen.decision) - decisionRank(base.decision);
    if (rankDiff > 0) convertedIds.push(scen.personaId);
    else if (rankDiff < 0) lostIds.push(scen.personaId);
    else unchangedCount += 1;
  });
  counts.sort(
    (a, b) =>
      decisionRank(b.from) - decisionRank(a.from) ||
      decisionRank(b.to) - decisionRank(a.to),
  );
  const countObjections = (
    decisions: SwarmtankDecision[],
  ): Array<[string, number]> => {
    const out: Array<[string, number]> = [];
    for (const d of decisions) {
      if (!d.keyObjection) continue;
      const entry = out.find(([item]) => item === d.keyObjection);
      if (entry) entry[1] += 1;
      else out.push([d.keyObjection, 1]);
    }
    return out;
  };
  const baselineCounts = countObjections(baselineDecisions);
  const scenarioCounts = countObjections(scenarioDecisions);
  const resolved: ScenarioObjectionDelta[] = [];
  for (const [item, baseCount] of baselineCounts) {
    const scenCount =
      scenarioCounts.find(([i]) => i === item)?.[1] ?? 0;
    if (scenCount < baseCount) {
      resolved.push({ item, baselineCount: baseCount, scenarioCount: scenCount });
    }
  }
  const introduced: ScenarioObjectionDelta[] = [];
  for (const [item, scenCount] of scenarioCounts) {
    const baseCount =
      baselineCounts.find(([i]) => i === item)?.[1] ?? 0;
    if (scenCount > baseCount) {
      introduced.push({ item, baselineCount: baseCount, scenarioCount: scenCount });
    }
  }
  resolved.sort(
    (a, b) =>
      b.baselineCount - b.scenarioCount - (a.baselineCount - a.scenarioCount) ||
      (a.item < b.item ? -1 : a.item > b.item ? 1 : 0),
  );
  introduced.sort(
    (a, b) =>
      b.scenarioCount - b.baselineCount - (a.scenarioCount - a.baselineCount) ||
      (a.item < b.item ? -1 : a.item > b.item ? 1 : 0),
  );
  let priceElasticity: number | null = null;
  if (
    baselinePrice !== null &&
    scenarioPrice !== null &&
    baselinePrice > 0 &&
    baseline.conversionRate > 0 &&
    Math.abs(scenarioPrice - baselinePrice) >= Number.EPSILON
  ) {
    priceElasticity = round2(
      ((scenario.conversionRate - baseline.conversionRate) /
        baseline.conversionRate) /
        ((scenarioPrice - baselinePrice) / baselinePrice),
    );
  }
  return {
    transitions: counts,
    convertedIds,
    lostIds,
    unchangedCount,
    resolvedObjections: resolved,
    introducedObjections: introduced,
    baseline,
    scenario,
    priceElasticity,
  };
}

export function defaultScenarioFor(run: {
  input: SwarmtankInput;
}): SwarmtankScenario {
  return {
    priceMonthly: null,
    activeFeatures: [...(run.input.features ?? [])],
    trustSignals: {
      soc2Security: false,
      freeTrialAvailable: false,
      slaGuarantee: false,
      moneyBackGuarantee: false,
    },
    setupDifficulty: 'moderate',
  };
}

/** Stable key for the arena's scenario effect. */
export function scenarioSignature(scenario: SwarmtankScenario): string {
  const trust = scenario.trustSignals ?? {
    soc2Security: false,
    freeTrialAvailable: false,
    slaGuarantee: false,
    moneyBackGuarantee: false,
  };
  return JSON.stringify([
    scenario.priceMonthly ?? null,
    scenario.activeFeatures ?? [],
    trust.soc2Security,
    trust.freeTrialAvailable,
    trust.slaGuarantee,
    trust.moneyBackGuarantee,
    scenario.setupDifficulty ?? 'moderate',
  ]);
}

export function scenariosEqual(
  a: SwarmtankScenario,
  b: SwarmtankScenario,
): boolean {
  return scenarioSignature(a) === scenarioSignature(b);
}
