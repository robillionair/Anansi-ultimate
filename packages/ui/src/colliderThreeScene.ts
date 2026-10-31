/**
 * M8 Three.js visualization: build a WebGL scene graph
 * from the exact derived visualization scene (see
 * colliderVisualizationScene). This module is pure with
 * respect to data — no I/O, no API calls, no randomness
 * — and it creates no renderer: the component owns the
 * WebGLRenderer lifecycle. Because scene-graph
 * construction, raycasting math, and disposal need no GL
 * context, node tests can build, pick, and dispose this
 * scene headlessly.
 *
 * Visual language (MASTER_SPEC §26): dark navy field
 * #0b2545, Beam A violet #a78bfa, Beam B blue #60a5fa,
 * restrained white #f5f5f4. Two toroidal particle
 * streams (one per beam, opposite senses), concept
 * fragments on inclined orbits, a central collision
 * marker, recombined output nodes placed from the same
 * deterministic layout as the Canvas view, and edges
 * that are exactly the derived section-flow edges —
 * never text-mined links. Labels are text sprites
 * (canvas-generated textures): saved text is drawn,
 * never executed, and never reaches innerHTML.
 */

import * as THREE from 'three';
import {
  layoutVisualizationScene,
  type VisualizationScene,
} from './colliderVisualizationScene';

export const THREE_FIELD = '#0b2545';
const BEAM_COLORS: Record<string, string> = {
  'beam-a': '#a78bfa',
  'beam-b': '#60a5fa',
};
const HUB_COLOR = '#e7e5e4';
const ITEM_COLOR = '#c4b5fd';
const EDGE_COLOR = '#57534e';
const LABEL_COLOR = '#f5f5f4';
const SELECT_COLOR = '#fbbf24';
const CAMERA_DISTANCE = 900;
const CAMERA_FOV = 45;

const PARTICLES_PER_RING = 120;
const FRAGMENT_COUNT = 64;

/** A canvas factory, injectable so tests can drive the outcomes. */
export type WebGLCanvasFactory = () => {
  getContext: (kind: string) => unknown;
} | null;

function defaultCanvasFactory(): ReturnType<WebGLCanvasFactory> {
  if (typeof document === 'undefined') return null;
  return document.createElement('canvas');
}

/**
 * Graceful capability detection: true only when a
 * webgl2 or webgl context can actually be created. A
 * throwing factory or a null context means "no WebGL",
 * never an exception to the caller.
 */
export function detectWebGL(
  createCanvas: WebGLCanvasFactory = defaultCanvasFactory,
): boolean {
  let canvas: ReturnType<WebGLCanvasFactory>;
  try {
    canvas = createCanvas();
  } catch {
    return false;
  }
  if (!canvas) return false;
  for (const kind of ['webgl2', 'webgl']) {
    try {
      if (canvas.getContext(kind)) return true;
    } catch {
      // Try the next context kind.
    }
  }
  return false;
}

export type ReducedMotionProbe = {
  mediaMatches?: boolean;
  datasetReducedMotion?: string | null;
};

/**
 * Reduced motion is true when the system media query
 * matches OR the workspace setting says so (the app
 * writes the workspace preference to
 * document.documentElement.dataset.reducedMotion while
 * a workspace is open). With no arguments the live
 * globals are read; tests pass a probe instead.
 */
export function isReducedMotion(probe?: ReducedMotionProbe): boolean {
  if (probe) {
    return (
      probe.mediaMatches === true || probe.datasetReducedMotion === 'true'
    );
  }
  if (typeof window !== 'undefined' && window.matchMedia) {
    try {
      if (
        window.matchMedia('(prefers-reduced-motion: reduce)').matches
      ) {
        return true;
      }
    } catch {
      // Fall through to the dataset check.
    }
  }
  if (typeof document !== 'undefined') {
    return document.documentElement.dataset.reducedMotion === 'true';
  }
  return false;
}

export type BuiltThreeScene = {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  /** Node id -> world position (same layout as the Canvas view). */
  positions: Map<string, THREE.Vector3>;
  /** Instance index -> node id, in nodeMesh order. */
  nodeIds: string[];
  counts: {
    rings: number;
    particles: number;
    fragments: number;
    nodes: number;
    edges: number;
    sprites: number;
  };
  /** Raycast pick: NDC coords -> node id or null. */
  pick: (ndcX: number, ndcY: number) => string | null;
  setSelected: (id: string | null) => void;
  /** Advance the orbital animation to absolute time t (seconds). */
  animate: (t: number) => void;
  /** Dispose every geometry, material, and texture in the scene. */
  dispose: () => void;
};

function labelTexture(label: string): THREE.Texture | null {
  if (typeof document === 'undefined') return null;
  const text = label.length > 18 ? `${label.slice(0, 17)}…` : label;
  const canvas = document.createElement('canvas');
  const ctx = canvas.getContext('2d');
  if (!ctx) return null;
  ctx.font = '20px sans-serif';
  const textWidth = Math.ceil(ctx.measureText(text).width);
  canvas.width = textWidth + 16;
  canvas.height = 30;
  ctx.font = '20px sans-serif';
  ctx.fillStyle = 'rgba(11, 37, 69, 0.85)';
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  ctx.fillStyle = LABEL_COLOR;
  ctx.fillText(text, 8, 21);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

export type ThreeSceneOptions = {
  /**
   * Override sprite creation (tests inject a stub; the
   * default draws the label on a canvas texture). A null
   * return skips that node's sprite without failing.
   */
  createLabelSprite?: (
    label: string,
    position: THREE.Vector3,
  ) => THREE.Sprite | null;
};

function defaultLabelSprite(
  label: string,
  position: THREE.Vector3,
): THREE.Sprite | null {
  const texture = labelTexture(label);
  if (!texture) return null;
  const sprite = new THREE.Sprite(
    new THREE.SpriteMaterial({ map: texture, transparent: true, depthWrite: false }),
  );
  // The texture canvas is ~30px tall; scale keeps labels
  // legible without dwarfing the nodes.
  const image = texture.image as unknown as {
    width: number;
    height: number;
  } | null;
  const aspect = image
    ? image.width / Math.max(image.height, 1)
    : 4;
  sprite.scale.set(10 * aspect, 10, 1);
  sprite.position.copy(position);
  return sprite;
}

/**
 * Build the scene for one pinned version. Torus rings
 * and fragment orbits live in tilted spinner groups so
 * animate(t) rotates the streams without moving the
 * layout; node spheres sit on the layout plane at z 0.
 */
export function buildThreeScene(
  scene: VisualizationScene,
  width: number,
  height: number,
  options: ThreeSceneOptions = {},
): BuiltThreeScene {
  const createLabelSprite = options.createLabelSprite ?? defaultLabelSprite;
  const layout = layoutVisualizationScene(scene, width, height);
  const world = (id: string): THREE.Vector3 | null => {
    const pos = layout.positions.get(id);
    if (!pos) return null;
    return new THREE.Vector3(pos.x - width / 2, height / 2 - pos.y, 0);
  };

  const threeScene = new THREE.Scene();
  threeScene.background = new THREE.Color(THREE_FIELD);
  const camera = new THREE.PerspectiveCamera(
    CAMERA_FOV,
    width / Math.max(height, 1),
    1,
    5000,
  );
  camera.position.set(0, 0, CAMERA_DISTANCE);
  camera.lookAt(0, 0, 0);

  const positions = new Map<string, THREE.Vector3>();
  for (const node of scene.nodes) {
    const pos = world(node.id);
    if (pos) positions.set(node.id, pos);
  }

  // Toroidal particle streams: one tilted ring per beam,
  // particles instanced along the ring, opposite senses.
  const streamDefs = [
    { beam: 'beam-a', radius: 220, tiltX: 1.1, tiltY: 0.3, speed: 0.25 },
    { beam: 'beam-b', radius: 180, tiltX: -0.9, tiltY: -0.4, speed: -0.18 },
  ];
  const spinners: { group: THREE.Group; speed: number }[] = [];
  for (const def of streamDefs) {
    const tilt = new THREE.Group();
    tilt.rotation.set(def.tiltX, def.tiltY, 0);
    const spinner = new THREE.Group();
    const ring = new THREE.Mesh(
      new THREE.TorusGeometry(def.radius, 3, 12, 96),
      new THREE.MeshBasicMaterial({
        color: BEAM_COLORS[def.beam] ?? HUB_COLOR,
        transparent: true,
        opacity: 0.55,
      }),
    );
    spinner.add(ring);
    const particles = new THREE.InstancedMesh(
      new THREE.SphereGeometry(4, 8, 8),
      new THREE.MeshBasicMaterial({
        color: BEAM_COLORS[def.beam] ?? HUB_COLOR,
      }),
      PARTICLES_PER_RING,
    );
    const matrix = new THREE.Matrix4();
    for (let i = 0; i < PARTICLES_PER_RING; i++) {
      const angle = (i / PARTICLES_PER_RING) * Math.PI * 2;
      matrix.makeTranslation(
        def.radius * Math.cos(angle),
        def.radius * Math.sin(angle),
        0,
      );
      particles.setMatrixAt(i, matrix);
    }
    particles.instanceMatrix.needsUpdate = true;
    spinner.add(particles);
    tilt.add(spinner);
    threeScene.add(tilt);
    spinners.push({ group: spinner, speed: def.speed });
  }

  // Orbital concept fragments on an inclined orbit.
  const fragmentTilt = new THREE.Group();
  fragmentTilt.rotation.set(0.5, 0.9, 0);
  const fragmentSpinner = new THREE.Group();
  const fragments = new THREE.InstancedMesh(
    new THREE.OctahedronGeometry(6),
    new THREE.MeshBasicMaterial({ color: ITEM_COLOR }),
    FRAGMENT_COUNT,
  );
  {
    const matrix = new THREE.Matrix4();
    for (let i = 0; i < FRAGMENT_COUNT; i++) {
      const angle = (i / FRAGMENT_COUNT) * Math.PI * 2;
      matrix.makeTranslation(
        260 * Math.cos(angle),
        260 * Math.sin(angle),
        0,
      );
      fragments.setMatrixAt(i, matrix);
    }
    fragments.instanceMatrix.needsUpdate = true;
  }
  fragmentSpinner.add(fragments);
  fragmentTilt.add(fragmentSpinner);
  threeScene.add(fragmentTilt);
  spinners.push({ group: fragmentSpinner, speed: 0.12 });

  // Central collision marker: solid core plus a soft glow.
  const collisionGroup = new THREE.Group();
  const core = new THREE.Mesh(
    new THREE.SphereGeometry(14, 24, 24),
    new THREE.MeshBasicMaterial({ color: LABEL_COLOR }),
  );
  const glow = new THREE.Mesh(
    new THREE.SphereGeometry(26, 24, 24),
    new THREE.MeshBasicMaterial({
      color: LABEL_COLOR,
      transparent: true,
      opacity: 0.22,
      depthWrite: false,
    }),
  );
  collisionGroup.add(core, glow);
  threeScene.add(collisionGroup);

  // Section-flow edges between hub positions.
  const edgePoints: number[] = [];
  for (const edge of scene.edges) {
    const from = world(edge.from);
    const to = world(edge.to);
    if (!from || !to) continue;
    edgePoints.push(from.x, from.y, from.z, to.x, to.y, to.z);
  }
  const edgeGeometry = new THREE.BufferGeometry();
  edgeGeometry.setAttribute(
    'position',
    new THREE.Float32BufferAttribute(edgePoints, 3),
  );
  const edgeLines = new THREE.LineSegments(
    edgeGeometry,
    new THREE.LineBasicMaterial({
      color: EDGE_COLOR,
      transparent: true,
      opacity: 0.8,
    }),
  );
  threeScene.add(edgeLines);

  // Output nodes: one instanced sphere per node, colored
  // by section, sized by role, selectable by raycast.
  const nodeIds: string[] = [];
  const baseColors: THREE.Color[] = [];
  const nodeMesh = new THREE.InstancedMesh(
    new THREE.SphereGeometry(1, 16, 16),
    new THREE.MeshBasicMaterial({ color: 0xffffff }),
    scene.nodes.length,
  );
  {
    const matrix = new THREE.Matrix4();
    const color = new THREE.Color();
    scene.nodes.forEach((node, i) => {
      const pos = positions.get(node.id);
      const layoutPos = layout.positions.get(node.id);
      if (!pos || !layoutPos) return;
      matrix.makeScale(layoutPos.r, layoutPos.r, layoutPos.r);
      matrix.setPosition(pos.x, pos.y, pos.z);
      nodeMesh.setMatrixAt(i, matrix);
      const hex =
        node.section === 'beam-a' || node.section === 'beam-b'
          ? (BEAM_COLORS[node.section] ?? HUB_COLOR)
          : node.role === 'section'
            ? HUB_COLOR
            : ITEM_COLOR;
      color.set(hex);
      nodeMesh.setColorAt(i, color);
      baseColors.push(color.clone());
      nodeIds.push(node.id);
    });
    nodeMesh.instanceMatrix.needsUpdate = true;
    if (nodeMesh.instanceColor) nodeMesh.instanceColor.needsUpdate = true;
    nodeMesh.computeBoundingSphere();
  }
  threeScene.add(nodeMesh);

  // Label sprites above each node (skipped when no canvas
  // is available, e.g. headless node tests).
  let spriteCount = 0;
  for (const node of scene.nodes) {
    const pos = positions.get(node.id);
    const layoutPos = layout.positions.get(node.id);
    if (!pos || !layoutPos) continue;
    const sprite = createLabelSprite(
      node.label,
      new THREE.Vector3(pos.x, pos.y + layoutPos.r + 16, pos.z),
    );
    if (!sprite) continue;
    threeScene.add(sprite);
    spriteCount++;
  }

  const raycaster = new THREE.Raycaster();
  const pick = (ndcX: number, ndcY: number): string | null => {
    camera.updateMatrixWorld();
    raycaster.setFromCamera(new THREE.Vector2(ndcX, ndcY), camera);
    const hits = raycaster.intersectObject(nodeMesh, false);
    const first = hits[0];
    if (!first || first.instanceId === undefined) return null;
    return nodeIds[first.instanceId] ?? null;
  };

  const setSelected = (id: string | null) => {
    const color = new THREE.Color();
    nodeIds.forEach((nodeId, i) => {
      color.copy(baseColors[i] ?? new THREE.Color(ITEM_COLOR));
      if (nodeId === id) color.set(SELECT_COLOR);
      nodeMesh.setColorAt(i, color);
    });
    if (nodeMesh.instanceColor) nodeMesh.instanceColor.needsUpdate = true;
  };

  const animate = (t: number) => {
    for (const spinner of spinners) {
      spinner.group.rotation.z = t * spinner.speed;
    }
    const pulse = 1 + 0.12 * Math.sin(t * 2);
    collisionGroup.scale.setScalar(pulse);
  };

  const dispose = () => {
    threeScene.traverse(object => {
      const mesh = object as THREE.Mesh;
      if (mesh.geometry) mesh.geometry.dispose();
      const materials = Array.isArray(mesh.material)
        ? mesh.material
        : mesh.material
          ? [mesh.material]
          : [];
      for (const material of materials) {
        const withMap = material as THREE.Material & { map?: THREE.Texture | null };
        if (withMap.map) withMap.map.dispose();
        material.dispose();
      }
    });
  };

  return {
    scene: threeScene,
    camera,
    positions,
    nodeIds,
    counts: {
      rings: streamDefs.length,
      particles: streamDefs.length * PARTICLES_PER_RING,
      fragments: FRAGMENT_COUNT,
      nodes: nodeIds.length,
      edges: edgePoints.length / 6,
      sprites: spriteCount,
    },
    pick,
    setSelected,
    animate,
    dispose,
  };
}
