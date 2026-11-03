// LARP Mode Slice 3: deterministic force-directed graph layout for the
// attention spider, plus graph construction from telemetry nodes.
// Pure and unit-tested — no GL, no React.
//
// Honesty rule for edges: relationship records carry no weight, and no
// attention weights exist anywhere in this app, so NO numeric weight is
// ever displayed. Relationship edges (recorded, bright) and layout
// links (deterministic ring skeleton, dim) are distinguished by kind,
// never by a fabricated weight number.

import type { MemoryNode } from '../telemetry.ts';
import { mulberry32 } from './particleLayout.ts';

export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

export interface GraphNode {
  id: string;
  label: string;
  kind: string;
  cited: boolean;
}

export type GraphEdgeKind = 'relationship' | 'layout';

export interface GraphEdge {
  source: string;
  target: string;
  kind: GraphEdgeKind;
}

export interface Graph {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export const MAX_GRAPH_NODES = 64;
export const LAYOUT_ITERATIONS = 150;
export const LAYOUT_BOUND = 200;

export function latestCitedIds(events: { kind: string; citedIds?: string[] }[]): Set<string> {
  const cited = new Set<string>();
  for (const event of events) {
    if (event.kind === 'compilation' && event.citedIds) {
      for (const id of event.citedIds) cited.add(id);
    }
  }
  return cited;
}

export function buildGraph(nodes: MemoryNode[], citedIds: Set<string>): Graph {
  const kept = nodes.slice(0, MAX_GRAPH_NODES);
  const ids = new Set(kept.map((n) => n.id));
  const graphNodes: GraphNode[] = kept.map((n) => ({
    id: n.id,
    label: n.title,
    kind: n.kind,
    cited: citedIds.has(n.id),
  }));
  const edges: GraphEdge[] = [];
  const seen = new Set<string>();
  const addEdge = (source: string, target: string, kind: GraphEdgeKind) => {
    if (source === target) return;
    const key = [source, target].sort().join('|') + ':' + kind;
    if (seen.has(key)) return;
    seen.add(key);
    edges.push({ source, target, kind });
  };
  for (const node of kept) {
    for (const related of node.relationships ?? []) {
      if (ids.has(related)) addEdge(node.id, related, 'relationship');
    }
  }
  for (let i = 0; i < kept.length; i++) {
    addEdge(kept[i].id, kept[(i + 1) % kept.length].id, 'layout');
  }
  return { nodes: graphNodes, edges };
}

export function layoutGraph(
  graph: Graph,
  seed: number = 20261103,
  iterations: number = LAYOUT_ITERATIONS,
): Map<string, Vec3> {
  const rand = mulberry32(seed);
  const positions = new Map<string, Vec3>();
  for (const node of graph.nodes) {
    const theta = rand() * Math.PI * 2;
    const phi = Math.acos(2 * rand() - 1);
    const r = 60;
    positions.set(node.id, {
      x: r * Math.sin(phi) * Math.cos(theta),
      y: r * Math.cos(phi) * 0.6,
      z: r * Math.sin(phi) * Math.sin(theta),
    });
  }
  const ids = graph.nodes.map((n) => n.id);
  const adjacency = new Map<string, string[]>();
  for (const edge of graph.edges) {
    adjacency.set(edge.source, [...(adjacency.get(edge.source) ?? []), edge.target]);
    adjacency.set(edge.target, [...(adjacency.get(edge.target) ?? []), edge.source]);
  }
  for (let step = 0; step < iterations; step++) {
    const forces = new Map<string, Vec3>();
    for (const id of ids) forces.set(id, { x: 0, y: 0, z: 0 });
    for (let i = 0; i < ids.length; i++) {
      for (let j = i + 1; j < ids.length; j++) {
        const a = positions.get(ids[i])!;
        const b = positions.get(ids[j])!;
        const dx = a.x - b.x;
        const dy = a.y - b.y;
        const dz = a.z - b.z;
        const dist = Math.max(1, Math.hypot(dx, dy, dz));
        const push = Math.min(80, 2400 / (dist * dist));
        const fa = forces.get(ids[i])!;
        const fb = forces.get(ids[j])!;
        fa.x += (dx / dist) * push;
        fa.y += (dy / dist) * push;
        fa.z += (dz / dist) * push;
        fb.x -= (dx / dist) * push;
        fb.y -= (dy / dist) * push;
        fb.z -= (dz / dist) * push;
      }
    }
    for (const edge of graph.edges) {
      const a = positions.get(edge.source)!;
      const b = positions.get(edge.target)!;
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const dz = b.z - a.z;
      const dist = Math.max(1, Math.hypot(dx, dy, dz));
      const rest = 30;
      const pull = (dist - rest) * 0.02;
      const fa = forces.get(edge.source)!;
      const fb = forces.get(edge.target)!;
      fa.x += (dx / dist) * pull;
      fa.y += (dy / dist) * pull;
      fa.z += (dz / dist) * pull;
      fb.x -= (dx / dist) * pull;
      fb.y -= (dy / dist) * pull;
      fb.z -= (dz / dist) * pull;
    }
    const cooling = 1 - step / iterations;
    for (const id of ids) {
      const p = positions.get(id)!;
      const f = forces.get(id)!;
      p.x += (f.x * 0.5 - p.x * 0.01) * cooling;
      p.y += (f.y * 0.5 - p.y * 0.01) * cooling;
      p.z += (f.z * 0.5 - p.z * 0.01) * cooling;
      const len = Math.hypot(p.x, p.y, p.z);
      if (len > LAYOUT_BOUND) {
        p.x = (p.x / len) * LAYOUT_BOUND;
        p.y = (p.y / len) * LAYOUT_BOUND;
        p.z = (p.z / len) * LAYOUT_BOUND;
      }
    }
  }
  return positions;
}
