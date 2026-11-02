/**
 * M11 GHOST-PROMPT Slice 2: build a WebGL scene graph from the pure
 * draft layout (see ghostPromptThreeLayout). This module is pure with
 * respect to data — no I/O, no API calls, no randomness — and it creates
 * no renderer: the component owns the WebGLRenderer lifecycle. Because
 * scene-graph construction, raycasting math, and disposal need no GL
 * context, node tests can build, pick, and dispose this scene headlessly.
 *
 * Visual language (MASTER_SPEC §56, §65): a crisp white baseline
 * carrying the retained text, translucent violet deleted nodes floating
 * above their edit offsets, subtle thread segments down to the baseline,
 * restrained concentric ripple rings where pauses occurred, and thin
 * underline bars for revision clusters. No particles, no camera motion;
 * the only animation is a slow breathing of the pause rings.
 */

import * as THREE from 'three';
import {
  detectWebGL,
  isReducedMotion,
} from './colliderThreeScene.ts';
import {
  GHOST_BASELINE_HALF_WIDTH,
  GHOST_CAMERA_DISTANCE,
  GHOST_CAMERA_FOV,
  GHOST_CAMERA_HEIGHT,
  layoutGhostDraft,
  type GhostDraftLayout,
  type GhostDraftSnapshot,
} from './ghostPromptThreeLayout.ts';

export { detectWebGL, isReducedMotion };

export const GHOST_NODE_COLOR = '#8b5cf6';
export const GHOST_NODE_RADIUS = 8;
export const GHOST_BASELINE_COLOR = '#f5f5f4';
export const GHOST_SELECTED_SCALE = 1.5;

export type BuiltGhostScene = {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  /** Fragment seq -> node mesh, in layout order. */
  nodeMeshes: Map<number, THREE.Mesh>;
  /** Fragment seq -> world position (same layout as the 2D view). */
  positions: Map<number, THREE.Vector3>;
  counts: {
    nodes: number;
    threads: number;
    pauses: number;
    clusters: number;
  };
  /** Raycast pick: NDC coords -> fragment seq or null. */
  pick: (ndcX: number, ndcY: number) => number | null;
  setSelected: (seq: number | null) => void;
  /** Advance the pause-ring breathing to absolute time t (seconds). */
  animate: (t: number) => void;
  /** Dispose every geometry, material, and texture in the scene. */
  dispose: () => void;
  layout: GhostDraftLayout;
  selectedSeq: number | null;
};

const THREAD_COLOR = '#a78bfa';
const CLUSTER_COLOR = '#7c3aed';
const RING_COLOR = '#c4b5fd';

export function buildGhostScene(snapshot: GhostDraftSnapshot): BuiltGhostScene {
  const layout = layoutGhostDraft(snapshot);
  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(
    GHOST_CAMERA_FOV,
    1,
    1,
    2000,
  );
  camera.position.set(0, GHOST_CAMERA_HEIGHT, GHOST_CAMERA_DISTANCE);
  camera.lookAt(0, GHOST_CAMERA_HEIGHT, 0);
  camera.updateMatrixWorld();

  const disposables: Array<{ dispose: () => void }> = [];
  const track = <T extends { dispose: () => void }>(value: T): T => {
    disposables.push(value);
    return value;
  };

  // Baseline: crisp white bar, 400 wide.
  const baselineGeometry = track(new THREE.BoxGeometry(GHOST_BASELINE_HALF_WIDTH * 2, 1, 1));
  const baselineMaterial = track(
    new THREE.MeshBasicMaterial({ color: GHOST_BASELINE_COLOR }),
  );
  const baseline = new THREE.Mesh(baselineGeometry, baselineMaterial);
  baseline.position.set(0, 0, 0);
  scene.add(baseline);

  // Deleted nodes: one translucent violet sphere per fragment.
  const nodeGeometry = track(new THREE.SphereGeometry(GHOST_NODE_RADIUS, 24, 16));
  const nodeMeshes = new Map<number, THREE.Mesh>();
  const positions = new Map<number, THREE.Vector3>();
  for (const node of layout.nodes) {
    const material = track(
      new THREE.MeshBasicMaterial({
        color: GHOST_NODE_COLOR,
        transparent: true,
        opacity: 0.55,
      }),
    );
    const mesh = new THREE.Mesh(nodeGeometry, material);
    mesh.position.set(node.x, node.y, 0);
    mesh.userData.fragmentSeq = node.seq;
    scene.add(mesh);
    nodeMeshes.set(node.seq, mesh);
    positions.set(node.seq, new THREE.Vector3(node.x, node.y, 0));
  }

  // Threads: a single line segments object for all nodes.
  if (layout.threads.length > 0) {
    const points: number[] = [];
    for (const thread of layout.threads) {
      points.push(thread.fromX, thread.fromY, 0, thread.toX, thread.toY, 0);
    }
    const threadGeometry = track(new THREE.BufferGeometry());
    threadGeometry.setAttribute(
      'position',
      new THREE.Float32BufferAttribute(points, 3),
    );
    const threadMaterial = track(
      new THREE.LineBasicMaterial({ color: THREAD_COLOR, transparent: true, opacity: 0.7 }),
    );
    scene.add(new THREE.LineSegments(threadGeometry, threadMaterial));
  }

  // Pause markers: two shared ring geometries, one pair per marker.
  const ringInner = track(new THREE.RingGeometry(5, 6, 32));
  const ringOuter = track(new THREE.RingGeometry(9, 10, 32));
  const ringMaterial = track(
    new THREE.MeshBasicMaterial({
      color: RING_COLOR,
      transparent: true,
      opacity: 0.8,
      side: THREE.DoubleSide,
    }),
  );
  const ringGroups: THREE.Group[] = [];
  for (const pause of layout.pauses) {
    const group = new THREE.Group();
    const inner = new THREE.Mesh(ringInner, ringMaterial);
    const outer = new THREE.Mesh(ringOuter, ringMaterial);
    group.add(inner, outer);
    group.position.set(pause.x, 0.5, 0);
    // Rings face the frontal camera (XY plane): a top-down lay-flat
    // would read as a thin line from this camera.
    group.userData.phase = pause.elapsedMs / 1000;
    scene.add(group);
    ringGroups.push(group);
  }

  // Cluster bars: one thin box per cluster spanning its x-range.
  for (const cluster of layout.clusters) {
    const width = Math.max(4, cluster.maxX - cluster.minX);
    const barGeometry = track(new THREE.BoxGeometry(width, 1, 1));
    const barMaterial = track(
      new THREE.MeshBasicMaterial({ color: CLUSTER_COLOR, transparent: true, opacity: 0.6 }),
    );
    const bar = new THREE.Mesh(barGeometry, barMaterial);
    bar.position.set((cluster.minX + cluster.maxX) / 2, 0.5, 0);
    scene.add(bar);
  }

  let selectedSeq: number | null = null;
  const raycaster = new THREE.Raycaster();
  const pointer = new THREE.Vector2();
  const pick = (ndcX: number, ndcY: number): number | null => {
    // Headless callers never render, so refresh world matrices here;
    // the component's render loop does the same every frame.
    scene.updateMatrixWorld(true);
    pointer.set(ndcX, ndcY);
    raycaster.setFromCamera(pointer, camera);
    const hits = raycaster.intersectObjects([...nodeMeshes.values()], false);
    if (hits.length === 0) return null;
    const hit = hits[0].object as THREE.Mesh;
    return (hit.userData.fragmentSeq as number) ?? null;
  };
  const setSelected = (seq: number | null): void => {
    selectedSeq = seq;
    for (const [nodeSeq, mesh] of nodeMeshes) {
      const scale = nodeSeq === seq ? GHOST_SELECTED_SCALE : 1;
      mesh.scale.setScalar(scale);
    }
  };
  const animate = (t: number): void => {
    for (const group of ringGroups) {
      const phase = group.userData.phase as number;
      const scale = 1 + 0.08 * Math.sin(t * 0.8 + phase);
      group.scale.setScalar(scale);
    }
  };
  let disposed = false;
  const dispose = (): void => {
    if (disposed) return;
    disposed = true;
    const seen = new Set<object>();
    for (const disposable of disposables) {
      if (!seen.has(disposable)) {
        seen.add(disposable);
        disposable.dispose();
      }
    }
  };

  return {
    scene,
    camera,
    nodeMeshes,
    positions,
    counts: {
      nodes: layout.nodes.length,
      threads: layout.threads.length,
      pauses: layout.pauses.length,
      clusters: layout.clusters.length,
    },
    pick,
    setSelected,
    animate,
    dispose,
    layout,
    get selectedSeq() {
      return selectedSeq;
    },
  };
}
