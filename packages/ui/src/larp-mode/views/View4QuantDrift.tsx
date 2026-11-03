// LARP Mode Slice 3: View 4 — QUANT DRIFT & COLLIDER SPECTRUM.
// High-DPI 2D canvas over real bus series only: drift scores, token
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

const MONO_FONT = '"Berkeley Mono", "JetBrains Mono", ui-monospace, Menlo, monospace';

export function View4QuantDrift({ stream = telemetryStream }: { stream?: typeof telemetryStream }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const readoutRef = useRef<HTMLDivElement>(null);
  const [events, setEvents] = useState(() => stream.snapshot());
  const seriesRef = useRef<SpectrumSeries>(spectrumSeries(events));
  seriesRef.current = spectrumSeries(events);

  useEffect(() => stream.subscribe(() => setEvents(stream.snapshot())), [stream]);

  function draw(): void {
    const container = containerRef.current;
    const canvas = canvasRef.current;
    if (!container || !canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
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
    const series = seriesRef.current;
    const plot = (points: { x: number; y: number }[], top: number, height: number, color: string) => {
      if (points.length === 0) return;
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.beginPath();
      points.forEach((p, i) => {
        const px = p.x * w;
        const py = top + height - p.y * height;
        if (i === 0) ctx.moveTo(px, py);
        else ctx.lineTo(px, py);
      });
      ctx.stroke();
    };
    // Drift band (top 40%).
    ctx.strokeStyle = '#16161a';
    ctx.strokeRect(0, 0, w, h * 0.4);
    plot(series.drift, 0, h * 0.4, '#00f0ff');
    // Compression band (middle 40%, zero line centered).
    const compTop = h * 0.45;
    const compHeight = h * 0.4;
    ctx.strokeStyle = '#16161a';
    ctx.strokeRect(0, compTop, w, compHeight);
    ctx.strokeStyle = '#3a3a44';
    ctx.beginPath();
    ctx.moveTo(0, compTop + compHeight / 2);
    ctx.lineTo(w, compTop + compHeight / 2);
    ctx.stroke();
    if (series.compression.length > 0) {
      ctx.strokeStyle = '#ff8a3d';
      ctx.lineWidth = 1;
      ctx.beginPath();
      series.compression.forEach((p, i) => {
        const px = p.x * w;
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
      ctx.moveTo(marker.x * w, 0);
      ctx.lineTo(marker.x * w, h * 0.85);
      ctx.stroke();
    }
    ctx.globalAlpha = 1;
    // Confidence strip (bottom).
    const stripY = h * 0.92;
    series.confidences.forEach((c, i) => {
      const px = series.confidences.length <= 1 ? w / 2 : (i / (series.confidences.length - 1)) * w;
      ctx.fillStyle = c >= 0.5 ? '#ffffff' : '#ff8a3d';
      ctx.beginPath();
      ctx.arc(px, stripY, 2.5, 0, Math.PI * 2);
      ctx.fill();
    });
    ctx.fillStyle = '#a1a1aa';
    ctx.font = `10px ${MONO_FONT}`;
    ctx.textAlign = 'left';
    ctx.fillText('EVENT ORDER', 4, h - 4);
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
    const fx = (event.clientX - rect.left) / Math.max(1, rect.width);
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

  // Redraw after every render whose events changed: the subscription
  // callback fires before React re-renders, so drawing only inside a
  // subscription would read the stale series (the bug the e2e Ask leg
  // caught — markers stayed 0 after a completed run).
  useEffect(() => {
    draw();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [events]);

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
