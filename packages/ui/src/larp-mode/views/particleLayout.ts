// LARP Mode Slice 2: deterministic particle layout for the memory
// vault. Seeded (mulberry32) so the same seed + same nodes produce
// byte-identical arrays — unit-tested, and the vault looks identical
// across reloads. Particles cluster around centroids derived from the
// latest memory-event nodes (golden-angle spiral on a shell); with no
// nodes, three fallback centroids keep the vault renderable.

import type { MemoryNode } from '../telemetry.ts';

export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

export const PARTICLE_COUNT = 50000;
export const CLUSTER_SPREAD = 14;
export const CLUSTER_RADIUS = 60;

export const FALLBACK_CENTROIDS: Vec3[] = [
  { x: -40, y: 20, z: 0 },
  { x: 40, y: -10, z: -20 },
  { x: 0, y: -30, z: 30 },
];

export function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function gaussian(rand: () => number): number {
  let u = 0;
  while (u === 0) u = rand();
  return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * rand());
}

export function clusterCentroids(nodes: MemoryNode[]): Vec3[] {
  if (nodes.length === 0) return [...FALLBACK_CENTROIDS];
  return nodes.map((_, i) => {
    const t = (i + 0.5) / nodes.length;
    const phi = Math.acos(1 - 2 * t);
    const theta = Math.PI * (1 + Math.sqrt(5)) * i;
    return {
      x: CLUSTER_RADIUS * Math.sin(phi) * Math.cos(theta),
      y: CLUSTER_RADIUS * Math.cos(phi) * 0.6,
      z: CLUSTER_RADIUS * Math.sin(phi) * Math.sin(theta),
    };
  });
}

export interface ParticleData {
  positions: Float32Array;
  scales: Float32Array;
  seeds: Float32Array;
}

export function generateParticles(
  seed: number,
  centroids: Vec3[],
  count: number = PARTICLE_COUNT,
): ParticleData {
  const rand = mulberry32(seed);
  const positions = new Float32Array(count * 3);
  const scales = new Float32Array(count);
  const seeds = new Float32Array(count);
  const fallback = FALLBACK_CENTROIDS[0];
  for (let i = 0; i < count; i++) {
    const c = centroids.length > 0 ? centroids[Math.floor(rand() * centroids.length)] : fallback;
    positions[i * 3] = c.x + gaussian(rand) * CLUSTER_SPREAD;
    positions[i * 3 + 1] = c.y + gaussian(rand) * CLUSTER_SPREAD;
    positions[i * 3 + 2] = c.z + gaussian(rand) * CLUSTER_SPREAD;
    scales[i] = 0.5 + rand() * 2.5;
    seeds[i] = rand();
  }
  return { positions, scales, seeds };
}
