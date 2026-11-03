// LARP Mode Slice 3: pure spectrum mapping for the quant drift view.
// X is EVENT ORDER (index-normalized 0..1), not wall time: the bus
// mixes Date.now() producer events with small-int test events, and an
// "EVENT ORDER" axis is honest where a time axis would not be.
//
// Series, all from real bus events only:
// - drift: drift-event scores (0..1).
// - compression: token-delta compression percent, clamped to -100..100.
// - markers: compilation events (vertical ticks).
// - confidences: finite confidence values from the latest memory event.
// Collision resonance is NOT mapped: no resonance score exists in the
// bus, so the view's legend states that plainly instead of rendering a
// fabricated series.

import type { TelemetryEvent } from '../telemetry.ts';

export interface SpectrumPoint {
  x: number;
  y: number;
}

export interface SpectrumSeries {
  drift: SpectrumPoint[];
  compression: SpectrumPoint[];
  markers: { x: number; label: string }[];
  confidences: number[];
}

export function compressionPercent(
  rawInput: number,
  compiledInput: number,
  rawOutput: number,
  compiledOutput: number,
): number {
  const raw = rawInput + rawOutput;
  const compiled = compiledInput + compiledOutput;
  if (raw <= 0) return 0;
  return Math.max(-100, Math.min(100, ((raw - compiled) / raw) * 100));
}

export function spectrumSeries(events: TelemetryEvent[]): SpectrumSeries {
  const drift: SpectrumPoint[] = [];
  const compression: SpectrumPoint[] = [];
  const markers: { x: number; label: string }[] = [];
  let confidences: number[] = [];
  const total = events.length;
  const x = (i: number) => (total <= 1 ? 0.5 : i / (total - 1));
  events.forEach((event, i) => {
    if (event.kind === 'drift') {
      drift.push({ x: x(i), y: event.score });
    } else if (event.kind === 'token-delta') {
      compression.push({
        x: x(i),
        y: compressionPercent(
          event.rawInputTokens,
          event.compiledInputTokens,
          event.rawOutputTokens,
          event.compiledOutputTokens,
        ),
      });
    } else if (event.kind === 'compilation') {
      markers.push({ x: x(i), label: event.label });
    } else if (event.kind === 'memory') {
      confidences = event.nodes
        .map((n) => n.confidence)
        .filter((c): c is number => typeof c === 'number' && Number.isFinite(c));
    }
  });
  return { drift, compression, markers, confidences };
}
