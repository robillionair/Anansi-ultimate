/**
 * M8 Three.js visualization: the WebGL canvas for the
 * saved-experiment view. Owns the WebGLRenderer
 * lifecycle only — the scene itself comes from
 * colliderThreeScene, selection and pause state stay
 * with the parent (so the list alternative and the 3D
 * view always agree), and any WebGL failure reports
 * through onUnavailable so the parent falls back to
 * the Canvas 2D view without losing the list, the
 * detail panel, or provenance.
 *
 * Lifecycle guarantees: the renderer is created once
 * per pinned scene; the animation frame is cancelled
 * and every geometry, material, and texture is
 * disposed on unmount and on every identity switch
 * before the next scene builds; reduced motion or
 * pause renders exactly one static frame and starts
 * no loop; pixel ratio is clamped to 2 and resizes
 * update the camera and renderer together.
 */

import { useEffect, useRef } from 'react';
import * as THREE from 'three';
import {
  buildThreeScene,
  detectWebGL,
  type BuiltThreeScene,
} from './colliderThreeScene';
import type { VisualizationScene } from './colliderVisualizationScene';

const VIEW_HEIGHT = 480;

export function ColliderThreeView({
  scene,
  selectedId,
  onSelect,
  paused,
  reducedMotion,
  onUnavailable,
}: {
  scene: VisualizationScene;
  selectedId: string | null;
  onSelect: (id: string | null) => void;
  paused: boolean;
  reducedMotion: boolean;
  onUnavailable: () => void;
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const builtRef = useRef<BuiltThreeScene | null>(null);
  const rendererRef = useRef<THREE.WebGLRenderer | null>(null);

  // Build the renderer and scene for this pinned
  // version; tear everything down before the next one.
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
      renderer.setSize(width, VIEW_HEIGHT, false);
      built.camera.aspect = width / VIEW_HEIGHT;
      built.camera.updateProjectionMatrix();
      render();
    };
    const width = canvas.clientWidth || 640;
    renderer.setSize(width, VIEW_HEIGHT, false);
    const built = buildThreeScene(scene, width, VIEW_HEIGHT);
    built.camera.aspect = width / VIEW_HEIGHT;
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
    // scene identity drives rebuilds, not the callback.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scene]);

  // Animation: one static frame under reduced motion or
  // pause, otherwise a slow orbital loop.
  useEffect(() => {
    const built = builtRef.current;
    const renderer = rendererRef.current;
    if (!built || !renderer) return;
    if (paused || reducedMotion) {
      renderer.render(built.scene, built.camera);
      return;
    }
    let frame = 0;
    const start = performance.now();
    const tick = (now: number) => {
      built.animate((now - start) / 1000);
      renderer.render(built.scene, built.camera);
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [paused, reducedMotion, scene]);

  // Selection: recolor the picked instance and repaint
  // (a single frame even while the loop is paused).
  useEffect(() => {
    const built = builtRef.current;
    const renderer = rendererRef.current;
    if (!built || !renderer) return;
    built.setSelected(selectedId);
    renderer.render(built.scene, built.camera);
  }, [selectedId]);

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
      aria-label={`3D visualization of the saved experiment: two toroidal beam streams feeding six result sections, ${scene.nodes.length} nodes in total. Use the node list below to select an element.`}
      width={640}
      height={VIEW_HEIGHT}
      style={{ width: '100%', maxWidth: 640, background: '#0b2545' }}
      onClick={onCanvasClick}
    />
  );
}
