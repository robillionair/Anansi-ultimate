// LARP Mode Slice 4: View 5 — ATTENTION TENSOR HEATMAP // TOKEN
// ATTENTION MATRIX. High-DPI 2D canvas (Canvas 2D chosen over a WebGL
// shader grid, same rationale as View 2: token labels need crisp
// text). Rows = raw-prompt words, columns = compiled-prompt words,
// cell = word presence — a genuine alignment matrix, NOT attention
// weights (none exist in this app) and NOT provider tokens (this app
// has no tokenizer; sequences are whitespace words, labeled as such).
// No attention heads are measured; the legend says so. Hover readout
// via the View 4 window-listener + rect-check pattern (HUD is
// click-through). Redraws in an effect on [events] (the Slice 3 View 4
// lesson: never draw only inside a subscription callback). Empty state
// when no ask-tokens event exists, never a fabricated matrix.
// data-raw-tokens / data-compiled-tokens / data-matches are the
// e2e-visible counters.

import { useEffect, useRef, useState } from 'react';
import { alignmentMatrix, latestAskTokens, type AlignmentMatrix } from './tokenAlignment.ts';
import { telemetryStream } from '../telemetry.ts';
import { registerDisposer } from '../store.ts';

const MONO_FONT = '"Berkeley Mono", "JetBrains Mono", ui-monospace, Menlo, monospace';
const GUTTER = 100;
const BOTTOM_MARGIN = 44;

export function View5AttentionHeatmap({ stream = telemetryStream }: { stream?: typeof telemetryStream }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const readoutRef = useRef<HTMLDivElement>(null);
  const [events, setEvents] = useState(() => stream.snapshot());
  const latest = latestAskTokens(events);
  const matrix: AlignmentMatrix | null = latest
    ? alignmentMatrix(latest.rawTokens, latest.compiledTokens)
    : null;
  const matrixRef = useRef(matrix);
  matrixRef.current = matrix;
  const layoutRef = useRef({ gutter: GUTTER, cellW: 10, cellH: 10 });

  useEffect(() => stream.subscribe(() => setEvents(stream.snapshot())), [stream]);

  function draw(): void {
    const container = containerRef.current;
    const canvas = canvasRef.current;
    if (!container || !canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const m = matrixRef.current;
    const w = canvas.clientWidth || 640;
    const h = canvas.clientHeight || 480;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) {
      canvas.width = Math.round(w * dpr);
      canvas.height = Math.round(h * dpr);
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);
    ctx.fillStyle = '#030305';
    ctx.fillRect(0, 0, w, h);
    if (!m || m.rows.length === 0 || m.cols.length === 0) {
      container.dataset.rawTokens = '0';
      container.dataset.compiledTokens = '0';
      container.dataset.matches = '0';
      return;
    }
    const gridH = h - BOTTOM_MARGIN;
    const cellW = (w - GUTTER) / m.cols.length;
    const cellH = gridH / m.rows.length;
    layoutRef.current = { gutter: GUTTER, cellW, cellH };
    ctx.font = `10px ${MONO_FONT}`;
    m.rows.forEach((row, ri) => {
      const y = ri * cellH;
      ctx.fillStyle = '#e8e8ec';
      ctx.textAlign = 'right';
      ctx.fillText(row.slice(0, 12), GUTTER - 4, y + cellH / 2 + 3);
      m.cols.forEach((_, ci) => {
        ctx.fillStyle = m.matches[ri][ci] ? '#ff8a3d' : '#18181f';
        ctx.fillRect(GUTTER + ci * cellW + 0.5, y + 0.5, cellW - 1, cellH - 1);
      });
    });
    container.dataset.rawTokens = String(m.rows.length);
    container.dataset.compiledTokens = String(m.cols.length);
    container.dataset.matches = String(m.matchCount);
  }

  function onMove(event: MouseEvent): void {
    const canvas = canvasRef.current;
    const m = matrixRef.current;
    if (!canvas || !m) return;
    const rect = canvas.getBoundingClientRect();
    if (
      event.clientX < rect.left ||
      event.clientX > rect.right ||
      event.clientY < rect.top ||
      event.clientY > rect.bottom
    ) {
      if (readoutRef.current) readoutRef.current.textContent = 'Hover the matrix to inspect a cell.';
      return;
    }
    const { gutter, cellW, cellH } = layoutRef.current;
    const ci = Math.floor((event.clientX - rect.left - gutter) / cellW);
    const ri = Math.floor((event.clientY - rect.top) / cellH);
    if (readoutRef.current) {
      if (ri >= 0 && ri < m.rows.length && ci >= 0 && ci < m.cols.length) {
        readoutRef.current.textContent = `raw "${m.rows[ri]}" · compiled "${m.cols[ci]}" · ${m.matches[ri][ci] ? 'match' : 'no match'}`;
      } else {
        readoutRef.current.textContent = 'No cell under cursor.';
      }
    }
  }

  useEffect(() => {
    draw();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [events]);

  useEffect(() => {
    window.addEventListener('mousemove', onMove);
    const unregister = registerDisposer(() => window.removeEventListener('mousemove', onMove));
    return () => {
      window.removeEventListener('mousemove', onMove);
      unregister();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (!matrix || matrix.rows.length === 0 || matrix.cols.length === 0) {
    return (
      <div ref={containerRef} className="larp-view-container" aria-label="ATTENTION TENSOR HEATMAP // TOKEN ATTENTION MATRIX">
        <div className="larp-view-fallback">
          <p>No compiled prompt yet — ask a question in Ask ANANSI.</p>
        </div>
      </div>
    );
  }

  return (
    <div ref={containerRef} className="larp-view-container" aria-label="ATTENTION TENSOR HEATMAP // TOKEN ATTENTION MATRIX">
      <canvas ref={canvasRef} className="larp-view-canvas" aria-label="Token attention matrix" />
      <div ref={readoutRef} className="larp-spectrum-readout" aria-live="off">
        Hover the matrix to inspect a cell.
      </div>
      <div className="larp-spectrum-legend">
        <span>Raw {matrix.rows.length}</span>{' '}
        <span>Compiled {matrix.cols.length}</span>{' '}
        <span>Matches {matrix.matchCount}</span>{' '}
        <span>No attention heads measured — single word-presence matrix.</span>{' '}
        <span>Words, not provider tokens.</span>
      </div>
    </div>
  );
}
