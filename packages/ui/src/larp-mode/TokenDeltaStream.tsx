// LARP Mode Slice 1: token compression delta stream. Pure list over
// token-delta events, latest first, capped at 50 rendered rows. No
// synthesized rows: an empty stream renders an explicit empty state.

import type { TelemetryEvent } from './telemetry.ts';

export type TokenDeltaEvent = Extract<TelemetryEvent, { kind: 'token-delta' }>;

export function tokenDeltaEvents(events: TelemetryEvent[]): TokenDeltaEvent[] {
  return events.filter((event): event is TokenDeltaEvent => event.kind === 'token-delta');
}

export function compressionDelta(event: TokenDeltaEvent): string {
  const raw = event.rawInputTokens + event.rawOutputTokens;
  const compiled = event.compiledInputTokens + event.compiledOutputTokens;
  if (raw === 0) return 'n/a';
  const pct = ((compiled - raw) / raw) * 100;
  return `${pct >= 0 ? '+' : ''}${pct.toFixed(1)}%`;
}

export function TokenDeltaStream({ events }: { events: TelemetryEvent[] }) {
  const deltas = tokenDeltaEvents(events).slice(-50).reverse();
  return (
    <section className="larp-panel" aria-labelledby="larp-delta-heading">
      <h3 id="larp-delta-heading">Token Compression Delta Stream</h3>
      {deltas.length === 0 ? (
        <p className="larp-empty">No telemetry events yet.</p>
      ) : (
        <ol className="larp-delta-log">
          {deltas.map((event, index) => (
            <li key={`${event.at}-${index}`}>
              <span>
                {event.rawInputTokens + event.rawOutputTokens} →{' '}
                {event.compiledInputTokens + event.compiledOutputTokens}
              </span>{' '}
              <span>{compressionDelta(event)}</span>{' '}
              <span>{event.latencyMs} ms</span>
              {event.estimated ? <span>est.</span> : null}
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}
