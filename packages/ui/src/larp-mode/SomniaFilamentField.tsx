import { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { createLarpComposer } from './postprocessing/LarpComposer';
import { createLarpRenderLoop } from './renderLoop';
import { bindSceneNavigation } from './views/sceneNavigation';
import { mulberry32 } from './views/particleLayout';

function filamentShell(radius: number, count: number, seed: number, opacity: number) {
  const random = mulberry32(seed);
  const positions: number[] = [];
  const colors: number[] = [];
  const palette = ['#79d88a', '#7dbf73', '#e5f1d1', '#b0d6a4', '#d87564'].map(c => new THREE.Color(c));
  const sample = () => {
    const y = 2 * random() - 1;
    const angle = random() * Math.PI * 2;
    const horizontal = Math.sqrt(1 - y * y);
    return new THREE.Vector3(Math.cos(angle) * horizontal, y, Math.sin(angle) * horizontal);
  };
  for (let i = 0; i < count; i++) {
    const from = sample();
    const to = sample();
    const tint = palette[i % palette.length].clone().multiplyScalar(.30 + random() * .70);
    const bend = .52 + random() * .48;
    const steps = 18;
    const at = (t: number) => from.clone().lerp(to,t).normalize()
      .multiplyScalar(radius * (1 - Math.sin(Math.PI * t) * (1 - bend)));
    let previous = at(0);
    for (let j = 1; j <= steps; j++) {
      const current = at(j / steps);
      positions.push(...previous.toArray(), ...current.toArray());
      const strength = .55 + .45 * Math.sin(Math.PI * j / steps);
      for (let k = 0; k < 2; k++) colors.push(tint.r * strength,tint.g * strength,tint.b * strength);
      previous = current;
    }
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position',new THREE.Float32BufferAttribute(positions,3));
  geometry.setAttribute('color',new THREE.Float32BufferAttribute(colors,3));
  const material = new THREE.LineBasicMaterial({
    vertexColors:true, transparent:true, opacity, depthWrite:false, blending:THREE.AdditiveBlending,
  });
  return new THREE.LineSegments(geometry,material);
}

function buildField() {
  const scene = new THREE.Scene();
  scene.background = new THREE.Color('#020604');
  scene.fog = new THREE.FogExp2('#020604', .0022);
  const main = filamentShell(104, 570, 9184, .30);
  const middle = filamentShell(83, 160, 8211, .25);
  const satellites = [
    filamentShell(43, 190, 4931, .29),
    filamentShell(35, 130, 1187, .31),
    filamentShell(49, 140, 7391, .23),
  ];
  satellites[0].position.set(119, 43, -31);
  satellites[1].position.set(63, -77, 59);
  satellites[2].position.set(-106, 51, -46);
  main.position.x = middle.position.x = -23;
  scene.add(main,middle,...satellites);

  const random = mulberry32(9311);
  const points: number[] = [];
  for(let i=0;i<1050;i++)points.push((random()-.5)*600,(random()-.5)*380,(random()-.5)*440);
  const pointGeometry=new THREE.BufferGeometry().setAttribute('position',new THREE.Float32BufferAttribute(points,3));
  const dust=new THREE.Points(pointGeometry,new THREE.PointsMaterial({
    color:'#92b89c',size:.55,opacity:.32,transparent:true,depthWrite:false,
  }));
  scene.add(dust);

  const orbit=new THREE.Group();
  for (const [r,tilt] of [[107,.35],[113,1.1],[119,1.8]]) {
    const points=Array.from({length:192},(_,i)=>{
      const angle=i*Math.PI/96;
      return new THREE.Vector3(Math.cos(angle)*r,0,Math.sin(angle)*r);
    });
    const line=new THREE.LineLoop(new THREE.BufferGeometry().setFromPoints(points),
      new THREE.LineBasicMaterial({color:'#8fd6a1',transparent:true,opacity:.28,depthWrite:false}));
    line.rotation.x=tilt;
    orbit.add(line);
  }
  orbit.position.x=-23;
  scene.add(orbit);
  return {scene, main, middle, satellites, orbit};
}

export function SomniaFilamentField({ active, paused, onUnavailable }: {
  active: boolean; paused: boolean; onUnavailable: () => void;
}) {
  const canvasRef=useRef<HTMLCanvasElement>(null);
  const settings=useRef({active,paused});
  const redraw=useRef<(() => void)|null>(null);
  useEffect(()=>{
    settings.current={active,paused};
    redraw.current?.();
  },[active,paused]);

  useEffect(()=>{
    const canvas=canvasRef.current;
    if(!canvas)return;
    let renderer:THREE.WebGLRenderer;
    try { renderer=new THREE.WebGLRenderer({canvas,antialias:true,powerPreference:'low-power'}); }
    catch { onUnavailable(); return; }
    const field=buildField();
    const camera=new THREE.PerspectiveCamera(44,1,1,1400);
    camera.position.set(28,24,310);
    camera.lookAt(0,0,0);
    const disposeField=()=>field.scene.traverse(object=>{
      const mesh=object as THREE.Mesh;
      mesh.geometry?.dispose();
      if(mesh.material&&!Array.isArray(mesh.material))mesh.material.dispose();
    });
    let composer: ReturnType<typeof createLarpComposer>;
    try { composer=createLarpComposer(renderer,field.scene,camera,1,1); }
    catch {disposeField();renderer.dispose();onUnavailable();return;}
    composer.bloomPass.threshold=.21;
    composer.bloomPass.strength=.59;
    const reduced=window.matchMedia('(prefers-reduced-motion: reduce)');
    const render=()=>composer.render();
    const resize=()=>{
      const width=Math.max(canvas.clientWidth,1),height=Math.max(canvas.clientHeight,1);
      renderer.setSize(width,height,false);
      composer.setSize(width,height);
      camera.aspect=width/height;
      camera.updateProjectionMatrix();
      render();
    };
    const navigation=bindSceneNavigation(camera,canvas,render,{min:95,max:620,pan:170});
    let elapsed=0;
    let previous=performance.now();
    const loop=createLarpRenderLoop(now=>{
      const moving=!settings.current.paused&&!reduced.matches&&document.documentElement.dataset.reducedMotion!=='true';
      if(moving)elapsed+=Math.min(50,now-previous)/1000;
      previous=now;
      const pulse=settings.current.active ? .5+.5*Math.sin(elapsed*3.6) : 0;
      field.main.rotation.set(.13,elapsed*.068,.20);
      field.middle.rotation.set(elapsed*-.067,elapsed*.036,-.2);
      field.orbit.rotation.y=elapsed*.084;
      for(const [index,satellite] of field.satellites.entries()){
        satellite.rotation.x=elapsed*(.10+index*.025);
        satellite.rotation.y=elapsed*-.057;
      }
      field.main.material.opacity=.30+pulse*.12;
      composer.bloomPass.strength=.59+pulse*.35;
      composer.grainPass.uniforms.uTime.value=elapsed;
      render();
    });
    redraw.current=render;
    const observer=new ResizeObserver(resize);
    observer.observe(canvas);
    resize();
    const lost=(event:Event)=>{event.preventDefault();onUnavailable();};
    canvas.addEventListener('webglcontextlost',lost);
    return ()=>{
      redraw.current=null;
      canvas.removeEventListener('webglcontextlost',lost);
      observer.disconnect();
      loop.dispose();
      navigation.dispose();
      composer.dispose();
      disposeField();
      renderer.dispose();
    };
  },[onUnavailable]);

  return <canvas ref={canvasRef} className="somnia-filament-canvas" data-interactive="true" tabIndex={0}
    aria-label="SOMNIA 3D filament field. Drag to orbit, scroll to zoom, or press Home to reset." />;
}
