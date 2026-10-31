// M9 SwarmTank Slice 3: scenario re-clustering.
// Instance slots stay in initial layout order
// (persona-stable); updateDecisions retargets
// colors and markers immediately and interpolates
// positions over 600 ms, snapping under reduced
// motion. Ring lines and marker meshes are
// prebuilt, so a transition allocates nothing.

import test from 'node:test';
import assert from 'node:assert/strict';
import * as THREE from 'three';
import {
  layoutSwarmtankArena,
} from '../src/swarmtankArenaLayout';
import {
  buildSwarmtankArena,
} from '../src/swarmtankThreeScene';
import type { SwarmtankRunDoc } from '@anansi/contracts';

function persona(index: number) {
  return {
    id: `persona-${String(index + 1).padStart(3, '0')}`,
    name: `Persona ${index + 1}`,
    role: 'Small agency owner',
    companySize: '11-50',
    approximateBudget: 3000,
    needs: ['Fast onboarding'],
    priorities: ['Time saved'],
    skepticism: 0.4,
    priceSensitivity: 0.6,
    purchaseConstraints: [],
    requiredTrustSignals: [],
    expectedImplementationEffort: 'Medium',
    synthetic: true,
  };
}

function docWith(decisions: string[]): SwarmtankRunDoc {
  const personas = decisions.map((_, i) => persona(i));
  const decisionDocs = decisions.map((decision, i) => ({
    personaId: personas[i].id,
    decision,
    reason: 'Score 0.70.',
    keyObjection: decision === 'Convert' ? null : 'Price sensitivity',
    perceivedValue: 0.7,
    willingnessToPay: 1470,
    score: 0.7,
  }));
  return {
    format: 'anansi-swarmtank-run',
    version: 1,
    input: {
      description: 'A scheduling tool.',
      pricing: '$49',
      priceAmount: 49,
      segment: 'small agencies',
      industry: 'services',
      features: [],
      valueProposition: 'Saves time.',
    },
    seed: 'animation-seed',
    preset: 20,
    personas,
    decisions: decisionDocs,
    contentHash: 'a'.repeat(64),
  } as unknown as SwarmtankRunDoc;
}

function instancedMeshes(scene: THREE.Scene): THREE.InstancedMesh[] {
  const found: THREE.InstancedMesh[] = [];
  scene.traverse(object => {
    if (object instanceof THREE.InstancedMesh) found.push(object);
  });
  return found;
}

function matrixAt(mesh: THREE.InstancedMesh, index: number): THREE.Matrix4 {
  const matrix = new THREE.Matrix4();
  mesh.getMatrixAt(index, matrix);
  return matrix;
}

const FLIP_FROM = ['Convert', 'Confused', 'Reject'];
const FLIP_TO = ['Reject', 'Convert', 'Confused'];

test('updateDecisions retargets colors immediately, before any interpolation', () => {
  const doc = docWith(FLIP_FROM);
  const built = buildSwarmtankArena(
    layoutSwarmtankArena(doc.personas, doc.decisions),
    640,
    480,
  );
  const flipped = docWith(FLIP_TO);
  built.updateDecisions(
    layoutSwarmtankArena(flipped.personas, flipped.decisions),
    false,
  );
  const [bodies] = instancedMeshes(built.scene);
  const color = new THREE.Color();
  bodies.getColorAt(0, color);
  assert.equal(`#${color.getHexString()}`, '#ef4444');
  bodies.getColorAt(1, color);
  assert.equal(`#${color.getHexString()}`, '#22c55e');
  bodies.getColorAt(2, color);
  assert.equal(`#${color.getHexString()}`, '#f59e0b');
  assert.equal(built.counts.markers, 1);
  assert.deepEqual(built.agentIds, [
    'persona-001',
    'persona-002',
    'persona-003',
  ]);
  built.dispose();
});

test('transition interpolates through the midpoint and converges exactly', () => {
  const doc = docWith(FLIP_FROM);
  const from = layoutSwarmtankArena(doc.personas, doc.decisions);
  const flipped = docWith(FLIP_TO);
  const to = layoutSwarmtankArena(flipped.personas, flipped.decisions);
  const built = buildSwarmtankArena(from, 640, 480);
  const [bodies] = instancedMeshes(built.scene);
  built.updateDecisions(to, false);
  built.advanceTransition(0.3);
  built.animate(0);
  // easeInOutCubic(0.5) = 0.5: every slot sits
  // exactly halfway between its old and new spot.
  const mid = matrixAt(bodies, 0);
  const fromAgent = from.agents[0];
  const toAgent = to.agents.find(a => a.personaId === 'persona-001')!;
  const midRadius = (fromAgent.radius + toAgent.radius) / 2;
  let delta = (toAgent.angle - fromAgent.angle) % (Math.PI * 2);
  if (delta > Math.PI) delta -= Math.PI * 2;
  if (delta < -Math.PI) delta += Math.PI * 2;
  const midAngle = fromAgent.angle + delta * 0.5;
  const epsilon = 1e-6;
  assert.ok(Math.abs(mid.elements[12] - midRadius * Math.cos(midAngle)) < epsilon);
  assert.ok(Math.abs(mid.elements[14] - midRadius * Math.sin(midAngle)) < epsilon);
  // Convergence: after the full 600 ms the scene
  // matches a fresh build of the target layout.
  // Slots are persona-stable, so compare by
  // persona id, not by instance index — the fresh
  // build orders instances by decision group.
  built.advanceTransition(0.6);
  built.animate(0);
  const fresh = buildSwarmtankArena(to, 640, 480);
  const [freshBodies] = instancedMeshes(fresh.scene);
  const freshIndex = new Map(
    to.agents.map((agent, i) => [agent.personaId, i]),
  );
  for (let i = 0; i < 3; i++) {
    const a = matrixAt(bodies, i);
    const b = matrixAt(freshBodies, freshIndex.get(built.agentIds[i]) ?? -1);
    assert.ok(Math.abs(a.elements[12] - b.elements[12]) < epsilon);
    assert.ok(Math.abs(a.elements[14] - b.elements[14]) < epsilon);
  }
  built.dispose();
  fresh.dispose();
});

test('instant update snaps under reduced motion', () => {
  const doc = docWith(FLIP_FROM);
  const flipped = docWith(FLIP_TO);
  const to = layoutSwarmtankArena(flipped.personas, flipped.decisions);
  const built = buildSwarmtankArena(
    layoutSwarmtankArena(doc.personas, doc.decisions),
    640,
    480,
  );
  const [bodies] = instancedMeshes(built.scene);
  built.updateDecisions(to, true);
  built.animate(0);
  const fresh = buildSwarmtankArena(to, 640, 480);
  const [freshBodies] = instancedMeshes(fresh.scene);
  const freshIndex = new Map(
    to.agents.map((agent, i) => [agent.personaId, i]),
  );
  const epsilon = 1e-6;
  for (let i = 0; i < 3; i++) {
    const a = matrixAt(bodies, i);
    const b = matrixAt(freshBodies, freshIndex.get(built.agentIds[i]) ?? -1);
    assert.ok(Math.abs(a.elements[12] - b.elements[12]) < epsilon);
    assert.ok(Math.abs(a.elements[14] - b.elements[14]) < epsilon);
  }
  built.dispose();
  fresh.dispose();
});

test('markers follow the new Confused set and hide the rest', () => {
  const doc = docWith(FLIP_FROM);
  const flipped = docWith(FLIP_TO);
  const to = layoutSwarmtankArena(flipped.personas, flipped.decisions);
  const built = buildSwarmtankArena(
    layoutSwarmtankArena(doc.personas, doc.decisions),
    640,
    480,
  );
  const [, , markers] = instancedMeshes(built.scene);
  built.updateDecisions(to, true);
  built.animate(0);
  // Only persona-003 is Confused now, first in its
  // group: angle 0, radius 120, bob phase 0 at t=0.
  const marker = matrixAt(markers, 0);
  assert.ok(Math.abs(marker.elements[12] - 120) < 1e-6);
  assert.ok(Math.abs(marker.elements[13] - 18) < 1e-6);
  assert.ok(Math.abs(marker.elements[14] - 0) < 1e-6);
  // Unused marker slots collapse to scale 0.
  const hidden = matrixAt(markers, 1);
  assert.equal(hidden.elements[0], 0);
  assert.equal(hidden.elements[5], 0);
  assert.equal(hidden.elements[10], 0);
  built.dispose();
});

test('fifty transition ticks keep exactly three instanced meshes and dispose cleanly', () => {
  const doc = docWith(FLIP_FROM);
  const flipped = docWith(FLIP_TO);
  const to = layoutSwarmtankArena(flipped.personas, flipped.decisions);
  const built = buildSwarmtankArena(
    layoutSwarmtankArena(doc.personas, doc.decisions),
    640,
    480,
  );
  built.updateDecisions(to, false);
  for (let tick = 0; tick < 50; tick++) {
    built.advanceTransition(0.016);
    built.animate(tick * 0.016);
    assert.equal(instancedMeshes(built.scene).length, 3);
  }
  assert.equal(built.counts.markers, 1);
  assert.equal(built.counts.agents, 3);
  built.dispose();
});
