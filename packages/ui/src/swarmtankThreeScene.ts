import { buildSwarmtankWiring } from './larp-mode/swarmtankWiring';
import { buildSwarmtankSet } from './larp-mode/swarmtankSet';
/**
 * M9 SwarmTank Slice 2: build a WebGL scene graph
 * from the pure arena layout (see
 * swarmtankArenaLayout). This module is pure with
 * respect to data — no I/O, no API calls, no
 * randomness — and it creates no renderer: the
 * component owns the WebGLRenderer lifecycle.
 * Because scene-graph construction, raycasting math,
 * and disposal need no GL context, node tests can
 * build, pick, and dispose this scene headlessly.
 *
 * Visual language (MASTER_SPEC §38, §65): dark navy
 * field #0b2545 (shared with §26), a central product
 * monolith on a circular stage, concentric
 * amphitheater rings, one instanced agent per persona
 * — Convert green #22c55e closest to the pedestal
 * facing it, Confused amber #f59e0b orbiting the
 * perimeter tangentially with hesitation markers,
 * Reject red #ef4444 outermost facing away. Agent
 * facing is a flat nose cone; bodies alone carry no
 * orientation. Labels are canvas text sprites: saved
 * text is drawn, never executed, and never inserted
 * as markup. Only the pedestal carries a label —
 * per-agent text lives in the detail card and the
 * table, not in a texture per agent.
 *
 * M9 SwarmTank Slice 3: instance slots stay in
 * initial layout order (persona-stable) and
 * updateDecisions retargets them — colors and
 * marker visibility follow the target decision
 * immediately, positions and facings interpolate
 * over 600 ms with easeInOutCubic. Ring lines for
 * every radius the layout can produce are prebuilt
 * and toggled visible, so a transition creates or
 * destroys no geometry. Markers are one mesh sized
 * for all slots, ordered Confused-first in slot
 * order; unused slots collapse to scale 0.
 */

import * as THREE from 'three';
import {
  detectWebGL,
  isReducedMotion,
} from './colliderThreeScene';
import {
  layoutSwarmtankArena,
  type ArenaAgent,
  type ArenaLayout,
} from './swarmtankArenaLayout';

export { detectWebGL, isReducedMotion };

export const ARENA_FIELD = '#0b2545';
export const ARENA_CAMERA_HEIGHT = 640;
export const ARENA_CAMERA_FOV = 45;
export const ARENA_VIEW_HEIGHT = 480;
const STAGE_COLOR = '#d6d3d1';
const MONOLITH_COLOR = '#f5f5f4';
const RING_COLOR = '#57534e';
const LABEL_COLOR = '#f5f5f4';
const DECISION_COLORS: Record<string, string> = {
  Convert: '#22c55e',
  Confused: '#f59e0b',
  Reject: '#ef4444',
};
const MARKER_COLOR = '#fde68a';
const BODY_RADIUS = 5;
const BODY_Y = 6;
const NOSE_OFFSET = 8;
const MARKER_BASE_Y = 18;
const ORBIT_SPEED = 0.05;
const TRANSITION_SECONDS = 0.6;

export type BuiltArenaScene = {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  layout: ArenaLayout;
  /** Persona ids in bodies-mesh instance order. */
  agentIds: string[];
  counts: {
    agents: number;
    rings: number;
    markers: number;
  };
  /** Raycast pick: NDC coords -> persona id or null. */
  pick: (ndcX: number, ndcY: number) => string | null;
  /** Scale the selected body to 1.6x; null restores all. */
  setSelected: (id: string | null) => void;
  /** Advance the Confused orbit and marker bob to time t (seconds). */
  animate: (t: number) => void;
  /**
   * Retarget every slot to a new layout (same
   * personas, new decisions). Colors and markers
   * follow immediately; positions interpolate.
   * Pass instant = true to snap (reduced motion).
   */
  updateDecisions: (nextLayout: ArenaLayout, instant: boolean) => void;
  /** Advance the transition clock by dt seconds. Returns progress 0..1. */
  advanceTransition: (dt: number) => number;
  /** Dispose every geometry, material, and texture in the scene. */
  dispose: () => void;
};

function labelTexture(label: string): THREE.Texture | null {
  if (typeof document === 'undefined') return null;
  const text = label.length > 24 ? `${label.slice(0, 23)}…` : label;
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

export type ArenaSceneOptions = {
  cinematic?: boolean;
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
  const image = texture.image as unknown as {
    width: number;
    height: number;
  } | null;
  const aspect = image ? image.width / Math.max(image.height, 1) : 4;
  sprite.scale.set(10 * aspect, 10, 1);
  sprite.position.copy(position);
  return sprite;
}

function facingFor(
  decision: string,
  angle: number,
): { x: number; z: number } {
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  if (decision === 'Convert') return { x: -cos, z: -sin };
  if (decision === 'Reject') return { x: cos, z: sin };
  return { x: -sin, z: cos };
}

function easeInOutCubic(p: number): number {
  return p < 0.5 ? 4 * p * p * p : 1 - Math.pow(-2 * p + 2, 3) / 2;
}

function lerp(a: number, b: number, p: number): number {
  return a + (b - a) * p;
}

/** Shortest-arc interpolation between two angles. */
function lerpAngle(from: number, to: number, p: number): number {
  let delta = (to - from) % (Math.PI * 2);
  if (delta > Math.PI) delta -= Math.PI * 2;
  if (delta < -Math.PI) delta += Math.PI * 2;
  return from + delta * p;
}

/**
 * Every ring radius the layout module can produce
 * for up to 200 agents in one decision group
 * (base + ring * 26, capacity floor(2πr/14) per
 * ring). Prebuilt once; transitions only toggle
 * visibility, never allocate geometry.
 */
function possibleRingRadii(): number[] {
  const out = new Set<number>();
  for (const base of [60, 120, 180]) {
    let ring = 0;
    let placed = 0;
    while (placed < 200) {
      const radius = base + ring * 26;
      out.add(radius);
      placed += Math.max(1, Math.floor((2 * Math.PI * radius) / 14));
      ring += 1;
    }
  }
  return [...out].sort((a, b) => a - b);
}

/**
 * Build the arena for one saved run. The camera is
 * fixed top-down (position (0, 640, 0), up (0, 0, -1),
 * fov 45) so every ring and facing direction stays
 * readable and picking math is exact: NDC x =
 * x / (tan(22.5°) · 640 · aspect), NDC y =
 * −z / (tan(22.5°) · 640).
 */
export function buildSwarmtankArena(
  layout: ArenaLayout,
  width: number,
  height: number,
  options: ArenaSceneOptions = {},
): BuiltArenaScene {
  const createLabelSprite = options.createLabelSprite ?? defaultLabelSprite;
  const agents: ArenaAgent[] = layout.agents;
  const agentIds = agents.map(agent => agent.personaId);

  const threeScene = new THREE.Scene();
  threeScene.background = new THREE.Color(ARENA_FIELD);
  const camera = new THREE.PerspectiveCamera(
    ARENA_CAMERA_FOV,
    width / Math.max(height, 1),
    1,
    5000,
  );
  camera.up.set(0, 0, -1);
  camera.position.set(0, ARENA_CAMERA_HEIGHT, 0);
  camera.lookAt(0, 0, 0);

  const animateSet = options.cinematic ? buildSwarmtankSet(threeScene) : undefined;
  if (options.cinematic) {
    camera.up.set(0, 1, 0);
    const distance = Math.max(1, 1.28 / camera.aspect);
    camera.position.set(357 * distance, 407 * distance, 540 * distance);
    camera.lookAt(0, 0, 0);
    animateSet?.(0);
  } else {
    const stage = new THREE.Mesh(
      new THREE.CylinderGeometry(40, 40, 8, 48),
      new THREE.MeshBasicMaterial({ color: STAGE_COLOR }),
    );
    stage.position.set(0, 4, 0);
    const monolith = new THREE.Mesh(
      new THREE.BoxGeometry(24, 60, 24),
      new THREE.MeshBasicMaterial({ color: MONOLITH_COLOR }),
    );
    monolith.position.set(0, 38, 0);
    threeScene.add(stage, monolith);
  }
  const pedestalSprite = options.cinematic ? null : createLabelSprite('Product concept', new THREE.Vector3(0, 76, 0));
  if (pedestalSprite) threeScene.add(pedestalSprite);

  // Prebuilt amphitheater ring lines; only the
  // radii the current layout uses are visible.
  const possibleRadii = possibleRingRadii();
  const ringLines: Array<{ radius: number; line: THREE.Line }> = [];
  for (const radius of options.cinematic ? [] : possibleRadii) {
    const points: THREE.Vector3[] = [];
    for (let i = 0; i <= 64; i++) {
      const angle = (i / 64) * Math.PI * 2;
      points.push(
        new THREE.Vector3(
          radius * Math.cos(angle),
          0.5,
          radius * Math.sin(angle),
        ),
      );
    }
    const ringGeometry = new THREE.BufferGeometry().setFromPoints(points);
    const line = new THREE.Line(
      ringGeometry,
      new THREE.LineBasicMaterial({
        color: RING_COLOR,
        transparent: true,
        opacity: 0.8,
      }),
    );
    line.visible = !options.cinematic && layout.ringRadii.includes(radius);
    threeScene.add(line);
    ringLines.push({ radius, line });
  }

  // Agent bodies: one instanced sphere per persona,
  // colored by decision. Only bodies are pickable.
  const wiring = options.cinematic ? buildSwarmtankWiring(threeScene, agents.length) : undefined;
  const bodies = new THREE.InstancedMesh(
    new THREE.SphereGeometry(BODY_RADIUS, 16, 16),
    new THREE.MeshBasicMaterial({ color: 0xffffff }),
    Math.max(agents.length, 1),
  );
  // Agent noses: flat cones pointing along facing.
  const noseGeometry = new THREE.ConeGeometry(3, 9, 12);
  noseGeometry.rotateX(Math.PI / 2);
  const noses = new THREE.InstancedMesh(
    noseGeometry,
    new THREE.MeshBasicMaterial({ color: 0xffffff }),
    Math.max(agents.length, 1),
  );
  noses.visible = !options.cinematic;
  // Hesitation markers: one mesh for all slots,
  // Confused-first in slot order; unused slots
  // collapse to scale 0.
  const markers = new THREE.InstancedMesh(
    new THREE.OctahedronGeometry(3),
    new THREE.MeshBasicMaterial({ color: MARKER_COLOR }),
    Math.max(agents.length, 1),
  );

  markers.visible = !options.cinematic;

  // Per-slot transition state, in initial layout
  // order. Slots never reorder: a persona keeps
  // its instance index across decisions.
  const angleFrom = agents.map(agent => agent.angle);
  const radiusFrom = agents.map(agent => agent.radius);
  const angleTo = agents.map(agent => agent.angle);
  const radiusTo = agents.map(agent => agent.radius);
  const decisionTo = agents.map(agent => agent.decision);
  let markerSlots: number[] = agents
    .map((agent, i) => (agent.decision === 'Confused' ? i : -1))
    .filter(i => i >= 0);
  let transitionProgress = 1;
  let lastT = 0;
  let selectedId: string | null = null;
  const bodyMatrix = new THREE.Matrix4();
  const noseMatrix = new THREE.Matrix4();
  const markerMatrix = new THREE.Matrix4();
  const position = new THREE.Vector3();
  const quaternion = new THREE.Quaternion();
  const noseQuaternion = new THREE.Quaternion();
  const scale = new THREE.Vector3();
  const up = new THREE.Vector3(0, 1, 0);

  /** Base (orbit-free) angle/radius of a slot at current progress. */
  function slotBase(i: number): { angle: number; radius: number } {
    const p = easeInOutCubic(transitionProgress);
    return {
      angle: lerpAngle(angleFrom[i] ?? 0, angleTo[i] ?? 0, p),
      radius: lerp(radiusFrom[i] ?? 0, radiusTo[i] ?? 0, p),
    };
  }

  /** Full position angle of a slot at time t (orbit on top). */
  function slotAngle(i: number, t: number): number {
    const base = slotBase(i).angle;
    return decisionTo[i] === 'Confused' ? base + t * ORBIT_SPEED : base;
  }

  function refreshBodies(t: number) {
    const color = new THREE.Color();
    agents.forEach((agent, i) => {
      const { radius } = slotBase(i);
      const angle = slotAngle(i, t);
      const x = radius * Math.cos(angle);
      const z = radius * Math.sin(angle);
      const decision = decisionTo[i] ?? agent.decision;
      const facing = facingFor(decision, angle);
      const selected = agent.personaId === selectedId;
      if (wiring) {
        const spread = 1 + 75 / Math.max(radius, 1);
        position.set(x * spread, 22 + (i * 37 % 67), z * spread);
        wiring.place(i, position);
      } else {
        position.set(x, BODY_Y, z);
      }
      quaternion.identity();
      scale.setScalar(selected ? 1.6 : 1);
      bodyMatrix.compose(position, quaternion, scale);
      bodies.setMatrixAt(i, bodyMatrix);
      color.set(DECISION_COLORS[decision] ?? '#ffffff');
      bodies.setColorAt(i, color);
      noseQuaternion.setFromAxisAngle(up, Math.atan2(facing.x, facing.z));
      position.set(
        x + facing.x * NOSE_OFFSET,
        BODY_Y,
        z + facing.z * NOSE_OFFSET,
      );
      noseMatrix.compose(position, noseQuaternion, new THREE.Vector3(1, 1, 1));
      noses.setMatrixAt(i, noseMatrix);
      color.set(DECISION_COLORS[decision] ?? '#ffffff');
      noses.setColorAt(i, color);
    });
    wiring?.update();
    bodies.instanceMatrix.needsUpdate = true;
    if (bodies.instanceColor) bodies.instanceColor.needsUpdate = true;
    noses.instanceMatrix.needsUpdate = true;
    if (noses.instanceColor) noses.instanceColor.needsUpdate = true;
  }

  function refreshMarkers(t: number) {
    const zero = new THREE.Matrix4().makeScale(0, 0, 0);
    for (let m = 0; m < Math.max(agents.length, 1); m++) {
      const slot = markerSlots[m];
      if (slot === undefined) {
        markers.setMatrixAt(m, zero);
        continue;
      }
      const { radius } = slotBase(slot);
      const angle = slotAngle(slot, t);
      position.set(
        radius * Math.cos(angle),
        MARKER_BASE_Y + 3 * Math.sin(t * 2 + m),
        radius * Math.sin(angle),
      );
      quaternion.identity();
      scale.setScalar(1);
      markerMatrix.compose(position, quaternion, scale);
      markers.setMatrixAt(m, markerMatrix);
    }
    markers.instanceMatrix.needsUpdate = true;
  }

  refreshBodies(0);
  refreshMarkers(0);
  if (options.cinematic) {
    // The network can move beyond its initial cluster after scenario edits.
    const outerRadius = Math.max(...possibleRadii) + 75;
    bodies.boundingSphere = new THREE.Sphere(new THREE.Vector3(0, 55, 0), outerRadius + 50);
  } else {
    bodies.computeBoundingSphere();
  }
  threeScene.add(bodies, noses, markers);

  const raycaster = new THREE.Raycaster();
  const pick = (ndcX: number, ndcY: number): string | null => {
    camera.updateMatrixWorld();
    raycaster.setFromCamera(new THREE.Vector2(ndcX, ndcY), camera);
    const hits = raycaster.intersectObject(bodies, false);
    const first = hits[0];
    if (!first || first.instanceId === undefined) return null;
    return agentIds[first.instanceId] ?? null;
  };

  const setSelected = (id: string | null) => {
    selectedId = id;
    refreshBodies(lastT);
  };

  const animate = (t: number) => {
    animateSet?.(t);
    lastT = t;
    refreshBodies(t);
    refreshMarkers(t);
  };

  const advanceTransition = (dt: number): number => {
    if (transitionProgress < 1) {
      transitionProgress = Math.min(
        1,
        transitionProgress + dt / TRANSITION_SECONDS,
      );
    }
    return transitionProgress;
  };

  const built: BuiltArenaScene = {
    scene: threeScene,
    camera,
    layout,
    agentIds,
    counts: {
      agents: agents.length,
      rings: layout.ringRadii.length,
      markers: markerSlots.length,
    },
    pick,
    setSelected,
    animate,
    updateDecisions: (nextLayout, instant) => {
      const nextById = new Map(
        nextLayout.agents.map(agent => [agent.personaId, agent]),
      );
      agents.forEach((agent, i) => {
        const current = slotBase(i);
        angleFrom[i] = current.angle;
        radiusFrom[i] = current.radius;
        const next = nextById.get(agent.personaId);
        if (next) {
          angleTo[i] = next.angle;
          radiusTo[i] = next.radius;
          decisionTo[i] = next.decision;
        }
      });
      markerSlots = agents
        .map((agent, i) =>
          (decisionTo[i] ?? agent.decision) === 'Confused' ? i : -1,
        )
        .filter(i => i >= 0);
      for (const ring of ringLines) {
        ring.line.visible = nextLayout.ringRadii.includes(ring.radius);
      }
      transitionProgress = instant ? 1 : 0;
      built.layout = nextLayout;
      built.counts = {
        agents: agents.length,
        rings: nextLayout.ringRadii.length,
        markers: markerSlots.length,
      };
      refreshBodies(lastT);
      refreshMarkers(lastT);
    },
    advanceTransition,
    dispose: () => {
      const geometries = new Set<THREE.BufferGeometry>();
      const sceneMaterials = new Set<THREE.Material>();
      const textures = new Set<THREE.Texture>();
      threeScene.traverse(object => {
        const mesh = object as THREE.Mesh;
        if (mesh.geometry) geometries.add(mesh.geometry);
        const materials = Array.isArray(mesh.material)
          ? mesh.material
          : mesh.material
            ? [mesh.material]
            : [];
        for (const material of materials) {
          const withMap = material as THREE.Material & {
            map?: THREE.Texture | null;
          };
          if (withMap.map) textures.add(withMap.map);
          sceneMaterials.add(material);
        }
      });
      textures.forEach(texture => texture.dispose());
      sceneMaterials.forEach(material => material.dispose());
      geometries.forEach(geometry => geometry.dispose());
    },
  };
  return built;
}

export { layoutSwarmtankArena };
