// LARP Mode Slice 2: multi-pass WebGL post-processing pipeline.
// Render -> UnrealBloom (mipmap internally) -> chromatic aberration ->
// film grain -> Output. DPR is clamped to 2 (low-power GPU profile).
// Pass assembly goes through an injected factory so unit tests can
// assert the order without GL; the real factory constructs three.js
// passes. Disposal releases composer targets, every pass that owns
// resources, and (via the view) geometries, materials, and the renderer
// itself. Every dispose helper tolerates being called twice, because
// the store's disposeAll() on OFF and React's unmount cleanup both run
// it.

import * as THREE from 'three';
import { EffectComposer } from 'three/addons/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/addons/postprocessing/RenderPass.js';
import { ShaderPass } from 'three/addons/postprocessing/ShaderPass.js';
import { UnrealBloomPass } from 'three/addons/postprocessing/UnrealBloomPass.js';
import { OutputPass } from 'three/addons/postprocessing/OutputPass.js';
import { SMAAPass } from 'three/addons/postprocessing/SMAAPass.js';
import { chromaticAberrationShader, filmGrainShader } from './shaders.ts';

export const MAX_PIXEL_RATIO = 2;
export const BLOOM_STRENGTH = 0.85;
export const BLOOM_RADIUS = 0.65;
export const BLOOM_THRESHOLD = 0.32;

export function clampPixelRatio(dpr: number): number {
  if (!Number.isFinite(dpr) || dpr <= 0) return 1;
  return Math.min(dpr, MAX_PIXEL_RATIO);
}

/** Keep the same scene and optics while bounding work on a slow renderer. */
export function nextRenderScale(frameMs:number,current:number):number {
  const scale=Number.isFinite(current)?Math.max(.5,Math.min(1,current)):1;
  return Number.isFinite(frameMs)&&frameMs>50?Math.max(.5,scale*.75):scale;
}

export type LarpPassKind = 'render' | 'bloom' | 'smaa' | 'aberration' | 'grain' | 'output';

export const LARP_PASS_ORDER: LarpPassKind[] = ['render', 'bloom', 'smaa', 'aberration', 'grain', 'output'];

/** Assembly with an injected pass factory — the unit-testable shape. */
export function buildLarpPasses<T>(create: (kind: LarpPassKind) => T): T[] {
  return LARP_PASS_ORDER.map(create);
}

export interface LarpComposerHandle {
  composer: EffectComposer;
  grainPass: ShaderPass;
  /** Exposed so a view can animate a discharge flash (Slice 4). */
  bloomPass: UnrealBloomPass;
  render: () => void;
  setSize: (width: number, height: number) => void;
  dispose: () => void;
}

export function createLarpComposer(
  renderer: THREE.WebGLRenderer,
  scene: THREE.Scene,
  camera: THREE.Camera,
  width: number,
  height: number,
): LarpComposerHandle {
  const composer = new EffectComposer(renderer);
  composer.setPixelRatio(clampPixelRatio(window.devicePixelRatio || 1));
  composer.setSize(width, height);
  composer.addPass(new RenderPass(scene, camera));
  const bloom = new UnrealBloomPass(
    new THREE.Vector2(width, height),
    BLOOM_STRENGTH,
    BLOOM_RADIUS,
    BLOOM_THRESHOLD,
  );
  composer.addPass(bloom);
  composer.addPass(new SMAAPass());
  composer.addPass(new ShaderPass(chromaticAberrationShader));
  const grainPass = new ShaderPass(filmGrainShader);
  composer.addPass(grainPass);
  composer.addPass(new OutputPass());
  const passes = composer.passes.slice();
  let disposed = false;
  let renderScale=1;
  renderer.domElement.dataset.renderScale='1';
  return {
    composer,
    grainPass,
    bloomPass: bloom,
    render: () => {
      if(disposed)return;
      const began=performance.now();composer.render();
      const next=nextRenderScale(performance.now()-began,renderScale);
      if(next!==renderScale){renderScale=next;composer.setPixelRatio(clampPixelRatio(window.devicePixelRatio||1)*renderScale);renderer.domElement.dataset.renderScale=String(renderScale);}
    },
    setSize: (w: number, h: number) => composer.setSize(w, h),
    dispose: () => {
      if (disposed) return;
      disposed = true;
      for (const pass of passes) {
        const maybe = pass as unknown as {
          dispose?: () => void;
          material?: { dispose: () => void };
        };
        if (typeof maybe.dispose === 'function') maybe.dispose();
        if (maybe.material && typeof maybe.material.dispose === 'function') {
          maybe.material.dispose();
        }
      }
      composer.dispose();
    },
  };
}
