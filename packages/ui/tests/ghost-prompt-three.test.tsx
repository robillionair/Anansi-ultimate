// M11 GHOST-PROMPT Slice 2: the 3D layout is a deterministic function of
// the session snapshot, the scene builder picks and disposes headlessly
// (it creates no renderer), reduced motion and WebGL detection behave
// through their probes, and the component renders the canvas or the
// static 2D view — with zero innerHTML in the new files and three
// pinned to 0.187.0.

import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { renderToStaticMarkup } from 'react-dom/server';
import React from 'react';
import * as THREE from 'three';
import { GhostSession } from '../src/ghostPrompt.ts';
import {
  GHOST_CAMERA_DISTANCE,
  GHOST_CAMERA_FOV,
  GHOST_CAMERA_HEIGHT,
  ghostProjectToNdc,
  layoutGhostDraft,
  type GhostDraftSnapshot,
} from '../src/ghostPromptThreeLayout.ts';
import {
  buildGhostScene,
  detectWebGL,
  isReducedMotion,
} from '../src/ghostPromptThreeScene.ts';
import { GhostPrompt3D } from '../src/GhostPrompt3D.tsx';

function snapshot(): GhostDraftSnapshot {
  const session = GhostSession.create('ghost-three', { id: 'ask-prompt', isPassword: false });
  assert.ok(session);
  session.enable();
  session.record('', 'red fox jumps', 'insert', 0);
  session.record('red fox jumps', 'blue fox jumps', 'replace', 3000);
  session.record('blue fox jumps', 'blue jumps', null, 3500);
  return {
    currentText: session.currentText,
    fragments: session.deletedFragments(),
    pauses: session.pauses,
    events: session.events,
  };
}

test('ghost 3D layout is deterministic and maps offsets to the baseline', () => {
  const first = layoutGhostDraft(snapshot());
  const second = layoutGhostDraft(snapshot());
  assert.deepEqual(first, second);
  assert.equal(first.maxOffset, 10);
  assert.equal(first.nodes.length, 2);
  assert.equal(first.nodes[0].id, 'fragment-2');
  assert.equal(first.nodes[0].x, -200);
  assert.equal(first.nodes[0].y, 50);
  assert.equal(first.nodes[0].row, 0);
  // The full-text delete maximizes the shared prefix ('blue '), and the
  // non-overlap rule caps the suffix at 5, so the recorded fragment is
  // 'fox ' at offset 5, not ' fox' at 4.
  assert.equal(first.nodes[1].text, 'fox ');
  assert.equal(first.nodes[1].offset, 5);
  assert.equal(first.nodes[1].x, 0);
  assert.equal(first.nodes[1].y, 72);
  assert.equal(first.nodes[1].row, 1);
  assert.deepEqual(first.threads[0], {
    nodeSeq: 2,
    fromX: -200,
    fromY: 50,
    toX: -200,
    toY: 0,
  });
  assert.equal(first.pauses.length, 1);
  assert.equal(first.pauses[0].durationMs, 3000);
  assert.equal(first.pauses[0].x, -200);
  assert.equal(first.clusters.length, 1);
  assert.deepEqual(first.clusters[0].seqs, [1, 2, 3]);
  assert.equal(first.clusters[0].minX, -200);
  assert.equal(first.clusters[0].maxX, 0);
});

test('ghost 3D layout excludes single-edit runs from clusters', () => {
  const far = layoutGhostDraft({
    currentText: 'x'.repeat(120),
    fragments: [],
    pauses: [],
    events: [
      { seq: 1, kind: 'insert', offset: 0, deleted: '', inserted: 'a', textLenAfter: 1, deletedHash: '', elapsedMs: 0 },
      { seq: 2, kind: 'insert', offset: 100, deleted: '', inserted: 'b', textLenAfter: 2, deletedHash: '', elapsedMs: 100 },
    ],
  });
  assert.equal(far.clusters.length, 0);
});

test('ghost projection matches the fixed camera formula', () => {
  const center = ghostProjectToNdc(0, GHOST_CAMERA_HEIGHT, 16 / 9);
  assert.deepEqual(center, { ndcX: 0, ndcY: 0 });
  const left = ghostProjectToNdc(-200, GHOST_CAMERA_HEIGHT, 16 / 9);
  const tanHalf = Math.tan(((GHOST_CAMERA_FOV / 2) * Math.PI) / 180);
  const expected = -200 / (tanHalf * GHOST_CAMERA_DISTANCE * (16 / 9));
  assert.ok(Math.abs(left.ndcX - expected) < 1e-9);
  assert.ok(left.ndcX < -0.84 && left.ndcX > -0.86);
});

test('ghost scene picks, selects, and animates only the pause rings', () => {
  const built = buildGhostScene(snapshot());
  assert.deepEqual(built.counts, { nodes: 2, threads: 2, pauses: 1, clusters: 1 });
  built.camera.aspect = 16 / 9;
  built.camera.updateProjectionMatrix();
  const target = built.positions.get(2)!;
  const projected = target.clone().project(built.camera);
  assert.equal(built.pick(projected.x, projected.y), 2);
  assert.equal(built.pick(0.99, 0.99), null);
  built.setSelected(2);
  assert.equal(built.nodeMeshes.get(2)!.scale.x, 1.5);
  assert.equal(built.nodeMeshes.get(3)!.scale.x, 1);
  assert.equal(built.selectedSeq, 2);
  built.setSelected(null);
  assert.equal(built.nodeMeshes.get(2)!.scale.x, 1);
  const rings = built.scene.children.filter((child) => child.type === 'Group');
  assert.equal(rings.length, 1);
  const before = rings[0].scale.x;
  built.animate(before === 1 ? 5 : 0);
  assert.notEqual(rings[0].scale.x, before);
  built.dispose();
});

test('ghost scene disposal fires once per geometry and material', () => {
  const built = buildGhostScene(snapshot());
  const seen = new Set<object>();
  const fired: object[] = [];
  built.scene.traverse((object) => {
    const mesh = object as THREE.Mesh;
    if (mesh.geometry && !seen.has(mesh.geometry)) {
      seen.add(mesh.geometry);
      mesh.geometry.addEventListener('dispose', () => fired.push(mesh.geometry));
    }
    const material = mesh.material as THREE.Material | undefined;
    if (material && !seen.has(material)) {
      seen.add(material);
      material.addEventListener('dispose', () => fired.push(material));
    }
  });
  const unique = seen.size;
  assert.ok(unique > 0);
  built.dispose();
  assert.equal(fired.length, unique);
  assert.doesNotThrow(() => built.dispose());
  assert.equal(fired.length, unique);
});

test('ghost capability probes behave without a browser', () => {
  assert.equal(
    detectWebGL(() => {
      throw Error('no canvas');
    }),
    false,
  );
  assert.equal(detectWebGL(() => null), false);
  assert.equal(isReducedMotion({ mediaMatches: true }), true);
  assert.equal(isReducedMotion({ datasetReducedMotion: 'true' }), true);
  assert.equal(isReducedMotion({ mediaMatches: false, datasetReducedMotion: 'false' }), false);
  assert.equal(isReducedMotion(), false);
});

test('ghost 3D component renders canvas or the static 2D view', () => {
  const snap = snapshot();
  const animated = renderToStaticMarkup(<GhostPrompt3D snapshot={snap} />);
  assert.ok(animated.includes('data-three-canvas'));
  assert.ok(animated.includes('Reduce motion (static view)'));
  assert.ok(!animated.includes('Motion reduced — static view.'));
  const staticHtml = renderToStaticMarkup(
    <GhostPrompt3D snapshot={snap} initialReducedMotion />,
  );
  assert.ok(staticHtml.includes('Motion reduced — static view.'));
  assert.ok(staticHtml.includes('<svg'));
  assert.ok(staticHtml.includes('<ol'));
  assert.ok(staticHtml.includes('Edit 2: replace at offset 0'));
  assert.ok(staticHtml.includes('Pause of 3000ms before edit 2.'));
  assert.ok(staticHtml.includes('Select fragment red'));
});

test('ghost 3D files carry no innerHTML and three stays pinned', () => {
  for (const file of [
    '../src/ghostPromptThreeLayout.ts',
    '../src/ghostPromptThreeScene.ts',
    '../src/GhostPrompt3D.tsx',
  ]) {
    const source = readFileSync(new URL(file, import.meta.url), 'utf8');
    assert.ok(!source.includes('innerHTML'), file);
  }
  const uiPackage = JSON.parse(
    readFileSync(new URL('../package.json', import.meta.url), 'utf8'),
  ) as { dependencies: Record<string, string> };
  assert.equal(uiPackage.dependencies.three, '0.187.0');
});
