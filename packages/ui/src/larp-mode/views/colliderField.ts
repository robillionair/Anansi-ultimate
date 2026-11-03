// LARP Mode Slice 4: pure beam math for the collider field.
// Each beam is a cubic bezier from its origin to the interaction point
// at the origin (beam A from the left, beam B from the right, slight
// vertical arcs). Doppler coloring is a STYLIZED visual effect: the
// color is a function of the simulated trajectory parameter (beam
// color far away, white-hot at the interaction point) — not measured
// physics.

export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

export type BeamId = 'A' | 'B';

const CONTROL: Record<BeamId, { p0: Vec3; p1: Vec3; p2: Vec3; p3: Vec3 }> = {
  A: {
    p0: { x: -140, y: -30, z: 0 },
    p1: { x: -80, y: 40, z: 20 },
    p2: { x: -30, y: -20, z: -10 },
    p3: { x: 0, y: 0, z: 0 },
  },
  B: {
    p0: { x: 140, y: 30, z: 0 },
    p1: { x: 80, y: -40, z: -20 },
    p2: { x: 30, y: 20, z: 10 },
    p3: { x: 0, y: 0, z: 0 },
  },
};

export function beamPoint(beam: BeamId, t: number): Vec3 {
  const c = CONTROL[beam];
  const u = 1 - t;
  return {
    x: u * u * u * c.p0.x + 3 * u * u * t * c.p1.x + 3 * u * t * t * c.p2.x + t * t * t * c.p3.x,
    y: u * u * u * c.p0.y + 3 * u * u * t * c.p1.y + 3 * u * t * t * c.p2.y + t * t * t * c.p3.y,
    z: u * u * u * c.p0.z + 3 * u * u * t * c.p1.z + 3 * u * t * t * c.p2.z + t * t * t * c.p3.z,
  };
}

const BEAM_RGB: Record<BeamId, [number, number, number]> = {
  A: [0, 0.94, 1],
  B: [1, 0.54, 0.24],
};

/** Beam color at t=0, white-hot at t=1 (the interaction point). */
export function dopplerColor(beam: BeamId, t: number): [number, number, number] {
  const base = BEAM_RGB[beam];
  return [
    base[0] + (1 - base[0]) * t,
    base[1] + (1 - base[1]) * t,
    base[2] + (1 - base[2]) * t,
  ];
}
