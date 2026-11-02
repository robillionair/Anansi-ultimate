import { useEffect, useRef, useState } from 'react';
import * as THREE from 'three';
import {
  buildGhostScene,
  detectWebGL,
  isReducedMotion,
} from './ghostPromptThreeScene.ts';
import {
  GHOST_BASELINE_HALF_WIDTH,
  layoutGhostDraft,
  type GhostDraftSnapshot,
} from './ghostPromptThreeLayout.ts';

function snapshotKey(snapshot: GhostDraftSnapshot): string {
  return JSON.stringify(snapshot);
}

function pauseBeforeEdit(
  snapshot: GhostDraftSnapshot,
  seq: number,
): number | null {
  const event = snapshot.events.find((item) => item.seq === seq);
  if (!event) return null;
  let found: number | null = null;
  for (const pause of snapshot.pauses) {
    if (pause.elapsedMs <= event.elapsedMs) found = pause.durationMs;
  }
  return found;
}

export function GhostPrompt3D({
  snapshot,
  initialReducedMotion,
}: {
  snapshot: GhostDraftSnapshot;
  initialReducedMotion?: boolean;
}) {
  const [reduceMotion, setReduceMotion] = useState(
    initialReducedMotion ?? isReducedMotion(),
  );
  const [webglFailed, setWebglFailed] = useState(false);
  const [selectedSeq, setSelectedSeq] = useState<number | null>(null);
  const selectedSeqRef = useRef<number | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const builtRef = useRef<ReturnType<typeof buildGhostScene> | null>(null);
  const rendererRef = useRef<THREE.WebGLRenderer | null>(null);
  const key = snapshotKey(snapshot);
  const layout = layoutGhostDraft(snapshot);
  const selectedFragment = snapshot.fragments.find(
    (fragment) => fragment.seq === selectedSeq,
  );
  const selectedPause =
    selectedSeq === null ? null : pauseBeforeEdit(snapshot, selectedSeq);

  // A new snapshot is a new draft: drop any selection into the old one.
  useEffect(() => {
    selectedSeqRef.current = null;
    setSelectedSeq(null);
  }, [key]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (reduceMotion || webglFailed || !canvas) return;
    if (!detectWebGL()) {
      setWebglFailed(true);
      return;
    }
    let renderer: THREE.WebGLRenderer;
    try {
      renderer = new THREE.WebGLRenderer({
        canvas,
        antialias: true,
        powerPreference: 'low-power',
      });
    } catch {
      setWebglFailed(true);
      return;
    }
    renderer.setPixelRatio(
      Math.min(typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1, 2),
    );
    const built = buildGhostScene(snapshot);
    builtRef.current = built;
    rendererRef.current = renderer;
    const resize = () => {
      const width = canvas.clientWidth || 640;
      const height = canvas.clientHeight || 360;
      renderer.setSize(width, height, false);
      built.camera.aspect = width / height;
      built.camera.updateProjectionMatrix();
    };
    resize();
    const onContextLost = (event: Event) => {
      event.preventDefault();
      setWebglFailed(true);
    };
    canvas.addEventListener('webglcontextlost', onContextLost);
    const onResize = () => resize();
    window.addEventListener('resize', onResize);
    let frame = 0;
    const loop = () => {
      built.animate(performance.now() / 1000);
      renderer.render(built.scene, built.camera);
      frame = requestAnimationFrame(loop);
    };
    renderer.render(built.scene, built.camera);
    frame = requestAnimationFrame(loop);
    const onClick = (event: MouseEvent) => {
      const rect = canvas.getBoundingClientRect();
      const ndcX = ((event.clientX - rect.left) / rect.width) * 2 - 1;
      const ndcY = -((event.clientY - rect.top) / rect.height) * 2 + 1;
      const seq = built.pick(ndcX, ndcY);
      if (seq !== null) {
        built.setSelected(seq);
        selectedSeqRef.current = seq;
        setSelectedSeq(seq);
      }
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'ArrowRight' && event.key !== 'ArrowLeft') return;
      const seqs = layout.nodes.map((node) => node.seq);
      if (seqs.length === 0) return;
      event.preventDefault();
      const current =
        selectedSeqRef.current === null
          ? -1
          : seqs.indexOf(selectedSeqRef.current);
      const next =
        event.key === 'ArrowRight'
          ? seqs[(current + 1) % seqs.length]
          : seqs[(current - 1 + seqs.length) % seqs.length];
      built.setSelected(next);
      selectedSeqRef.current = next;
      setSelectedSeq(next);
    };
    canvas.addEventListener('click', onClick);
    canvas.addEventListener('keydown', onKeyDown);
    return () => {
      cancelAnimationFrame(frame);
      canvas.removeEventListener('webglcontextlost', onContextLost);
      canvas.removeEventListener('click', onClick);
      canvas.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('resize', onResize);
      built.dispose();
      renderer.dispose();
      builtRef.current = null;
      rendererRef.current = null;
    };
    // Rebuild when the snapshot content or the motion mode changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, reduceMotion, webglFailed]);

  const selectFromList = (seq: number) => {
    builtRef.current?.setSelected(seq);
    selectedSeqRef.current = seq;
    setSelectedSeq(seq);
  };

  const historyItems: Array<{ key: string; text: string }> = [];
  const bySeq = [...snapshot.events].sort((a, b) => a.seq - b.seq);
  for (const event of bySeq) {
    for (const pause of snapshot.pauses.filter(
      (item) => item.elapsedMs === event.elapsedMs,
    )) {
      historyItems.push({
        key: `pause-${event.seq}`,
        text: `Pause of ${pause.durationMs}ms before edit ${event.seq}.`,
      });
    }
    historyItems.push({
      key: `edit-${event.seq}`,
      text:
        `Edit ${event.seq}: ${event.kind} at offset ${event.offset}` +
        (event.deleted !== '' ? `, deleted "${event.deleted}"` : '') +
        (event.inserted !== '' ? `, inserted "${event.inserted}"` : '') +
        '.',
    });
  }
  for (const pause of snapshot.pauses.filter(
    (item) => !bySeq.some((event) => event.elapsedMs === item.elapsedMs),
  )) {
    historyItems.push({
      key: `pause-trailing-${pause.elapsedMs}`,
      text: `Pause of ${pause.durationMs}ms at ${pause.elapsedMs}ms.`,
    });
  }

  const showStatic = reduceMotion || webglFailed;

  return (
    <section aria-label="GHOST-PROMPT 3D draft view">
      <h4>GHOST-PROMPT 3D draft view</h4>
      <label>
        <input
          type="checkbox"
          checked={reduceMotion}
          onChange={(event) => setReduceMotion(event.target.checked)}
        />
        Reduce motion (static view)
      </label>
      {showStatic && (
        <p>
          {webglFailed
            ? 'WebGL unavailable — showing the accessible 2D view.'
            : 'Motion reduced — static view.'}
        </p>
      )}
      {showStatic ? (
        <div>
          <svg
            viewBox="-220 -20 440 150"
            width="100%"
            height="260"
            role="img"
            aria-label="2D timeline of retained text, deleted fragments, pauses, and revision clusters"
          >
            <line
              x1={-GHOST_BASELINE_HALF_WIDTH}
              y1={0}
              x2={GHOST_BASELINE_HALF_WIDTH}
              y2={0}
              stroke="#f5f5f4"
              strokeWidth={2}
            />
            {layout.clusters.map((cluster) => (
              <rect
                key={cluster.id}
                x={cluster.minX}
                y={-4}
                width={Math.max(4, cluster.maxX - cluster.minX)}
                height={8}
                fill="#7c3aed"
                opacity={0.6}
              />
            ))}
            {layout.threads.map((thread) => (
              <line
                key={thread.nodeSeq}
                x1={thread.fromX}
                y1={thread.fromY}
                x2={thread.toX}
                y2={thread.toY}
                stroke="#a78bfa"
                strokeWidth={1}
                opacity={0.7}
              />
            ))}
            {layout.pauses.map((pause, index) => (
              <polygon
                key={index}
                points={`${pause.x},-8 ${pause.x + 6},0 ${pause.x},8 ${pause.x - 6},0`}
                fill="#c4b5fd"
              />
            ))}
            {layout.nodes.map((node) => (
              <circle
                key={node.id}
                cx={node.x}
                cy={node.y}
                r={6}
                fill="#8b5cf6"
                opacity={0.55}
              />
            ))}
          </svg>
          <div>
            {layout.nodes.map((node) => (
              <button
                key={node.id}
                type="button"
                onClick={() => selectFromList(node.seq)}
              >
                Select fragment {node.text}
              </button>
            ))}
          </div>
          <ol aria-label="Edit history">
            {historyItems.map((item) => (
              <li key={item.key}>{item.text}</li>
            ))}
          </ol>
        </div>
      ) : (
        <canvas
          ref={canvasRef}
          data-three-canvas
          tabIndex={0}
          aria-label="3D draft visualization canvas"
          style={{ width: '100%', height: '360px', display: 'block' }}
        />
      )}
      {selectedFragment && (
        <section aria-label="Selected fragment">
          <h5>Selected fragment</h5>
          <p>
            <s>{selectedFragment.text}</s> <span>deleted</span>
          </p>
          <p>Offset: {selectedFragment.offset}</p>
          <p>Hash: {selectedFragment.hash}</p>
          {selectedPause !== null && <p>Pause before edit: {selectedPause}ms</p>}
        </section>
      )}
    </section>
  );
}
