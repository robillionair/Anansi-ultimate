import {useEffect,useRef,useState} from 'react';
import * as THREE from 'three';
import {createLarpComposer,clampPixelRatio} from './postprocessing/LarpComposer.ts';
import {createLarpRenderLoop} from './renderLoop.ts';
import {registerDisposer} from './store.ts';
import type {EntityPhase} from './entityChatRun.ts';
export const ENTITY_PORTRAIT=new URL('./assets/anansi-entity.png',import.meta.url).href;
export function entityActivity(phase:EntityPhase){return phase==='streaming'?.85:phase==='waiting'?.45:phase==='cancelling'?.25:phase==='error'||phase==='offline'?.05:.12;}
export function EntityAvatar({phase,characters}:{phase:EntityPhase;characters:number}){
 const container=useRef<HTMLDivElement>(null),canvas=useRef<HTMLCanvasElement>(null);
 const activity=useRef({phase,characters,changedAt:performance.now()}),redraw=useRef(()=>{});
 const [failed,setFailed]=useState(false);
 useEffect(()=>{activity.current={phase,characters,changedAt:performance.now()};redraw.current();},[phase,characters]);
 useEffect(()=>{
  if(failed||!canvas.current||!container.current)return;
  const el=canvas.current,host=container.current;let disposed=false;
  let renderer:THREE.WebGLRenderer;
  try{renderer=new THREE.WebGLRenderer({canvas:el,antialias:true});}catch{setFailed(true);return;}
  renderer.setPixelRatio(clampPixelRatio(window.devicePixelRatio||1));
  const scene=new THREE.Scene();scene.background=new THREE.Color('#030609');
  const camera=new THREE.PerspectiveCamera(42,1,.1,1000);camera.position.set(0,0,205);
  const preference=window.matchMedia('(prefers-reduced-motion: reduce)');
  const reduced=()=>preference.matches||document.documentElement.dataset.reducedMotion==='true';
  const texture=new THREE.TextureLoader().load(ENTITY_PORTRAIT,()=>{if(disposed){texture.dispose();return;}host.dataset.portraitReady='true';redraw.current();},undefined,()=>{if(!disposed)setFailed(true);});
  texture.colorSpace=THREE.SRGBColorSpace;
  const geometry=new THREE.PlaneGeometry(116,174,100,150);
  const material=new THREE.ShaderMaterial({uniforms:{uPortrait:{value:texture},uTime:{value:0},uEnergy:{value:.12},uSpeaking:{value:0}},
   vertexShader:`varying vec2 vUv;uniform float uTime,uEnergy,uSpeaking;
    void main(){vUv=uv;vec3 p=position;vec2 q=(uv-.5)*2.;
    p.z+=sqrt(max(0.,1.-dot(q,q)))*17.;
    p.z+=exp(-q.x*q.x*120.-(q.y+.16)*(q.y+.16)*45.)*9.;
    float jaw=1.-smoothstep(.24,.44,uv.y);
    p.y-=jaw*uSpeaking*(.6+.5*sin(uTime*9.))*1.6;
    p.z+=sin(q.y*8.+uTime*1.7)*uEnergy*.35;
    gl_Position=projectionMatrix*modelViewMatrix*vec4(p,1.);}`,
   fragmentShader:`varying vec2 vUv;uniform sampler2D uPortrait;uniform float uTime,uEnergy;
    void main(){vec3 c=texture2D(uPortrait,vUv).rgb;float light=max(c.r,max(c.g,c.b));if(light<.025)discard;
    float scan=.96+.04*sin(gl_FragCoord.y*1.8+uTime*.6);
    gl_FragColor=vec4(c*(.7+uEnergy*.28)*scan,smoothstep(.02,.15,light));}`,
   transparent:true,depthWrite:false,blending:THREE.AdditiveBlending});
  const face=new THREE.Mesh(geometry,material);scene.add(face);
  // Calibrated orbital scaffolding is decorative, not measured neural connectivity.
  const orbitGroup=new THREE.Group();scene.add(orbitGroup);
  const orbitMaterial=new THREE.MeshBasicMaterial({color:'#75cbd5',transparent:true,opacity:.16,depthWrite:false});
  const ringGeometries:THREE.TorusGeometry[]=[];
  for(let n=0;n<3;n++){const g=new THREE.TorusGeometry(73+n*5,.035,4,180);ringGeometries.push(g);const ring=new THREE.Mesh(g,orbitMaterial);ring.rotation.set(n*.7,n*.42,0);orbitGroup.add(ring);}
  const starPositions=new Float32Array(3000*3);let seed=64281;const rand=()=>{seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed/4294967296;};
  for(let n=0;n<3000;n++){const a=rand()*Math.PI*2,r=70+rand()*22;starPositions.set([Math.cos(a)*r,Math.sin(a)*r*.9,-15+rand()*15],n*3);}
  const starGeometry=new THREE.BufferGeometry();starGeometry.setAttribute('position',new THREE.BufferAttribute(starPositions,3));
  const starMaterial=new THREE.PointsMaterial({color:'#6ccfd6',size:.35,transparent:true,opacity:.5,depthWrite:false,blending:THREE.AdditiveBlending});scene.add(new THREE.Points(starGeometry,starMaterial));
  const composer=createLarpComposer(renderer,scene,camera,1,1);composer.bloomPass.strength=.5;composer.bloomPass.threshold=.55;composer.bloomPass.radius=.6;
  const gaze={x:0,y:0};let frames=0;
  function paint(time:number){if(disposed)return;const t=reduced()?0:time/1000;
   const state=activity.current;const energy=entityActivity(state.phase);
   material.uniforms.uTime.value=t;material.uniforms.uEnergy.value=energy;
   material.uniforms.uSpeaking.value=reduced()?0:state.phase==='streaming'?Math.exp(-Math.max(0,performance.now()-state.changedAt)/1200):0;
   face.rotation.y=reduced()?0:Math.sin(t*.23)*.025+gaze.x*.075;face.rotation.x=reduced()?0:Math.sin(t*.18)*.012-gaze.y*.035;
   face.position.y=reduced()?0:Math.sin(t*.65)*.7;orbitGroup.rotation.z=t*.016;orbitMaterial.opacity=.12+energy*.14;
   composer.grainPass.uniforms.uTime.value=t;composer.render();host.dataset.frames=String(++frames);
  }
  const loop=createLarpRenderLoop(paint);redraw.current=loop.redraw;
  function resize(){const w=host.clientWidth||800,h=host.clientHeight||700;renderer.setSize(w,h,false);camera.aspect=w/h;camera.position.z=Math.max(205,104/(2*Math.tan(THREE.MathUtils.degToRad(21))*camera.aspect));camera.updateProjectionMatrix();composer.setSize(w,h);loop.redraw();}
  resize();paint(0);const observer=new ResizeObserver(resize);observer.observe(host);
  const move=(e:MouseEvent)=>{const box=host.getBoundingClientRect();if(e.clientX<box.left||e.clientX>box.right||e.clientY<box.top||e.clientY>box.bottom)return;gaze.x=(e.clientX-box.left)/box.width-.5;gaze.y=(e.clientY-box.top)/box.height-.5;loop.redraw();};
  window.addEventListener('mousemove',move,{passive:true});
  function cleanup(){if(disposed)return;disposed=true;redraw.current=()=>{};loop.dispose();observer.disconnect();window.removeEventListener('mousemove',move);texture.dispose();geometry.dispose();material.dispose();ringGeometries.forEach(g=>g.dispose());orbitMaterial.dispose();starGeometry.dispose();starMaterial.dispose();composer.dispose();renderer.dispose();renderer.forceContextLoss();}
  const unregister=registerDisposer(cleanup);return()=>{cleanup();unregister();};
 },[failed]);
 return <div ref={container} className="larp-view-container entity-avatar" aria-label="ANANSI holographic entity" data-phase={phase}>
  {failed?<img className="entity-portrait-fallback" src={ENTITY_PORTRAIT} alt="ANANSI holographic face — static graphics fallback"/>:<canvas ref={canvas} className="larp-view-canvas" aria-label="Animated ANANSI holographic face"/>}
  <div className="entity-state"><i/>{phase==='waiting'?'AWAITING MODEL':phase==='streaming'?'GENERATING TEXT':phase.toUpperCase()}<small>ACTIVITY VISUALIZATION / NOT INTERNAL REASONING</small></div>
 </div>;
}
