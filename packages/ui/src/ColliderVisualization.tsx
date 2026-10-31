/**
 * M8 visualization: the saved-experiment view's scene.
 * Renders the derived scene (see colliderVisualizationScene)
 * on one canvas plus a complete text/list alternative
 * that is always present and is the keyboard path.
 *
 * No API calls, no inference: derivation is synchronous
 * from the already-loaded document. Selection and
 * animation state are keyed by the pinned identity
 * (`artifactId@version`): switching experiments or
 * versions clears the selection and cancels the
 * animation frame before the new scene renders.
 * Reduced motion renders one static frame; otherwise a
 * slow orbital animation runs with a Pause/Resume
 * control and moves angles only — never nodes, edges,
 * or selection. All text is rendered via React text
 * nodes or canvas fillText, never via innerHTML, so
 * saved experiment text is displayed, never executed.
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { ColliderExperimentDoc } from '@anansi/contracts';
import {
  VISUALIZATION_SECTIONS,
  deriveVisualizationScene,
  hitTestVisualization,
  layoutVisualizationScene,
  sectionTitleFor,
  type VisualizationScene,
} from './colliderVisualizationScene';
import { ColliderThreeView } from './ColliderThreeView';

const BEAM_COLORS: Record<string, string> = {
  'beam-a': '#a78bfa',
  'beam-b': '#60a5fa',
};
const HUB_COLOR = '#e7e5e4';
const ITEM_COLOR = '#c4b5fd';
const EDGE_COLOR = '#57534e';

function drawScene(
  canvas: HTMLCanvasElement,
  scene: VisualizationScene,
  selectedId: string | null,
  angleOffset: number,
) {
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  const width = canvas.width;
  const height = canvas.height;
  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = '#0b2545';
  ctx.fillRect(0, 0, width, height);
  const layout = layoutVisualizationScene(scene, width, height);
  const cx = width / 2;
  const cy = height / 2;
  // Collision point at the center.
  ctx.beginPath();
  ctx.arc(cx, cy, 8, 0, Math.PI * 2);
  ctx.fillStyle = '#f5f5f4';
  ctx.fill();
  // Edges between section hubs.
  ctx.strokeStyle = EDGE_COLOR;
  ctx.lineWidth = 1;
  for (const edge of scene.edges) {
    const from = layout.positions.get(edge.from);
    const to = layout.positions.get(edge.to);
    if (!from || !to) continue;
    ctx.beginPath();
    ctx.moveTo(from.x, from.y);
    ctx.lineTo(to.x, to.y);
    ctx.stroke();
  }
  // Beam arcs: two streams feeding the collision point.
  ctx.lineWidth = 2;
  for (const beam of ['beam-a', 'beam-b']) {
    const hub = layout.positions.get(beam);
    if (!hub) continue;
    ctx.strokeStyle = BEAM_COLORS[beam] ?? HUB_COLOR;
    ctx.beginPath();
    ctx.moveTo(hub.x, hub.y);
    ctx.quadraticCurveTo(cx, cy, cx, cy);
    ctx.stroke();
  }
  // Nodes: items orbit their hub by the animation offset.
  for (const node of scene.nodes) {
    const pos = layout.positions.get(node.id);
    if (!pos) continue;
    let { x, y } = pos;
    if (node.role === 'item') {
      const hub = layout.positions.get(node.section);
      if (hub) {
        const dx = pos.x - hub.x;
        const dy = pos.y - hub.y;
        const dist = Math.hypot(dx, dy) || 1;
        const base = Math.atan2(dy, dx);
        const rotated = base + angleOffset;
        x = hub.x + dist * Math.cos(rotated);
        y = hub.y + dist * Math.sin(rotated);
      }
    }
    const selected = node.id === selectedId;
    ctx.beginPath();
    ctx.arc(x, y, pos.r + (selected ? 4 : 0), 0, Math.PI * 2);
    ctx.fillStyle =
      node.section === 'beam-a' || node.section === 'beam-b'
        ? (BEAM_COLORS[node.section] ?? HUB_COLOR)
        : node.role === 'section'
          ? HUB_COLOR
          : ITEM_COLOR;
    ctx.fill();
    if (selected) {
      ctx.lineWidth = 2;
      ctx.strokeStyle = '#fbbf24';
      ctx.stroke();
    }
    // VIS-02: labels were drawn in the canvas background
    // color (#0b2545) and unreadable. Draw restrained
    // white text over a dark backing rect so labels stay
    // legible over nodes, edges, and the navy field.
    const label =
      node.label.length > 18 ? `${node.label.slice(0, 17)}…` : node.label;
    ctx.font = '11px sans-serif';
    ctx.textAlign = 'center';
    const labelWidth = ctx.measureText(label).width;
    const labelY = y + pos.r + 14;
    ctx.fillStyle = 'rgba(11, 37, 69, 0.85)';
    ctx.fillRect(x - labelWidth / 2 - 3, labelY - 11, labelWidth + 6, 15);
    ctx.fillStyle = '#f5f5f4';
    ctx.fillText(label, x, labelY);
  }
}

export function ColliderVisualization({
  doc,
  artifactId,
  version,
}: {
  doc: ColliderExperimentDoc;
  artifactId: string;
  version: number;
}) {
  const scene = useMemo(
    () => deriveVisualizationScene(doc, artifactId, version),
    [doc, artifactId, version],
  );
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [paused, setPaused] = useState(false);
  const [canvasFailed, setCanvasFailed] = useState(false);
  // The identity whose 3D view reported WebGL failure.
  // Derived per identity (not reset by an effect) so a
  // fallback signal can never be clobbered by the
  // identity-change effect: child effects run before
  // parent effects, so resetting here on identity
  // change would overwrite the child's signal in the
  // same commit. A new identity simply does not match
  // the failed one, so the 3D view is tried again.
  const [failedIdentity, setFailedIdentity] = useState<string | null>(null);
  const [reducedMotion, setReducedMotion] = useState(false);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const angleRef = useRef(0);
  // Stable per identity so the 3D view's build effect
  // does not rebuild when the parent re-renders.
  const handleThreeUnavailable = useCallback(() => {
    setFailedIdentity(scene.identity);
  }, [scene.identity]);
  const webglFailed = failedIdentity === scene.identity;

  // Identity change clears stale selection and animation
  // state before the new scene renders.
  useEffect(() => {
    setSelectedId(null);
    setPaused(false);
    angleRef.current = 0;
  }, [scene.identity]);

  useEffect(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return;
    const query = window.matchMedia('(prefers-reduced-motion: reduce)');
    // System preference OR the workspace setting (the
    // app writes it to documentElement.dataset).
    const update = () =>
      setReducedMotion(
        query.matches ||
          document.documentElement.dataset.reducedMotion === 'true',
      );
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }, []);

  // Draw: one static frame under reduced motion or when
  // paused; otherwise a slow orbital animation loop.
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    if (!canvas.getContext('2d')) {
      setCanvasFailed(true);
      return;
    }
    const dpr = Math.min(
      typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1,
      2,
    );
    const cssWidth = canvas.clientWidth || 640;
    const cssHeight = 480;
    canvas.width = Math.round(cssWidth * dpr);
    canvas.height = Math.round(cssHeight * dpr);
    let frame = 0;
    const draw = () => {
      drawScene(canvas, scene, selectedId, angleRef.current);
    };
    if (reducedMotion || paused) {
      draw();
      return;
    }
    const tick = () => {
      angleRef.current += 0.002;
      draw();
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [scene, selectedId, paused, reducedMotion, webglFailed]);

  const selected = scene.nodes.find(n => n.id === selectedId) ?? null;

  function onCanvasClick(event: React.MouseEvent<HTMLCanvasElement>) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / Math.max(rect.width, 1);
    const scaleY = canvas.height / Math.max(rect.height, 1);
    const x = (event.clientX - rect.left) * scaleX;
    const y = (event.clientY - rect.top) * scaleY;
    // Hit-test against the CURRENT positions: items orbit
    // their hub by angleRef, so rotate each item offset
    // around its own hub (not the canvas center).
    const base = layoutVisualizationScene(scene, canvas.width, canvas.height);
    const positions = new Map(base.positions);
    for (const node of scene.nodes) {
      if (node.role !== 'item') continue;
      const pos = base.positions.get(node.id);
      const hub = base.positions.get(node.section);
      if (!pos || !hub) continue;
      const dx = pos.x - hub.x;
      const dy = pos.y - hub.y;
      const dist = Math.hypot(dx, dy) || 1;
      const angle = Math.atan2(dy, dx) + angleRef.current;
      positions.set(node.id, {
        ...pos,
        x: hub.x + dist * Math.cos(angle),
        y: hub.y + dist * Math.sin(angle),
      });
    }
    setSelectedId(hitTestVisualization({ ...base, positions }, x, y));
  }

  return (
    <section aria-labelledby="collider-visualization-heading">
      <h3 id="collider-visualization-heading">Experiment visualization</h3>
      <p>
        Visualization of experiment {artifactId} version {version} — provider{' '}
        {scene.provider}, model {scene.model}. Derived from the saved record;
        opening this view, selecting nodes, or switching versions sends
        nothing and starts no inference.
      </p>
      <p role="status">
        {scene.nodes.length} nodes, {scene.edges.length} relationships. The
        relationships are the pipeline flow between sections, not links
        mined from the text.
      </p>
      {webglFailed ? (
        canvasFailed ? (
          <p>
            Canvas rendering is unavailable here; the full node list below is
            the visualization.
          </p>
        ) : (
          <canvas
            ref={canvasRef}
            data-canvas-2d
            role="img"
            aria-label={`Visualization of the saved experiment: two beams feeding six result sections, ${scene.nodes.length} nodes in total. Use the node list below to select an element.`}
            width={640}
            height={480}
            style={{ width: '100%', maxWidth: 640, background: '#0b2545' }}
            onClick={onCanvasClick}
          />
        )
      ) : (
        <ColliderThreeView
          scene={scene}
          selectedId={selectedId}
          onSelect={setSelectedId}
          paused={paused}
          reducedMotion={reducedMotion}
          onUnavailable={handleThreeUnavailable}
        />
      )}
      {reducedMotion ? (
        <p>Motion reduced — static view.</p>
      ) : (
        <button type="button" onClick={() => setPaused(p => !p)}>
          {paused ? 'Resume motion' : 'Pause motion'}
        </button>
      )}
      <h4 id="collider-visualization-list-heading">
        Node list (text alternative)
      </h4>
      <p className="help">
        Every node in the visualization, grouped by section. Selecting a node
        shows its exact source text and provenance below.
      </p>
      {VISUALIZATION_SECTIONS.map(section => {
        const items = scene.nodes.filter(n => n.section === section.section);
        if (items.length === 0) return null;
        return (
          <div key={section.section}>
            <h5>{section.title}</h5>
            <ul>
              {items.map(node => (
                <li key={node.id}>
                  <button
                    type="button"
                    aria-pressed={selectedId === node.id}
                    onClick={() =>
                      setSelectedId(current =>
                        current === node.id ? null : node.id,
                      )
                    }
                  >
                    {node.role === 'section' ? `${node.label} (section)` : node.label}
                  </button>
                </li>
              ))}
            </ul>
          </div>
        );
      })}
      {selected && (
        <section aria-labelledby="collider-visualization-detail-heading">
          <h4 id="collider-visualization-detail-heading">
            Selected: {sectionTitleFor(selected.section)} — {selected.label}
          </h4>
          <pre>{selected.text}</pre>
          <ul>
            {selected.provenance.map((line, i) => (
              <li key={i}>{line}</li>
            ))}
          </ul>
        </section>
      )}
    </section>
  );
}
