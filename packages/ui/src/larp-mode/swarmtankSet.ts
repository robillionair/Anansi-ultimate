import * as THREE from 'three';
import { mulberry32 } from './views/particleLayout.ts';

export function buildSwarmtankSet(scene: THREE.Scene) {
  scene.background = new THREE.Color('#02060c');
  scene.fog = new THREE.FogExp2('#02060c', 0.00043);
  scene.add(new THREE.HemisphereLight('#a5ceef', '#222641', 1.6));
  const key = new THREE.DirectionalLight('#82d5ff', 2.8);
  key.position.set(-350, 440, 160);
  const rim = new THREE.DirectionalLight('#ffc284', 1.8);
  rim.position.set(250, 180, -390);
  scene.add(key, rim);

  const deck = new THREE.MeshStandardMaterial({ color: '#121c29', metalness: 0.65, roughness: 0.45 });
  const titanium = new THREE.MeshStandardMaterial({ color: '#405365', metalness: 0.72, roughness: 0.33 });
  const cyan = new THREE.MeshBasicMaterial({ color: new THREE.Color('#52b7d6').multiplyScalar(1.45) });
  const amber = new THREE.MeshBasicMaterial({ color: new THREE.Color('#e7a85f').multiplyScalar(1.25) });
  const floor = new THREE.Mesh(
    new THREE.PlaneGeometry(2300, 2300),
    new THREE.ShaderMaterial({
      vertexShader: `
        varying vec2 vPosition;
        void main() {
          vPosition = position.xy;
          gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.);
        }`,
      fragmentShader: `
        varying vec2 vPosition;
        void main() {
          float radius = length(vPosition);
          float glow = exp(-radius * .0028);
          vec2 grid = abs(fract(vPosition / 38. - .5) - .5) / max(fwidth(vPosition / 38.), vec2(.001));
          float line = 1. - min(min(grid.x, grid.y), 1.);
          gl_FragColor = vec4(vec3(.0006,.0018,.0034) + vec3(.012,.040,.062) * glow * (.55 + line * .55), 1.);
          #include <tonemapping_fragment>
          #include <colorspace_fragment>
        }`,
    }),
  );
  floor.rotation.x = -Math.PI / 2;
  floor.position.y = -65;
  scene.add(floor);

  for (const [top, bottom, height, y, material] of [
    [332, 289, 22, -40, deck],
    [339, 333, 10, -24, titanium],
    [330, 339, 17, -10, deck],
    [39, 43, 12, 5, titanium],
    [30, 35, 12, 15, deck],
  ] as const) {
    const mesh = new THREE.Mesh(new THREE.CylinderGeometry(top, bottom, height, 96), material);
    mesh.position.y = y;
    scene.add(mesh);
  }

  function ring(radius: number, width: number, height: number, material: THREE.Material, arc = Math.PI * 2, start = 0) {
    const mesh = new THREE.Mesh(new THREE.TorusGeometry(radius, width, 6, 128, arc), material);
    mesh.rotation.x = -Math.PI / 2;
    mesh.rotation.z = start;
    mesh.position.y = height;
    scene.add(mesh);
  }
  ring(333, 1.45, -18, cyan);
  ring(322, 0.8, 0, titanium);
  ring(43, 1.15, 9, cyan);
  ring(29, 0.7, 21, amber);
  const graduations: THREE.Vector3[] = [];
  for (let index = 0; index < 160; index++) {
    const angle = index * Math.PI / 80;
    const inner = index % 10 === 0 ? 285 : index % 5 === 0 ? 296 : 308;
    for (const radius of [inner, 315]) {
      graduations.push(new THREE.Vector3(radius * Math.cos(angle), 0.2, radius * Math.sin(angle)));
    }
  }
  scene.add(new THREE.LineSegments(
    new THREE.BufferGeometry().setFromPoints(graduations),
    new THREE.LineBasicMaterial({ color: '#477b96' }),
  ));

  const panelGeometry = new THREE.BoxGeometry(27, 14, 65);
  const lampGeometry = new THREE.BoxGeometry(2, 1.8, 34);
  for (let index = 0; index < 12; index++) {
    const angle = index * Math.PI / 6;
    const housing = new THREE.Group();
    housing.rotation.y = angle;
    housing.position.set(324 * Math.cos(angle), 3, -324 * Math.sin(angle));
    const panel = new THREE.Mesh(panelGeometry, deck);
    const lamp = new THREE.Mesh(lampGeometry, index % 3 === 0 ? amber : cyan);
    lamp.position.set(7, 8, 0);
    housing.add(panel, lamp);
    scene.add(housing);
    ring(316, 1.25, 1, index % 3 === 0 ? amber : cyan, .34, angle + .095);
  }

  // The suspended core is a stage prop. Only persona bodies are raycast targets.
  const coreGeometry = new THREE.OctahedronGeometry(28);
  const core = new THREE.Mesh(coreGeometry, new THREE.MeshStandardMaterial({
    color: '#84e5ff', emissive: '#17698b', emissiveIntensity: 1.1, metalness: 0.43, roughness: 0.2, flatShading: true,
  }));
  core.scale.set(.48, .72, .48);
  core.position.y = 72;
  const cage = new THREE.LineSegments(
    new THREE.EdgesGeometry(coreGeometry),
    new THREE.LineBasicMaterial({ color: '#9bdcf5' }),
  );
  cage.scale.set(.62, .94, .62);
  cage.position.copy(core.position);
  scene.add(core, cage);
  for (let index = 0; index < 3; index++) {
    ring(37 + index * 4, .65, 29 + index * 7, index === 1 ? amber : cyan, Math.PI * 1.55, index * 2);
  }
  const shaft = new THREE.Mesh(
    new THREE.CylinderGeometry(13, 25, 105, 40, 1, true),
    new THREE.ShaderMaterial({
      uniforms: { tint: { value: new THREE.Color('#379bd2') } },
      vertexShader: `
        varying vec3 vNormal, vView;
        varying float vHeight;
        void main() {
          vec4 p = modelViewMatrix * vec4(position, 1.);
          vNormal = normalMatrix * normal;
          vView = -p.xyz;
          vHeight = position.y;
          gl_Position = projectionMatrix * p;
        }`,
      fragmentShader: `
        uniform vec3 tint;
        varying vec3 vNormal, vView;
        varying float vHeight;
        void main() {
          float edge = pow(1. - abs(dot(normalize(vNormal), normalize(vView))), 2.);
          float fade = smoothstep(-53., -38., vHeight) * (1. - smoothstep(4., 53., vHeight));
          gl_FragColor = vec4(tint, (.03 + .22 * edge) * fade);
        }`,
      transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, side: THREE.DoubleSide,
    }),
  );
  shaft.position.y = 74;
  shaft.scale.set(.55, .44, .55);
  shaft.position.y = 40;
  scene.add(shaft);

  const random = mulberry32(90274);
  const dust = new Float32Array(1650 * 3);
  for (let index = 0; index < dust.length; index += 3) {
    const angle = random() * Math.PI * 2;
    const radius = 125 + Math.sqrt(random()) * 670;
    dust.set([Math.cos(angle) * radius, -54 + random() ** 2 * 345, Math.sin(angle) * radius], index);
  }
  const dustGeometry = new THREE.BufferGeometry();
  dustGeometry.setAttribute('position', new THREE.BufferAttribute(dust, 3));
  scene.add(new THREE.Points(dustGeometry, new THREE.PointsMaterial({
    color: '#8b83ae', size: 1.4, transparent: true, opacity: .27, depthWrite: false,
  })));
  return (time: number) => {
    core.rotation.y = .42 + time * .26;
    cage.rotation.y = -.24 - time * .12;
    core.position.y = cage.position.y = 38 + 2 * Math.sin(time * 1.2);
  };
}
