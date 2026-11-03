// LARP Mode Slice 2: cubic bezier math for the neural funnel, plus the
// photon pulse lifecycle. Pure and unit-tested. Filaments are fixed
// geometry: 4 amber/gold input streams converge on the singularity,
// 3 cyan outputs fan out to the L1/L2/L3 blocks.

export interface Pt {
  x: number;
  y: number;
}

export function cubicBezierPoint(p0: Pt, p1: Pt, p2: Pt, p3: Pt, t: number): Pt {
  const u = 1 - t;
  return {
    x: u * u * u * p0.x + 3 * u * u * t * p1.x + 3 * u * t * t * p2.x + t * t * t * p3.x,
    y: u * u * u * p0.y + 3 * u * u * t * p1.y + 3 * u * t * t * p2.y + t * t * t * p3.y,
  };
}

export interface Filament {
  id: string;
  points: [Pt, Pt, Pt, Pt];
  color: string;
}

export interface FunnelLayout {
  inputs: Filament[];
  outputs: Filament[];
  singularity: Pt;
  blocks: { label: string; x: number; y: number }[];
}

export function filamentLayout(width: number, height: number): FunnelLayout {
  const singularity: Pt = { x: width * 0.5, y: height * 0.5 };
  const inputYs = [0.15, 0.35, 0.65, 0.85];
  const inputColors = ['#ff8a3d', '#e8b93c', '#ff8a3d', '#e8b93c'];
  const inputs: Filament[] = inputYs.map((f, i) => ({
    id: `in-${i}`,
    points: [
      { x: 0, y: height * f },
      { x: width * 0.22, y: height * f },
      { x: width * 0.34, y: height * 0.5 },
      singularity,
    ],
    color: inputColors[i],
  }));
  const blockDefs = [
    { label: 'L1', y: 0.25 },
    { label: 'L2', y: 0.5 },
    { label: 'L3', y: 0.75 },
  ];
  const outputs: Filament[] = blockDefs.map((b, i) => ({
    id: `out-${i}`,
    points: [
      singularity,
      { x: width * 0.66, y: height * 0.5 },
      { x: width * 0.78, y: height * b.y },
      { x: width * 0.88, y: height * b.y },
    ],
    color: '#00f0ff',
  }));
  return {
    inputs,
    outputs,
    singularity,
    blocks: blockDefs.map((b) => ({ label: b.label, x: width * 0.88, y: height * b.y })),
  };
}

export interface Pulse {
  id: number;
  filamentIndex: number;
  t: number;
}

export const PULSE_TRAVERSE_MS = 2400;

export function spawnPulse(id: number, filamentIndex: number): Pulse {
  return { id, filamentIndex, t: 0 };
}

export function advancePulses(pulses: Pulse[], dtMs: number): Pulse[] {
  const step = dtMs / PULSE_TRAVERSE_MS;
  const next: Pulse[] = [];
  for (const pulse of pulses) {
    const t = pulse.t + step;
    if (t < 1) next.push({ ...pulse, t });
  }
  return next;
}
