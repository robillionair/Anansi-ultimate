/**
 * M9 SwarmTank Slice 3: interactive scenario
 * controls. Every change recomputes locally in
 * the panel via swarmtankSensitivity — no network,
 * no model call. The re-run button deliberately
 * reuses the existing deterministic evaluate op
 * with the scenario inputs; it never claims a
 * model-generated simulation.
 */

import type { SwarmtankRunDoc, SwarmtankScenario } from '@anansi/contracts';
import { useState } from 'react';

type TrustKey = keyof SwarmtankScenario['trustSignals'];

const TRUST_TOGGLES: Array<{ key: TrustKey; label: string }> = [
  { key: 'soc2Security', label: 'SOC 2 compliance' },
  { key: 'freeTrialAvailable', label: 'Free trial available' },
  { key: 'slaGuarantee', label: 'SLA guarantee' },
  { key: 'moneyBackGuarantee', label: 'Money-back guarantee' },
];

type Props = {
  run: SwarmtankRunDoc;
  scenario: SwarmtankScenario;
  onScenarioChange: (next: SwarmtankScenario) => void;
  onReset: () => void;
  onRerun: () => void;
  rerunDisabled: boolean;
};

export default function SwarmtankScenarioControls({
  run,
  scenario,
  onScenarioChange,
  onReset,
  onRerun,
  rerunDisabled,
}: Props) {
  const baselinePrice = run.input.priceAmount ?? null;
  const effectivePrice =
    scenario.priceMonthly ?? baselinePrice ?? 0;
  const sliderMax =
    baselinePrice !== null
      ? Math.ceil(baselinePrice * 10)
      : Math.max(
          100,
          ...run.personas.map(p => p.approximateBudget),
        );
  const [priceText, setPriceText] = useState<string | null>(null);
  const displayPrice = priceText ?? String(effectivePrice);

  const setPrice = (value: number) => {
    if (!Number.isFinite(value) || value < 0) return;
    const capped =
      baselinePrice !== null
        ? Math.min(value, baselinePrice * 10)
        : value;
    onScenarioChange({ ...scenario, priceMonthly: capped });
  };

  const presets: Array<{ label: string; value: number | null }> = [
    { label: 'Free Tier ($0)', value: 0 },
  ];
  if (baselinePrice !== null) {
    presets.push(
      { label: '-50%', value: Math.round(baselinePrice * 0.5 * 100) / 100 },
      { label: 'Baseline', value: baselinePrice },
      { label: '+50%', value: Math.round(baselinePrice * 1.5 * 100) / 100 },
      { label: '+100%', value: baselinePrice * 2 },
    );
  }

  return (
    <section aria-label="SwarmTank scenario controls">
      <h3>Scenario controls</h3>
      <p>
        ⚡ Instant Synthetic Sensitivity Calculation — these controls
        recompute every persona locally with the deterministic
        sensitivity rules. No model call, no network.
      </p>
      <div>
        <label htmlFor="swarmtank-scenario-price">
          Monthly price (scenario)
        </label>{' '}
        <input
          id="swarmtank-scenario-price"
          type="range"
          min={0}
          max={sliderMax}
          step={1}
          value={effectivePrice}
          onChange={event => {
            setPriceText(null);
            setPrice(Number(event.target.value));
          }}
        />{' '}
        <input
          aria-label="Scenario monthly price (exact value)"
          type="number"
          min={0}
          value={displayPrice}
          onChange={event => {
            setPriceText(event.target.value);
            const parsed = Number(event.target.value);
            if (event.target.value !== '' && Number.isFinite(parsed)) {
              setPrice(parsed);
            }
          }}
        />
      </div>
      <div>
        {presets.map(preset => (
          <button
            key={preset.label}
            type="button"
            onClick={() =>
              preset.value !== null && setPrice(preset.value)
            }
          >
            {preset.label}
          </button>
        ))}
      </div>
      <fieldset>
        <legend>Active features</legend>
        {(run.input.features ?? []).length === 0 ? (
          <p>No baseline features to toggle.</p>
        ) : (
          (run.input.features ?? []).map(feature => (
            <label key={feature}>
              <input
                type="checkbox"
                checked={scenario.activeFeatures.includes(feature)}
                onChange={event => {
                  const next = event.target.checked
                    ? [...scenario.activeFeatures, feature]
                    : scenario.activeFeatures.filter(f => f !== feature);
                  onScenarioChange({ ...scenario, activeFeatures: next });
                }}
              />{' '}
              {feature}
            </label>
          ))
        )}
      </fieldset>
      <fieldset>
        <legend>Trust signals</legend>
        {TRUST_TOGGLES.map(toggle => (
          <label key={toggle.key}>
            <input
              type="checkbox"
              checked={scenario.trustSignals[toggle.key]}
              onChange={event =>
                onScenarioChange({
                  ...scenario,
                  trustSignals: {
                    ...scenario.trustSignals,
                    [toggle.key]: event.target.checked,
                  },
                })
              }
            />{' '}
            {toggle.label}
          </label>
        ))}
      </fieldset>
      <fieldset>
        <legend>Setup difficulty</legend>
        {(['effortless', 'moderate', 'complex'] as const).map(option => (
          <label key={option}>
            <input
              type="radio"
              name="swarmtank-setup-difficulty"
              value={option}
              checked={scenario.setupDifficulty === option}
              onChange={() =>
                onScenarioChange({ ...scenario, setupDifficulty: option })
              }
            />{' '}
            {option}
          </label>
        ))}
      </fieldset>
      <div>
        <button type="button" onClick={onReset}>
          Reset to Baseline
        </button>{' '}
        <button type="button" onClick={onRerun} disabled={rerunDisabled}>
          🤖 Re-run Full AI Simulation
        </button>
      </div>
      <p>
        Re-run re-evaluates the scenario inputs through the same
        deterministic evaluate operation as the baseline run, with
        the same seed. It makes no model call.
      </p>
    </section>
  );
}
