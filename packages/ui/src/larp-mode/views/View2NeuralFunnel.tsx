// LARP Mode Slice 2: View 2 — NEURAL FUNNEL // CONTEXT COMPILER.
// High-DPI canvas curves are composited through the shared WebGL optics,
// with direct Canvas 2D fallback. Four semantic inputs, each with braided
// ornamental sub-filaments, converge on the singularity node
// (ANANSI.COMPILER // COMPRESSED) and fan out as three cyan outputs to
// the L1/L2/L3 blocks. Photon pulses spawn from real compilation events:
// live via subscription, plus a mount-time seed from compilation events
// in the stream snapshot (last 60 s, capped at 8) so mount order cannot
// lose a pulse. data-pulses-fired (cumulative, via ref) is the
// e2e-visible counter. Disposal cancels RAF, unsubscribes, and
// unregisters the store disposer.

import { useEffect, useRef } from 'react';
import { filamentLayout, cubicBezierPoint, spawnPulse, advancePulses, type Pulse } from './bezier.ts';
import { telemetryStream, type TelemetryEvent } from '../telemetry.ts';
import { registerDisposer } from '../store.ts';
import { createLarpRenderLoop } from '../renderLoop.ts';
import { createCanvasFilmSurface } from '../postprocessing/CanvasFilmSurface.ts';

const MONO_FONT = '"Berkeley Mono", "JetBrains Mono", ui-monospace, Menlo, monospace';

export function View2NeuralFunnel({ stream = telemetryStream }: { stream?: typeof telemetryStream }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const container = containerRef.current;
    const canvas = canvasRef.current;
    if (!container || !canvas) return;
    const surface = createCanvasFilmSurface(canvas);
    if (!surface) return;
    const ctx = surface.ctx;
    const els = { container, canvas, ctx };

    let pulses: Pulse[] = [];
    let nextId = 1;
    let fired = 0;
    let cleaned = false;
    let last = performance.now();
    const motionPreference = window.matchMedia('(prefers-reduced-motion: reduce)');
    const isReduced = () => document.documentElement.dataset.reducedMotion === 'true' || motionPreference.matches;

    function spawn(): void {
      const layout = filamentLayout(els.canvas.clientWidth || 640, els.canvas.clientHeight || 480);
      pulses.push(spawnPulse(nextId++, fired % layout.inputs.length));
      fired += 1;
      els.container.dataset.pulsesFired = String(fired);
    }

    for (const event of stream.snapshot()) {
      if (event.kind === 'compilation' && Date.now() - event.at < 60000 && fired < 8) spawn();
    }
    const unsubscribe = stream.subscribe((event: TelemetryEvent) => {
      if (!cleaned && event.kind === 'compilation') spawn();
    });

    function draw(): void {
      const w = els.canvas.clientWidth || 640;
      const h = els.canvas.clientHeight || 480;
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      surface!.resize(w,h,dpr);
      els.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      els.ctx.clearRect(0, 0, w, h);
      els.ctx.fillStyle = '#030305';
      els.ctx.fillRect(0, 0, w, h);
      const layout = filamentLayout(w, h);
      els.ctx.lineCap = 'round';
      // Braided sub-filaments are scenic geometry, not fabricated compilation events.
      const time = performance.now() / 1000;
      const reduced = isReduced();
      for (const [index, filament] of [...layout.inputs, ...layout.outputs].entries()) {
        const [p0, p1, p2, p3] = filament.points;
        const input = index < layout.inputs.length;
        for (let strand = 0; strand < 28; strand++) {
          const spread = (strand - 13.5) / 13.5;
          const wave = reduced ? 0 : Math.sin(time * .18 + strand * .35 + index) * 5;
          const fan = spread * h * .16;
          els.ctx.strokeStyle = strand % 7 === 0 ? '#f0caca' : input ? '#d4b988' : '#8fc6cc';
          els.ctx.globalAlpha = .12 + (1 - Math.abs(spread)) * .25;
          els.ctx.lineWidth = strand % 9 === 0 ? 1 : .55;
          els.ctx.beginPath();
          els.ctx.moveTo(p0.x, p0.y + (input ? fan : spread * 2));
          els.ctx.bezierCurveTo(p1.x, p1.y + fan + wave, p2.x, p2.y - fan * .4, p3.x, p3.y + (input ? spread * 2 : fan));
          els.ctx.stroke();
        }
      }
      const glow = els.ctx.createRadialGradient(layout.singularity.x, layout.singularity.y, 0, layout.singularity.x, layout.singularity.y, 80);
      glow.addColorStop(0, 'rgba(255,235,201,.75)');
      glow.addColorStop(.08, 'rgba(245,201,155,.28)');
      glow.addColorStop(1, 'rgba(216,141,158,0)');
      els.ctx.globalAlpha = 1;
      els.ctx.fillStyle = glow;
      els.ctx.fillRect(layout.singularity.x - 80, layout.singularity.y - 80, 160, 160);
      els.ctx.globalAlpha = 1;
      for (const block of layout.blocks) {
        els.ctx.strokeStyle = '#00f0ff';
        els.ctx.lineWidth = 1;
        els.ctx.strokeRect(block.x - 28, block.y - 14, 56, 28);
        els.ctx.fillStyle = '#e8e8ec';
        els.ctx.font = `10px ${MONO_FONT}`;
        els.ctx.textAlign = 'center';
        els.ctx.textBaseline = 'middle';
        els.ctx.fillText(block.label, block.x, block.y);
      }
      els.ctx.beginPath();
      els.ctx.arc(layout.singularity.x, layout.singularity.y, 10, 0, Math.PI * 2);
      els.ctx.fillStyle = '#030305';
      els.ctx.fill();
      els.ctx.strokeStyle = '#00f0ff';
      els.ctx.stroke();
      els.ctx.fillStyle = '#e8e8ec';
      els.ctx.font = `10px ${MONO_FONT}`;
      els.ctx.textAlign = 'center';
      els.ctx.fillText('ANANSI.COMPILER // COMPRESSED', layout.singularity.x, layout.singularity.y - 22);
      for (const pulse of pulses) {
        const filament = layout.inputs[pulse.filamentIndex % layout.inputs.length];
        const pos = cubicBezierPoint(...filament.points, pulse.t);
        els.ctx.globalAlpha = 0.25;
        els.ctx.fillStyle = filament.color;
        els.ctx.beginPath();
        els.ctx.arc(pos.x, pos.y, 6, 0, Math.PI * 2);
        els.ctx.fill();
        els.ctx.globalAlpha = 1;
        els.ctx.beginPath();
        els.ctx.arc(pos.x, pos.y, 2.5, 0, Math.PI * 2);
        els.ctx.fill();
      }
      els.ctx.globalAlpha = 1;
      surface!.present();
    }

    function frame(now: number): void {
      if (cleaned) return;
      if (!isReduced()) pulses = advancePulses(pulses, now - last);
      last = now;
      draw();

    }
    draw();
    const loop = createLarpRenderLoop(frame, stream);

    function cleanup(): void {
      if (cleaned) return;
      cleaned = true;
      loop.dispose();
      unsubscribe();
      surface!.dispose();
    }
    const unregister = registerDisposer(cleanup);
    return () => {
      cleanup();
      unregister();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div ref={containerRef} className="larp-view-container" aria-label="NEURAL FUNNEL // CONTEXT COMPILER">
      <canvas ref={canvasRef} className="larp-view-canvas" aria-label="Neural funnel bezier streams" />
    </div>
  );
}
