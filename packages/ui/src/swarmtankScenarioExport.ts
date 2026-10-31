/**
 * M9 SwarmTank Slice 3: deterministic comparison
 * export. Both outputs are pure functions of the
 * baseline run, the scenario, and the computed
 * comparison — fixed key order, fixed rounding,
 * personas in stored order — so the same inputs
 * always produce the same bytes. No backend call.
 */

import type {
  SwarmtankDecision,
  SwarmtankPersona,
  SwarmtankRunDoc,
  SwarmtankScenario,
} from '@anansi/contracts';
import type { ScenarioComparison } from './swarmtankSensitivity';
import { fmt2 } from './swarmtankSensitivity';

export const SCENARIO_EXPORT_DISCLAIMER =
  'Synthetic scenario output: simulated personas, not real customers. Not a substitute for real interviews, conversion data, or market research.';

type ExportInput = {
  run: SwarmtankRunDoc;
  runArtifactId: string;
  runVersion: number;
  runContentHash: string;
  scenario: SwarmtankScenario;
  scenarioDecisions: SwarmtankDecision[];
  branchArtifactId: string | null;
  branchVersion: number | null;
  comparison: ScenarioComparison;
};

function signed(value: number): string {
  return value > 0 ? `+${fmt2(value)}` : fmt2(value);
}

function priceText(price: number | null | undefined): string {
  return price === null || price === undefined ? 'unstated' : `$${price}`;
}

function perPersonaRows(input: ExportInput) {
  const scenarioById = new Map(
    input.scenarioDecisions.map(d => [d.personaId, d]),
  );
  return input.run.personas.map(persona => {
    const baseline = input.run.decisions.find(d => d.personaId === persona.id);
    const scenario = scenarioById.get(persona.id);
    return {
      personaId: persona.id,
      name: persona.name,
      role: persona.role,
      baselineDecision: baseline?.decision ?? '—',
      scenarioDecision: scenario?.decision ?? '—',
      baselineObjection: baseline?.keyObjection ?? null,
      scenarioObjection: scenario?.keyObjection ?? null,
    };
  });
}

export function scenarioComparisonJson(input: ExportInput): string {
  const effectivePrice =
    input.scenario.priceMonthly ?? input.run.input.priceAmount;
  const doc = {
    disclaimer: SCENARIO_EXPORT_DISCLAIMER,
    baseline: {
      artifactId: input.runArtifactId,
      version: input.runVersion,
      contentHash: input.runContentHash,
      priceMonthly: input.run.input.priceAmount,
      features: input.run.input.features,
      metrics: input.comparison.baseline,
    },
    scenario: {
      branchArtifactId: input.branchArtifactId,
      branchVersion: input.branchVersion,
      priceMonthly: input.scenario.priceMonthly,
      effectivePriceMonthly: effectivePrice,
      activeFeatures: input.scenario.activeFeatures,
      trustSignals: input.scenario.trustSignals,
      setupDifficulty: input.scenario.setupDifficulty,
      metrics: input.comparison.scenario,
      priceElasticity: input.comparison.priceElasticity,
    },
    transitions: input.comparison.transitions,
    convertedIds: input.comparison.convertedIds,
    lostIds: input.comparison.lostIds,
    unchangedCount: input.comparison.unchangedCount,
    resolvedObjections: input.comparison.resolvedObjections,
    introducedObjections: input.comparison.introducedObjections,
    personas: perPersonaRows(input),
  };
  return `${JSON.stringify(doc, null, 2)}\n`;
}

export function scenarioComparisonMarkdown(input: ExportInput): string {
  const { baseline, scenario } = input.comparison;
  const effectivePrice =
    input.scenario.priceMonthly ?? input.run.input.priceAmount;
  const lines: string[] = [];
  lines.push('# SwarmTank scenario comparison');
  lines.push('');
  lines.push(SCENARIO_EXPORT_DISCLAIMER);
  lines.push('');
  lines.push(
    `Baseline run: ${input.runArtifactId} version ${input.runVersion}; ` +
      `price ${priceText(input.run.input.priceAmount)}.`,
  );
  if (input.branchArtifactId !== null) {
    lines.push(
      `Saved branch: ${input.branchArtifactId} version ${input.branchVersion}.`,
    );
  }
  lines.push('');
  lines.push('## Metrics');
  lines.push('');
  lines.push(
    `- Conversion rate: ${fmt2(baseline.conversionRate)}% baseline -> ` +
      `${fmt2(scenario.conversionRate)}% scenario (${signed(
        scenario.conversionRate - baseline.conversionRate,
      )} points).`,
  );
  lines.push(
    `- Average willingness to pay: $${fmt2(baseline.averageWillingnessToPay)} ` +
      `-> $${fmt2(scenario.averageWillingnessToPay)} (${signed(
        scenario.averageWillingnessToPay - baseline.averageWillingnessToPay,
      )}).`,
  );
  lines.push(
    `- Average perceived value: ${fmt2(baseline.averagePerceivedValue)} -> ` +
      `${fmt2(scenario.averagePerceivedValue)} (${signed(
        scenario.averagePerceivedValue - baseline.averagePerceivedValue,
      )}).`,
  );
  lines.push(
    `- Price elasticity: ${
      input.comparison.priceElasticity === null
        ? 'not defined for this price change'
        : fmt2(input.comparison.priceElasticity)
    }.`,
  );
  lines.push(
    `- Scenario price: ${priceText(effectivePrice)}; active features: ` +
      `${input.scenario.activeFeatures.length} of ${(input.run.input.features ?? []).length}.`,
  );
  lines.push('');
  lines.push('## Persona migration');
  lines.push('');
  lines.push(
    `- Converted: ${input.comparison.convertedIds.length} ` +
      `(${input.comparison.convertedIds.join(', ') || 'none'}).`,
  );
  lines.push(
    `- Lost: ${input.comparison.lostIds.length} ` +
      `(${input.comparison.lostIds.join(', ') || 'none'}).`,
  );
  lines.push(`- Unchanged: ${input.comparison.unchangedCount}.`);
  lines.push('');
  lines.push('## Objections');
  lines.push('');
  lines.push(
    `- Resolved: ${
      input.comparison.resolvedObjections.length
        ? input.comparison.resolvedObjections
            .map(d => `${d.item} (${d.baselineCount} -> ${d.scenarioCount})`)
            .join('; ')
        : 'none'
    }.`,
  );
  lines.push(
    `- Introduced: ${
      input.comparison.introducedObjections.length
        ? input.comparison.introducedObjections
            .map(d => `${d.item} (${d.baselineCount} -> ${d.scenarioCount})`)
            .join('; ')
        : 'none'
    }.`,
  );
  lines.push('');
  lines.push('## Per-persona decisions');
  lines.push('');
  lines.push('| Persona | Baseline | Scenario |');
  lines.push('| --- | --- | --- |');
  for (const row of perPersonaRows(input)) {
    lines.push(
      `| ${row.name} (${row.personaId}) | ${row.baselineDecision} | ${row.scenarioDecision} |`,
    );
  }
  lines.push('');
  return lines.join('\n');
}

export type { SwarmtankDecision, SwarmtankPersona };
