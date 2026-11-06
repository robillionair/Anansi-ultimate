import { createPortal } from 'react-dom';
// ---------------------------------------------------------------------------
// SwarmTank (M9 Slices 1–3): synthetic personas,
// seeded decisions, report, scenario controls with
// instant local sensitivity, scenario branches, and
// the Forge bridge. This panel makes no provider
// calls on load — or ever, in these slices:
// generation, evaluation, and scenario branching are
// deterministic backend operations over seeded
// archetypes, and scenario recomputation in the
// browser is pure local arithmetic. Every persona is
// labeled simulated.
// ---------------------------------------------------------------------------

import { useState } from 'react';
import type {
  ArtifactRecord,
  Client,
  CountedItem,
  SwarmtankDecision,
  SwarmtankInput,
  SwarmtankPersona,
  SwarmtankReport,
  SwarmtankRunDoc,
  SwarmtankScenario,
  SwarmtankScenarioDoc,
  SwarmtankSource,
} from '@anansi/contracts';
import { SwarmtankArena } from './SwarmtankArena';
import SwarmtankScenarioControls from './SwarmtankScenarioControls';
import SwarmtankComparison from './SwarmtankComparison';
import {
  computeScenarioComparison,
  defaultScenarioFor,
  evaluateScenarioPopulation,
  scenarioSignature,
  scenariosEqual,
  type ScenarioComparison,
} from './swarmtankSensitivity';
import {
  scenarioComparisonJson,
  scenarioComparisonMarkdown,
} from './swarmtankScenarioExport';

// ---------------------------------------------------------------------------
// Strict client-side parsing for saved run bodies
// (mirrors the backend bounds in
// crates/core/src/swarmtank.rs). Hash verification
// stays backend-side: the browser has no sync
// SHA-256, so this parser checks shape and bounds
// only, and an unrenderable body never renders.
// ---------------------------------------------------------------------------

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function exactKeys(value: Record<string, unknown>, keys: string[]): boolean {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  return (
    actual.length === expected.length && actual.every((key, i) => key === expected[i])
  );
}

function textField(value: unknown, max: number, allowEmpty: boolean): string | null {
  if (typeof value !== 'string') return null;
  if (value.trim().length === 0 && !allowEmpty) return null;
  if (value.length > max) return null;
  return value;
}

function textList(value: unknown, max: number, maxChars: number): string[] | null {
  if (!Array.isArray(value) || value.length > max) return null;
  const out: string[] = [];
  for (const item of value) {
    const text = textField(item, maxChars, false);
    if (text === null) return null;
    out.push(text);
  }
  return out;
}

function numberIn(value: unknown, min: number, max: number): number | null {
  if (typeof value !== 'number' || !Number.isFinite(value)) return null;
  if (value < min || value > max) return null;
  return value;
}

function parseInput(value: unknown): SwarmtankInput | null {
  if (!isRecord(value)) return null;
  if (
    !exactKeys(value, [
      'description',
      'pricing',
      'priceAmount',
      'segment',
      'industry',
      'features',
      'valueProposition',
    ])
  ) {
    return null;
  }
  const description = textField(value.description, 8000, false);
  const pricing = textField(value.pricing, 500, true);
  const segment = textField(value.segment, 500, true);
  const industry = textField(value.industry, 500, true);
  const valueProposition = textField(value.valueProposition, 3000, true);
  const features = textList(value.features, 50, 300);
  if (
    description === null ||
    pricing === null ||
    segment === null ||
    industry === null ||
    valueProposition === null ||
    features === null
  ) {
    return null;
  }
  let priceAmount: number | null = null;
  if (value.priceAmount !== null) {
    priceAmount = numberIn(value.priceAmount, 0, Number.MAX_SAFE_INTEGER);
    if (priceAmount === null) return null;
  }
  return {
    description,
    pricing,
    priceAmount,
    segment,
    industry,
    features,
    valueProposition,
  };
}

function parseSource(value: unknown): SwarmtankSource | null {
  if (!isRecord(value)) return null;
  if (!exactKeys(value, ['kind', 'artifactId', 'version', 'contentHash'])) return null;
  if (
    (value.kind !== 'experiment' && value.kind !== 'concept') ||
    typeof value.artifactId !== 'string' ||
    value.artifactId.trim() === '' ||
    typeof value.version !== 'number' ||
    !Number.isInteger(value.version) ||
    value.version < 1 ||
    typeof value.contentHash !== 'string' ||
    !/^[0-9a-f]{64}$/.test(value.contentHash)
  ) {
    return null;
  }
  return {
    kind: value.kind,
    artifactId: value.artifactId,
    version: value.version,
    contentHash: value.contentHash,
  };
}

function parsePersona(value: unknown, index: number): SwarmtankPersona | null {
  if (!isRecord(value)) return null;
  if (
    !exactKeys(value, [
      'id',
      'name',
      'role',
      'companySize',
      'approximateBudget',
      'needs',
      'priorities',
      'skepticism',
      'priceSensitivity',
      'purchaseConstraints',
      'requiredTrustSignals',
      'expectedImplementationEffort',
      'synthetic',
    ])
  ) {
    return null;
  }
  if (value.id !== `persona-${String(index + 1).padStart(3, '0')}`) return null;
  const name = textField(value.name, 120, false);
  const role = textField(value.role, 120, false);
  const companySize = textField(value.companySize, 500, false);
  const budget = numberIn(value.approximateBudget, 0, Number.MAX_SAFE_INTEGER);
  const needs = textList(value.needs, 12, 300);
  const priorities = textList(value.priorities, 12, 300);
  const constraints = textList(value.purchaseConstraints, 12, 300);
  const signals = textList(value.requiredTrustSignals, 12, 300);
  const effort = textField(value.expectedImplementationEffort, 500, false);
  const skepticism = numberIn(value.skepticism, 0, 1);
  const priceSensitivity = numberIn(value.priceSensitivity, 0, 1);
  if (
    name === null ||
    role === null ||
    companySize === null ||
    budget === null ||
    needs === null ||
    priorities === null ||
    constraints === null ||
    signals === null ||
    effort === null ||
    skepticism === null ||
    priceSensitivity === null ||
    value.synthetic !== true
  ) {
    return null;
  }
  return {
    id: value.id as string,
    name,
    role,
    companySize,
    approximateBudget: budget,
    needs,
    priorities,
    skepticism,
    priceSensitivity,
    purchaseConstraints: constraints,
    requiredTrustSignals: signals,
    expectedImplementationEffort: effort,
    synthetic: true,
  };
}

function parseDecision(value: unknown): SwarmtankDecision | null {
  if (!isRecord(value)) return null;
  if (
    !exactKeys(value, [
      'personaId',
      'decision',
      'reason',
      'keyObjection',
      'perceivedValue',
      'willingnessToPay',
      'score',
    ])
  ) {
    return null;
  }
  if (
    typeof value.personaId !== 'string' ||
    (value.decision !== 'Convert' &&
      value.decision !== 'Confused' &&
      value.decision !== 'Reject') ||
    typeof value.reason !== 'string'
  ) {
    return null;
  }
  if (
    value.keyObjection !== null &&
    (typeof value.keyObjection !== 'string' || value.keyObjection.trim() === '')
  ) {
    return null;
  }
  if (value.decision === 'Convert' && value.keyObjection !== null) return null;
  if (value.decision !== 'Convert' && value.keyObjection === null) return null;
  const perceivedValue = numberIn(value.perceivedValue, 0, 1);
  const score = numberIn(value.score, 0, 1);
  const willingnessToPay = numberIn(value.willingnessToPay, 0, Number.MAX_SAFE_INTEGER);
  if (perceivedValue === null || score === null || willingnessToPay === null) return null;
  return {
    personaId: value.personaId,
    decision: value.decision,
    reason: value.reason,
    keyObjection: value.keyObjection as string | null,
    perceivedValue,
    willingnessToPay,
    score,
  };
}

export function parseSwarmtankRunBody(body: string): SwarmtankRunDoc | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(body);
  } catch {
    return null;
  }
  if (!isRecord(parsed)) return null;
  const keys = Object.keys(parsed);
  const required = [
    'format',
    'version',
    'input',
    'seed',
    'preset',
    'personas',
    'decisions',
    'contentHash',
  ];
  if (
    !required.every(key => keys.includes(key)) ||
    !keys.every(key => required.includes(key) || key === 'source')
  ) {
    return null;
  }
  if (parsed.format !== 'anansi-swarmtank-run' || parsed.version !== 1) return null;
  if (
    typeof parsed.seed !== 'string' ||
    parsed.seed.trim() === '' ||
    (parsed.preset !== 20 && parsed.preset !== 50 && parsed.preset !== 100) ||
    typeof parsed.contentHash !== 'string' ||
    !/^[0-9a-f]{64}$/.test(parsed.contentHash)
  ) {
    return null;
  }
  const input = parseInput(parsed.input);
  if (input === null) return null;
  let source: SwarmtankSource | undefined;
  if (parsed.source !== undefined && parsed.source !== null) {
    const parsedSource = parseSource(parsed.source);
    if (parsedSource === null) return null;
    source = parsedSource;
  }
  if (!Array.isArray(parsed.personas) || parsed.personas.length !== parsed.preset) {
    return null;
  }
  if (!Array.isArray(parsed.decisions) || parsed.decisions.length !== parsed.preset) {
    return null;
  }
  const personas: SwarmtankPersona[] = [];
  for (let i = 0; i < parsed.personas.length; i++) {
    const persona = parsePersona(parsed.personas[i], i);
    if (persona === null) return null;
    personas.push(persona);
  }
  const decisions: SwarmtankDecision[] = [];
  for (const raw of parsed.decisions) {
    const decision = parseDecision(raw);
    if (decision === null) return null;
    decisions.push(decision);
  }
  return {
    format: 'anansi-swarmtank-run',
    version: 1,
    input,
    seed: parsed.seed,
    preset: parsed.preset,
    personas,
    decisions,
    source,
    contentHash: parsed.contentHash,
  };
}

// ---------------------------------------------------------------------------
// M9 Slice 3: strict client-side parsing for saved
// scenario branch bodies (mirrors
// crates/core/src/swarmtank.rs). Same caveat as the
// run parser: shape and bounds only, no hash check
// in the browser. A branch body that does not parse
// never renders.
// ---------------------------------------------------------------------------

function parseScenarioTransition(value: unknown): {
  from: string;
  to: string;
  count: number;
} | null {
  if (!isRecord(value)) return null;
  if (!exactKeys(value, ['from', 'to', 'count'])) return null;
  if (
    (value.from !== 'Convert' &&
      value.from !== 'Confused' &&
      value.from !== 'Reject') ||
    (value.to !== 'Convert' &&
      value.to !== 'Confused' &&
      value.to !== 'Reject')
  ) {
    return null;
  }
  const count = numberIn(value.count, 1, Number.MAX_SAFE_INTEGER);
  if (count === null) return null;
  return { from: value.from, to: value.to, count };
}

function parseScenarioMetrics(value: unknown): {
  total: number;
  converts: number;
  confused: number;
  rejects: number;
  conversionRate: number;
  averageWillingnessToPay: number;
  averagePerceivedValue: number;
} | null {
  if (!isRecord(value)) return null;
  if (
    !exactKeys(value, [
      'total',
      'converts',
      'confused',
      'rejects',
      'conversionRate',
      'averageWillingnessToPay',
      'averagePerceivedValue',
    ])
  ) {
    return null;
  }
  const total = numberIn(value.total, 0, Number.MAX_SAFE_INTEGER);
  const converts = numberIn(value.converts, 0, Number.MAX_SAFE_INTEGER);
  const confused = numberIn(value.confused, 0, Number.MAX_SAFE_INTEGER);
  const rejects = numberIn(value.rejects, 0, Number.MAX_SAFE_INTEGER);
  const conversionRate = numberIn(value.conversionRate, 0, 100);
  const averageWillingnessToPay = numberIn(
    value.averageWillingnessToPay,
    0,
    Number.MAX_SAFE_INTEGER,
  );
  const averagePerceivedValue = numberIn(value.averagePerceivedValue, 0, 1);
  if (
    total === null ||
    converts === null ||
    confused === null ||
    rejects === null ||
    conversionRate === null ||
    averageWillingnessToPay === null ||
    averagePerceivedValue === null
  ) {
    return null;
  }
  return {
    total,
    converts,
    confused,
    rejects,
    conversionRate,
    averageWillingnessToPay,
    averagePerceivedValue,
  };
}

function parseScenario(value: unknown): SwarmtankScenario | null {
  if (!isRecord(value)) return null;
  if (
    !exactKeys(value, [
      'priceMonthly',
      'activeFeatures',
      'trustSignals',
      'setupDifficulty',
    ])
  ) {
    return null;
  }
  let priceMonthly: number | null = null;
  if (value.priceMonthly !== null) {
    priceMonthly = numberIn(value.priceMonthly, 0, Number.MAX_SAFE_INTEGER);
    if (priceMonthly === null) return null;
  }
  const activeFeatures = textList(value.activeFeatures, 50, 300);
  if (activeFeatures === null) return null;
  if (!isRecord(value.trustSignals)) return null;
  if (
    !exactKeys(value.trustSignals, [
      'soc2Security',
      'freeTrialAvailable',
      'slaGuarantee',
      'moneyBackGuarantee',
    ])
  ) {
    return null;
  }
  for (const key of Object.keys(value.trustSignals)) {
    if (typeof value.trustSignals[key] !== 'boolean') return null;
  }
  if (
    value.setupDifficulty !== 'effortless' &&
    value.setupDifficulty !== 'moderate' &&
    value.setupDifficulty !== 'complex'
  ) {
    return null;
  }
  return {
    priceMonthly,
    activeFeatures,
    trustSignals: value.trustSignals as SwarmtankScenario['trustSignals'],
    setupDifficulty: value.setupDifficulty,
  };
}

export function parseSwarmtankScenarioDoc(
  body: string,
): SwarmtankScenarioDoc | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(body);
  } catch {
    return null;
  }
  if (!isRecord(parsed)) return null;
  if (
    !exactKeys(parsed, [
      'format',
      'version',
      'parentArtifactId',
      'parentVersion',
      'parentContentHash',
      'personas',
      'baselineInput',
      'baselineDecisions',
      'scenario',
      'decisions',
      'transitions',
      'metrics',
      'contentHash',
    ])
  ) {
    return null;
  }
  if (parsed.format !== 'anansi-swarmtank-scenario' || parsed.version !== 1) {
    return null;
  }
  if (
    typeof parsed.parentArtifactId !== 'string' ||
    parsed.parentArtifactId.trim() === '' ||
    typeof parsed.parentVersion !== 'number' ||
    !Number.isInteger(parsed.parentVersion) ||
    parsed.parentVersion < 1 ||
    typeof parsed.parentContentHash !== 'string' ||
    !/^[0-9a-f]{64}$/.test(parsed.parentContentHash) ||
    typeof parsed.contentHash !== 'string' ||
    !/^[0-9a-f]{64}$/.test(parsed.contentHash)
  ) {
    return null;
  }
  const baselineInput = parseInput(parsed.baselineInput);
  if (baselineInput === null) return null;
  const scenario = parseScenario(parsed.scenario);
  if (scenario === null) return null;
  if (!Array.isArray(parsed.personas) || parsed.personas.length === 0) return null;
  if (
    !Array.isArray(parsed.baselineDecisions) ||
    parsed.baselineDecisions.length !== parsed.personas.length ||
    !Array.isArray(parsed.decisions) ||
    parsed.decisions.length !== parsed.personas.length
  ) {
    return null;
  }
  const personas: SwarmtankPersona[] = [];
  for (let i = 0; i < parsed.personas.length; i++) {
    const persona = parsePersona(parsed.personas[i], i);
    if (persona === null) return null;
    personas.push(persona);
  }
  const baselineDecisions: SwarmtankDecision[] = [];
  for (const raw of parsed.baselineDecisions) {
    const decision = parseDecision(raw);
    if (decision === null) return null;
    baselineDecisions.push(decision);
  }
  const decisions: SwarmtankDecision[] = [];
  for (const raw of parsed.decisions) {
    const decision = parseDecision(raw);
    if (decision === null) return null;
    decisions.push(decision);
  }
  if (!Array.isArray(parsed.transitions)) return null;
  const transitions: Array<{ from: string; to: string; count: number }> = [];
  for (const raw of parsed.transitions) {
    const transition = parseScenarioTransition(raw);
    if (transition === null) return null;
    transitions.push(transition);
  }
  if (!isRecord(parsed.metrics)) return null;
  if (!exactKeys(parsed.metrics, ['baseline', 'scenario', 'priceElasticity'])) {
    return null;
  }
  const baselineMetrics = parseScenarioMetrics(parsed.metrics.baseline);
  const scenarioMetrics = parseScenarioMetrics(parsed.metrics.scenario);
  if (baselineMetrics === null || scenarioMetrics === null) return null;
  let priceElasticity: number | null = null;
  if (parsed.metrics.priceElasticity !== null) {
    priceElasticity = numberIn(
      parsed.metrics.priceElasticity,
      -Number.MAX_SAFE_INTEGER,
      Number.MAX_SAFE_INTEGER,
    );
    if (priceElasticity === null) return null;
  }
  return {
    format: 'anansi-swarmtank-scenario',
    version: 1,
    parentArtifactId: parsed.parentArtifactId,
    parentVersion: parsed.parentVersion,
    parentContentHash: parsed.parentContentHash,
    personas,
    baselineInput,
    baselineDecisions,
    scenario,
    decisions,
    transitions,
    metrics: {
      baseline: baselineMetrics,
      scenario: scenarioMetrics,
      priceElasticity,
    },
    contentHash: parsed.contentHash,
  };
}

/**
 * Display-only report derived from a branch doc's
 * embedded baseline decisions, used when the branch
 * parent run is not in this project's artifact list
 * (for example after a backup/restore remap, where
 * the parent pin is historical provenance). Never
 * sent back to the backend.
 */
function deriveLocalReport(
  personas: SwarmtankPersona[],
  decisions: SwarmtankDecision[],
): SwarmtankReport {
  const total = decisions.length;
  const converts = decisions.filter(d => d.decision === 'Convert').length;
  const confused = decisions.filter(d => d.decision === 'Confused').length;
  const rejects = decisions.filter(d => d.decision === 'Reject').length;
  const countBy = (items: string[]): CountedItem[] => {
    const counts = new Map<string, number>();
    for (const item of items) counts.set(item, (counts.get(item) ?? 0) + 1);
    return [...counts.entries()]
      .map(([item, count]) => ({ item, count }))
      .sort((a, b) => b.count - a.count || (a.item < b.item ? -1 : 1));
  };
  const objections = decisions
    .map(d => d.keyObjection)
    .filter((item): item is string => item !== null);
  const confusionPoints = decisions
    .filter(d => d.decision === 'Confused')
    .map(d => d.keyObjection)
    .filter((item): item is string => item !== null);
  return {
    averageWillingnessToPay:
      total > 0
        ? Math.round(
            (decisions.reduce((sum, d) => sum + d.willingnessToPay, 0) / total) *
              100,
          ) / 100
        : 0,
    confused,
    conversionRate:
      total > 0 ? Math.round((100 * converts) / total * 100) / 100 : 0,
    converts,
    objectionBreakdown: countBy(objections),
    recommendations: [],
    rejects,
    segments: countBy(personas.map(p => p.role)),
    topConfusionPoints: countBy(confusionPoints).slice(0, 5),
    topObjections: countBy(objections).slice(0, 5),
    total,
  };
}

// ---------------------------------------------------------------------------
// Panel: input, import, generate, evaluate, report,
// export, and the Forge bridge. Rendering,
// selecting, or previewing saves nothing; only
// Evaluate registers a run, and only the explicit
// transfer action revises a Forge concept.
// ---------------------------------------------------------------------------

import { parseColliderExperimentBody } from './Collider';
import { parseTransferConceptBody } from './Transfer';

function emptyInput(): SwarmtankInput {
  return {
    description: '',
    pricing: '',
    priceAmount: null,
    segment: '',
    industry: '',
    features: [],
    valueProposition: '',
  };
}

function downloadJson(filename: string, text: string) {
  const blob = new Blob([text], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

// Pure table filtering, unit-tested in
// packages/ui/tests/swarmtank.test.tsx.
export function filterRunPersonas(
  run: SwarmtankRunDoc,
  search: string,
  decisionFilter: string,
): SwarmtankPersona[] {
  const decisionByPersona = new Map(
    run.decisions.map(decision => [decision.personaId, decision]),
  );
  return run.personas.filter(persona => {
    const decision = decisionByPersona.get(persona.id);
    if (decisionFilter !== 'All' && decision?.decision !== decisionFilter) return false;
    if (search.trim() === '') return true;
    const haystack =
      `${persona.name} ${persona.role} ${persona.companySize} ${persona.needs.join(' ')}`.toLowerCase();
    return haystack.includes(search.trim().toLowerCase());
  });
}

// Presentational report view: metrics, §40
// recommendation fields, and the persona table.
// Plain text only, no HTML rendering of content.
export function SwarmtankReportView({
  report,
  run,
  runKey,
  selectedPersonaId,
  onSelectPersona,
  search,
  decisionFilter,
  onSearchChange,
  onFilterChange,
  onExport,
  exportDisabled,
  scenario,
  onScenarioChange,
  onScenarioReset,
  onScenarioRerun,
  rerunDisabled,
  branchSaved,
  onSaveBranch,
  saveBranchDisabled,
  onExportComparisonJson,
  onExportComparisonMarkdown,
  arenaHost,
}: {
  arenaHost?: HTMLElement;
  report: SwarmtankReport;
  run: SwarmtankRunDoc;
  runKey: string;
  selectedPersonaId: string | null;
  onSelectPersona: (id: string | null) => void;
  search: string;
  decisionFilter: string;
  onSearchChange: (value: string) => void;
  onFilterChange: (value: string) => void;
  onExport: () => void;
  exportDisabled: boolean;
  scenario: SwarmtankScenario | null;
  onScenarioChange: (next: SwarmtankScenario) => void;
  onScenarioReset: () => void;
  onScenarioRerun: () => void;
  rerunDisabled: boolean;
  branchSaved: boolean;
  onSaveBranch: () => void;
  saveBranchDisabled: boolean;
  onExportComparisonJson: () => void;
  onExportComparisonMarkdown: () => void;
}) {
  const decisionByPersona = new Map(
    run.decisions.map(decision => [decision.personaId, decision]),
  );
  const visiblePersonas = filterRunPersonas(run, search, decisionFilter);
  // Instant local sensitivity: recomputed on every
  // render from the baseline run, no network.
  const scenarioDecisions: SwarmtankDecision[] =
    scenario === null
      ? run.decisions
      : evaluateScenarioPopulation(
          run.personas,
          run.input,
          run.decisions,
          scenario,
        );
  const comparison: ScenarioComparison | null =
    scenario === null
      ? null
      : computeScenarioComparison(
          run.decisions,
          scenarioDecisions,
          run.input.priceAmount,
          scenario.priceMonthly ?? run.input.priceAmount,
        );
  const layoutKey =
    scenario === null
      ? `${runKey}|baseline`
      : `${runKey}|${scenarioSignature(scenario)}`;
  const arena = (
    <SwarmtankArena
      cinematic={Boolean(arenaHost)}
      run={run}
      runKey={runKey}
      effectiveDecisions={scenarioDecisions}
      layoutKey={layoutKey}
      selectedPersonaId={selectedPersonaId}
      onSelectPersona={onSelectPersona}
    />
  );
  return (
    <section aria-label="SwarmTank report">
      <h3>SwarmTank report</h3>
      <p>
        Synthetic scenario output: simulated personas, not real customers.
        Not a substitute for real interviews, conversion data, or market
        research.
      </p>
      <p>
        Conversion rate: {report.conversionRate}% ({report.converts} of{' '}
        {report.total} convert, {report.confused} confused, {report.rejects}{' '}
        reject).
      </p>
      <p>Average willingness to pay: {report.averageWillingnessToPay}.</p>
      <p>
        Top confusion points:{' '}
        {report.topConfusionPoints.length
          ? report.topConfusionPoints
              .map(entry => `${entry.item} (${entry.count})`)
              .join('; ')
          : 'None.'}
      </p>
      <p>
        Top objections:{' '}
        {report.topObjections.length
          ? report.topObjections
              .map(entry => `${entry.item} (${entry.count})`)
              .join('; ')
          : 'None.'}
      </p>
      <p>
        Objection breakdown:{' '}
        {report.objectionBreakdown.length
          ? report.objectionBreakdown
              .map(entry => `${entry.item} (${entry.count})`)
              .join('; ')
          : 'None.'}
      </p>
      <p>
        Segments:{' '}
        {report.segments.map(entry => `${entry.item} (${entry.count})`).join('; ')}
      </p>
      {report.recommendations.map((rec, i) => (
        <section key={i} aria-label={`Recommendation ${i + 1}`}>
          <p>
            <strong>{rec.title}.</strong> {rec.detail}
          </p>
          <p>Assumption: {rec.assumption}</p>
          <p>Synthetic evidence: {rec.syntheticEvidence}</p>
          <p>Uncertainty: {rec.uncertainty}</p>
          <p>Validate with real customers: {rec.validation}</p>
        </section>
      ))}
      {scenario !== null && comparison !== null ? (
        <>
          <SwarmtankScenarioControls
            run={run}
            scenario={scenario}
            onScenarioChange={onScenarioChange}
            onReset={onScenarioReset}
            onRerun={onScenarioRerun}
            rerunDisabled={rerunDisabled}
          />
          <p>
            Scenario conversion rate: {comparison.scenario.conversionRate}%
            (baseline {comparison.baseline.conversionRate}%, signed delta{' '}
            {comparison.scenario.conversionRate -
              comparison.baseline.conversionRate >=
            0
              ? '+'
              : ''}
            {Math.round(
              (comparison.scenario.conversionRate -
                comparison.baseline.conversionRate) *
                100,
            ) / 100}{' '}
            points). Average willingness to pay:{' '}
            {comparison.scenario.averageWillingnessToPay} (baseline{' '}
            {comparison.baseline.averageWillingnessToPay}). Average perceived
            value: {comparison.scenario.averagePerceivedValue} (baseline{' '}
            {comparison.baseline.averagePerceivedValue}). Net shifts: +
            {comparison.convertedIds.length} Converted,{' '}
            {comparison.lostIds.length} Regressed,{' '}
            {comparison.unchangedCount} Unchanged.
          </p>
        </>
      ) : null}
      {arenaHost ? createPortal(arena, arenaHost) : arena}
      {scenario !== null && comparison !== null ? (
        <SwarmtankComparison
          comparison={comparison}
          personas={run.personas}
          scenario={scenario}
          branchSaved={branchSaved}
          onSaveBranch={onSaveBranch}
          onExportJson={onExportComparisonJson}
          onExportMarkdown={onExportComparisonMarkdown}
          saveDisabled={saveBranchDisabled}
        />
      ) : null}
      <p>
        <label>
          Search personas{' '}
          <input
            aria-label="Search personas"
            value={search}
            onChange={event => onSearchChange(event.target.value)}
          />
        </label>{' '}
        <label>
          Filter by decision{' '}
          <select
            aria-label="Filter by decision"
            value={decisionFilter}
            onChange={event => onFilterChange(event.target.value)}
          >
            <option value="All">All decisions</option>
            <option value="Convert">Convert</option>
            <option value="Confused">Confused</option>
            <option value="Reject">Reject</option>
          </select>
        </label>
      </p>
      <table>
        <thead>
          <tr>
            <th>Persona</th>
            <th>Role</th>
            <th>Decision</th>
            <th>Key objection</th>
            <th>Perceived value</th>
            <th>Willingness to pay</th>
            <th>Arena</th>
          </tr>
        </thead>
        <tbody>
          {visiblePersonas.map(persona => {
            const decision = decisionByPersona.get(persona.id);
            const selected = persona.id === selectedPersonaId;
            return (
              <tr key={persona.id}>
                <td>
                  {persona.name} <span>Simulated persona.</span>{' '}
                  {selected ? <strong>Selected in arena.</strong> : null}
                </td>
                <td>{persona.role}</td>
                <td>{decision?.decision ?? '—'}</td>
                <td>{decision?.keyObjection ?? '—'}</td>
                <td>{decision?.perceivedValue ?? '—'}</td>
                <td>{decision?.willingnessToPay ?? '—'}</td>
                <td>
                  <button
                    aria-label={`Select ${persona.name} in arena`}
                    onClick={() => onSelectPersona(persona.id)}
                  >
                    Select in arena
                  </button>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <p>
        <button disabled={exportDisabled} onClick={onExport}>
          Export report
        </button>
      </p>
    </section>
  );
}

export function SwarmtankPanel({
  client,
  workspaceId,
  projectId,
  artifacts,
  onSaved,
  notify,
  busy,
  arenaHost,
}: {
  arenaHost?: HTMLElement;
  client: Client;
  workspaceId: string;
  projectId: string;
  artifacts: ArtifactRecord[];
  onSaved: () => Promise<void>;
  notify: (message: string, isError: boolean) => void;
  busy: boolean;
}) {
  const [input, setInput] = useState<SwarmtankInput>(emptyInput());
  const [preset, setPreset] = useState(20);
  const [seed, setSeed] = useState('');
  const [source, setSource] = useState<SwarmtankSource | undefined>(undefined);
  const [preview, setPreview] = useState<SwarmtankPersona[] | null>(null);
  const [previewBatches, setPreviewBatches] = useState(0);
  const [savedRun, setSavedRun] = useState<SwarmtankRunDoc | null>(null);
  const [savedArtifact, setSavedArtifact] = useState<ArtifactRecord | null>(null);
  const [report, setReport] = useState<SwarmtankReport | null>(null);
  const [reportArtifact, setReportArtifact] = useState<ArtifactRecord | null>(null);
  const [scenario, setScenario] = useState<SwarmtankScenario | null>(null);
  const [branchArtifact, setBranchArtifact] = useState<ArtifactRecord | null>(null);
  const [branchDoc, setBranchDoc] = useState<SwarmtankScenarioDoc | null>(null);
  const [transferMode, setTransferMode] = useState<'baseline' | 'scenario'>(
    'baseline',
  );
  const [selectedPersonaId, setSelectedPersonaId] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const [decisionFilter, setDecisionFilter] = useState('All');
  const [importExperimentId, setImportExperimentId] = useState('');
  const [importConceptId, setImportConceptId] = useState('');
  const [transferConceptId, setTransferConceptId] = useState('');
  const [running, setRunning] = useState(false);

  const experiments = artifacts.filter(artifact => artifact.type === 'experiment');
  const concepts = artifacts.filter(artifact => artifact.type === 'concept');
  const runs = artifacts.filter(artifact => artifact.type === 'simulation');
  const branches = artifacts.filter(artifact => artifact.type === 'scenario');

  function setField<K extends keyof SwarmtankInput>(key: K, value: SwarmtankInput[K]) {
    setInput(current => ({ ...current, [key]: value }));
  }

  async function importExperiment() {
    if (!importExperimentId) return;
    setRunning(true);
    try {
      const details = await client.readProjectArtifact(
        workspaceId,
        importExperimentId,
        undefined,
      );
      const doc = parseColliderExperimentBody(details.body);
      if (doc === null) {
        notify('That experiment body is not a readable saved experiment.', true);
        return;
      }
      setInput(current => ({
        ...current,
        description: doc.result.productization.conceptBrief,
        features: doc.result.conceptualPrimitives.slice(0, 50),
      }));
      setSource({
        kind: 'experiment',
        artifactId: details.artifact.artifactId,
        version: details.artifact.version,
        contentHash: details.artifact.contentHash,
      });
      notify('Imported from the saved experiment. Fields the record does not state stay empty.', false);
    } catch (err) {
      notify(err instanceof Error ? err.message : 'Import failed. Retry.', true);
    } finally {
      setRunning(false);
    }
  }

  async function importConcept() {
    if (!importConceptId) return;
    setRunning(true);
    try {
      const details = await client.readProjectArtifact(workspaceId, importConceptId, undefined);
      const doc = parseTransferConceptBody(details.body);
      if (doc === null) {
        notify('That concept body is not a readable Forge concept.', true);
        return;
      }
      setInput(current => ({ ...current, description: doc.approvedBrief }));
      setSource({
        kind: 'concept',
        artifactId: details.artifact.artifactId,
        version: details.artifact.version,
        contentHash: details.artifact.contentHash,
      });
      notify('Imported from the Forge concept. Fields the record does not state stay empty.', false);
    } catch (err) {
      notify(err instanceof Error ? err.message : 'Import failed. Retry.', true);
    } finally {
      setRunning(false);
    }
  }

  async function generate() {
    setRunning(true);
    try {
      const response = await client.generateSwarmtankPersonas(workspaceId, {
        projectId,
        input,
        preset,
        seed: seed.trim() === '' ? undefined : seed,
        source,
      });
      setPreview(response.personas);
      setPreviewBatches(response.batches.length);
      if (seed.trim() === '') setSeed(response.seed);
      notify(
        `Generated ${response.personas.length} simulated personas in ${response.batches.length} batches. Nothing is saved until you evaluate.`,
        false,
      );
    } catch (err) {
      notify(err instanceof Error ? err.message : 'Generation failed. Retry.', true);
    } finally {
      setRunning(false);
    }
  }

  async function evaluate() {
    setRunning(true);
    try {
      const response = await client.evaluateSwarmtank(workspaceId, {
        projectId,
        input,
        preset,
        seed: seed.trim() === '' ? undefined : seed,
        source,
      });
      const doc = parseSwarmtankRunBody(response.body);
      if (doc === null) {
        notify('The saved run body is not readable.', true);
        return;
      }
      setSavedRun(doc);
      setSavedArtifact(response.artifact);
      setReport(response.report);
      setReportArtifact(response.artifact);
      setScenario(defaultScenarioFor(doc));
      setBranchArtifact(null);
      setBranchDoc(null);
      setTransferMode('baseline');
      setSelectedPersonaId(null);
      setPreview(null);
      await onSaved();
      notify('Evaluated and saved the SwarmTank run.', false);
    } catch (err) {
      notify(err instanceof Error ? err.message : 'Evaluation failed. Retry.', true);
    } finally {
      setRunning(false);
    }
  }

  async function viewReport(artifact: ArtifactRecord) {
    setRunning(true);
    try {
      const response = await client.reportSwarmtank(workspaceId, {
        artifactId: artifact.artifactId,
        version: artifact.version,
      });
      const details = await client.readProjectArtifact(
        workspaceId,
        artifact.artifactId,
        artifact.version,
      );
      const doc = parseSwarmtankRunBody(details.body);
      if (doc === null) {
        notify('The saved run body is not readable.', true);
        return;
      }
      setSavedRun(doc);
      setSavedArtifact(response.artifact);
      setReport(response.report);
      setReportArtifact(response.artifact);
      setScenario(defaultScenarioFor(doc));
      setBranchArtifact(null);
      setBranchDoc(null);
      setTransferMode('baseline');
      setSelectedPersonaId(null);
    } catch (err) {
      notify(err instanceof Error ? err.message : 'Report failed. Retry.', true);
    } finally {
      setRunning(false);
    }
  }

  function changeScenario(next: SwarmtankScenario) {
    setScenario(next);
    // A saved branch describes its own scenario; once
    // the live scenario drifts from it, the link ends.
    if (branchDoc !== null && !scenariosEqual(branchDoc.scenario, next)) {
      setBranchArtifact(null);
      setBranchDoc(null);
      setTransferMode('baseline');
    }
  }

  function resetScenario() {
    if (savedRun === null) return;
    setScenario(defaultScenarioFor(savedRun));
    setBranchArtifact(null);
    setBranchDoc(null);
    setTransferMode('baseline');
  }

  async function rerunScenario() {
    if (savedRun === null || scenario === null) return;
    setRunning(true);
    try {
      const response = await client.evaluateSwarmtank(workspaceId, {
        projectId,
        input: {
          ...savedRun.input,
          priceAmount: scenario.priceMonthly ?? savedRun.input.priceAmount,
          features: scenario.activeFeatures,
        },
        preset: savedRun.preset,
        seed: savedRun.seed,
        source: savedRun.source,
      });
      const doc = parseSwarmtankRunBody(response.body);
      if (doc === null) {
        notify('The re-run body is not readable.', true);
        return;
      }
      setSavedRun(doc);
      setSavedArtifact(response.artifact);
      setReport(response.report);
      setReportArtifact(response.artifact);
      setScenario(defaultScenarioFor(doc));
      setBranchArtifact(null);
      setBranchDoc(null);
      setTransferMode('baseline');
      setSelectedPersonaId(null);
      await onSaved();
      notify('Re-ran the scenario inputs as a new deterministic run.', false);
    } catch (err) {
      notify(err instanceof Error ? err.message : 'Re-run failed. Retry.', true);
    } finally {
      setRunning(false);
    }
  }

  async function saveBranch() {
    if (savedRun === null || savedArtifact === null || scenario === null) return;
    setRunning(true);
    try {
      const response = await client.saveSwarmtankScenario(workspaceId, {
        projectId,
        parentArtifactId: savedArtifact.artifactId,
        parentVersion: savedArtifact.version,
        scenario,
      });
      const doc = parseSwarmtankScenarioDoc(response.body);
      setBranchArtifact(response.artifact);
      setBranchDoc(doc);
      await onSaved();
      notify(
        `Saved the scenario as a branch (version ${response.artifact.version}). The baseline run is unchanged.`,
        false,
      );
    } catch (err) {
      notify(err instanceof Error ? err.message : 'Branch save failed. Retry.', true);
    } finally {
      setRunning(false);
    }
  }

  async function viewBranch(artifact: ArtifactRecord) {
    setRunning(true);
    try {
      const details = await client.readProjectArtifact(
        workspaceId,
        artifact.artifactId,
        artifact.version,
      );
      const doc = parseSwarmtankScenarioDoc(details.body);
      if (doc === null) {
        notify('That branch body is not a readable scenario branch.', true);
        return;
      }
      // Fast path: the branch belongs to the run
      // already on screen — keep it, apply the branch.
      if (
        savedArtifact !== null &&
        savedArtifact.artifactId === doc.parentArtifactId &&
        savedArtifact.version === doc.parentVersion &&
        savedRun !== null
      ) {
        setScenario(doc.scenario);
        setBranchArtifact(artifact);
        setBranchDoc(doc);
        setSelectedPersonaId(null);
        return;
      }
      const parent = runs.find(
        item =>
          item.artifactId === doc.parentArtifactId &&
          item.version === doc.parentVersion,
      );
      if (parent !== undefined) {
        await viewReport(parent);
        setScenario(doc.scenario);
        setBranchArtifact(artifact);
        setBranchDoc(doc);
        return;
      }
      // Restored-workspace fallback: the parent pin
      // is historical provenance and the parent record
      // may carry a new id. Synthesize the display
      // run from the branch's embedded baseline.
      const preset = doc.personas.length;
      if (preset !== 20 && preset !== 50 && preset !== 100) {
        notify('That branch embeds an unsupported population size.', true);
        return;
      }
      setSavedRun({
        format: 'anansi-swarmtank-run',
        version: 1,
        input: doc.baselineInput,
        seed: `branch-${artifact.artifactId}`,
        preset,
        personas: doc.personas,
        decisions: doc.baselineDecisions,
        contentHash: doc.parentContentHash,
      });
      setSavedArtifact({
        artifactId: doc.parentArtifactId,
        version: doc.parentVersion,
        type: 'simulation',
        title: 'SwarmTank run (from branch)',
        timestamp: artifact.timestamp,
        sourceReferences: [],
        validationStatus: 'validated',
        privacy: 'private',
        projectId,
        contentHash: doc.parentContentHash,
      });
      setReport(deriveLocalReport(doc.personas, doc.baselineDecisions));
      setReportArtifact({
        artifactId: doc.parentArtifactId,
        version: doc.parentVersion,
        type: 'simulation',
        title: 'SwarmTank run (from branch)',
        timestamp: artifact.timestamp,
        sourceReferences: [],
        validationStatus: 'validated',
        privacy: 'private',
        projectId,
        contentHash: doc.parentContentHash,
      });
      setScenario(doc.scenario);
      setBranchArtifact(artifact);
      setBranchDoc(doc);
      setTransferMode('baseline');
      setSelectedPersonaId(null);
    } catch (err) {
      notify(err instanceof Error ? err.message : 'Branch view failed. Retry.', true);
    } finally {
      setRunning(false);
    }
  }

  function exportComparisonJson() {
    if (savedRun === null || savedArtifact === null || scenario === null) return;
    const scenarioDecisions = evaluateScenarioPopulation(
      savedRun.personas,
      savedRun.input,
      savedRun.decisions,
      scenario,
    );
    const comparison = computeScenarioComparison(
      savedRun.decisions,
      scenarioDecisions,
      savedRun.input.priceAmount,
      scenario.priceMonthly ?? savedRun.input.priceAmount,
    );
    downloadJson(
      `anansi-swarmtank-${savedArtifact.artifactId}-scenario-comparison.json`,
      scenarioComparisonJson({
        run: savedRun,
        runArtifactId: savedArtifact.artifactId,
        runVersion: savedArtifact.version,
        runContentHash: savedArtifact.contentHash,
        scenario,
        scenarioDecisions,
        branchArtifactId: branchArtifact?.artifactId ?? null,
        branchVersion: branchArtifact?.version ?? null,
        comparison,
      }),
    );
  }

  function exportComparisonMarkdown() {
    if (savedRun === null || savedArtifact === null || scenario === null) return;
    const scenarioDecisions = evaluateScenarioPopulation(
      savedRun.personas,
      savedRun.input,
      savedRun.decisions,
      scenario,
    );
    const comparison = computeScenarioComparison(
      savedRun.decisions,
      scenarioDecisions,
      savedRun.input.priceAmount,
      scenario.priceMonthly ?? savedRun.input.priceAmount,
    );
    const blob = new Blob(
      [
        scenarioComparisonMarkdown({
          run: savedRun,
          runArtifactId: savedArtifact.artifactId,
          runVersion: savedArtifact.version,
          runContentHash: savedArtifact.contentHash,
          scenario,
          scenarioDecisions,
          branchArtifactId: branchArtifact?.artifactId ?? null,
          branchVersion: branchArtifact?.version ?? null,
          comparison,
        }),
      ],
      { type: 'text/markdown' },
    );
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = `anansi-swarmtank-${savedArtifact.artifactId}-scenario-comparison.md`;
    anchor.click();
    URL.revokeObjectURL(url);
  }

  function exportReport() {
    if (report === null || reportArtifact === null) return;
    downloadJson(
      `anansi-swarmtank-${reportArtifact.artifactId}-v${reportArtifact.version}-report.json`,
      JSON.stringify(
        {
          artifactId: reportArtifact.artifactId,
          version: reportArtifact.version,
          contentHash: reportArtifact.contentHash,
          disclaimer:
            'Synthetic scenario output: simulated personas, not real customers. Not a substitute for real interviews, conversion data, or market research.',
          report,
        },
        null,
        2,
      ),
    );
  }

  async function transferToForge() {
    if (savedRun === null || savedArtifact === null) return;
    const conceptId =
      source !== undefined && source.kind === 'concept'
        ? source.artifactId
        : transferConceptId;
    if (!conceptId) {
      notify('Choose a Forge concept to apply the recommendations to.', true);
      return;
    }
    const concept = concepts.find(item => item.artifactId === conceptId);
    if (!concept) {
      notify('The chosen Forge concept is not in this project.', true);
      return;
    }
    setRunning(true);
    try {
      const response = await client.transferSwarmtankToForge(workspaceId, {
        projectId,
        runArtifactId: savedArtifact.artifactId,
        runVersion: savedArtifact.version,
        conceptArtifactId: conceptId,
        expectedConceptVersion: concept.version,
        ...(transferMode === 'scenario' && branchArtifact !== null
          ? {
              scenarioArtifactId: branchArtifact.artifactId,
              scenarioVersion: branchArtifact.version,
            }
          : {}),
      });
      if (response.alreadyApplied) {
        notify('Already applied: this run already revised that Forge concept.', false);
      } else {
        notify(
          `Applied: Forge concept revised to version ${response.artifact.version} with the SwarmTank revision note.`,
          false,
        );
      }
      await onSaved();
    } catch (err) {
      notify(err instanceof Error ? err.message : 'Transfer failed. Retry.', true);
    } finally {
      setRunning(false);
    }
  }

  return (
    <section aria-label="SwarmTank">
      <p className="help">
        SwarmTank explores how simulated buyer profiles might respond to a
        product. This is scenario planning: every persona below is simulated,
        not a real customer, and nothing here measures real demand. Opening
        this panel triggers no provider call; generation and evaluation are
        deterministic backend operations over seeded archetypes.
      </p>
      <h3>SwarmTank input</h3>
      <label>
        Product description{' '}
        <textarea
          aria-label="SwarmTank description"
          value={input.description}
          onChange={event => setField('description', event.target.value)}
        />
      </label>
      <label>
        Pricing{' '}
        <input
          aria-label="SwarmTank pricing"
          value={input.pricing}
          onChange={event => setField('pricing', event.target.value)}
        />
      </label>
      <label>
        Monthly price amount (optional number){' '}
        <input
          aria-label="SwarmTank monthly price amount"
          type="number"
          value={input.priceAmount ?? ''}
          onChange={event =>
            setField(
              'priceAmount',
              event.target.value === '' ? null : Number(event.target.value),
            )
          }
        />
      </label>
      <label>
        Target customer segment{' '}
        <input
          aria-label="SwarmTank segment"
          value={input.segment}
          onChange={event => setField('segment', event.target.value)}
        />
      </label>
      <label>
        Industry{' '}
        <input
          aria-label="SwarmTank industry"
          value={input.industry}
          onChange={event => setField('industry', event.target.value)}
        />
      </label>
      <label>
        Features (one per line){' '}
        <textarea
          aria-label="SwarmTank features"
          value={(input.features ?? []).join('\n')}
          onChange={event =>
            setField(
              'features',
              event.target.value.split('\n').filter(line => line.trim() !== ''),
            )
          }
        />
      </label>
      <label>
        Value proposition{' '}
        <textarea
          aria-label="SwarmTank value proposition"
          value={input.valueProposition}
          onChange={event => setField('valueProposition', event.target.value)}
        />
      </label>
      <label>
        Population{' '}
        <select
          aria-label="SwarmTank population"
          value={preset}
          onChange={event => setPreset(Number(event.target.value))}
        >
          <option value={20}>20 personas</option>
          <option value={50}>50 personas</option>
          <option value={100}>100 personas</option>
        </select>
      </label>
      <label>
        Seed (optional; empty generates and records one){' '}
        <input
          aria-label="SwarmTank seed"
          value={seed}
          onChange={event => setSeed(event.target.value)}
        />
      </label>
      {source ? (
        <p>
          Imported from {source.kind} {source.artifactId} version {source.version}.
        </p>
      ) : null}
      {experiments.length > 0 ? (
        <p>
          <label>
            Import from saved experiment{' '}
            <select
              aria-label="Import from saved experiment"
              value={importExperimentId}
              onChange={event => setImportExperimentId(event.target.value)}
            >
              <option value="">Choose an experiment</option>
              {experiments.map(artifact => (
                <option key={artifact.artifactId} value={artifact.artifactId}>
                  {artifact.title}
                </option>
              ))}
            </select>
          </label>{' '}
          <button disabled={running || busy || !importExperimentId} onClick={importExperiment}>
            Import experiment
          </button>
        </p>
      ) : null}
      {concepts.length > 0 ? (
        <p>
          <label>
            Import from Forge concept{' '}
            <select
              aria-label="Import from Forge concept"
              value={importConceptId}
              onChange={event => setImportConceptId(event.target.value)}
            >
              <option value="">Choose a concept</option>
              {concepts.map(artifact => (
                <option key={artifact.artifactId} value={artifact.artifactId}>
                  {artifact.title}
                </option>
              ))}
            </select>
          </label>{' '}
          <button disabled={running || busy || !importConceptId} onClick={importConcept}>
            Import concept
          </button>
        </p>
      ) : null}
      <p>
        <button disabled={running || busy || input.description.trim() === ''} onClick={generate}>
          Generate personas
        </button>{' '}
        <button disabled={running || busy || input.description.trim() === ''} onClick={evaluate}>
          Evaluate decisions
        </button>
      </p>
      {preview !== null ? (
        <section aria-label="Simulated personas preview">
          <h3>Simulated personas (preview, not saved)</h3>
          <p>
            {preview.length} simulated personas in {previewBatches} batches. Every
            persona below is simulated. Evaluate to save this population as a run.
          </p>
          <ul>
            {preview.map(persona => (
              <li key={persona.id}>
                <strong>{persona.name}</strong> — {persona.role}, company size{' '}
                {persona.companySize}, budget {persona.approximateBudget}.{' '}
                <span>Simulated persona.</span> Skepticism {persona.skepticism},
                price sensitivity {persona.priceSensitivity}. Needs:{' '}
                {persona.needs.join('; ')}.
              </li>
            ))}
          </ul>
        </section>
      ) : null}
      {runs.length > 0 ? (
        <section aria-label="Saved SwarmTank runs">
          <h3>Saved SwarmTank runs</h3>
          <ul>
            {runs.map(artifact => (
              <li key={artifact.artifactId}>
                {artifact.title} (version {artifact.version}){' '}
                <button disabled={running || busy} onClick={() => viewReport(artifact)}>
                  View report
                </button>
              </li>
            ))}
          </ul>
        </section>
      ) : null}
      {branches.length > 0 ? (
        <section aria-label="Saved SwarmTank scenario branches">
          <h3>Saved scenario branches</h3>
          <ul>
            {branches.map(artifact => (
              <li key={artifact.artifactId}>
                {artifact.title} (version {artifact.version}){' '}
                <button disabled={running || busy} onClick={() => viewBranch(artifact)}>
                  View comparison
                </button>
              </li>
            ))}
          </ul>
        </section>
      ) : null}
      {report !== null && savedRun !== null && reportArtifact !== null ? (
        <>
          <SwarmtankReportView
            arenaHost={arenaHost}
            report={report}
            run={savedRun}
            runKey={`${reportArtifact.artifactId}-v${reportArtifact.version}`}
            selectedPersonaId={selectedPersonaId}
            onSelectPersona={setSelectedPersonaId}
            search={search}
            decisionFilter={decisionFilter}
            onSearchChange={setSearch}
            onFilterChange={setDecisionFilter}
            onExport={exportReport}
            exportDisabled={running || busy}
            scenario={scenario}
            onScenarioChange={changeScenario}
            onScenarioReset={resetScenario}
            onScenarioRerun={rerunScenario}
            rerunDisabled={running || busy}
            branchSaved={branchArtifact !== null}
            onSaveBranch={saveBranch}
            saveBranchDisabled={
              running ||
              busy ||
              scenario === null ||
              branchArtifact !== null ||
              (savedRun !== null &&
                scenario !== null &&
                scenariosEqual(scenario, defaultScenarioFor(savedRun)))
            }
            onExportComparisonJson={exportComparisonJson}
            onExportComparisonMarkdown={exportComparisonMarkdown}
          />
          {source === undefined || source.kind !== 'concept' ? (
            <p>
              <label>
                Apply recommendations to Forge concept{' '}
                <select
                  aria-label="Apply recommendations to Forge concept"
                  value={transferConceptId}
                  onChange={event => setTransferConceptId(event.target.value)}
                >
                  <option value="">Choose a concept</option>
                  {concepts.map(artifact => (
                    <option key={artifact.artifactId} value={artifact.artifactId}>
                      {artifact.title}
                    </option>
                  ))}
                </select>
              </label>
            </p>
          ) : (
            <p>
              Associated Forge concept: {source.artifactId} (from import).
            </p>
          )}
          <p>
            <button
              disabled={
                running ||
                busy ||
                (source === undefined || source.kind !== 'concept'
                  ? transferConceptId === ''
                  : false)
              }
              onClick={transferToForge}
            >
              {branchArtifact !== null
                ? transferMode === 'scenario'
                  ? 'Apply Scenario Recommendations to Forge'
                  : 'Apply Baseline Recommendations to Forge'
                : 'Apply Recommendations to Forge'}
            </button>
          </p>
          {branchArtifact !== null ? (
            <p>
              <label>
                <input
                  type="radio"
                  name="swarmtank-transfer-mode"
                  checked={transferMode === 'baseline'}
                  onChange={() => setTransferMode('baseline')}
                />{' '}
                Apply Baseline Recommendations to Forge
              </label>{' '}
              <label>
                <input
                  type="radio"
                  name="swarmtank-transfer-mode"
                  checked={transferMode === 'scenario'}
                  onChange={() => setTransferMode('scenario')}
                />{' '}
                Apply Scenario Recommendations to Forge
              </label>
            </p>
          ) : null}
        </>
      ) : null}
    </section>
  );
}
