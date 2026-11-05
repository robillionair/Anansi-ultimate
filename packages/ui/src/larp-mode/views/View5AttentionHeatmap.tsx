// LARP Mode Slice 4: View 5 — ATTENTION TENSOR HEATMAP // TOKEN
// ATTENTION MATRIX. Crisp canvas typography and cells are composited
// through shared WebGL optics with direct 2D fallback. Rows are raw-prompt
// words; columns are compiled-prompt words;
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
import {useInstrumentSurface} from '../useInstrumentSurface.ts';
import {drawInstrumentStage,matrixLayout,matrixCellAt} from './instrumentScene.ts';

const MONO_FONT = '"Berkeley Mono", "JetBrains Mono", ui-monospace, Menlo, monospace';

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
  const layoutRef = useRef(matrixLayout(640,480,1,1));

  useEffect(() => stream.subscribe(() => setEvents(stream.snapshot())), [stream]);

  function draw(ctx:CanvasRenderingContext2D,w:number,h:number): void {
    const container=containerRef.current;if(!container)return;
    const m=matrixRef.current;drawInstrumentStage(ctx,w,h);
    if (!m || m.rows.length === 0 || m.cols.length === 0) {
      container.dataset.rawTokens = '0';
      container.dataset.compiledTokens = '0';
      container.dataset.matches = '0';
      return;
    }
    const layout=matrixLayout(w,h,m.rows.length,m.cols.length);
    const {left,top,cellW,cellH}=layout;layoutRef.current=layout;
    ctx.fillStyle='#ad9475';ctx.font=`9px ${MONO_FONT}`;ctx.textAlign='left';
    ctx.fillText('RAW WORDS ↓ / COMPILED WORDS →',left,top-18);
    ctx.strokeStyle='#6a4e35';ctx.lineWidth=.6;ctx.strokeRect(left-3,top-3,layout.width+6,layout.height+6);
    ctx.font = `10px ${MONO_FONT}`;
    m.rows.forEach((row, ri) => {
      const y = top+ri * cellH;
      ctx.fillStyle = '#e8e8ec';
      ctx.textAlign = 'right';
      if(cellH>=10||ri%Math.ceil(12/cellH)===0)ctx.fillText(row.slice(0, 12),left-8,y+Math.min(cellH/2+3,12));
      m.cols.forEach((_, ci) => {
        const match=m.matches[ri][ci];
        ctx.fillStyle=match?'#dcb37b':'rgba(47,41,45,.65)';
        ctx.fillRect(left+ci*cellW+.25,y+.25,Math.max(.3,cellW-.5),Math.max(.3,cellH-.5));
        if(match&&cellW>3&&cellH>3){ctx.fillStyle='#fff0d5';ctx.fillRect(left+ci*cellW+cellW*.3,y+cellH*.3,Math.max(.5,cellW*.4),Math.max(.5,cellH*.4));}
      });
    });
    ctx.font=`8px ${MONO_FONT}`;ctx.fillStyle='#b49d7e';
    const stride=Math.max(1,Math.ceil(35/cellW));
    m.cols.forEach((word,ci)=>{if(ci%stride)return;ctx.save();ctx.translate(left+(ci+.5)*cellW,top+layout.height+9);ctx.rotate(.65);ctx.textAlign='left';ctx.fillText(word.slice(0,9),0,0);ctx.restore();});
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
    const cell=matrixCellAt(event.clientX-rect.left,event.clientY-rect.top,layoutRef.current,m.rows.length,m.cols.length);
    const ci=cell?.col??-1,ri=cell?.row??-1;
    if (readoutRef.current) {
      if (ri >= 0 && ri < m.rows.length && ci >= 0 && ci < m.cols.length) {
        readoutRef.current.textContent = `raw "${m.rows[ri]}" · compiled "${m.cols[ci]}" · ${m.matches[ri][ci] ? 'match' : 'no match'}`;
      } else {
        readoutRef.current.textContent = 'No cell under cursor.';
      }
    }
  }

  useInstrumentSurface(canvasRef,draw,!!matrix&&matrix.rows.length>0&&matrix.cols.length>0);

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
