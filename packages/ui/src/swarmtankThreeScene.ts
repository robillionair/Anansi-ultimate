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

  // Central product monolith on a circular stage.
  const stage = new THREE.Mesh(
    new THREE.CylinderGeometry(40, 40, 8, 48),
    new THREE.MeshBasicMaterial({ color: STAGE_COLOR }),
  );
  stage.position.set(0, 4, 0);
  threeScene.add(stage);
  const monolith = new THREE.Mesh(
    new THREE.BoxGeometry(24, 60, 24),
    new THREE.MeshBasicMaterial({ color: MONOLITH_COLOR }),
  );
  monolith.position.set(0, 38, 0);
  threeScene.add(monolith);
  const pedestalSprite = createLabelSprite(
    'Product concept',
    new THREE.Vector3(0, 76, 0),
  );
  if (pedestalSprite) threeScene.add(pedestalSprite);

  // Amphitheater ring lines, one per used radius.
  for (const radius of layout.ringRadii) {
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
    const ring = new THREE.Line(
      ringGeometry,
      new THREE.LineBasicMaterial({
        color: RING_COLOR,
        transparent: true,
        opacity: 0.8,
      }),
    );
    threeScene.add(ring);
  }

  // Agent bodies: one instanced sphere per persona,
  // colored by decision. Only bodies are pickable.
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
  // Hesitation markers above Confused agents.
  const confusedAgents = agents.filter(agent => agent.decision === 'Confused');
  const markers = new THREE.InstancedMesh(
    new THREE.OctahedronGeometry(3),
    new THREE.MeshBasicMaterial({ color: MARKER_COLOR }),
    Math.max(confusedAgents.length, 1),
  );
  const confusedIndices = confusedAgents.map(agent =>
    agents.findIndex(a => a.personaId === agent.personaId),
  );

  const currentAngles = agents.map(agent => agent.angle);
  let selectedId: string | null = null;
  const bodyMatrix = new THREE.Matrix4();
  const noseMatrix = new THREE.Matrix4();
  const markerMatrix = new THREE.Matrix4();
  const position = new THREE.Vector3();
  const quaternion = new THREE.Quaternion();
  const noseQuaternion = new THREE.Quaternion();
  const scale = new THREE.Vector3();
  const up = new THREE.Vector3(0, 1, 0);

  function refreshBodies() {
    const color = new THREE.Color();
    agents.forEach((agent, i) => {
      const angle = currentAngles[i] ?? agent.angle;
      const x = agent.radius * Math.cos(angle);
      const z = agent.radius * Math.sin(angle);
      const facing = facingFor(agent.decision, angle);
      const selected = agent.personaId === selectedId;
      position.set(x, BODY_Y, z);
      quaternion.identity();
      scale.setScalar(selected ? 1.6 : 1);
      bodyMatrix.compose(position, quaternion, scale);
      bodies.setMatrixAt(i, bodyMatrix);
      color.set(DECISION_COLORS[agent.decision] ?? '#ffffff');
      bodies.setColorAt(i, color);
      noseQuaternion.setFromAxisAngle(up, Math.atan2(facing.x, facing.z));
      position.set(
        x + facing.x * NOSE_OFFSET,
        BODY_Y,
        z + facing.z * NOSE_OFFSET,
      );
      noseMatrix.compose(position, noseQuaternion, new THREE.Vector3(1, 1, 1));
      noses.setMatrixAt(i, noseMatrix);
      color.set(DECISION_COLORS[agent.decision] ?? '#ffffff');
      noses.setColorAt(i, color);
    });
    bodies.instanceMatrix.needsUpdate = true;
    if (bodies.instanceColor) bodies.instanceColor.needsUpdate = true;
    noses.instanceMatrix.needsUpdate = true;
    if (noses.instanceColor) noses.instanceColor.needsUpdate = true;
  }

  function refreshMarkers(t: number) {
    confusedAgents.forEach((agent, i) => {
      const index = confusedIndices[i] ?? -1;
      if (index < 0) return;
      const angle = currentAngles[index] ?? agent.angle;
      const x = agent.radius * Math.cos(angle);
      const z = agent.radius * Math.sin(angle);
      position.set(x, MARKER_BASE_Y + 3 * Math.sin(t * 2 + i), z);
      quaternion.identity();
      scale.setScalar(1);
      markerMatrix.compose(position, quaternion, scale);
      markers.setMatrixAt(i, markerMatrix);
    });
    markers.instanceMatrix.needsUpdate = true;
  }

  refreshBodies();
  refreshMarkers(0);
  bodies.computeBoundingSphere();
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
    refreshBodies();
  };

  const animate = (t: number) => {
    agents.forEach((agent, i) => {
      if (agent.decision === 'Confused') {
        currentAngles[i] = agent.angle + t * ORBIT_SPEED;
      }
    });
    refreshBodies();
    refreshMarkers(t);
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
    layout,
    agentIds,
    counts: {
      agents: agents.length,
      rings: layout.ringRadii.length,
      markers: confusedAgents.length,
    },
    pick,
    setSelected,
    animate,
    dispose,
  };
}

export { layoutSwarmtankArena };
