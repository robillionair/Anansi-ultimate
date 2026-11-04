// LARP Mode Slice 2: View 1 — SECOND BRAIN // 3D MEMORY VAULT.
// 50,000 instanced particles (one THREE.Points draw call) clustered
// around centroids from the latest memory-event nodes, rendered through
// the LARP post-processing pipeline (bloom, aberration, grain, output).
// The bokeh shader renders focal-plane particles as sharp white-hot
// cores and off-plane ones as translucent discs. Cluster tags are
// projected per frame through the camera matrix into CSS positions,
// updated imperatively via refs (no React state per frame). A
// data-frames counter on the container is the e2e-visible proof of
// rendering. WebGL creation failure renders an explicit fallback panel
// instead of a blank canvas. Disposal cancels RAF, disposes the
// composer/passes/geometry/material/renderer, and forces context loss;
// the same cleanup is registered as a store disposer so OFF disposes
// even if React cleanup ordering surprises us.

import { useEffect, useRef, useState } from 'react';
import * as THREE from 'three';
import { createLarpComposer, clampPixelRatio } from '../postprocessing/LarpComposer.ts';
import { clusterCentroids, generateParticles } from './particleLayout.ts';
import { BOKEH_VERTEX, BOKEH_FRAGMENT, VAULT_CAMERA_Z } from './vaultShaders.ts';
import { latestMemoryNodes } from '../MemoryMatrix.tsx';
import { telemetryStream } from '../telemetry.ts';
import { registerDisposer } from '../store.ts';
import { createLarpRenderLoop } from '../renderLoop.ts';
import { createOrbitalAtmosphere } from './orbitalAtmosphere.ts';

const PARTICLE_SEED = 20261103;

export function View1MemoryVault({ stream = telemetryStream }: { stream?: typeof telemetryStream }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const tagLayerRef = useRef<HTMLDivElement>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    const container = containerRef.current;
    const canvas = canvasRef.current;
    const tagLayer = tagLayerRef.current;
    if (!container || !canvas || !tagLayer) return;
    const els = { container, canvas, tagLayer };
    const nodes = latestMemoryNodes(stream.snapshot());
    const centroids = clusterCentroids(nodes);
    const labels = nodes.length > 0 ? nodes.map((n) => n.title) : ['AMBIENT / MEMORY', 'AMBIENT / PROJECTS', 'AMBIENT / NOTES'];
    const motionPreference = window.matchMedia('(prefers-reduced-motion: reduce)');
    const isReduced = () => document.documentElement.dataset.reducedMotion === 'true' || motionPreference.matches;

    let renderer: THREE.WebGLRenderer;
    try {
      renderer = new THREE.WebGLRenderer({ canvas: els.canvas, antialias: true });
    } catch {
      setFailed(true);
      return;
    }
    renderer.setPixelRatio(clampPixelRatio(window.devicePixelRatio || 1));
    const scene = new THREE.Scene();
    scene.background = new THREE.Color('#030305');
    const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 1000);
    camera.position.set(0, 0, VAULT_CAMERA_Z);
    camera.lookAt(0, 0, 0);

    const data = generateParticles(PARTICLE_SEED, centroids);
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(data.positions, 3));
    geometry.setAttribute('aScale', new THREE.BufferAttribute(data.scales, 1));
    geometry.setAttribute('aSeed', new THREE.BufferAttribute(data.seeds, 1));
    const material = new THREE.ShaderMaterial({
      vertexShader: BOKEH_VERTEX,
      fragmentShader: BOKEH_FRAGMENT,
      uniforms: { uFocalDepth: { value: VAULT_CAMERA_Z }, uTime: { value: 0 } },
      transparent: true,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
    });
    const points = new THREE.Points(geometry, material);
    scene.add(points);
    const atmosphere = createOrbitalAtmosphere(scene, 9000);

    const tagDivs = centroids.map((_, i) => {
      const div = document.createElement('div');
      div.className = 'larp-tag';
      div.textContent = labels[i] ?? `Cluster ${i + 1}`;
      els.tagLayer.appendChild(div);
      return div;
    });
    const projected = new THREE.Vector3();

    function resize(): void {
      const w = els.container.clientWidth || 640;
      const h = els.container.clientHeight || 480;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      composerHandle.setSize(w, h);
    }

    const composerHandle = createLarpComposer(
      renderer,
      scene,
      camera,
      els.container.clientWidth || 640,
      els.container.clientHeight || 480,
    );
    resize();

    let frames = 0;
    let cleaned = false;
    const observer = new ResizeObserver(() => { resize(); loop.redraw(); });
    observer.observe(els.container);

    function frame(time: number): void {
      if (cleaned) return;
      const t = isReduced() ? 0 : time / 1000;
      points.rotation.y = Math.sin(t*.06)*.16;
      material.uniforms.uTime.value = t;
      atmosphere.update(t);
      composerHandle.grainPass.uniforms.uTime.value = isReduced() ? 0 : time / 1000;
      composerHandle.render();
      frames += 1;
      els.container.dataset.frames = String(frames);
      scene.updateMatrixWorld(true);
      for (let i = 0; i < centroids.length; i++) {
        const c = centroids[i];
        projected.set(c.x, c.y, c.z).applyMatrix4(points.matrixWorld).project(camera);
        const div = tagDivs[i];
        if (projected.z > 1) {
          div.style.display = 'none';
        } else {
          div.style.display = 'block';
          div.style.left = `${(projected.x * 0.5 + 0.5) * 100}%`;
          div.style.top = `${(-projected.y * 0.5 + 0.5) * 100}%`;
        }
      }
    }
    frame(0);
    const loop = createLarpRenderLoop(frame, stream);

    function cleanup(): void {
      if (cleaned) return;
      cleaned = true;
      loop.dispose();
      observer.disconnect();
      composerHandle.dispose();
      geometry.dispose();
      material.dispose();
      atmosphere.dispose();
      renderer.dispose();
      renderer.forceContextLoss();
      els.tagLayer.innerHTML = '';
    }
    const unregister = registerDisposer(cleanup);
    return () => {
      cleanup();
      unregister();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (failed) {
    return (
      <div className="larp-view-container" aria-label="SECOND BRAIN // 3D MEMORY VAULT">
        <div className="larp-view-fallback">
          <p>WebGL unavailable — the memory vault cannot render here.</p>
          <p>Centroids: Memory · Projects · Notes</p>
        </div>
      </div>
    );
  }

  return (
    <div ref={containerRef} className="larp-view-container" aria-label="SECOND BRAIN // 3D MEMORY VAULT">
      <canvas ref={canvasRef} className="larp-view-canvas" aria-label="Memory vault particle field" />
      <div ref={tagLayerRef} className="larp-tag-layer" aria-hidden="true" />
    </div>
  );
}
