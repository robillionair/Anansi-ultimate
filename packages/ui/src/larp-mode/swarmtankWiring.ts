import * as THREE from 'three';

/** Visual wiring follows persona positions, with no inferred relationship data. */
export function buildSwarmtankWiring(scene: THREE.Scene, count: number) {
  const nodes = Array.from({ length: count }, () => new THREE.Vector3());
  const connections = nodes.flatMap((_, index) => [
    [index, index + 1],
    [index, index + 3],
  ].filter(([, end]) => end < count));
  const positions = new THREE.BufferAttribute(new Float32Array(connections.length * 24 * 3), 3);
  positions.setUsage(THREE.DynamicDrawUsage);
  const geometry = new THREE.BufferGeometry().setAttribute('position', positions);
  const material = new THREE.LineBasicMaterial({
    color: '#83bdce', transparent: true, opacity: .36, depthWrite: false,
  });
  const cables = new THREE.LineSegments(geometry, material);
  cables.frustumCulled = false;
  scene.add(cables);

  const cursor = new THREE.Vector3();
  const previous = new THREE.Vector3();
  return {
    place(index: number, position: THREE.Vector3) {
      nodes[index].copy(position);
    },
    update() {
      let vertex = 0;
      for (const [from, to] of connections) {
        const start = nodes[from];
        const end = nodes[to];
        const sag = Math.min(32, start.distanceTo(end) * .12);
        previous.copy(start);
        for (let step = 1; step <= 12; step++) {
          const t = step / 12;
          cursor.lerpVectors(start, end, t);
          cursor.y -= Math.sin(t * Math.PI) * sag;
          positions.setXYZ(vertex++, previous.x, previous.y, previous.z);
          positions.setXYZ(vertex++, cursor.x, cursor.y, cursor.z);
          previous.copy(cursor);
        }
      }
      positions.needsUpdate = true;
    },
  };
}
