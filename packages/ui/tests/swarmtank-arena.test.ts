// M9 SwarmTank Slice 2: the arena layout is a
// deterministic function of run order, the scene
// builder mounts agents by decision with exact
// picking, selection scales the picked body, only
// Confused agents move, and disposal never throws —
// all headless, because the builder creates no
// renderer. The arena section renders its canvas,
// toggle, and (when selected) the detail card.

import test from 'node:test';
import assert from 'node:assert/strict';
import { renderToStaticMarkup } from 'react-dom/server';
import React from 'react';
import * as THREE from 'three';
import {
  layoutSwarmtankArena,
} from '../src/swarmtankArenaLayout';
import {
  buildSwarmtankArena,
} from '../src/swarmtankThreeScene';
import { SwarmtankArena } from '../src/SwarmtankArena';
import type { SwarmtankRunDoc } from '@anansi/contracts';

function persona(index: number) {
  return {
    id: `persona-${String(index + 1).padStart(3, '0')}`,
    name: index === 0 ? 'Ava Keller' : `Persona ${index + 1}`,
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
    seed: 'arena-seed',
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

test('layout is deterministic and clusters by decision radius', () => {
  const doc = docWith([
    'Convert', 'Convert', 'Confused', 'Confused', 'Reject', 'Reject',
  ]);
  const first = layoutSwarmtankArena(doc.personas, doc.decisions);
  const second = layoutSwarmtankArena(doc.personas, doc.decisions);
  assert.deepEqual(first, second);
  const byId = new Map(first.agents.map(agent => [agent.personaId, agent]));
  const converts = first.agents.filter(agent => agent.decision === 'Convert');
  const confused = first.agents.filter(agent => agent.decision === 'Confused');
  const rejects = first.agents.filter(agent => agent.decision === 'Reject');
  assert.ok(Math.max(...converts.map(a => a.radius)) < Math.min(...confused.map(a => a.radius)));
  assert.ok(Math.max(...confused.map(a => a.radius)) < Math.min(...rejects.map(a => a.radius)));
  assert.ok(byId.get('persona-001') !== undefined);
  assert.equal(first.ringRadii.length, 3);
});

test('layout facing points Convert inward, Reject outward, Confused tangentially', () => {
  const doc = docWith(['Convert', 'Confused', 'Reject']);
  const layout = layoutSwarmtankArena(doc.personas, doc.decisions);
  for (const agent of layout.agents) {
    const dot = agent.facingX * agent.x + agent.facingZ * agent.z;
    if (agent.decision === 'Convert') assert.ok(dot < 0, 'Convert faces the pedestal');
    if (agent.decision === 'Reject') assert.ok(dot > 0, 'Reject faces away');
    if (agent.decision === 'Confused') assert.ok(Math.abs(dot) < 1e-9, 'Confused faces tangentially');
  }
});

test('layout spills a full ring outward', () => {
  const decisions = Array.from({ length: 30 }, () => 'Convert');
  const doc = docWith(decisions);
  const layout = layoutSwarmtankArena(doc.personas, doc.decisions);
  const ringZero = layout.agents.filter(agent => agent.ring === 0);
  const ringOne = layout.agents.filter(agent => agent.ring === 1);
  // Ring 0 at radius 60 holds floor(2π·60/14) = 26 agents.
  assert.equal(ringZero.length, 26);
  assert.equal(ringOne.length, 4);
  assert.ok(ringOne[0].radius > ringZero[0].radius);
});

test('builder mounts decision-colored agents and picks by raycast', () => {
  const doc = docWith([
    'Convert', 'Convert', 'Confused', 'Confused', 'Reject', 'Reject',
  ]);
  const layout = layoutSwarmtankArena(doc.personas, doc.decisions);
  const built = buildSwarmtankArena(layout, 640, 480);
  assert.equal(built.counts.agents, 6);
  assert.equal(built.counts.markers, 2);
  assert.equal(built.counts.rings, 3);
  const [bodies] = instancedMeshes(built.scene);
  const color = new THREE.Color();
  bodies.getColorAt(0, color);
  assert.equal(`#${color.getHexString()}`, '#22c55e');
  bodies.getColorAt(2, color);
  assert.equal(`#${color.getHexString()}`, '#f59e0b');
  bodies.getColorAt(4, color);
  assert.equal(`#${color.getHexString()}`, '#ef4444');
  // Project a known agent through the real camera and pick it. The
  // camera matrix must be updated before projecting (pick does this
  // internally for its own raycast).
  const agent = layout.agents[1];
  built.camera.updateMatrixWorld();
  const projected = new THREE.Vector3(agent.x, 6, agent.z).project(built.camera);
  assert.equal(built.pick(projected.x, projected.y), agent.personaId);
  // Empty space (far corner) picks nothing.
  assert.equal(built.pick(0.99, 0.99), null);
  built.dispose();
});

test('setSelected scales only the picked body and restores on clear', () => {
  const doc = docWith(['Convert', 'Confused', 'Reject']);
  const layout = layoutSwarmtankArena(doc.personas, doc.decisions);
  const built = buildSwarmtankArena(layout, 640, 480);
  const [bodies] = instancedMeshes(built.scene);
  const matrix = new THREE.Matrix4();
  bodies.getMatrixAt(1, matrix);
  assert.equal(matrix.elements[0], 1);
  built.setSelected('persona-002');
  bodies.getMatrixAt(1, matrix);
  assert.ok(Math.abs(matrix.elements[0] - 1.6) < 1e-6);
  bodies.getMatrixAt(0, matrix);
  assert.equal(matrix.elements[0], 1);
  built.setSelected(null);
  bodies.getMatrixAt(1, matrix);
  assert.equal(matrix.elements[0], 1);
  built.dispose();
});

test('animate moves only Confused agents and bobs their markers', () => {
  const doc = docWith(['Convert', 'Confused', 'Reject']);
  const layout = layoutSwarmtankArena(doc.personas, doc.decisions);
  const built = buildSwarmtankArena(layout, 640, 480);
  const [bodies, , markers] = instancedMeshes(built.scene);
  const convertBefore = new THREE.Matrix4();
  const confusedBefore = new THREE.Matrix4();
  const markerBefore = new THREE.Matrix4();
  const after = new THREE.Matrix4();
  bodies.getMatrixAt(0, convertBefore);
  bodies.getMatrixAt(1, confusedBefore);
  markers.getMatrixAt(0, markerBefore);
  built.animate(10);
  bodies.getMatrixAt(0, after);
  assert.equal(after.elements[12], convertBefore.elements[12]);
  assert.equal(after.elements[14], convertBefore.elements[14]);
  bodies.getMatrixAt(1, after);
  assert.ok(
    after.elements[12] !== confusedBefore.elements[12] ||
      after.elements[14] !== confusedBefore.elements[14],
    'Confused agent orbited',
  );
  markers.getMatrixAt(0, after);
  assert.ok(after.elements[13] !== markerBefore.elements[13], 'marker bobbed');
  built.dispose();
});

test('arena section renders canvas, toggle, and the selected detail card', () => {
  const doc = docWith(['Convert', 'Confused', 'Reject']);
  const plain = renderToStaticMarkup(
    React.createElement(SwarmtankArena, {
      run: doc,
      runKey: 'arena-run-v1',
      selectedPersonaId: null,
      onSelectPersona: () => {},
    }),
  );
  assert.ok(plain.includes('SwarmTank 3D arena'));
  assert.ok(plain.includes('Reduce motion (static arena view)'));
  assert.ok(plain.includes('data-three-canvas'));
  assert.ok(!plain.includes('Selected persona'));
  const selected = renderToStaticMarkup(
    React.createElement(SwarmtankArena, {
      run: doc,
      runKey: 'arena-run-v1',
      selectedPersonaId: 'persona-001',
      onSelectPersona: () => {},
    }),
  );
  assert.ok(selected.includes('Selected persona'));
  assert.ok(selected.includes('Ava Keller'));
  assert.ok(selected.includes('Small agency owner'));
  assert.ok(selected.includes('Approximate budget: 3000'));
  assert.ok(selected.includes('None — converted.'));
});
