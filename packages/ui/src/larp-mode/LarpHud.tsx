// LARP Mode Slice 1: perimeter HUD shell. Fixed 100vw x 100vh grid
// frame; the frame itself is pointer-events: none so the underlying app
// stays usable — only the view selector buttons take pointer events.
// Mounting the HUD starts the frame scheduler and registers its RAF
// handle, visibility listener, and activity listeners as store disposers;
// unmounting (toggle OFF) runs the React cleanups and the store already
// ran disposeAll() before notifying, so no RAF handle or listener can
// survive OFF. The centerpiece is an explicit placeholder: selector state
// works, view content arrives in Slice 2.

import { useEffect, useState } from 'react';
import { createFrameScheduler } from './frameScheduler.ts';
import { registerDisposer } from './store.ts';
import { telemetryStream, type TelemetryEvent } from './telemetry.ts';
import { MemoryMatrix, latestMemoryNodes } from './MemoryMatrix.tsx';
import { TokenDeltaStream } from './TokenDeltaStream.tsx';
import { CoherenceWaveform } from './CoherenceWaveform.tsx';
import { View1MemoryVault } from './views/View1MemoryVault.tsx';
import { View2NeuralFunnel } from './views/View2NeuralFunnel.tsx';
import { View3AttentionSpider } from './views/View3AttentionSpider.tsx';
import { View4QuantDrift } from './views/View4QuantDrift.tsx';
import { View5AttentionHeatmap } from './views/View5AttentionHeatmap.tsx';
import { View6ColliderField } from './views/View6ColliderField.tsx';

const VIEW_BUTTONS = [1, 2, 3, 4, 5, 6];

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return (
    target.tagName === 'INPUT' ||
    target.tagName === 'TEXTAREA' ||
    target.tagName === 'SELECT' ||
    target.isContentEditable
  );
}

export function LarpHud({
  stream = telemetryStream,
  initialView = 1,
}: {
  stream?: typeof telemetryStream;
  initialView?: number;
}) {
  const [events, setEvents] = useState<TelemetryEvent[]>(() => stream.snapshot());
  const [view, setView] = useState(initialView);

  useEffect(() => stream.subscribe(() => setEvents(stream.snapshot())), [stream]);

  // Bare 1-6 hot-switches views. Typing in an input must never switch
  // views, so editable targets are ignored. Modifier keys are ignored
  // too (Ctrl+Shift+L belongs to the toggle, not the switcher).
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      if (isEditableTarget(event.target)) return;
      if (['1', '2', '3', '4', '5', '6'].includes(event.key)) setView(Number(event.key));
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  useEffect(() => {
    const scheduler = createFrameScheduler(
      {
        requestFrame: (callback) => window.requestAnimationFrame(callback),
        cancelFrame: (handle) => window.cancelAnimationFrame(handle),
        now: () => performance.now(),
      },
      () => {},
    );
    scheduler.start();
    const onVisibility = () => scheduler.setHidden(document.hidden);
    const onActivity = () => scheduler.notifyInteraction();
    document.addEventListener('visibilitychange', onVisibility);
    window.addEventListener('pointerdown', onActivity);
    window.addEventListener('keydown', onActivity);
    const unregister = [
      registerDisposer(() => scheduler.stop()),
      registerDisposer(() => document.removeEventListener('visibilitychange', onVisibility)),
      registerDisposer(() => window.removeEventListener('pointerdown', onActivity)),
      registerDisposer(() => window.removeEventListener('keydown', onActivity)),
    ];
    return () => {
      scheduler.stop();
      document.removeEventListener('visibilitychange', onVisibility);
      window.removeEventListener('pointerdown', onActivity);
      window.removeEventListener('keydown', onActivity);
      unregister.forEach((off) => off());
    };
  }, []);

  return (
    <div className="larp-hud" role="complementary" aria-label="LARP telemetry frame">
      <div className="larp-rail larp-rail-top">
        <span className="larp-hud-title">ANANSI // LARP TELEMETRY</span>
        <span className="larp-hud-status">[ LARP: ACTIVE ]</span>
      </div>
      <div className="larp-rail larp-rail-left">
        <MemoryMatrix nodes={latestMemoryNodes(events)} />
      </div>
      <div className="larp-center">
        <section
          className={'larp-centerpiece larp-centerpiece-view'}
          aria-labelledby="larp-center-heading"
        >
          <h3 id="larp-center-heading">Centerpiece</h3>
          <div className="larp-view-buttons" role="group" aria-label="LARP centerpiece view">
            {VIEW_BUTTONS.map((n) => (
              <button
                key={n}
                type="button"
                className={view === n ? 'larp-view-button larp-view-button-on' : 'larp-view-button'}
                aria-pressed={view === n}
                aria-label={`LARP view ${n}`}
                onClick={() => setView(n)}
              >
                {n}
              </button>
            ))}
          </div>
          {view === 1 ? (
            <View1MemoryVault stream={stream} />
          ) : view === 2 ? (
            <View2NeuralFunnel stream={stream} />
          ) : view === 3 ? (
            <View3AttentionSpider stream={stream} />
          ) : view === 4 ? (
            <View4QuantDrift stream={stream} />
          ) : view === 5 ? (
            <View5AttentionHeatmap stream={stream} />
          ) : view === 6 ? (
            <View6ColliderField stream={stream} />
          ) : null}
        </section>
      </div>
      <div className="larp-rail larp-rail-right">
        <TokenDeltaStream events={events} />
        <CoherenceWaveform events={events} />
      </div>
      <div className="larp-rail larp-rail-bottom">
        <span>TELEMETRY BUS · IN-MEMORY ONLY · NO PROVIDER TRAFFIC</span>
      </div>
    </div>
  );
}
