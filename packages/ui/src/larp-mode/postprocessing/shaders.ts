// LARP Mode Slice 2: post-processing shaders. Pure shader objects —
// no GL needed to construct or unit-test them. The aberration pass
// shifts R/B channels radially from the viewport center by uOffset
// (0.0012, subtle by design). The grain pass adds 2D simplex noise
// (Ashima implementation) scaled by uAmount (0.05).

export interface LarpShader {
  uniforms: Record<string, { value: unknown }>;
  vertexShader: string;
  fragmentShader: string;
}

const PASS_THROUGH_VERTEX = /* glsl */ `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

export const CHROMATIC_OFFSET = 0.0012;

export const chromaticAberrationShader: LarpShader = {
  uniforms: {
    tDiffuse: { value: null },
    uOffset: { value: CHROMATIC_OFFSET },
  },
  vertexShader: PASS_THROUGH_VERTEX,
  fragmentShader: /* glsl */ `
    uniform sampler2D tDiffuse;
    uniform float uOffset;
    varying vec2 vUv;
    void main() {
      vec2 dir = vUv - 0.5;
      vec4 cr = texture2D(tDiffuse, vUv + dir * uOffset);
      vec4 cg = texture2D(tDiffuse, vUv);
      vec4 cb = texture2D(tDiffuse, vUv - dir * uOffset);
      gl_FragColor = vec4(cr.r, cg.g, cb.b, 1.0);
    }
  `,
};

export const FILM_GRAIN_AMOUNT = 0.05;

export const filmGrainShader: LarpShader = {
  uniforms: {
    tDiffuse: { value: null },
    uTime: { value: 0 },
    uAmount: { value: FILM_GRAIN_AMOUNT },
  },
  vertexShader: PASS_THROUGH_VERTEX,
  fragmentShader: /* glsl */ `
    uniform sampler2D tDiffuse;
    uniform float uTime;
    uniform float uAmount;
    varying vec2 vUv;
    vec3 permute(vec3 x) { return mod(((x * 34.0) + 1.0) * x, 289.0); }
    float snoise(vec2 v) {
      const vec4 C = vec4(0.211324865405187, 0.366025403784439, -0.577350269189626, 0.024390243902439);
      vec2 i = floor(v + dot(v, C.yy));
      vec2 x0 = v - i + dot(i, C.xx);
      vec2 i1 = (x0.x > x0.y) ? vec2(1.0, 0.0) : vec2(0.0, 1.0);
      vec4 x12 = x0.xyxy + C.xxzz;
      x12.xy -= i1;
      i = mod(i, 289.0);
      vec3 p = permute(permute(i.y + vec3(0.0, i1.y, 1.0)) + i.x + vec3(0.0, i1.x, 1.0));
      vec3 m = max(0.5 - vec3(dot(x0, x0), dot(x12.xy, x12.xy), dot(x12.zw, x12.zw)), 0.0);
      m = m * m;
      m = m * m;
      vec3 x = 2.0 * fract(p * C.www) - 1.0;
      vec3 h = abs(x) - 0.5;
      vec3 ox = floor(x + 0.5);
      vec3 a0 = x - ox;
      m *= 1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h);
      vec3 g;
      g.x = a0.x * x0.x + h.x * x0.y;
      g.yz = a0.yz * x12.xz + h.yz * x12.yw;
      return 130.0 * dot(m, g);
    }
    void main() {
      vec4 color = texture2D(tDiffuse, vUv);
      float n = snoise(vUv * 480.0 + uTime * 60.0);
      color.rgb += n * uAmount;
      gl_FragColor = color;
    }
  `,
};
