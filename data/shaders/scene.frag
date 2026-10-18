#version 300 es
precision highp float;
// Flat low-poly light: one sun, a sky-to-ground fill blended by which way
// the face points, and distance fog that fades to the background. Every
// value comes from the core as a uniform (crates/core/src/gfx/render.rs).
in vec3 vWorld;
in vec3 vNormal;
in vec3 vColour;
uniform vec3 uEye;
uniform vec2 uFog;
uniform vec3 uFogColour;
uniform vec3 uSunDir;
uniform vec3 uSun;
uniform vec3 uSky;
uniform vec3 uGround;
uniform vec2 uHemi;
out vec4 outColour;
void main() {
  vec3 n = normalize(vNormal);
  // either winding: a face seen from behind is lit on the side seen
  if (!gl_FrontFacing) n = -n;
  float sun = max(dot(n, uSunDir), 0.0);
  vec3 fill = mix(uGround, uSky, clamp(n.y * uHemi.x + uHemi.y, 0.0, 1.0));
  vec3 lit = vColour * (fill + uSun * sun);
  float f = smoothstep(uFog.x, uFog.y, length(vWorld - uEye));
  outColour = vec4(mix(lit, uFogColour, f), 1.0);
}
