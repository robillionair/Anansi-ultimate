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
  uniform float uTime;
  varying float vSharp;
  varying float vSeed;
  varying float vDepth;
  void main() {
    vec3 p = position;
    p += vec3(sin(uTime*.13+aSeed*32.),cos(uTime*.11+aSeed*19.),sin(uTime*.09+aSeed*17.))*1.1;
    vec4 mv = modelViewMatrix * vec4(p, 1.0);
    float depth = -mv.z;
    vDepth = depth;
    vSharp = 1.0 - smoothstep(0.0, 70.0, abs(depth - uFocalDepth));
    vSeed = aSeed;
    float blur = (1.0-vSharp)*(1.0-vSharp);
    gl_PointSize = clamp(aScale * (0.52 + blur * 4.0) * (240.0 / max(depth,1.0)), .7, 19.0);
    gl_Position = projectionMatrix * mv;
  }
`;

export const BOKEH_FRAGMENT = /* glsl */ `
  varying float vSharp;
  varying float vSeed;
  varying float vDepth;
  void main() {
    vec2 c = gl_PointCoord - 0.5;
    float d = length(c);
    float disc = 1.0-smoothstep(.12,.5,d);
    float core = exp(-d*d*52.0);
    float alpha = mix(disc*.12,core*.8,vSharp)*(.3+vSeed*.7);
    vec3 amber = mix(vec3(.68,.22,.09),vec3(1.,.72,.35),vSeed);
    vec3 color = mix(amber,vec3(.7,.84,1.),pow(vSeed,9.));
    color=mix(color,vec3(1.,.91,.73),vSharp*.5);
    gl_FragColor = vec4(color*(.8+core*1.8),alpha);
  }
`;
