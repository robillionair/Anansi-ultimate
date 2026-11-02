// M11 GHOST-PROMPT Slice 2: pure 3D layout for a draft session.
//
// No Three.js import here, so node tests and the e2e spec can use this
// module directly. The layout is a deterministic function of the session
// snapshot: the same snapshot always yields the same positions.
//
// Coordinate frame: the retained baseline runs along X at y = 0, spanning
// [-200, 200]. Deleted nodes float above it in three staggered rows.
// Threads run straight down to each node's baseline edit point. Pause
// markers sit at the baseline point of the next edit after the pause.
// Revision clusters underline baseline spans where consecutive edits
// touched the same region.
//
// Fixed camera (analytic projection, no GL context needed):
// PerspectiveCamera fov 45 at (0, 30, 320), lookAt (0, 30, 0). All content
// lies in the z = 0 plane, so NDC coordinates are exact:
//   ndcX = x / (tan(22.5deg) * 320 * aspect)
//   ndcY = (y - 30) / (tan(22.5deg) * 320)

import type {
  GhostDeletedFragment,
  GhostEditEvent,
  GhostPauseEvent,
} from './ghostPrompt.ts';

export const GHOST_BASELINE_HALF_WIDTH = 200;
export const GHOST_CAMERA_FOV = 45;
export const GHOST_CAMERA_HEIGHT = 30;
export const GHOST_CAMERA_DISTANCE = 320;
export const GHOST_NODE_ROWS = 3;
export const GHOST_NODE_ROW_HEIGHT = 22;
export const GHOST_NODE_BASE_Y = 50;
export const GHOST_CLUSTER_GAP = 8;

export interface GhostDraftSnapshot {
  currentText: string;
  fragments: GhostDeletedFragment[];
  pauses: GhostPauseEvent[];
  events: GhostEditEvent[];
}

export interface GhostLayoutNode {
  id: string;
  seq: number;
  offset: number;
  text: string;
  hash: string;
  x: number;
  y: number;
  row: number;
}

export interface GhostLayoutThread {
  nodeSeq: number;
  fromX: number;
  fromY: number;
  toX: number;
  toY: number;
}

export interface GhostLayoutPause {
  durationMs: number;
  elapsedMs: number;
  x: number;
}

export interface GhostLayoutCluster {
  id: string;
  seqs: number[];
  minX: number;
  maxX: number;
}

export interface GhostDraftLayout {
  nodes: GhostLayoutNode[];
  threads: GhostLayoutThread[];
  pauses: GhostLayoutPause[];
  clusters: GhostLayoutCluster[];
  maxOffset: number;
}

export function ghostOffsetToX(offset: number, maxOffset: number): number {
  return -GHOST_BASELINE_HALF_WIDTH + (offset / maxOffset) * (GHOST_BASELINE_HALF_WIDTH * 2);
}

function snapshotMaxOffset(snapshot: GhostDraftSnapshot): number {
  let max = Math.max(1, Array.from(snapshot.currentText).length);
  for (const fragment of snapshot.fragments) {
    max = Math.max(max, fragment.offset + Array.from(fragment.text).length);
  }
  return max;
}

export function layoutGhostDraft(snapshot: GhostDraftSnapshot): GhostDraftLayout {
  const maxOffset = snapshotMaxOffset(snapshot);
  const nodes: GhostLayoutNode[] = snapshot.fragments.map((fragment, index) => {
    const row = index % GHOST_NODE_ROWS;
    return {
      id: `fragment-${fragment.seq}`,
      seq: fragment.seq,
      offset: fragment.offset,
      text: fragment.text,
      hash: fragment.hash,
      x: ghostOffsetToX(fragment.offset, maxOffset),
      y: GHOST_NODE_BASE_Y + row * GHOST_NODE_ROW_HEIGHT,
      row,
    };
  });
  const threads: GhostLayoutThread[] = nodes.map((node) => ({
    nodeSeq: node.seq,
    fromX: node.x,
    fromY: node.y,
    toX: node.x,
    toY: 0,
  }));
  const sortedEvents = [...snapshot.events].sort((a, b) => a.elapsedMs - b.elapsedMs);
  const pauses: GhostLayoutPause[] = snapshot.pauses.map((pause) => {
    const next = sortedEvents.find((event) => event.elapsedMs >= pause.elapsedMs);
    const x = next
      ? ghostOffsetToX(next.offset, maxOffset)
      : GHOST_BASELINE_HALF_WIDTH;
    return { durationMs: pause.durationMs, elapsedMs: pause.elapsedMs, x };
  });
  const clusters: GhostLayoutCluster[] = [];
  let run: GhostEditEvent[] = [];
  const flush = () => {
    if (run.length > 1) {
      const xs = run.map((event) => ghostOffsetToX(event.offset, maxOffset));
      clusters.push({
        id: `cluster-${run[0].seq}`,
        seqs: run.map((event) => event.seq),
        minX: Math.min(...xs),
        maxX: Math.max(...xs),
      });
    }
    run = [];
  };
  const bySeq = [...snapshot.events].sort((a, b) => a.seq - b.seq);
  for (const event of bySeq) {
    const previous = run[run.length - 1];
    if (previous && Math.abs(event.offset - previous.offset) > GHOST_CLUSTER_GAP) {
      flush();
    }
    run.push(event);
  }
  flush();
  return { nodes, threads, pauses, clusters, maxOffset };
}

/** Project a layout position to NDC with the fixed camera. */
export function ghostProjectToNdc(
  x: number,
  y: number,
  aspect: number,
): { ndcX: number; ndcY: number } {
  const tanHalf = Math.tan(((GHOST_CAMERA_FOV / 2) * Math.PI) / 180);
  return {
    ndcX: x / (tanHalf * GHOST_CAMERA_DISTANCE * aspect),
    ndcY: (y - GHOST_CAMERA_HEIGHT) / (tanHalf * GHOST_CAMERA_DISTANCE),
  };
}
