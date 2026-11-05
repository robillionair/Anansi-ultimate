// LARP Mode Slice 3: View 4 — QUANT DRIFT & COLLIDER SPECTRUM.
// Event-driven high-DPI instruments through shared GPU optics (2D fallback),
// over real bus series only: drift scores, token
// compression percent, compilation markers, and latest confidence
// values. X is EVENT ORDER, not wall time (see spectrum.ts). The legend
// carries per-series sample counts and states plainly that collision
// resonance is not measured — no resonance score exists in the bus, so
// none is rendered. Hover crosshair + readout shows the nearest
// sample's values (readout updated imperatively, no re-render per
// mousemove). data-markers / data-confidences are the e2e-visible
// counters. Redraws run after renders whose events changed (never only
// inside a subscription callback, which fires before React re-renders
// and would read the stale series). Disposal removes the window mouse
// listener via the store disposer.

import { useEffect, useRef, useState } from 'react';
import { spectrumSeries, type SpectrumSeries } from './spectrum.ts';
import { telemetryStream } from '../telemetry.ts';
import { registerDisposer } from '../store.ts';
import {useInstrumentSurface} from '../useInstrumentSurface.ts';
import {drawInstrumentStage,instrumentBounds} from './instrumentScene.ts';

const MONO_FONT = '"Berkeley Mono", "JetBrains Mono", ui-monospace, Menlo, monospace';

export function View4QuantDrift({ stream = telemetryStream }: { stream?: typeof telemetryStream }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const readoutRef = useRef<HTMLDivElement>(null);
  const [events, setEvents] = useState(() => stream.snapshot());
  const seriesRef = useRef<SpectrumSeries>(spectrumSeries(events));
  seriesRef.current = spectrumSeries(events);

  useEffect(() => stream.subscribe(() => setEvents(stream.snapshot())), [stream]);

  function draw(ctx:CanvasRenderingContext2D,w:number,h:number): void {
    const container=containerRef.current;if(!container)return;
    drawInstrumentStage(ctx,w,h);
    const b=instrumentBounds(w,h),band=b.height*.38;
    ctx.font=`9px ${MONO_FONT}`;
    for(const [name,top] of [['CONTEXT DRIFT / 0 — 1',b.top],['INPUT REDUCTION / −100 — +100%',b.top+b.height*.48]] as const){
      ctx.fillStyle='#bba17f';ctx.textAlign='left';ctx.fillText(name,b.left,top-15);
      ctx.strokeStyle='#302620';ctx.lineWidth=.6;ctx.strokeRect(b.left,top,b.width,band);
      for(let x=1;x<12;x++){ctx.strokeStyle='#201c1c';ctx.beginPath();ctx.moveTo(b.left+x*b.width/12,top);ctx.lineTo(b.left+x*b.width/12,top+band);ctx.stroke();}
      for(let y=1;y<4;y++){ctx.beginPath();ctx.moveTo(b.left,top+y*band/4);ctx.lineTo(b.left+b.width,top+y*band/4);ctx.stroke();}
    }
    const series = seriesRef.current;
    const plot = (points: { x: number; y: number }[], top: number, height: number, color: string) => {
      if (points.length === 0) return;
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.beginPath();
      points.forEach((p, i) => {
        const px = b.left + p.x * b.width;
        const py = top + height - p.y * height;
        if (i === 0) ctx.moveTo(px, py);
        else ctx.lineTo(px, py);
      });
      ctx.stroke();
      for(const p of points){ctx.fillStyle=color;ctx.beginPath();ctx.arc(b.left+p.x*b.width,top+height-p.y*height,2,0,Math.PI*2);ctx.fill();}
    };
    plot(series.drift,b.top,band,'#a6cbd7');
    const compTop=b.top+b.height*.48,compHeight=band;
    ctx.strokeStyle='#544035';ctx.beginPath();ctx.moveTo(b.left,compTop+band/2);ctx.lineTo(b.left+b.width,compTop+band/2);ctx.stroke();
    if (series.compression.length > 0) {
      ctx.strokeStyle = '#e6bb81';
      ctx.lineWidth = 1;
      ctx.beginPath();
      series.compression.forEach((p, i) => {
        const px = b.left + p.x * b.width;
        const py = compTop + compHeight / 2 - (p.y / 100) * (compHeight / 2);
        if (i === 0) ctx.moveTo(px, py);
        else ctx.lineTo(px, py);
      });
      ctx.stroke();
    }
    // Compilation markers across the full height.
    ctx.strokeStyle = '#ff1e7a';
    ctx.globalAlpha = 0.5;
    for (const marker of series.markers) {
      ctx.beginPath();
      ctx.moveTo(b.left + marker.x * b.width, b.top);
      ctx.lineTo(b.left + marker.x * b.width, b.top+b.height*.86);
      ctx.stroke();
    }
    ctx.globalAlpha = 1;
    // Confidence strip (bottom).
    const stripY = b.top+b.height*.96;
    series.confidences.forEach((c, i) => {
      const px = series.confidences.length <= 1 ? b.left+b.width / 2 : b.left+(i / (series.confidences.length - 1)) * b.width;
      ctx.fillStyle = c >= 0.5 ? '#ffffff' : '#ff8a3d';
      ctx.beginPath();
      ctx.arc(px, stripY, 2.5, 0, Math.PI * 2);
      ctx.fill();
      const bar=40*Math.max(0,Math.min(1,c));ctx.fillStyle='#ad8d6544';ctx.fillRect(px-3,stripY-bar,6,bar);
    });
    ctx.fillStyle = '#a1a1aa';
    ctx.font = `10px ${MONO_FONT}`;
    ctx.textAlign = 'left';
    ctx.fillText(`EVENT ORDER / ${series.markers.length} COMPILATIONS / ${series.confidences.length} CONFIDENCE SAMPLES`,b.left,b.top+b.height+20);
    if(!series.drift.length&&series.compression.length){ctx.fillStyle='#83735f';ctx.fillText('DRIFT NOT REPORTED',b.left+12,b.top+band*.5);}
    if(!series.drift.length&&!series.compression.length){ctx.fillStyle='#83735f';ctx.fillText('AWAITING MEASURED SIGNAL — NO SYNTHETIC SERIES',b.left+12,b.top+band*.5);}
    container.dataset.markers = String(series.markers.length);
    container.dataset.confidences = String(series.confidences.length);
  }

  function onMove(event: MouseEvent): void {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    if (readoutRef.current) {
      if (
        event.clientX < rect.left ||
        event.clientX > rect.right ||
        event.clientY < rect.top ||
        event.clientY > rect.bottom
      ) {
        readoutRef.current.textContent = 'Hover the spectrum to inspect a sample.';
        return;
      }
    }
    const b=instrumentBounds(rect.width,rect.height);
    const fx = (event.clientX - rect.left-b.left) / b.width;
    const series = seriesRef.current;
    const nearest = (points: { x: number; y: number }[]) => {
      if (points.length === 0) return null;
      let best = points[0];
      for (const p of points) if (Math.abs(p.x - fx) < Math.abs(best.x - fx)) best = p;
      return best;
    };
    const drift = nearest(series.drift);
    const comp = nearest(series.compression);
    const marker = series.markers.reduce<{ x: number; label: string } | null>((best, m) => {
      if (!best || Math.abs(m.x - fx) < Math.abs(best.x - fx)) return m;
      return best;
    }, null);
    const parts: string[] = [];
    if (drift) parts.push(`Drift ${drift.y.toFixed(3)}`);
    if (comp) parts.push(`Compression ${comp.y.toFixed(1)}%`);
    if (marker && Math.abs(marker.x - fx) < 0.05) parts.push(`Marker ${marker.label}`);
    if (readoutRef.current) {
      readoutRef.current.textContent = parts.length > 0 ? parts.join(' · ') : 'No sample under cursor.';
    }
  }

  useInstrumentSurface(canvasRef,draw);

  useEffect(() => {
    // Window-level listener, not canvas-level: the HUD frame is
    // pointer-events none (click-through), so the canvas itself never
    // receives mouse events — but moves still bubble to window, and
    // the rect check recovers hover position.
    window.addEventListener('mousemove', onMove);
    const unregister = registerDisposer(() => window.removeEventListener('mousemove', onMove));
    return () => {
      window.removeEventListener('mousemove', onMove);
      unregister();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const series = seriesRef.current;
  return (
    <div ref={containerRef} className="larp-view-container" aria-label="QUANT DRIFT & COLLIDER SPECTRUM">
      <canvas ref={canvasRef} className="larp-view-canvas" aria-label="Quant drift spectrum" />
      <div ref={readoutRef} className="larp-spectrum-readout" aria-live="off">
        Hover the spectrum to inspect a sample.
      </div>
      <div className="larp-spectrum-legend">
        <span>Drift {series.drift.length}</span>{' '}
        <span>Compression {series.compression.length}</span>{' '}
        <span>Compilations {series.markers.length}</span>{' '}
        <span>Confidences {series.confidences.length}</span>{' '}
        <span>Collision resonance: not measured.</span>
      </div>
    </div>
  );
}
