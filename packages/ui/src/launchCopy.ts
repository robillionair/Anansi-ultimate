// M12 Slice 2: deterministic Launch Studio copy generator.
//
// Pure functions over parsed project evidence. No dates, no clocks, no
// randomness: the same selection always yields the same channel texts.
// Numbers appear only when they come from the selected evidence, and are
// labeled simulated where they are simulated. Nothing here invents
// testimonials, reviews, sales, or revenue (§61).

export interface LaunchSimReport {
  total: number;
  converts: number;
  confused: number;
  rejects: number;
  conversionRate: number;
  averageWillingnessToPay: number;
  topObjections: Array<{ item: string; count: number }>;
  topConfusionPoints: Array<{ item: string; count: number }>;
  objectionBreakdown: Array<{ item: string; count: number }>;
  segments: Array<{ item: string; count: number }>;
}

export interface LaunchSimInput {
  description: string;
  pricing?: string;
  priceAmount?: number | null;
  segment?: string;
  industry?: string;
  features?: string[];
  valueProposition?: string;
}

export interface LaunchSomniaScenario {
  id: string;
  instruction: string;
}

export interface LaunchSomniaProposal {
  id: string;
  action: string;
  target: string;
  rationale: string;
  proposedChange: string;
  evidence: string[];
}

export interface LaunchSomnia {
  scenarios: LaunchSomniaScenario[];
  results: Array<{ scenarioId: string; decision: string }>;
  scenariosTotal: number;
  completed: number;
  refused: number;
  aborted: number;
  simulatedToolCalls: number;
  violationsByCategory: Array<{ category: string; count: number }>;
  proposals: LaunchSomniaProposal[];
}

export interface LaunchEvidence {
  conceptName: string;
  conceptDescription: string;
  productizationBrief: string;
  specTitle?: string;
  specSummary?: string;
  specPurpose?: string;
  specIntendedUsers?: string;
  specFeatures?: string[];
  specPages?: string[];
  specDataAndState?: string;
  simInput?: LaunchSimInput;
  simReport?: LaunchSimReport;
  somnia?: LaunchSomnia;
}

export interface LaunchChannels {
  showHn: string;
  productHunt: string;
  xThread: string[];
  releaseNotes: string;
}

/** Gate: what is missing before generation is allowed. Empty means go. */
export function launchGate(
  hasConcept: boolean,
  hasSimulation: boolean,
  hasSomnia: boolean,
): string[] {
  const missing: string[] = [];
  if (!hasConcept) missing.push('a Forge concept artifact');
  if (!hasSimulation && !hasSomnia) {
    missing.push('a SwarmTank simulation or SOMNIA evaluation artifact');
  }
  return missing;
}

function countBy(items: string[]): Array<{ item: string; count: number }> {
  const counts = new Map<string, number>();
  for (const item of items) counts.set(item, (counts.get(item) ?? 0) + 1);
  return [...counts.entries()]
    .map(([item, count]) => ({ item, count }))
    .sort((a, b) => b.count - a.count || (a.item < b.item ? -1 : 1));
}

/**
 * Report derivation from a parsed SwarmTank run, mirroring the counting in
 * `deriveLocalReport` (Swarmtank.tsx), which is not exported. Decisions and
 * personas are the run's own records; nothing is estimated here.
 */
export function deriveSimReport(
  decisions: Array<{
    decision: string;
    keyObjection?: string | null;
    willingnessToPay: number;
  }>,
  personas: Array<{ role: string }>,
): LaunchSimReport {
  const total = decisions.length;
  const converts = decisions.filter(d => d.decision === 'Convert').length;
  const confused = decisions.filter(d => d.decision === 'Confused').length;
  const rejects = decisions.filter(d => d.decision === 'Reject').length;
  const objections = decisions
    .map(d => d.keyObjection)
    .filter((item): item is string => item != null);
  const confusionPoints = decisions
    .filter(d => d.decision === 'Confused')
    .map(d => d.keyObjection)
    .filter((item): item is string => item != null);
  return {
    total,
    converts,
    confused,
    rejects,
    conversionRate:
      total > 0 ? Math.round((100 * converts) / total * 100) / 100 : 0,
    averageWillingnessToPay:
      total > 0
        ? Math.round(
            (decisions.reduce((sum, d) => sum + d.willingnessToPay, 0) / total) *
              100,
          ) / 100
        : 0,
    topObjections: countBy(objections).slice(0, 5),
    topConfusionPoints: countBy(confusionPoints).slice(0, 5),
    objectionBreakdown: countBy(objections),
    segments: countBy(personas.map(p => p.role)),
  };
}

function productName(evidence: LaunchEvidence): string {
  return evidence.specTitle ?? evidence.conceptName;
}

function features(evidence: LaunchEvidence): string[] {
  if (evidence.specFeatures && evidence.specFeatures.length > 0) {
    return evidence.specFeatures;
  }
  return evidence.simInput?.features ?? [];
}

function audience(evidence: LaunchEvidence): string | null {
  return (
    evidence.specIntendedUsers ?? evidence.simInput?.segment?.trim()
  ) || null;
}

/** The local-first guarantee is claimed only when the spec itself says so. */
export function supportsLocalFirst(evidence: LaunchEvidence): boolean {
  const haystack = [
    evidence.specSummary ?? '',
    evidence.specPurpose ?? '',
    ...(evidence.specFeatures ?? []),
    evidence.specDataAndState ?? '',
  ].join('\n');
  return /local|offline|on-device|no server/i.test(haystack);
}

/** Truncate an inserted fragment to fit a budget, with an ellipsis. */
export function fit(text: string, max: number): string {
  if (text.length <= max) return text;
  if (max <= 1) return '…';
  return `${text.slice(0, max - 1)}…`;
}

function showHn(evidence: LaunchEvidence): string {
  const name = productName(evidence);
  const feats = features(evidence);
  const differentiators =
    feats.length > 0
      ? feats.slice(0, 3).map(f => `- ${f}`).join('\n')
      : '- See the feature list in the release notes.';
  const localFirst = supportsLocalFirst(evidence)
    ? '\n- Local-first: runs locally with your data on your device.'
    : '';
  const solution = evidence.specSummary ?? evidence.productizationBrief;
  const aud = audience(evidence);
  return [
    `Show HN: ${name}`,
    '',
    `Problem: ${evidence.conceptDescription}`,
    '',
    `Solution: ${solution}`,
    ...(aud ? ['', `For: ${aud}`] : []),
    '',
    'Technical differentiators:',
    differentiators + localFirst,
  ].join('\n');
}

function productHunt(evidence: LaunchEvidence): string {
  const name = productName(evidence);
  const feats = features(evidence);
  const first = feats[0];
  const tagline = first ? `${name} — ${first}` : name;
  const value =
    evidence.simInput?.valueProposition ?? evidence.specPurpose ?? null;
  const bullets =
    feats.length > 0
      ? feats.map(f => `- ${f}`).join('\n')
      : '- See the release notes for the feature list.';
  return [
    `Tagline: ${tagline}`,
    '',
    evidence.specSummary ?? evidence.productizationBrief,
    ...(value ? ['', `Value proposition: ${value}`] : []),
    '',
    'Features:',
    bullets,
  ].join('\n');
}

function xThread(evidence: LaunchEvidence): string[] {
  const name = productName(evidence);
  const feats = features(evidence);
  const hook = `1/5 ${fit(name, 240)} is here.`;
  const problem = `2/5 The problem: ${fit(evidence.conceptDescription, 240)}`;
  const architecture = supportsLocalFirst(evidence)
    ? '3/5 Architecture: local-first by design. Your data stays on your device; no server is required for core use.'
    : evidence.specPages && evidence.specPages.length > 0
      ? `3/5 Architecture: a dependency-free static prototype with ${fit(evidence.specPages.join(', '), 190)}. Prototype for review; not deployed.`
      : '3/5 Architecture: a dependency-free static prototype. Prototype for review; not executed, not deployed.';
  const featureLine =
    feats.length > 0
      ? `4/5 What it does: ${fit(feats.slice(0, 3).join('; '), 220)}`
      : '4/5 What it does: see the release notes for the full feature list.';
  const sim = evidence.simReport;
  const proof =
    sim && sim.total > 0
      ? ` In simulated evaluation, ${sim.conversionRate}% of personas converted (${sim.converts}/${sim.total}).`
      : '';
  const cta = `5/5 Try the prototype and tell us what breaks.${fit(proof, 200)}`;
  const posts = [hook, problem, architecture, featureLine, cta];
  return posts.map(post => (post.length <= 280 ? post : fit(post, 280)));
}

function releaseNotes(
  evidence: LaunchEvidence,
  included: Array<{ type: string; title: string; version: number }>,
): string {
  const name = productName(evidence);
  const feats = features(evidence);
  const deliverables =
    included.length > 0
      ? included
          .map(item => `- ${item.type}: ${item.title} (version ${item.version})`)
          .join('\n')
      : '- No artifacts selected for this bundle.';
  const capabilities =
    feats.length > 0
      ? feats.map(f => `- ${f}`).join('\n')
      : '- See the specification for the feature list.';
  const lines = [
    `# Release notes: ${name}`,
    '',
    'Status: Prototype for review; not executed, not deployed.',
    '',
    '## Deliverables',
    deliverables,
    '',
    '## Capabilities',
    capabilities,
  ];
  const sim = evidence.simReport;
  if (sim) {
    lines.push(
      '',
      '## Verification evidence (simulated)',
      `SwarmTank simulation over ${sim.total} personas: ${sim.converts} Convert, ${sim.confused} Confused, ${sim.rejects} Reject; conversion rate ${sim.conversionRate}% (simulated).`,
    );
    if (sim.topObjections.length > 0) {
      lines.push(
        'Top objections (simulated):',
        ...sim.topObjections.map(o => `- ${o.item} (${o.count})`),
      );
    }
  }
  const somnia = evidence.somnia;
  if (somnia) {
    lines.push(
      '',
      '## Pressure-test evidence (synthetic replay)',
      `SOMNIA evaluation over ${somnia.scenariosTotal} nightmare scenarios: ${somnia.completed} Complete, ${somnia.refused} Refuse, ${somnia.aborted} Abort; ${somnia.simulatedToolCalls} simulated tool calls.`,
    );
    if (somnia.violationsByCategory.length > 0) {
      lines.push(
        'Violations by category:',
        ...somnia.violationsByCategory.map(
          v => `- ${v.category}: ${v.count}`,
        ),
      );
    }
  }
  return lines.join('\n');
}

/**
 * Generate the four channels. Callers must check `launchGate` first: this
 * function assumes a concept and at least one of simulation / SOMNIA.
 */
export function generateLaunchCopy(
  evidence: LaunchEvidence,
  included: Array<{ type: string; title: string; version: number }>,
): LaunchChannels {
  return {
    showHn: showHn(evidence),
    productHunt: productHunt(evidence),
    xThread: xThread(evidence),
    releaseNotes: releaseNotes(evidence, included),
  };
}

/** Exact prompt shown in the provider-approval panel before any send. */
export function buildLaunchProviderPrompt(evidence: LaunchEvidence): string {
  const name = productName(evidence);
  const feats = features(evidence);
  const sections = [
    'Write launch marketing copy for the product below, in exactly four sections with these headings: "Show HN", "Product Hunt", "X thread (5 posts)", "Release notes".',
    'Use only the facts given. Do not invent testimonials, reviews, sales, revenue, or metrics. Label any simulation number as simulated.',
    '',
    `Product: ${name}`,
    `Problem: ${evidence.conceptDescription}`,
    `Solution: ${evidence.specSummary ?? evidence.productizationBrief}`,
  ];
  if (feats.length > 0) sections.push(`Features: ${feats.join('; ')}`);
  const aud = audience(evidence);
  if (aud) sections.push(`Audience: ${aud}`);
  const sim = evidence.simReport;
  if (sim) {
    sections.push(
      `Simulated evaluation: ${sim.total} personas, ${sim.converts} Convert, ${sim.confused} Confused, ${sim.rejects} Reject, conversion rate ${sim.conversionRate}%.`,
    );
    if (sim.topObjections.length > 0) {
      sections.push(
        `Top simulated objections: ${sim.topObjections.map(o => `${o.item} (${o.count})`).join('; ')}`,
      );
    }
  }
  const somnia = evidence.somnia;
  if (somnia) {
    sections.push(
      `Synthetic pressure test: ${somnia.scenariosTotal} scenarios, ${somnia.completed} Complete, ${somnia.refused} Refuse, ${somnia.aborted} Abort.`,
    );
  }
  sections.push(
    '',
    'Status line to include in the release notes: "Prototype for review; not executed, not deployed."',
  );
  return sections.join('\n');
}

/** SwarmTank summary + objection matrix for the bundle. Null without a run. */
export function buildSwarmtankSummary(evidence: LaunchEvidence): string | null {
  const sim = evidence.simReport;
  if (!sim) return null;
  const lines = [
    '# SwarmTank simulation summary',
    '',
    `Simulated personas: ${sim.total}. Convert: ${sim.converts}. Confused: ${sim.confused}. Reject: ${sim.rejects}.`,
    `Conversion rate (simulated): ${sim.conversionRate}%. Average willingness to pay (simulated): ${sim.averageWillingnessToPay}.`,
    '',
    '## Objection matrix (all objections, simulated)',
  ];
  if (sim.objectionBreakdown.length > 0) {
    lines.push(
      ...sim.objectionBreakdown.map(o => `- ${o.item}: ${o.count}`),
    );
  } else {
    lines.push('- No objections recorded.');
  }
  lines.push('', '## Confusion points (simulated)');
  if (sim.topConfusionPoints.length > 0) {
    lines.push(
      ...sim.topConfusionPoints.map(o => `- ${o.item}: ${o.count}`),
    );
  } else {
    lines.push('- No confusion points recorded.');
  }
  lines.push('', '## Segments (simulated)');
  if (sim.segments.length > 0) {
    lines.push(...sim.segments.map(s => `- ${s.item}: ${s.count}`));
  } else {
    lines.push('- No segment breakdown recorded.');
  }
  return lines.join('\n');
}

/** SOMNIA pressure-test notes for the bundle. Null without an evaluation. */
export function buildSomniaNotes(evidence: LaunchEvidence): string | null {
  const somnia = evidence.somnia;
  if (!somnia) return null;
  const lines = [
    '# SOMNIA pressure-test notes',
    '',
    'Synthetic replay — not historical events. These notes describe what the',
    'pressure test observed; they are not evidence that anything was fixed.',
    '',
    `Scenarios: ${somnia.scenariosTotal}. Complete: ${somnia.completed}. Refuse: ${somnia.refused}. Abort: ${somnia.aborted}. Simulated tool calls: ${somnia.simulatedToolCalls}.`,
    '',
    '## Scenarios and decisions',
  ];
  const byId = new Map(somnia.results.map(r => [r.scenarioId, r.decision]));
  for (const scenario of somnia.scenarios) {
    lines.push(
      `- ${scenario.id}: ${byId.get(scenario.id) ?? 'no result recorded'}`,
    );
  }
  if (somnia.violationsByCategory.length > 0) {
    lines.push(
      '',
      '## Violations by category',
      ...somnia.violationsByCategory.map(v => `- ${v.category}: ${v.count}`),
    );
  }
  lines.push('', '## Review-only proposals (not applied)');
  if (somnia.proposals.length > 0) {
    for (const proposal of somnia.proposals) {
      lines.push(
        `- ${proposal.id}: ${proposal.action} ${proposal.target} — ${proposal.rationale}`,
      );
    }
  } else {
    lines.push('- No consolidation proposals were derived.');
  }
  return lines.join('\n');
}

/** Estimated tokens, same estimator as the GHOST-PROMPT comparison. */
export function estimateTokens(text: string): number {
  return Math.ceil(Array.from(text).length / 4);
}

/** Provider dispatch seam: `askOnce` is injected so tests use a fake. */
export async function runLaunchProviderDraft(
  askOnce: (prompt: string) => Promise<string>,
  prompt: string,
): Promise<string> {
  return askOnce(prompt);
}
