import * as THREE from 'three';
import { createLarpComposer, clampPixelRatio } from './LarpComposer.ts';

/** Crisp canvas typography/curves enter the same optical pipeline as the particle scenes. */
export function createCanvasFilmSurface(output: HTMLCanvasElement, optics: {textSafe?:boolean} = {}) {
  const source = document.createElement('canvas');
  const ctx = source.getContext('2d');
  if (!ctx) return null;
  let renderer: THREE.WebGLRenderer;
  try { renderer = new THREE.WebGLRenderer({canvas:output,antialias:true}); }
  catch {
    // A failed WebGL acquisition does not claim a GPU presentation.
    const direct = output.getContext('2d');
    return direct ? {mode:'2d' as const,ctx:direct, resize(w:number,h:number,dpr:number){output.width=Math.round(w*dpr);output.height=Math.round(h*dpr);}, present(){},dispose(){}} : null;
  }
  renderer.setPixelRatio(clampPixelRatio(window.devicePixelRatio || 1));
  const scene=new THREE.Scene();
  const camera=new THREE.OrthographicCamera(-1,1,1,-1,0,2);
  camera.position.z=1;
  const texture=new THREE.CanvasTexture(source);
  texture.colorSpace=THREE.SRGBColorSpace;
  texture.minFilter=THREE.LinearFilter;
  const geometry=new THREE.PlaneGeometry(2,2);
  const material=new THREE.MeshBasicMaterial({map:texture,depthTest:false,depthWrite:false});
  scene.add(new THREE.Mesh(geometry,material));
  const film=createLarpComposer(renderer,scene,camera,1,1);
  // Instrument typography needs a lower exposure than particle filaments.
  if(optics.textSafe){film.bloomPass.strength=.35;film.bloomPass.threshold=.65;film.bloomPass.radius=.4;}
  let width=0,height=0,disposed=false;
  return {
    mode:'webgl' as const,
    ctx,
    resize(w:number,h:number,dpr:number){
      const pw=Math.round(w*dpr),ph=Math.round(h*dpr);
      if(width===pw&&height===ph)return;
      width=pw;height=ph;source.width=pw;source.height=ph;
      renderer.setSize(w,h,false);film.setSize(w,h);
    },
    present(){if(disposed)return;texture.needsUpdate=true;film.grainPass.uniforms.uTime.value=performance.now()/1000;film.render();},
    dispose(){if(disposed)return;disposed=true;film.dispose();texture.dispose();geometry.dispose();material.dispose();renderer.dispose();renderer.forceContextLoss();source.width=source.height=1;},
  };
}
