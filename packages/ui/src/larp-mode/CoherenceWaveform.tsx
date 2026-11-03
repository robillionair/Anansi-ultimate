// LARP Mode Slice 1: coherence waveforms. Two anti-aliased bezier layers
// on one high-DPI canvas: raw-token velocity (cyan) and compiled-token
// velocity (magenta), derived from token-delta event timestamps. Events
// closer than 1 ms apart collapse to a 1 ms gap so velocity cannot spike
// to infinity on simultaneous stamps. Fewer than two token events renders
// the empty state, not a flat line pretending to be data.

import { useEffect, useRef } from 'react';
import type { TelemetryEvent } from './telemetry.ts';
import { tokenDeltaEvents } from './TokenDeltaStream.tsx';

export interface VelocitySeries {
  raw: number[];
  compiled: number[];
}

export function tokenVelocitySeries(events: TelemetryEvent[]): VelocitySeries {
  const deltas = tokenDeltaEvents(events);
  const raw: number[] = [];
  const compiled: number[] = [];
  for (let i = 1; i < deltas.length; i++) {
    const dtMs = Math.max(1, deltas[i].at - deltas[i - 1].at);
    const rawTokens =
      deltas[i].rawInputTokens + deltas[i].rawOutputTokens - (deltas[i - 1].rawInputTokens + deltas[i - 1].rawOutputTokens);
    const compiledTokens =
      deltas[i].compiledInputTokens + deltas[i].compiledOutputTokens - (deltas[i - 1].compiledInputTokens + deltas[i - 1].compiledOutputTokens);
    raw.push((rawTokens / dtMs) * 1000);
    compiled.push((compiledTokens / dtMs) * 1000);
  }
  return { raw, compiled };
}

function drawSeries(
  ctx: CanvasRenderingContext2D,
  series: number[],
  width: number,
  height: number,
  color: string,
): void {
  if (series.length === 0) return;
  const max = Math.max(...series, 1);
  const points = series.map((value, index) => ({
    x: series.length === 1 ? width / 2 : (index / (series.length - 1)) * width,
    y: height - (value / max) * (height - 4) - 2,
  }));
  ctx.strokeStyle = color;
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(points[0].x, points[0].y);
  for (let i = 1; i < points.length; i++) {
    const prev = points[i - 1];
    const curr = points[i];
    const midX = (prev.x + curr.x) / 2;
    ctx.quadraticCurveTo(prev.x, prev.y, midX, (prev.y + curr.y) / 2);
    ctx.quadraticCurveTo(midX, (prev.y + curr.y) / 2, curr.x, curr.y);
  }
  ctx.stroke();
}

export function CoherenceWaveform({ events }: { events: TelemetryEvent[] }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const width = 240;
  const height = 80;
  const series = tokenVelocitySeries(events);

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
    drawSeries(ctx, series.raw, width, height, '#00f0ff');
    drawSeries(ctx, series.compiled, width, height, '#ff1e7a');
  }, [events]);

  return (
    <section className="larp-panel" aria-labelledby="larp-wave-heading">
      <h3 id="larp-wave-heading">Coherence Waveforms</h3>
      <canvas
        ref={ref}
        className="larp-wave-canvas"
        style={{ width: `${width}px`, height: `${height}px` }}
        role="img"
        aria-label="Token velocity coherence waveforms"
      />
      {series.raw.length === 0 && <p className="larp-empty">Need two token events for velocity.</p>}
    </section>
  );
}
