// LARP Mode Slice 1: in-memory telemetry bus. No live producer exists
// yet (no client method returns token counts or compiler events), so this
// slice delivers the bus, its validation, and the widget shells — widgets
// render explicit empty states until a real producer lands in Slice 2.
// Nothing here synthesizes events and presents them as live data.

export type MemoryNodeKind = 'note' | 'memory' | 'artifact';

export interface MemoryNode {
  id: string;
  kind: MemoryNodeKind;
  title: string;
}

export type TelemetryEvent =
  | {
      kind: 'token-delta';
      at: number;
      rawInputTokens: number;
      compiledInputTokens: number;
      rawOutputTokens: number;
      compiledOutputTokens: number;
      latencyMs: number;
    }
  | { kind: 'drift'; at: number; score: number }
  | { kind: 'memory'; at: number; nodes: MemoryNode[] }
  | { kind: 'compilation'; at: number; label: string };

export const TELEMETRY_RING_LIMIT = 500;

function cleanCount(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.floor(value));
}

function cleanDrift(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.min(1, Math.max(0, value));
}

/** Invalid payloads are clamped/floored, never thrown: a bad event must
 *  not take the HUD down. */
export function sanitizeTelemetryEvent(event: TelemetryEvent): TelemetryEvent {
  switch (event.kind) {
    case 'token-delta':
      return {
        kind: 'token-delta',
        at: event.at,
        rawInputTokens: cleanCount(event.rawInputTokens),
        compiledInputTokens: cleanCount(event.compiledInputTokens),
        rawOutputTokens: cleanCount(event.rawOutputTokens),
        compiledOutputTokens: cleanCount(event.compiledOutputTokens),
        latencyMs: cleanCount(event.latencyMs),
      };
    case 'drift':
      return { kind: 'drift', at: event.at, score: cleanDrift(event.score) };
    case 'memory':
      return {
        kind: 'memory',
        at: event.at,
        nodes: event.nodes.map((node) => ({ ...node })),
      };
    case 'compilation':
      return { kind: 'compilation', at: event.at, label: event.label };
  }
}

export class AnansiTelemetryStream {
  private events: TelemetryEvent[] = [];
  private listeners = new Set<(event: TelemetryEvent) => void>();

  push(event: TelemetryEvent): void {
    const clean = sanitizeTelemetryEvent(event);
    this.events.push(clean);
    if (this.events.length > TELEMETRY_RING_LIMIT) {
      this.events.splice(0, this.events.length - TELEMETRY_RING_LIMIT);
    }
    for (const listener of [...this.listeners]) listener(clean);
  }

  subscribe(listener: (event: TelemetryEvent) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  snapshot(): TelemetryEvent[] {
    return [...this.events];
  }

  clear(): void {
    this.events = [];
  }
}

/** Process-wide singleton the HUD subscribes to. Tests construct their
 *  own AnansiTelemetryStream instances instead. */
export const telemetryStream = new AnansiTelemetryStream();
