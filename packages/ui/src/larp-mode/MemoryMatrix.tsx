// LARP Mode Slice 1: L1/L2/L3 memory allocation matrix. Dense grid of
// 4px x 4px cells at devicePixelRatio. Slice 1 proxy mapping, stated
// openly: each node of the latest memory event occupies one cell, colored
// by node kind — memory nodes read as Pinned Rules (amber), notes as the
// Active Window (cyan), artifacts as Compiled Delta (magenta); every
// remaining cell is Evicted (dark). A real allocation-role taxonomy
// arrives with the Slice 2 producers; until then the grid is honestly
// all-Evicted when the stream holds no memory event.

import { useEffect, useRef } from 'react';
import type { MemoryNode, TelemetryEvent } from './telemetry.ts';

export type MatrixCell = 'pinned' | 'active' | 'delta' | 'evicted';

export const MATRIX_COLUMNS = 48;
export const MATRIX_ROWS = 16;
export const MATRIX_CELL_PX = 4;
export const MATRIX_GAP_PX = 1;

export const MATRIX_COLORS: Record<MatrixCell, string> = {
  pinned: '#ff8a3d',
  active: '#00f0ff',
  delta: '#ff1e7a',
  evicted: '#18181f',
};

export function matrixCellsForNodes(nodes: MemoryNode[]): MatrixCell[] {
  const total = MATRIX_COLUMNS * MATRIX_ROWS;
  const cells: MatrixCell[] = new Array(total).fill('evicted');
  nodes.slice(0, total).forEach((node, index) => {
    cells[index] =
      node.kind === 'memory' ? 'pinned' : node.kind === 'note' ? 'active' : 'delta';
  });
  return cells;
}

export function latestMemoryNodes(events: TelemetryEvent[]): MemoryNode[] {
  for (let i = events.length - 1; i >= 0; i--) {
    const event = events[i];
    if (event.kind === 'memory') return event.nodes;
  }
  return [];
}

export function MemoryMatrix({ nodes }: { nodes: MemoryNode[] }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const cells = matrixCellsForNodes(nodes);
  const pitch = MATRIX_CELL_PX + MATRIX_GAP_PX;
  const width = MATRIX_COLUMNS * pitch - MATRIX_GAP_PX;
  const height = MATRIX_ROWS * pitch - MATRIX_GAP_PX;

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, width, height);
    cells.forEach((cell, index) => {
      const x = (index % MATRIX_COLUMNS) * pitch;
      const y = Math.floor(index / MATRIX_COLUMNS) * pitch;
      ctx.fillStyle = MATRIX_COLORS[cell];
      ctx.fillRect(x, y, MATRIX_CELL_PX, MATRIX_CELL_PX);
    });
  }, [nodes]);

  return (
    <section className="larp-panel" aria-labelledby="larp-matrix-heading">
      <h3 id="larp-matrix-heading">L1 / L2 / L3 Memory Allocation</h3>
      <canvas
        ref={ref}
        className="larp-matrix-canvas"
        style={{ width: `${width}px`, height: `${height}px` }}
        role="img"
        aria-label="Memory allocation matrix"
      />
      <ul className="larp-legend">
        <li>
          <span className="larp-swatch" style={{ background: '#ff8a3d' }} /> Pinned Rules
        </li>
        <li>
          <span className="larp-swatch" style={{ background: '#00f0ff' }} /> Active Window
        </li>
        <li>
          <span className="larp-swatch" style={{ background: '#ff1e7a' }} /> Compiled Delta
        </li>
        <li>
          <span className="larp-swatch" style={{ background: '#18181f' }} /> Evicted
        </li>
      </ul>
      {nodes.length === 0 && <p className="larp-empty">No memory snapshot yet.</p>}
    </section>
  );
}
