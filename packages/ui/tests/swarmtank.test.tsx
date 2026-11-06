// M9 SwarmTank Slice 1: the run parser is a strict
// mirror of the backend bounds, the table filter
// narrows by search and decision, and the report
// view renders metrics, §40 recommendation fields,
// and labeled simulated personas — with zero
// innerHTML in the new files.

import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { renderToStaticMarkup } from 'react-dom/server';
import React from 'react';
import {
  parseSwarmtankRunBody,
  filterRunPersonas,
  SwarmtankReportView,
} from '../src/Swarmtank';
import type { SwarmtankRunDoc, SwarmtankReport } from '@anansi/contracts';

function persona(index: number, decision: string, objection: string | null) {
  return {
    persona: {
      id: `persona-${String(index + 1).padStart(3, '0')}`,
      name: index === 0 ? 'Ava Keller' : `Persona ${index + 1}`,
      role: index % 2 === 0 ? 'Small agency owner' : 'Technical founder',
      companySize: '11-50',
      approximateBudget: 3000,
      needs: ['Fast onboarding', 'Client reporting'],
      priorities: ['Time saved', 'Price'],
      skepticism: 0.4,
      priceSensitivity: 0.6,
      purchaseConstraints: ['Client budget cycles'],
      requiredTrustSignals: ['Free trial'],
      expectedImplementationEffort: 'Medium',
      synthetic: true,
    },
    decision: {
      personaId: `persona-${String(index + 1).padStart(3, '0')}`,
      decision,
      reason: 'Score 0.70: fit 0.70, budget 1.00, price 0.60, trust 0.55, switching 0.75, clarity 0.90, authority 0.90.',
      keyObjection: objection,
      perceivedValue: 0.7,
      willingnessToPay: 1470,
      score: 0.7,
    },
  };
}

function validDoc(): SwarmtankRunDoc {
  const parts = [];
  for (let i = 0; i < 20; i++) {
    parts.push(persona(i, i < 12 ? 'Convert' : i < 16 ? 'Confused' : 'Reject', i < 12 ? null : i < 16 ? 'Unclear value proposition' : 'Price sensitivity'));
  }
  return {
    format: 'anansi-swarmtank-run',
    version: 1,
    input: {
      description: 'A scheduling tool for small agencies.',
      pricing: '$49 per month',
      priceAmount: 49,
      segment: 'small agencies',
      industry: 'services',
      features: ['Automated client reporting'],
      valueProposition: 'Agencies save ten hours a week.',
    },
    seed: 'test-seed',
    preset: 20,
    personas: parts.map(part => part.persona),
    decisions: parts.map(part => part.decision),
    contentHash: 'a'.repeat(64),
  } as unknown as SwarmtankRunDoc;
}

function validReport(): SwarmtankReport {
  return {
    total: 20,
    converts: 12,
    confused: 4,
    rejects: 4,
    conversionRate: 60,
    averageWillingnessToPay: 1470,
    topConfusionPoints: [{ item: 'Unclear value proposition', count: 4 }],
    topObjections: [{ item: 'Price sensitivity', count: 4 }],
    objectionBreakdown: [
      { item: 'clarity', count: 4 },
      { item: 'price', count: 4 },
    ],
    segments: [
      { item: 'Small agency owner', count: 10 },
      { item: 'Technical founder', count: 10 },
    ],
    recommendations: [
      {
        title: 'Clarify the value proposition',
        detail: 'Rewrite the top message.',
        assumption: 'Personas score clarity from present fields.',
        syntheticEvidence: '4 clarity-category objections across 20 evaluated personas.',
        uncertainty: 'Synthetic only — unvalidated model output.',
        validation: 'Run five-second comprehension tests with real members of the segment.',
      },
    ],
  };
}

test('a valid run body parses with all twenty personas', () => {
  const parsed = parseSwarmtankRunBody(JSON.stringify(validDoc()));
  assert.ok(parsed !== null);
  assert.equal(parsed.personas.length, 20);
  assert.equal(parsed.decisions.length, 20);
  assert.ok(parsed.personas.every(item => item.synthetic === true));
  // A run generated before markets existed has no market.
  assert.equal(parsed.market, undefined);
});

test('a run body with a researched market parses and keeps the market', () => {
  const doc = validDoc();
  (doc.personas[0] as unknown as Record<string, unknown>).language = 'et';
  (doc as unknown as Record<string, unknown>).market = {
    profileId: 'EE',
    profileVersion: 1,
    territoryCode: 'EE',
    territoryName: 'Estonia',
    custom: false,
  };
  const parsed = parseSwarmtankRunBody(JSON.stringify(doc));
  assert.ok(parsed !== null);
  assert.equal(parsed.market?.profileId, 'EE');
  assert.equal(parsed.market?.territoryName, 'Estonia');
  assert.equal(parsed.market?.custom, false);
  assert.equal(parsed.personas[0].language, 'et');
  // The report view labels the researched market.
  const html = renderToStaticMarkup(
    React.createElement(SwarmtankReportView, {
      report: validReport(),
      run: parsed,
      runKey: 'test-v1',
      selectedPersonaId: null,
      onSelectPersona: () => {},
      search: '',
      decisionFilter: 'All',
      onSearchChange: () => {},
      onFilterChange: () => {},
      onExport: () => {},
      exportDisabled: false,
      scenario: null,
      onScenarioChange: () => {},
      onScenarioReset: () => {},
      onScenarioRerun: () => {},
      rerunDisabled: false,
      branchSaved: false,
      onSaveBranch: () => {},
      saveBranchDisabled: true,
      onExportComparisonJson: () => {},
      onExportComparisonMarkdown: () => {},
    }),
  );
  assert.ok(html.includes('Estonia'));
  assert.ok(html.includes('researched profile v1'));
});

test('a run body with a custom market parses and is labelled custom', () => {
  const doc = validDoc();
  (doc as unknown as Record<string, unknown>).market = {
    territoryCode: 'FR',
    territoryName: 'France',
    custom: true,
    customAssumptions: [
      { field: 'business_language', value: 'French', rationale: 'User-supplied.', editable: true },
    ],
  };
  const parsed = parseSwarmtankRunBody(JSON.stringify(doc));
  assert.ok(parsed !== null);
  assert.equal(parsed.market?.custom, true);
  assert.equal(parsed.market?.profileId, undefined);
  const html = renderToStaticMarkup(
    React.createElement(SwarmtankReportView, {
      report: validReport(),
      run: parsed,
      runKey: 'test-v1',
      selectedPersonaId: null,
      onSelectPersona: () => {},
      search: '',
      decisionFilter: 'All',
      onSearchChange: () => {},
      onFilterChange: () => {},
      onExport: () => {},
      exportDisabled: false,
      scenario: null,
      onScenarioChange: () => {},
      onScenarioReset: () => {},
      onScenarioRerun: () => {},
      rerunDisabled: false,
      branchSaved: false,
      onSaveBranch: () => {},
      saveBranchDisabled: true,
      onExportComparisonJson: () => {},
      onExportComparisonMarkdown: () => {},
    }),
  );
  assert.ok(html.includes('Custom assumptions'));
  assert.ok(html.includes('1 custom assumption(s) recorded'));
});

test('tampered market selections fail closed', () => {
  const cases: Array<(doc: Record<string, unknown>) => void> = [
    doc => {
      // A custom run must not name a researched profile.
      (doc as Record<string, unknown>).market = {
        profileId: 'EE',
        profileVersion: 1,
        territoryCode: 'EE',
        territoryName: 'Estonia',
        custom: true,
      };
    },
    doc => {
      // A researched profile must carry its version.
      (doc as Record<string, unknown>).market = {
        profileId: 'EE',
        territoryCode: 'EE',
        territoryName: 'Estonia',
        custom: false,
      };
    },
    doc => {
      // Unknown market fields are refused.
      (doc as Record<string, unknown>).market = { territoryCode: 'EE', invented: true };
    },
    doc => {
      // A persona language must be a short string.
      (doc.personas as Array<Record<string, unknown>>)[0].language = 42;
    },
  ];
  for (const mutate of cases) {
    const raw = JSON.parse(JSON.stringify(validDoc())) as Record<string, unknown>;
    mutate(raw);
    assert.equal(parseSwarmtankRunBody(JSON.stringify(raw)), null);
  }
});

test('tampered run bodies fail closed', () => {
  const cases: Array<(doc: Record<string, unknown>) => void> = [
    doc => {
      doc.format = 'anansi-collider-experiment';
    },
    doc => {
      doc.preset = 30;
    },
    doc => {
      (doc.personas as Array<Record<string, unknown>>)[3].id = 'persona-999';
    },
    doc => {
      (doc.personas as Array<Record<string, unknown>>)[0].synthetic = false;
    },
    doc => {
      (doc.personas as Array<Record<string, unknown>>)[0].skepticism = 1.5;
    },
    doc => {
      (doc.decisions as Array<Record<string, unknown>>)[0].keyObjection = 'Price sensitivity';
    },
    doc => {
      (doc.decisions as Array<Record<string, unknown>>)[12].keyObjection = null;
    },
    doc => {
      doc.extraField = true;
    },
    doc => {
      (doc.input as Record<string, unknown>).priceAmount = -5;
    },
    doc => {
      doc.contentHash = 'not-a-hash';
    },
  ];
  for (const mutate of cases) {
    const raw = JSON.parse(JSON.stringify(validDoc())) as Record<string, unknown>;
    mutate(raw);
    assert.equal(parseSwarmtankRunBody(JSON.stringify(raw)), null);
  }
  assert.equal(parseSwarmtankRunBody('not json'), null);
});

test('table filtering narrows by decision and search', () => {
  const doc = validDoc();
  assert.equal(filterRunPersonas(doc, '', 'All').length, 20);
  assert.equal(filterRunPersonas(doc, '', 'Convert').length, 12);
  assert.equal(filterRunPersonas(doc, '', 'Confused').length, 4);
  assert.equal(filterRunPersonas(doc, '', 'Reject').length, 4);
  assert.equal(filterRunPersonas(doc, 'Ava', 'All').length, 1);
  assert.equal(filterRunPersonas(doc, 'agency owner', 'Convert').length, 6);
  assert.equal(filterRunPersonas(doc, 'no such persona', 'All').length, 0);
});

test('report view renders metrics, recommendations, and labeled rows', () => {
  const doc = validDoc();
  const html = renderToStaticMarkup(
    React.createElement(SwarmtankReportView, {
      report: validReport(),
      run: doc,
      runKey: 'test-run-v1',
      selectedPersonaId: null,
      onSelectPersona: () => {},
      search: '',
      decisionFilter: 'All',
      onSearchChange: () => {},
      onFilterChange: () => {},
      onExport: () => {},
      exportDisabled: false,
      scenario: null,
      onScenarioChange: () => {},
      onScenarioReset: () => {},
      onScenarioRerun: () => {},
      rerunDisabled: false,
      branchSaved: false,
      onSaveBranch: () => {},
      saveBranchDisabled: true,
      onExportComparisonJson: () => {},
      onExportComparisonMarkdown: () => {},
    }),
  );
  assert.ok(html.includes('Conversion rate: 60%'));
  assert.ok(html.includes('Average willingness to pay: 1470.'));
  assert.ok(html.includes('Unclear value proposition (4)'));
  assert.ok(html.includes('Price sensitivity (4)'));
  assert.ok(html.includes('Clarify the value proposition.'));
  assert.ok(html.includes('Assumption: Personas score clarity from present fields.'));
  assert.ok(html.includes('Synthetic evidence: 4 clarity-category objections'));
  assert.ok(html.includes('Validate with real customers: Run five-second comprehension tests'));
  assert.ok(html.includes('Ava Keller'));
  assert.ok(html.includes('Simulated persona.'));
});

test('report view with a decision filter hides other rows', () => {
  const doc = validDoc();
  const html = renderToStaticMarkup(
    React.createElement(SwarmtankReportView, {
      report: validReport(),
      run: doc,
      runKey: 'test-run-v1',
      selectedPersonaId: null,
      onSelectPersona: () => {},
      search: '',
      decisionFilter: 'Reject',
      onSearchChange: () => {},
      onFilterChange: () => {},
      onExport: () => {},
      exportDisabled: false,
      scenario: null,
      onScenarioChange: () => {},
      onScenarioReset: () => {},
      onScenarioRerun: () => {},
      rerunDisabled: false,
      branchSaved: false,
      onSaveBranch: () => {},
      saveBranchDisabled: true,
      onExportComparisonJson: () => {},
      onExportComparisonMarkdown: () => {},
    }),
  );
  assert.ok(!html.includes('Ava Keller'));
  assert.ok(html.includes('Persona 17'));
});

test('swarmtank files contain zero innerHTML', () => {
  for (const file of ['packages/ui/src/Swarmtank.tsx', 'packages/ui/src/Transfer.tsx', 'packages/ui/src/swarmtankArenaLayout.ts', 'packages/ui/src/swarmtankThreeScene.ts', 'packages/ui/src/SwarmtankThreeView.tsx', 'packages/ui/src/SwarmtankArena.tsx', 'packages/ui/src/swarmtankSensitivity.ts', 'packages/ui/src/SwarmtankScenarioControls.tsx', 'packages/ui/src/SwarmtankComparison.tsx', 'packages/ui/src/swarmtankScenarioExport.ts']) {
    const source = readFileSync(file, 'utf8');
    assert.ok(!source.includes('innerHTML'), file);
    assert.ok(!source.includes('dangerouslySetInnerHTML'), file);
  }
});
