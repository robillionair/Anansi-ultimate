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
import { createLarpComposer } from './larp-mode/postprocessing/LarpComposer';
import { createLarpRenderLoop } from './larp-mode/renderLoop';
import { bindSceneNavigation } from './larp-mode/views/sceneNavigation';
import {
  ARENA_VIEW_HEIGHT,
  buildSwarmtankArena,
  detectWebGL,
  isReducedMotion,
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
  cinematic = false,
}: {
  cinematic?: boolean;
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
  const renderRef = useRef<(() => void) | null>(null);
  const pointerStart = useRef<{ x: number; y: number } | null>(null);

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
    const width = canvas.clientWidth || 640;
    const height = cinematic ? canvas.clientHeight || ARENA_VIEW_HEIGHT : ARENA_VIEW_HEIGHT;
    const built = buildSwarmtankArena(layout, width, height, { cinematic });
    let composer: ReturnType<typeof createLarpComposer> | undefined;
    if (cinematic) {
      try {
        composer = createLarpComposer(renderer, built.scene, built.camera, width, height);
      } catch {
        built.dispose();
        renderer.dispose();
        onUnavailable();
        return;
      }
    }
    const render = () => {
      if (composer) composer.render();
      else renderer.render(built.scene, built.camera);
    };
    const resize = () => {
      const width = canvas.clientWidth || 640;
      const height = cinematic ? canvas.clientHeight || ARENA_VIEW_HEIGHT : ARENA_VIEW_HEIGHT;
      renderer.setSize(width, height, false);
      composer?.setSize(width, height);
      built.camera.aspect = width / height;
      built.camera.updateProjectionMatrix();
      render();
    };
    builtRef.current = built;
    renderRef.current = render;
    const navigation = cinematic ? bindSceneNavigation(built.camera, canvas, render, {
      min: 120, max: 2000, pan: 420,
    }) : undefined;
    resize();
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
      navigation?.dispose();
      composer?.dispose();
      built.dispose();
      renderer.dispose();
      builtRef.current = null;
      renderRef.current = null;
    };
    // onUnavailable is a stable parent callback; the
    // run key drives rebuilds, not the callback.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [runKey, cinematic]);

  // Scenario re-clustering: retarget the existing
  // scene (never rebuild it) when the effective
  // layout changes. Declared after the build effect
  // so it runs after a rebuild on a run switch.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => {
    const built = builtRef.current;
    const render = renderRef.current;
    if (!built || !render) return;
    built.updateDecisions(layout, reducedMotion);
    render();
  }, [layoutKey, reducedMotion]);

  // Animation: one static frame under reduced motion,
  // otherwise a slow Confused orbit loop that also
  // advances any in-flight re-clustering transition.
  useEffect(() => {
    const built = builtRef.current;
    const render = renderRef.current;
    if (!built || !render) return;
    if (reducedMotion) {
      built.animate(0);
      render();
      return;
    }
    let frame = 0;
    const start = performance.now();
    let previous = start;
    const tick = (now: number) => {
      if (cinematic && isReducedMotion()) {
        built.advanceTransition(Infinity);
        built.animate(0);
        render();
        return;
      }
      built.advanceTransition(Math.min(now - previous, 100) / 1000);
      previous = now;
      built.animate((now - start) / 1000);
      render();
    };
    if (cinematic) {
      const loop = createLarpRenderLoop(tick);
      return () => loop.dispose();
    }
    const step = (now: number) => {
      tick(now);
      frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  }, [reducedMotion, runKey, cinematic]);

  // Selection: rescale the picked instance and repaint
  // (a single frame even while the loop is stopped).
  useEffect(() => {
    const built = builtRef.current;
    const render = renderRef.current;
    if (!built || !render) return;
    built.setSelected(selectedPersonaId);
    render();
  }, [selectedPersonaId, runKey, cinematic]);

  function onCanvasClick(event: React.MouseEvent<HTMLCanvasElement>) {
    const canvas = canvasRef.current;
    const built = builtRef.current;
    if (!canvas || !built) return;
    const start = pointerStart.current;
    pointerStart.current = null;
    if (cinematic && start && Math.hypot(event.clientX - start.x, event.clientY - start.y) > 5) return;
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
      data-interactive={cinematic || undefined}
      tabIndex={cinematic ? 0 : undefined}
      role="img"
      aria-label={`3D SwarmTank arena for this run: ${layout.agents.length} simulated persona agents around a central product pedestal, green for Convert, amber for Confused, red for Reject. Click an agent for details, or use the table below to select one.`}
      width={640}
      height={ARENA_VIEW_HEIGHT}
      style={cinematic ? undefined : { width: '100%', maxWidth: 640, background: '#0b2545' }}
      onPointerDownCapture={cinematic ? event => { pointerStart.current = { x: event.clientX, y: event.clientY }; } : undefined}
      onClick={onCanvasClick}
    />
  );
}
