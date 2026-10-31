import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { renderToStaticMarkup } from 'react-dom/server';
import React from 'react';
import {
  deriveVisualizationScene,
  hitTestVisualization,
  layoutVisualizationScene,
} from '../src/colliderVisualization';
import { ColliderVisualization } from '../src/ColliderVisualization';
import type { ColliderExperimentDoc } from '@anansi/contracts';

/**
 * M8 visualization: the scene is derived from the exact
 * saved document — node counts match, edges state only
 * the pipeline flow (never a text-mined link), layout is
 * deterministic, and selection shows exact text plus
 * pinned provenance.
 */

function fixtureDoc(): ColliderExperimentDoc {
  return {
    format: 'anansi-collider-experiment',
    version: 1,
    beamA: 'Beam A text about kitchens.',
    beamB: 'Beam B text about orbitals.',
    controls: {
      creativity: 'balanced',
      technicalDepth: 'standard',
      practicality: 'balanced',
      innovationDistance: 'moderate',
      budget: '',
      preferredOutputType: 'concept',
    },
    memoryContext: [],
    documentA: {
      label: 'kitchen.txt',
      kind: 'txt',
      bom: false,
      rawBytes: 26,
      rawHash: 'a'.repeat(64),
      textHash: 'b'.repeat(64),
      textChars: 26,
    },
    result: {
      sourceUnderstanding: 'Both beams describe bounded work.',
      conceptualPrimitives: ['Bounded work', 'Shared heat', 'Timed orbit'],
      crossDomainMappings: ['Bounded work maps to timed orbit', 'Shared heat maps to bounded work'],
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
    provider: 'test-provider',
    model: 'test-model',
    disclosure: {},
  } as unknown as ColliderExperimentDoc;
}

test('derived scene matches the document exactly', () => {
  const scene = deriveVisualizationScene(fixtureDoc(), 'artifact-1', 3);
  assert.equal(scene.identity, 'artifact-1@v3');
  assert.equal(scene.artifactId, 'artifact-1');
  assert.equal(scene.version, 3);
  assert.equal(scene.provider, 'test-provider');
  // 3 beam/understanding nodes + hubs (primitives, mappings,
  // combinations, feasibility, productization) + items:
  // 3 primitives + 2 mappings + 2 combinations +
  // (2 hypotheses + 1 dependency + 0 unknown + 1 risk +
  // 1 testability) + 5 productization fields = 25.
  assert.equal(scene.nodes.length, 25);
  assert.equal(scene.edges.length, 7);
  const beamA = scene.nodes.find(n => n.id === 'beam-a');
  assert.ok(beamA);
  assert.equal(beamA.text, 'Beam A text about kitchens.');
  assert.ok(beamA.provenance.some(p => p.includes('kitchen.txt')));
  assert.ok(beamA.provenance.some(p => p.includes('b'.repeat(64))));
  const primitive = scene.nodes.find(n => n.id === 'primitive-0');
  assert.ok(primitive);
  assert.equal(primitive.text, 'Bounded work');
  assert.ok(primitive.provenance.some(p => p.includes('artifact-1 version 3')));
  const combination = scene.nodes.find(n => n.id === 'combination-1');
  assert.ok(combination);
  assert.equal(combination.label, 'Heat timer');
  assert.equal(combination.text, 'Heat applied on a schedule.');
});

test('edges state only section flow, never item-to-item links', () => {
  const scene = deriveVisualizationScene(fixtureDoc(), 'artifact-1', 3);
  const sectionIds = new Set([
    'beam-a',
    'beam-b',
    'understanding',
    'primitives',
    'mappings',
    'combinations',
    'feasibility',
    'productization',
  ]);
  for (const edge of scene.edges) {
    // Both endpoints are section hubs: no mapping links to
    // a particular primitive, no feasibility item links to
    // a particular combination — the record states neither.
    assert.ok(sectionIds.has(edge.from), edge.from);
    assert.ok(sectionIds.has(edge.to), edge.to);
  }
  assert.deepEqual(
    scene.edges.map(e => `${e.from}->${e.to}`),
    [
      'beam-a->understanding',
      'beam-b->understanding',
      'understanding->primitives',
      'primitives->mappings',
      'mappings->combinations',
      'combinations->feasibility',
      'feasibility->productization',
    ],
  );
});

test('layout is deterministic, bounded, and hit-testable', () => {
  const scene = deriveVisualizationScene(fixtureDoc(), 'artifact-1', 3);
  const first = layoutVisualizationScene(scene, 640, 480);
  const second = layoutVisualizationScene(scene, 640, 480);
  assert.deepEqual(
    [...first.positions.entries()],
    [...second.positions.entries()],
  );
  for (const [id, pos] of first.positions) {
    assert.ok(pos.x >= 0 && pos.x <= 640, id);
    assert.ok(pos.y >= 0 && pos.y <= 480, id);
  }
  const primitivePos = first.positions.get('primitive-0');
  assert.ok(primitivePos);
  assert.equal(
    hitTestVisualization(first, primitivePos.x, primitivePos.y),
    'primitive-0',
  );
  assert.equal(hitTestVisualization(first, 0, 0), null);
});

test('different pinned versions derive different identities', () => {
  const a = deriveVisualizationScene(fixtureDoc(), 'artifact-1', 3);
  const b = deriveVisualizationScene(fixtureDoc(), 'artifact-1', 4);
  const c = deriveVisualizationScene(fixtureDoc(), 'artifact-2', 3);
  assert.notEqual(a.identity, b.identity);
  assert.notEqual(a.identity, c.identity);
  // Same version derives the same node list (restart /
  // restore equivalence at the derivation level).
  const again = deriveVisualizationScene(fixtureDoc(), 'artifact-1', 3);
  assert.deepEqual(
    again.nodes.map(n => n.id),
    a.nodes.map(n => n.id),
  );
});

test('visualization renders the list alternative with no selection initially', () => {
  const html = renderToStaticMarkup(
    React.createElement(ColliderVisualization, {
      doc: fixtureDoc(),
      artifactId: 'artifact-1',
      version: 3,
    }),
  );
  assert.ok(html.includes('Experiment visualization'));
  assert.ok(html.includes('experiment artifact-1 version 3'));
  assert.ok(html.includes('Node list (text alternative)'));
  assert.ok(html.includes('Bounded work'));
  assert.ok(html.includes('Heat timer'));
  assert.ok(!html.includes('Selected:'));
});

test('new visualization files never assign markup', () => {
  for (const file of [
    'packages/ui/src/colliderVisualization.ts',
    'packages/ui/src/ColliderVisualization.tsx',
  ]) {
    const src = readFileSync(file, 'utf8');
    assert.ok(!/\.innerHTML/.test(src), file);
    assert.ok(!/dangerouslySetInnerHTML/.test(src), file);
  }
});
