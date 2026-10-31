/**
 * M9 SwarmTank Slice 2: the WebGL canvas for the
 * arena. Owns the WebGLRenderer lifecycle only — the
 * scene itself comes from swarmtankThreeScene,
 * selection stays with the parent (so the table
 * alternative and the 3D view always agree), and any
 * WebGL failure reports through onUnavailable so the
 * parent falls back to the table without losing the
 * report or the detail card.
 *
 * Lifecycle guarantees: the renderer is created once
 * per run key; the animation frame is cancelled and
 * every geometry, material, and texture is disposed
 * on unmount and on every run switch before the next
 * scene builds; reduced motion renders exactly one
 * static frame and starts no loop; pixel ratio is
 * clamped to 2 and resizes update the camera and
 * renderer together.
 */

import { useEffect, useRef } from 'react';
import * as THREE from 'three';
import {
  ARENA_VIEW_HEIGHT,
  buildSwarmtankArena,
  detectWebGL,
  type BuiltArenaScene,
} from './swarmtankThreeScene';
import type { ArenaLayout } from './swarmtankArenaLayout';

export function SwarmtankThreeView({
  layout,
  layoutKey,
  runKey,
  selectedPersonaId,
  onSelect,
  reducedMotion,
  onUnavailable,
}: {
  layout: ArenaLayout;
  /** Changes when the effective layout changes (run or scenario). */
  layoutKey: string;
  runKey: string;
  selectedPersonaId: string | null;
  onSelect: (id: string | null) => void;
  reducedMotion: boolean;
  onUnavailable: () => void;
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const builtRef = useRef<BuiltArenaScene | null>(null);
  const rendererRef = useRef<THREE.WebGLRenderer | null>(null);

  // Build the renderer and scene for this run; tear
  // everything down before the next one.
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    if (!detectWebGL()) {
      onUnavailable();
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
      onUnavailable();
      return;
    }
    renderer.setPixelRatio(
      Math.min(
        typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1,
        2,
      ),
    );
    const render = () => {
      const built = builtRef.current;
      if (built) renderer.render(built.scene, built.camera);
    };
    const resize = () => {
      const built = builtRef.current;
      if (!built) return;
      const width = canvas.clientWidth || 640;
      renderer.setSize(width, ARENA_VIEW_HEIGHT, false);
      built.camera.aspect = width / ARENA_VIEW_HEIGHT;
      built.camera.updateProjectionMatrix();
      render();
    };
    const width = canvas.clientWidth || 640;
    renderer.setSize(width, ARENA_VIEW_HEIGHT, false);
    const built = buildSwarmtankArena(layout, width, ARENA_VIEW_HEIGHT);
    built.camera.aspect = width / ARENA_VIEW_HEIGHT;
    built.camera.updateProjectionMatrix();
    builtRef.current = built;
    rendererRef.current = renderer;
    render();
    const onLost = (event: Event) => {
      event.preventDefault();
      onUnavailable();
    };
    canvas.addEventListener('webglcontextlost', onLost);
    const observer =
      typeof ResizeObserver !== 'undefined'
        ? new ResizeObserver(resize)
        : null;
    observer?.observe(canvas);
    return () => {
      observer?.disconnect();
      canvas.removeEventListener('webglcontextlost', onLost);
      built.dispose();
      renderer.dispose();
      builtRef.current = null;
      rendererRef.current = null;
    };
    // onUnavailable is a stable parent callback; the
    // run key drives rebuilds, not the callback.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [runKey]);

  // Scenario re-clustering: retarget the existing
  // scene (never rebuild it) when the effective
  // layout changes. Declared after the build effect
  // so it runs after a rebuild on a run switch.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => {
    const built = builtRef.current;
    const renderer = rendererRef.current;
    if (!built || !renderer) return;
    built.updateDecisions(layout, reducedMotion);
    renderer.render(built.scene, built.camera);
  }, [layoutKey, reducedMotion]);

  // Animation: one static frame under reduced motion,
  // otherwise a slow Confused orbit loop that also
  // advances any in-flight re-clustering transition.
  useEffect(() => {
    const built = builtRef.current;
    const renderer = rendererRef.current;
    if (!built || !renderer) return;
    if (reducedMotion) {
      built.animate(0);
      renderer.render(built.scene, built.camera);
      return;
    }
    let frame = 0;
    const start = performance.now();
    let previous = start;
    const tick = (now: number) => {
      built.advanceTransition((now - previous) / 1000);
      previous = now;
      built.animate((now - start) / 1000);
      renderer.render(built.scene, built.camera);
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [reducedMotion, runKey]);

  // Selection: rescale the picked instance and repaint
  // (a single frame even while the loop is stopped).
  useEffect(() => {
    const built = builtRef.current;
    const renderer = rendererRef.current;
    if (!built || !renderer) return;
    built.setSelected(selectedPersonaId);
    renderer.render(built.scene, built.camera);
  }, [selectedPersonaId]);

  function onCanvasClick(event: React.MouseEvent<HTMLCanvasElement>) {
    const canvas = canvasRef.current;
    const built = builtRef.current;
    if (!canvas || !built) return;
    const rect = canvas.getBoundingClientRect();
    const ndcX =
      ((event.clientX - rect.left) / Math.max(rect.width, 1)) * 2 - 1;
    const ndcY =
      -(((event.clientY - rect.top) / Math.max(rect.height, 1)) * 2 - 1);
    onSelect(built.pick(ndcX, ndcY));
  }

  return (
    <canvas
      ref={canvasRef}
      data-three-canvas
      role="img"
      aria-label={`3D SwarmTank arena for this run: ${layout.agents.length} simulated persona agents around a central product pedestal, green for Convert, amber for Confused, red for Reject. Click an agent for details, or use the table below to select one.`}
      width={640}
      height={ARENA_VIEW_HEIGHT}
      style={{ width: '100%', maxWidth: 640, background: '#0b2545' }}
      onClick={onCanvasClick}
    />
  );
}
