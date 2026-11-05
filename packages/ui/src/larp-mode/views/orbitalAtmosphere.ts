import * as THREE from 'three';
import { mulberry32 } from './particleLayout.ts';

/** Ornamental orbital geometry, never represented as observed memory records. */
export function createOrbitalAtmosphere(scene: THREE.Scene, count = 14000) {
  const random = mulberry32(41782);
  const positions = new Float32Array(count * 3);
  const seeds = new Float32Array(count);
  for (let i = 0; i < count; i++) {
    positions.set([random() * Math.PI * 2, random(), random() * Math.PI * 2], i * 3);
    seeds[i] = random();
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
  geometry.setAttribute('aSeed', new THREE.BufferAttribute(seeds, 1));
  const material = new THREE.ShaderMaterial({
    uniforms: { uTime: { value: 0 }, uEnergy: { value: 0 }, uDrift: { value: 0 } },
    vertexShader: `
      attribute float aSeed;
      uniform float uTime, uEnergy, uDrift;
      varying float vSeed, vLight, vDepth;
      void main(){
        float a=position.x + uTime * (0.055 + aSeed * 0.045);
        float r=42.0 + position.y * 33.0;
        float braid=position.z + a*3.0 - uTime*.18;
        float tube=(2.2+position.y*6.0)*(1.0+uEnergy*.25);
        vec3 p=vec3(cos(a)*(r+cos(braid)*tube),sin(braid)*tube,sin(a)*(r+cos(braid)*tube));
        p.y += sin(a*5.0+uTime*.3)*uDrift*9.0;
        p.xy=mat2(.94,-.342,.342,.94)*p.xy;
        vec4 mv=modelViewMatrix*vec4(p,1.);
        vDepth=-mv.z;vSeed=aSeed;
        vLight=.22+.78*pow(.5+.5*sin(a*2.0+braid),3.);
        gl_PointSize=clamp((1.0+aSeed*2.4)*(200./max(vDepth,1.)),.6,5.);
        gl_Position=projectionMatrix*mv;
      }`,
    fragmentShader: `
      uniform float uEnergy;
      varying float vSeed,vLight,vDepth;
      void main(){
        float d=length(gl_PointCoord-.5);
        float core=exp(-d*d*55.);
        float halo=exp(-d*d*12.)*.22;
        vec3 c=mix(vec3(.62,.12,.5),vec3(1.,.48,.16),smoothstep(.1,.85,vSeed));
        c=mix(c,vec3(.65,.9,1.),pow(vSeed,12.));
        c=mix(c,vec3(1.,.82,.68),.42);
        gl_FragColor=vec4(c*(core*2.3+halo)*(1.+uEnergy*1.5), (core+halo)*vLight*.85);
      }`,
    transparent: true, depthWrite: false, blending: THREE.AdditiveBlending,
  });
  const points = new THREE.Points(geometry, material);
  points.rotation.x = .65;
  scene.add(points);
  const rings = new THREE.Group();
  const ringMaterial = new THREE.LineBasicMaterial({ color: '#a77a55', transparent: true, opacity: .12, depthWrite: false });
  const ringGeometries: THREE.BufferGeometry[] = [];
  for (let lane = 0; lane < 7; lane++) {
    const vertices: THREE.Vector3[] = [];
    for (let j = 0; j <= 200; j++) {
      const a = j / 200 * Math.PI * 2;
      const r = 43 + lane * 5;
      vertices.push(new THREE.Vector3(Math.cos(a) * r, Math.sin(a * 3 + lane) * (2 + lane), Math.sin(a) * r));
    }
    const g = new THREE.BufferGeometry().setFromPoints(vertices);
    ringGeometries.push(g); rings.add(new THREE.Line(g, ringMaterial));
  }
  rings.rotation.copy(points.rotation); scene.add(rings);
  return {
    update(time: number, energy = 0, drift = 0) {
      material.uniforms.uTime.value = time;
      material.uniforms.uEnergy.value = energy;
      material.uniforms.uDrift.value = drift;
      rings.rotation.y = time * .018;
    },
    dispose() {
      scene.remove(points, rings); geometry.dispose(); material.dispose();
      ringGeometries.forEach(g => g.dispose()); ringMaterial.dispose();
    },
  };
}
