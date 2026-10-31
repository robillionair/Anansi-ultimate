import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import * as THREE from 'three';
import {
  buildThreeScene,
  detectWebGL,
  isReducedMotion,
} from '../src/colliderThreeScene';
import { deriveVisualizationScene } from '../src/colliderVisualizationScene';
import type { ColliderExperimentDoc } from '@anansi/contracts';

/**
 * M8 Three.js visualization, headless: capability
 * detection, scene-graph construction from the exact
 * derived scene, raycast picking, selection recolor,
 * animation advance, and full disposal — all without
 * a GL context, because the builder creates no
 * renderer.
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

function stubSprite() {
  return new THREE.Sprite(
    new THREE.SpriteMaterial({ map: new THREE.Texture() }),
  );
}

test('capability detection follows the canvas factory, never throws', () => {
  assert.equal(
    detectWebGL(() => ({ getContext: () => null })),
    false,
  );
  assert.equal(
    detectWebGL(() => {
      throw new Error('no canvas');
    }),
    false,
  );
  assert.equal(detectWebGL(() => null), false);
  assert.equal(
    detectWebGL(() => ({ getContext: () => ({}) })),
    true,
  );
  // webgl2 missing but webgl present still counts.
  assert.equal(
    detectWebGL(() => ({
      getContext: (kind: string) => (kind === 'webgl' ? {} : null),
    })),
    true,
  );
  // No document in node: the default factory says no WebGL.
  assert.equal(detectWebGL(), false);
});

test('reduced motion honors the system query or the workspace setting', () => {
  assert.equal(isReducedMotion({ mediaMatches: true }), true);
  assert.equal(
    isReducedMotion({ datasetReducedMotion: 'true' }),
    true,
  );
  assert.equal(
    isReducedMotion({ mediaMatches: false, datasetReducedMotion: 'false' }),
    false,
  );
  assert.equal(isReducedMotion({}), false);
  assert.equal(isReducedMotion(), false);
});

test('scene graph matches the derived scene exactly', () => {
  const scene = deriveVisualizationScene(fixtureDoc(), 'artifact-1', 3);
  const built = buildThreeScene(scene, 640, 480, {
    createLabelSprite: () => stubSprite(),
  });
  assert.equal(built.counts.rings, 2);
  assert.equal(built.counts.particles, 240);
  assert.equal(built.counts.fragments, 64);
  assert.equal(built.counts.nodes, 25);
  assert.equal(built.counts.edges, 7);
  assert.equal(built.counts.sprites, 25);
  assert.equal(built.nodeIds.length, 25);
  assert.ok(built.nodeIds.includes('beam-a'));
  assert.ok(built.nodeIds.includes('productization-prototype-proposal'));
  // The scene background is the dark navy field.
  assert.equal(
    (built.scene.background as THREE.Color).getHexString(),
    '0b2545',
  );
  built.dispose();
});

test('layout mapping is deterministic across builds', () => {
  const scene = deriveVisualizationScene(fixtureDoc(), 'artifact-1', 3);
  const first = buildThreeScene(scene, 640, 480, {
    createLabelSprite: () => stubSprite(),
  });
  const second = buildThreeScene(scene, 640, 480, {
    createLabelSprite: () => stubSprite(),
  });
  for (const [id, pos] of first.positions) {
    const other = second.positions.get(id);
    assert.ok(other);
    assert.deepEqual(
      [other.x, other.y, other.z],
      [pos.x, pos.y, pos.z],
    );
  }
  first.dispose();
  second.dispose();
});

test('raycast picking maps a node position back to its id', () => {
  const scene = deriveVisualizationScene(fixtureDoc(), 'artifact-1', 3);
  const built = buildThreeScene(scene, 640, 480, {
    createLabelSprite: () => stubSprite(),
  });
  const beamA = built.positions.get('beam-a');
  assert.ok(beamA);
  // Project the Beam A hub through the fixed camera to
  // NDC, then pick there: the round trip must recover it.
  const projected = beamA.clone().project(built.camera);
  assert.equal(built.pick(projected.x, projected.y), 'beam-a');
  // A far corner picks nothing.
  assert.equal(built.pick(0.99, 0.99), null);
  assert.equal(built.pick(-0.99, -0.99), null);
  built.dispose();
});

test('selection recolors only the picked instance', () => {
  const scene = deriveVisualizationScene(fixtureDoc(), 'artifact-1', 3);
  const built = buildThreeScene(scene, 640, 480, {
    createLabelSprite: () => stubSprite(),
  });
  const color = new THREE.Color();
  const indexOf = (id: string) => built.nodeIds.indexOf(id);
  let nodeMesh: THREE.InstancedMesh | null = null;
  built.scene.traverse(object => {
    if (object instanceof THREE.InstancedMesh && object.count === 25) {
      nodeMesh = object;
    }
  });
  assert.ok(nodeMesh);
  const mesh = nodeMesh as unknown as THREE.InstancedMesh;
  built.setSelected('beam-a');
  mesh.getColorAt(indexOf('beam-a'), color);
  assert.equal(color.getHexString(), 'fbbf24');
  mesh.getColorAt(indexOf('beam-b'), color);
  assert.equal(color.getHexString(), '60a5fa');
  built.setSelected(null);
  mesh.getColorAt(indexOf('beam-a'), color);
  assert.equal(color.getHexString(), 'a78bfa');
  built.dispose();
});

test('animation advances spinners and pulses the collision marker', () => {
  const scene = deriveVisualizationScene(fixtureDoc(), 'artifact-1', 3);
  const built = buildThreeScene(scene, 640, 480, {
    createLabelSprite: () => stubSprite(),
  });
  built.animate(0);
  let spinnerRotation = 0;
  built.scene.traverse(object => {
    if (
      object instanceof THREE.Group &&
      object.children.some(c => c instanceof THREE.InstancedMesh)
    ) {
      spinnerRotation = object.rotation.z;
    }
  });
  built.animate(2);
  let laterRotation = 0;
  built.scene.traverse(object => {
    if (
      object instanceof THREE.Group &&
      object.children.some(c => c instanceof THREE.InstancedMesh)
    ) {
      laterRotation = object.rotation.z;
    }
  });
  assert.notEqual(laterRotation, spinnerRotation);
  built.dispose();
});

test('disposal releases geometries, materials, and textures', () => {
  const scene = deriveVisualizationScene(fixtureDoc(), 'artifact-1', 3);
  const built = buildThreeScene(scene, 640, 480, {
    createLabelSprite: () => stubSprite(),
  });
  let geometries = 0;
  let materials = 0;
  let textures = 0;
  const geometryDispose = THREE.BufferGeometry.prototype.dispose;
  const materialDispose = THREE.Material.prototype.dispose;
  const textureDispose = THREE.Texture.prototype.dispose;
  THREE.BufferGeometry.prototype.dispose = function () {
    geometries++;
    return geometryDispose.call(this);
  };
  THREE.Material.prototype.dispose = function () {
    materials++;
    return materialDispose.call(this);
  };
  THREE.Texture.prototype.dispose = function () {
    textures++;
    return textureDispose.call(this);
  };
  try {
    built.dispose();
  } finally {
    THREE.BufferGeometry.prototype.dispose = geometryDispose;
    THREE.Material.prototype.dispose = materialDispose;
    THREE.Texture.prototype.dispose = textureDispose;
  }
  // Geometries: 2 rings + 2 particle sets + fragments +
  // collision core + glow + edges + node spheres + 25
  // sprite materials share one stub material each.
  assert.ok(geometries >= 9, `geometries: ${geometries}`);
  assert.ok(materials >= 9, `materials: ${materials}`);
  assert.ok(textures >= 25, `textures: ${textures}`);
});

test('without a canvas, label sprites are skipped, not fatal', () => {
  const scene = deriveVisualizationScene(fixtureDoc(), 'artifact-1', 3);
  const built = buildThreeScene(scene, 640, 480);
  assert.equal(built.counts.sprites, 0);
  assert.equal(built.counts.nodes, 25);
  built.dispose();
});

test('new Three.js files never assign markup', () => {
  for (const file of [
    'packages/ui/src/colliderThreeScene.ts',
    'packages/ui/src/ColliderThreeView.tsx',
  ]) {
    const src = readFileSync(file, 'utf8');
    assert.ok(!/\.innerHTML/.test(src), file);
    assert.ok(!/dangerouslySetInnerHTML/.test(src), file);
  }
});
