// LARP Mode Slice 3: View 3 — ATTENTION SPIDER // GRAPH DRIFT ENGINE.
// A WebGL 3D force-directed hypergraph through the shared LARP
// post-processing pipeline. Nodes are the latest memory-event nodes
// (fragments); nodes cited by the latest Ask answer render white-hot.
// Edges are recorded relationships (bright) plus deterministic layout
// links (dim). NO numeric weight is displayed: relationship records
// carry no weight and no attention weights exist in this app, so
// presenting layout geometry as measured attention would be fabrication.
// Tags project per frame through the camera matrix (imperative refs).
// data-frames / data-nodes / data-edges are the e2e-visible counters.
// Empty stream → explicit panel, never a fake graph. Disposal mirrors
// Views 1–2 (composer, geometries, materials, renderer, forced context
// loss, store disposer).

import { useEffect, useRef, useState } from 'react';
import * as THREE from 'three';
import { createLarpComposer, clampPixelRatio } from '../postprocessing/LarpComposer.ts';
import { buildGraph, layoutGraph, latestCitedIds } from './forceLayout.ts';
import { latestMemoryNodes } from '../MemoryMatrix.tsx';
import { telemetryStream } from '../telemetry.ts';
import { registerDisposer } from '../store.ts';
import { createLarpRenderLoop } from '../renderLoop.ts';
import {bindSceneNavigation} from './sceneNavigation.ts';
import {generateParticles} from './particleLayout.ts';
import {BOKEH_VERTEX,BOKEH_FRAGMENT} from './vaultShaders.ts';
import {createOrbitalAtmosphere} from './orbitalAtmosphere.ts';

const NODE_COLORS: Record<string, string> = {
  memory: '#ff8a3d',
  note: '#00f0ff',
  artifact: '#ff1e7a',
};
const CITED_COLOR = '#ffffff';
const RELATIONSHIP_COLOR = '#c8f4ff';
const LAYOUT_COLOR = '#3a3a44';
const TAG_LIMIT = 24;

export function View3AttentionSpider({ stream = telemetryStream }: { stream?: typeof telemetryStream }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const tagLayerRef = useRef<HTMLDivElement>(null);
  const [failed, setFailed] = useState(false);
  const [empty, setEmpty] = useState(false);

  useEffect(() => {
    const container = containerRef.current;
    const canvas = canvasRef.current;
    const tagLayer = tagLayerRef.current;
    if (!container || !canvas || !tagLayer) return;
    const nodes = latestMemoryNodes(stream.snapshot());
    if (nodes.length === 0) {
      setEmpty(true);
      return;
    }
    const els = { container, canvas, tagLayer };
    const cited = latestCitedIds(stream.snapshot());
    const graph = buildGraph(nodes, cited);
    const positions = layoutGraph(graph);
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
    const atmosphere=createOrbitalAtmosphere(scene,8000);
    const cloud=generateParticles(61347,[...positions.values()],Math.min(22000,graph.nodes.length*1800));
    const cloudGeometry=new THREE.BufferGeometry();
    cloudGeometry.setAttribute('position',new THREE.BufferAttribute(cloud.positions,3));
    cloudGeometry.setAttribute('aScale',new THREE.BufferAttribute(cloud.scales,1));
    cloudGeometry.setAttribute('aSeed',new THREE.BufferAttribute(cloud.seeds,1));
    const cloudMaterial=new THREE.ShaderMaterial({vertexShader:BOKEH_VERTEX,fragmentShader:BOKEH_FRAGMENT,uniforms:{uFocalDepth:{value:190},uTime:{value:0}},transparent:true,depthWrite:false,blending:THREE.AdditiveBlending});
    group.add(new THREE.Points(cloudGeometry,cloudMaterial));

    const nodePositions = new Float32Array(graph.nodes.length * 3);
    const nodeColors = new Float32Array(graph.nodes.length * 3);
    const color = new THREE.Color();
    graph.nodes.forEach((node, i) => {
      const p = positions.get(node.id)!;
      nodePositions.set([p.x, p.y, p.z], i * 3);
      color.set(node.cited ? CITED_COLOR : (NODE_COLORS[node.kind] ?? '#e8e8ec'));
      nodeColors.set([color.r, color.g, color.b], i * 3);
    });
    const nodeGeometry = new THREE.BufferGeometry();
    nodeGeometry.setAttribute('position', new THREE.BufferAttribute(nodePositions, 3));
    nodeGeometry.setAttribute('color', new THREE.BufferAttribute(nodeColors, 3));
    const nodeMaterial = new THREE.ShaderMaterial({vertexColors:true,transparent:true,depthWrite:false,blending:THREE.AdditiveBlending,
      vertexShader:`varying vec3 vColor;void main(){vColor=color;vec4 mv=modelViewMatrix*vec4(position,1.);gl_PointSize=clamp(1500./max(-mv.z,1.),5.,14.);gl_Position=projectionMatrix*mv;}`,
      fragmentShader:`varying vec3 vColor;void main(){float d=length(gl_PointCoord-.5);float a=exp(-d*d*35.);gl_FragColor=vec4(vColor*2.,a*.9);}`});
    group.add(new THREE.Points(nodeGeometry, nodeMaterial));

    const edgePositions = new Float32Array(graph.edges.length * 24 * 6);
    const edgeColors = new Float32Array(graph.edges.length * 24 * 6);
    graph.edges.forEach((edge, i) => {
      const a = positions.get(edge.source)!;
      const b = positions.get(edge.target)!;
      const start=new THREE.Vector3(a.x,a.y,a.z),end=new THREE.Vector3(b.x,b.y,b.z);
      const bend=Math.min(30,start.distanceTo(end)*.25)*(i%2?1:-1);
      const c1=start.clone().lerp(end,.33).add(new THREE.Vector3(0,bend,12));
      const c2=start.clone().lerp(end,.67).add(new THREE.Vector3(0,bend,-12));
      const curve=new THREE.CubicBezierCurve3(start,c1,c2,end);
      color.set(edge.kind === 'relationship' ? RELATIONSHIP_COLOR : '#6b4e3b');
      for(let step=0;step<24;step++){
        const p=curve.getPoint(step/24),q=curve.getPoint((step+1)/24),offset=(i*24+step)*6;
        edgePositions.set([p.x,p.y,p.z,q.x,q.y,q.z],offset);
        edgeColors.set([color.r,color.g,color.b,color.r,color.g,color.b],offset);
      }
    });
    const edgeGeometry = new THREE.BufferGeometry();
    edgeGeometry.setAttribute('position', new THREE.BufferAttribute(edgePositions, 3));
    edgeGeometry.setAttribute('color', new THREE.BufferAttribute(edgeColors, 3));
    const edgeMaterial = new THREE.LineBasicMaterial({ vertexColors: true, transparent: true, opacity: 0.8 });
    group.add(new THREE.LineSegments(edgeGeometry, edgeMaterial));

    const tagDivs = graph.nodes.slice(0, TAG_LIMIT).map((node) => {
      const div = document.createElement('div');
      div.className = 'larp-tag';
      div.textContent = node.label;
      els.tagLayer.appendChild(div);
      return div;
    });
    const tagged = graph.nodes.slice(0, TAG_LIMIT);
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

    let navigation:ReturnType<typeof bindSceneNavigation>|null=null;
    let frames = 0;
    let cleaned = false;
    const observer = new ResizeObserver(() => { resize(); loop.redraw(); });
    observer.observe(els.container);
    els.container.dataset.nodes = String(graph.nodes.length);
    els.container.dataset.edges = String(graph.edges.length);

    function frame(time: number): void {
      if (cleaned) return;
      if (!isReduced()&&!navigation?.touched()) group.rotation.y += 0.0012;
      composerHandle.grainPass.uniforms.uTime.value = isReduced() ? 0 : time / 1000;
      atmosphere.update(isReduced()?0:time/1000);
      cloudMaterial.uniforms.uFocalDepth.value=navigation?.distance()??190;
      cloudMaterial.uniforms.uTime.value=isReduced()?0:time/1000;
      composerHandle.render();
      frames += 1;
      els.container.dataset.frames = String(frames);
      scene.updateMatrixWorld(true);
      for (let i = 0; i < tagged.length; i++) {
        const p = positions.get(tagged[i].id)!;
        projected.set(p.x, p.y, p.z).applyMatrix4(group.matrixWorld).project(camera);
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
    navigation=bindSceneNavigation(camera,els.canvas,loop.redraw);

    function cleanup(): void {
      if (cleaned) return;
      cleaned = true;
      navigation?.dispose();
      loop.dispose();
      observer.disconnect();
      composerHandle.dispose();
      atmosphere.dispose();cloudGeometry.dispose();cloudMaterial.dispose();
      nodeGeometry.dispose();
      edgeGeometry.dispose();
      nodeMaterial.dispose();
      edgeMaterial.dispose();
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
      <div className="larp-view-container" aria-label="ATTENTION SPIDER // GRAPH DRIFT ENGINE">
        <div className="larp-view-fallback">
          <p>WebGL unavailable — the attention spider cannot render here.</p>
        </div>
      </div>
    );
  }

  if (empty) {
    return (
      <div className="larp-view-container" aria-label="ATTENTION SPIDER // GRAPH DRIFT ENGINE">
        <div className="larp-view-fallback">
          <p>No memory nodes yet — open Memory to load records.</p>
        </div>
      </div>
    );
  }

  return (
    <div ref={containerRef} className="larp-view-container" aria-label="ATTENTION SPIDER // GRAPH DRIFT ENGINE">
      <canvas ref={canvasRef} className="larp-view-canvas" data-interactive="true" tabIndex={0} aria-description="Drag to rotate, scroll to zoom, right-drag or arrow keys to pan. Home resets the view." aria-label="Attention spider graph" />
      <div ref={tagLayerRef} className="larp-tag-layer" aria-hidden="true" />
      <div className="larp-spectrum-legend"><span>Bright links: recorded relationships.</span><span>Faint links: layout only.</span><span>Dust: ornamental, not additional records.</span></div>
    </div>
  );
}
