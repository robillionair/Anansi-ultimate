import test from 'node:test';
import assert from 'node:assert/strict';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import {
  ColliderTransferPanel,
  TransferConceptView,
  parseTransferConceptBody,
} from '../src/Transfer';
import type { ColliderExperimentDoc } from '@anansi/contracts';

function validConceptBody(): Record<string, unknown> {
  return {
    format: 'anansi-forge-concept',
    version: 1,
    sourceExperimentArtifactId: 'exp-1',
    sourceExperimentVersion: 1,
    sourceExperimentContentHash: 'a'.repeat(64),
    selectedCombinationIndex: 0,
    selectedCombination: {
      name: 'Mycelial order router',
      description: 'A routing layer that scores venues by local congestion signals.',
    },
    approvedBrief: 'Route venues by local congestion, edited by the user.',
    assumptions: ['Local congestion signals can rank venues without a central coordinator.'],
    unknowns: ['Exchange fees under rerouting are unknown.', 'A venue feed with latency and fill data'],
    productizationBrief: 'A routing advisor that proposes venue choices from local signals.',
    provider: 'ollama',
    model: 'llama-x',
  };
}

function validExperiment(): ColliderExperimentDoc {
  return {
    format: 'anansi-collider-experiment',
    version: 1,
    beamA: 'Trading signals.',
    beamB: 'Mycelium networks.',
    controls: {
      creativity: 'balanced',
      technicalDepth: 'standard',
      practicality: 'balanced',
      innovationDistance: 'moderate',
      budget: '',
      preferredOutputType: 'concept',
    },
    result: {
      sourceUnderstanding: 'Both beams describe decentralized coordination.',
      conceptualPrimitives: ['Distributed signaling', 'Adaptive routing', 'Local feedback'],
      crossDomainMappings: ['Order-book depth resembles nutrient gradients.', 'Redundant paths degrade gracefully.'],
      combinations: [
        { name: 'Mycelial order router', description: 'A routing layer that scores venues by local congestion signals.' },
        { name: 'Graceful-degradation mesh', description: 'A trade mesh that reroutes through redundant paths.' },
      ],
      feasibility: {
        hypotheses: ['Local congestion signals can rank venues without a central coordinator.'],
        dependencies: ['A venue feed with latency and fill data'],
        unknownCosts: ['Exchange fees under rerouting are unknown.'],
        risks: ['Stale signals could route into worse fills.'],
        testability: 'Replay historical venue data through the router.',
      },
      productization: {
        conceptBrief: 'A routing advisor that proposes venue choices from local signals.',
        architecture: 'Signal ingest, local scoring, policy engine.',
        roadmap: 'Phase 1: simulator.',
        testPlan: 'Replay-test the router.',
        prototypeProposal: 'A paper prototype plus a simulator notebook.',
      },
    },
    provider: 'ollama',
    model: 'llama-x',
    disclosure: {} as never,
  };
}

test('transfer concept parser accepts a valid body and rejects tampering', () => {
  const doc = parseTransferConceptBody(JSON.stringify(validConceptBody()));
  assert.ok(doc);
  assert.equal(doc.sourceExperimentArtifactId, 'exp-1');
  assert.equal(doc.approvedBrief, 'Route venues by local congestion, edited by the user.');

  const wrongFormat = { ...validConceptBody(), format: 'anansi-something-else' };
  assert.equal(parseTransferConceptBody(JSON.stringify(wrongFormat)), null);
  const unknownField = { ...validConceptBody(), surprise: 'not part of the contract' };
  assert.equal(parseTransferConceptBody(JSON.stringify(unknownField)), null);
  const emptyBrief = { ...validConceptBody(), approvedBrief: '   ' };
  assert.equal(parseTransferConceptBody(JSON.stringify(emptyBrief)), null);
  const overlongBrief = { ...validConceptBody(), approvedBrief: 'x'.repeat(4001) };
  assert.equal(parseTransferConceptBody(JSON.stringify(overlongBrief)), null);
  const badCombination = {
    ...validConceptBody(),
    selectedCombination: { name: 'Only a name' },
  };
  assert.equal(parseTransferConceptBody(JSON.stringify(badCombination)), null);
  const missingField = { ...validConceptBody() };
  delete missingField.provider;
  assert.equal(parseTransferConceptBody(JSON.stringify(missingField)), null);
  assert.equal(parseTransferConceptBody('not json at all'), null);
});

test('transfer concept view pins the experiment and labels origins apart', () => {
  const doc = parseTransferConceptBody(JSON.stringify(validConceptBody()))!;
  const html = renderToStaticMarkup(<TransferConceptView doc={doc} />);
  assert.match(html, /exp-1/);
  assert.match(html, /version 1/);
  assert.match(html, /user-authored/);
  assert.match(html, /model-derived/);
  assert.match(html, /Route venues by local congestion, edited by the user/);
  assert.match(html, /not specification approval/);
  assert.doesNotMatch(html, /specification approved/);
});

test('transfer panel renders selection, editable brief and honesty copy, and calls nothing on render', () => {
  let calls = 0;
  const client = {
    transferProjectCollider: async () => {
      calls += 1;
      throw new Error('must not be called on render');
    },
  } as never;
  const html = renderToStaticMarkup(
    <ColliderTransferPanel
      client={client}
      workspaceId="ws-1"
      projectId="proj-1"
      experiment={validExperiment()}
      experimentArtifact={{ artifactId: 'exp-1', version: 1 } as never}
      notify={() => {}}
      onTransferred={() => {}}
      busy={false}
    />,
  );
  assert.match(html, /Mycelial order router/);
  assert.match(html, /Graceful-degradation mesh/);
  assert.match(html, /A routing advisor that proposes venue choices from local signals/);
  assert.match(html, /Approve and send to Product Forge/);
  assert.match(html, /does not approve a specification/);
  assert.match(html, /saves nothing/);
  assert.equal(calls, 0);
});
