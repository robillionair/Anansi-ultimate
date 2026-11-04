// LARP Mode Slice 4: View 6 — COLLIDER FIELD // CONCEPT RESONANCE
// ACCELERATOR. Two particle beams streaming along beziers into the
// interaction point at the origin, through the shared LarpComposer
// pipeline. The beams are a STYLIZED continuous visualization, not
// measured collider telemetry; the discharge flash + counter respond
// ONLY to real collision events (completed LEXI-COLLIDER generations
// pushed by the collider producer). Doppler coloring is a visual
// effect (beam color far away, white-hot at the point), not measured
// physics. Discharges seed from collision events in the last 60 s so
// mount order cannot lose one (View 2's pulse-seeding pattern).
// data-frames / data-collisions are the e2e-visible counters.
// Disposal mirrors Views 1–3 (composer, geometries, materials,
// renderer, forced context loss, store disposer).

import { useEffect, useRef, useState } from 'react';
import * as THREE from 'three';
import { createLarpComposer, clampPixelRatio, BLOOM_STRENGTH } from '../postprocessing/LarpComposer.ts';
import { beamPoint, dopplerColor, type BeamId } from './colliderField.ts';
import { telemetryStream } from '../telemetry.ts';
import { registerDisposer } from '../store.ts';
import { createLarpRenderLoop } from '../renderLoop.ts';
import { createOrbitalAtmosphere } from './orbitalAtmosphere.ts';

const PARTICLES_PER_BEAM = 2000;
const FLASH_MS = 1200;
const FLASH_PEAK = 1.6;

interface Particle {
  t: number;
  speed: number;
}

function seededParticles(beam: BeamId, count: number): Particle[] {
  let seed = beam === 'A' ? 1234567 : 7654321;
  const next = () => {
    seed = (seed * 1103515245 + 12345) & 0x7fffffff;
    return seed / 0x7fffffff;
  };
  return Array.from({ length: count }, () => ({ t: next(), speed: 0.0012 + next() * 0.002 }));
}

export function View6ColliderField({ stream = telemetryStream }: { stream?: typeof telemetryStream }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [events, setEvents] = useState(() => stream.snapshot());
  const [failed, setFailed] = useState(false);

  useEffect(() => stream.subscribe(() => setEvents(stream.snapshot())), [stream]);

  const lastCollision = [...events].reverse().find((e) => e.kind === 'collision');
  const lastBeamA = lastCollision?.kind === 'collision' ? lastCollision.beamA : null;
  const lastBeamB = lastCollision?.kind === 'collision' ? lastCollision.beamB : null;

  useEffect(() => {
    const container = containerRef.current;
    const canvas = canvasRef.current;
    if (!container || !canvas) return;
    const els = { container, canvas };
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
    camera.position.set(0, 0, 190);
    camera.lookAt(0, 0, 0);
    const group = new THREE.Group();
    scene.add(group);
    const atmosphere = createOrbitalAtmosphere(scene, 32000);

    const beams: { id: BeamId; particles: Particle[]; positions: Float32Array; colors: Float32Array; geometry: THREE.BufferGeometry }[] = (
      ['A', 'B'] as BeamId[]
    ).map((id) => {
      const particles = seededParticles(id, PARTICLES_PER_BEAM);
      const positions = new Float32Array(PARTICLES_PER_BEAM * 3);
      const colors = new Float32Array(PARTICLES_PER_BEAM * 3);
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
      geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));
      return { id, particles, positions, colors, geometry };
    });
    const material = new THREE.ShaderMaterial({
      vertexColors: true,
      vertexShader: `varying vec3 vColor;
        void main(){vColor=color;vec4 mv=modelViewMatrix*vec4(position,1.);
        gl_PointSize=clamp(400./max(-mv.z,1.),.8,3.);gl_Position=projectionMatrix*mv;}`,
      fragmentShader: `varying vec3 vColor;
        void main(){float d=length(gl_PointCoord-.5);float a=exp(-d*d*42.);
        gl_FragColor=vec4(vColor*1.5,a*.7);}`,
      transparent:true, blending:THREE.AdditiveBlending, depthWrite:false,
    });
    for (const beam of beams) group.add(new THREE.Points(beam.geometry, material));

    // Interaction point marker: one white-hot point at the origin.
    const pointGeometry = new THREE.BufferGeometry();
    pointGeometry.setAttribute('position', new THREE.BufferAttribute(new Float32Array([0, 0, 0]), 3));
    const pointMaterial = new THREE.PointsMaterial({ size: 1.5, color: '#ffffff', transparent: true, opacity: 0.95 });
    group.add(new THREE.Points(pointGeometry, pointMaterial));

    function updateBeam(beam: (typeof beams)[number]): void {
      beam.particles.forEach((p, i) => {
        if (!isReduced()) p.t += p.speed;
        if (p.t >= 1) p.t -= 1;
        const pos = beamPoint(beam.id, p.t);
        // Fixed braided lanes add depth without inventing collision events.
        const lane = Math.sin(i * 13.37) * 3;
        beam.positions.set([pos.x, pos.y + lane * Math.sin(p.t * Math.PI), pos.z + Math.cos(i * 3.17) * 4], i * 3);
        const c = dopplerColor(beam.id, p.t);
        beam.colors.set(c, i * 3);
      });
      (beam.geometry.getAttribute('position') as THREE.BufferAttribute).needsUpdate = true;
      (beam.geometry.getAttribute('color') as THREE.BufferAttribute).needsUpdate = true;
    }

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
    let collisions = events.filter((e) => e.kind === 'collision').length;
    let flashStart = -1;
    const now = Date.now();
    if (!isReduced() && events.some((e) => e.kind === 'collision' && now - e.at < 60000)) {
      flashStart = performance.now();
    }
    els.container.dataset.collisions = String(collisions);
    const observer = new ResizeObserver(() => { resize(); loop.redraw(); });
    observer.observe(els.container);
    const unsubscribe = stream.subscribe((event) => {
      if (event.kind !== 'collision') return;
      collisions += 1;
      els.container.dataset.collisions = String(collisions);
      flashStart = performance.now();
    });

    function frame(time: number): void {
      if (cleaned) return;
      if (!isReduced()) group.rotation.y += 0.0012;
      if (isReduced()) { composerHandle.bloomPass.strength = BLOOM_STRENGTH; flashStart = -1; }
      if (flashStart >= 0) {
        const elapsed = (time - flashStart) / FLASH_MS;
        if (elapsed >= 1) {
          composerHandle.bloomPass.strength = BLOOM_STRENGTH;
          flashStart = -1;
        } else {
          composerHandle.bloomPass.strength = BLOOM_STRENGTH + (FLASH_PEAK - BLOOM_STRENGTH) * (1 - elapsed);
        }
      }
      atmosphere.update(isReduced() ? 0 : time / 1000, flashStart >= 0 ? Math.max(0, 1 - (time - flashStart) / FLASH_MS) : 0);
      composerHandle.grainPass.uniforms.uTime.value = isReduced() ? 0 : time / 1000;
      for (const beam of beams) updateBeam(beam);
      composerHandle.render();
      frames += 1;
      els.container.dataset.frames = String(frames);

    }

    frame(0);
    const loop = createLarpRenderLoop(frame, stream);

    function cleanup(): void {
      if (cleaned) return;
      cleaned = true;
      loop.dispose();
      atmosphere.dispose();
      observer.disconnect();
      unsubscribe();
      composerHandle.dispose();
      for (const beam of beams) beam.geometry.dispose();
      pointGeometry.dispose();
      material.dispose();
      pointMaterial.dispose();
      renderer.dispose();
      renderer.forceContextLoss();
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
      <div className="larp-view-container" aria-label="COLLIDER FIELD // CONCEPT RESONANCE ACCELERATOR">
        <div className="larp-view-fallback">
          <p>WebGL unavailable — the collider field cannot render here.</p>
        </div>
      </div>
    );
  }

  return (
    <div ref={containerRef} className="larp-view-container" aria-label="COLLIDER FIELD // CONCEPT RESONANCE ACCELERATOR">
      <canvas ref={canvasRef} className="larp-view-canvas" aria-label="Collider field beams" />
      <div className="larp-spectrum-legend">
        {lastBeamA !== null && lastBeamB !== null ? (
          <span>
            Last collision: {lastBeamA} × {lastBeamB}
          </span>
        ) : (
          <span>No collisions yet — run LEXI-COLLIDER in a project.</span>
        )}{' '}
        <span>Beams are a stylized visualization, not measured collider telemetry.</span>
      </div>
    </div>
  );
}
