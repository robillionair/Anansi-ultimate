import { createFrameScheduler } from './frameScheduler.ts';
import { telemetryStream } from './telemetry.ts';

/** The real renderer owns this clock; no independent, empty RAF loop. */
export function createLarpRenderLoop(draw: (time: number) => void, stream = telemetryStream) {
  let disposed = false;
  const media = window.matchMedia('(prefers-reduced-motion: reduce)');
  const reduced = () => media.matches || document.documentElement.dataset.reducedMotion === 'true';
  const hidden = () => document.hidden || !document.hasFocus();
  const scheduler = createFrameScheduler({
    requestFrame: cb => window.requestAnimationFrame(cb),
    cancelFrame: id => window.cancelAnimationFrame(id),
    now: () => performance.now(),
  }, time => { if (!disposed) draw(time); });
  function sync() {
    if (disposed) return;
    if (reduced()) {
      scheduler.stop();
      if (!hidden()) draw(performance.now());
    } else {
      scheduler.setHidden(hidden());
      scheduler.start();
    }
  }
  function activity() {
    scheduler.notifyInteraction();
    if (reduced() && !hidden() && !disposed) draw(performance.now());
  }
  const observer = new MutationObserver(sync);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-reduced-motion'] });
  document.addEventListener('visibilitychange', sync);
  window.addEventListener('focus', sync);
  window.addEventListener('blur', sync);
  window.addEventListener('pointermove', activity, { passive: true });
  window.addEventListener('keydown', activity);
  media.addEventListener('change', sync);
  const unsubscribe = stream.subscribe(() => { activity(); });
  sync();
  return {
    redraw: activity,
    fps: () => reduced() ? 0 : scheduler.fps(),
    dispose() {
      if (disposed) return;
      disposed = true;
      scheduler.stop(); observer.disconnect(); unsubscribe();
      document.removeEventListener('visibilitychange', sync);
      window.removeEventListener('focus', sync); window.removeEventListener('blur', sync);
      window.removeEventListener('pointermove', activity); window.removeEventListener('keydown', activity);
      media.removeEventListener('change', sync);
    },
  };
}
