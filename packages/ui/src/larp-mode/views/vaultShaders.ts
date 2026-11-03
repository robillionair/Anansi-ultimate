// LARP Mode Slice 2: optical bokeh shaders for the memory vault's
// particle field, kept in their own module so unit tests can assert on
// the shader sources without constructing GL objects. The vertex shader
// sharpens particles near the focal plane (uFocalDepth, the camera
// distance) and shrinks + dims off-plane ones; the fragment shader
// draws soft discs whose alpha falls with distance from the focal
// plane. The material pairs these with AdditiveBlending in the view.

export const VAULT_CAMERA_Z = 190;

export const BOKEH_VERTEX = /* glsl */ `
  attribute float aScale;
  attribute float aSeed;
  uniform float uFocalDepth;
  varying float vSharp;
  varying float vSeed;
  void main() {
    vec4 mv = modelViewMatrix * vec4(position, 1.0);
    float depth = -mv.z;
    vSharp = 1.0 - smoothstep(0.0, 70.0, abs(depth - uFocalDepth));
    vSeed = aSeed;
    gl_PointSize = aScale * (0.75 + vSharp * 3.0) * (300.0 / depth);
    gl_Position = projectionMatrix * mv;
  }
`;

export const BOKEH_FRAGMENT = /* glsl */ `
  varying float vSharp;
  varying float vSeed;
  void main() {
    vec2 c = gl_PointCoord - 0.5;
    float d = length(c);
    float alpha = smoothstep(0.5, 0.12, d) * (0.22 + 0.78 * vSharp);
    vec3 color = mix(vec3(0.15, 0.65, 1.0), vec3(1.0, 1.0, 1.0), vSharp);
    gl_FragColor = vec4(color * alpha, alpha);
  }
`;
