/**
 * M9 SwarmTank Slice 3: side-by-side comparison
 * studio. Baseline vs scenario metric cards, the
 * persona migration breakdown, resolved and
 * introduced objections, and the deterministic
 * JSON / Markdown export buttons.
 */

import type { SwarmtankPersona, SwarmtankScenario } from '@anansi/contracts';
import type { ScenarioComparison } from './swarmtankSensitivity';
import { fmt2 } from './swarmtankSensitivity';
import { SCENARIO_EXPORT_DISCLAIMER } from './swarmtankScenarioExport';

type Props = {
  comparison: ScenarioComparison;
  personas: SwarmtankPersona[];
  scenario: SwarmtankScenario;
  branchSaved: boolean;
  onSaveBranch: () => void;
  onExportJson: () => void;
  onExportMarkdown: () => void;
  saveDisabled: boolean;
};

function signedDelta(delta: number, suffix: string): string {
  return delta > 0 ? `+${fmt2(delta)}${suffix}` : `${fmt2(delta)}${suffix}`;
}

export default function SwarmtankComparison({
  comparison,
  personas,
  scenario,
  branchSaved,
  onSaveBranch,
  onExportJson,
  onExportMarkdown,
  saveDisabled,
}: Props) {
  const { baseline, scenario: scen } = comparison;
  const names = (ids: string[]) =>
    ids.length > 0
      ? ids.map(id => personas.find(p => p.id === id)?.name ?? id).join(', ')
      : 'none';
  return (
    <section aria-label="SwarmTank scenario comparison">
      <h3>Scenario comparison</h3>
      <p>{SCENARIO_EXPORT_DISCLAIMER}</p>
      <div>
        <p>
          Conversion rate: {fmt2(baseline.conversionRate)}% baseline →{' '}
          {fmt2(scen.conversionRate)}% scenario (
          {signedDelta(scen.conversionRate - baseline.conversionRate, ' points')}
          ).
        </p>
        <p>
          Average willingness to pay: ${fmt2(baseline.averageWillingnessToPay)}{' '}
          → ${fmt2(scen.averageWillingnessToPay)} (
          {signedDelta(
            scen.averageWillingnessToPay - baseline.averageWillingnessToPay,
            '',
          )}
          ).
        </p>
        <p>
          Average perceived value: {fmt2(baseline.averagePerceivedValue)} →{' '}
          {fmt2(scen.averagePerceivedValue)} (
          {signedDelta(
            scen.averagePerceivedValue - baseline.averagePerceivedValue,
            '',
          )}
          ).
        </p>
        <p>
          Price elasticity:{' '}
          {comparison.priceElasticity === null
            ? 'not defined for this price change'
            : fmt2(comparison.priceElasticity)}
          .
        </p>
        <p>
          Scenario price:{' '}
          {scenario.priceMonthly === null
            ? 'baseline price'
            : `$${scenario.priceMonthly}`}
          ; active features: {scenario.activeFeatures.length}; setup:{' '}
          {scenario.setupDifficulty}.
        </p>
      </div>
      <div>
        <h4>Persona migration</h4>
        <p>
          Converted ({comparison.convertedIds.length}):{' '}
          {names(comparison.convertedIds)}.
        </p>
        <p>
          Lost ({comparison.lostIds.length}): {names(comparison.lostIds)}.
        </p>
        <p>Unchanged: {comparison.unchangedCount}.</p>
        <ul>
          {comparison.transitions.map(transition => (
            <li key={`${transition.from}-${transition.to}`}>
              {transition.from} → {transition.to}: {transition.count}
            </li>
          ))}
        </ul>
      </div>
      <div>
        <h4>Objections</h4>
        <p>
          Resolved:{' '}
          {comparison.resolvedObjections.length > 0
            ? comparison.resolvedObjections
                .map(
                  d =>
                    `${d.item} (${d.baselineCount} → ${d.scenarioCount})`,
                )
                .join('; ')
            : 'none'}
          .
        </p>
        <p>
          Introduced:{' '}
          {comparison.introducedObjections.length > 0
            ? comparison.introducedObjections
                .map(
                  d =>
                    `${d.item} (${d.baselineCount} → ${d.scenarioCount})`,
                )
                .join('; ')
            : 'none'}
          .
        </p>
      </div>
      <div>
        <button type="button" onClick={onSaveBranch} disabled={saveDisabled}>
          {branchSaved ? 'Branch saved' : 'Save Scenario as Branch'}
        </button>{' '}
        <button type="button" onClick={onExportJson}>
          Export Comparison (JSON)
        </button>{' '}
        <button type="button" onClick={onExportMarkdown}>
          Export Comparison (Markdown)
        </button>
      </div>
      <p>
        Scenario decisions are computed locally from the baseline
        run; saving a branch stores them as a versioned simulation
        artifact without changing the baseline run.
      </p>
    </section>
  );
}
