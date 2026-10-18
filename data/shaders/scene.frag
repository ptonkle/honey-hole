#version 300 es
precision highp float;
// Flat low-poly light: one sun, a sky-to-ground fill blended by which way
// the face points, a tint toward the water's colour for what lies under the
// surface (y below 0), and distance fog that fades to the sky behind it.
// An unlit (glowing) colour skips the light and takes only part of the fog.
// Every value comes from the core as a uniform (crates/core/src/gfx/render.rs).
in vec3 vWorld;
in vec3 vNormal;
in vec3 vColour;
in float vGlow;
uniform vec3 uEye;
uniform vec2 uFog;
uniform vec3 uFogColour;
uniform vec3 uZenith;
uniform vec2 uSkyBand;
uniform vec3 uSunDir;
uniform vec3 uSun;
uniform vec3 uSky;
uniform vec3 uGround;
uniform vec2 uHemi;
uniform vec2 uUnder;
uniform vec3 uUnderColour;
uniform float uGlow;
out vec4 outColour;
void main() {
  vec3 n = normalize(vNormal);
  // either winding: a face seen from behind is lit on the side seen
  if (!gl_FrontFacing) n = -n;
  float sun = max(dot(n, uSunDir), 0.0);
  vec3 fill = mix(uGround, uSky, clamp(n.y * uHemi.x + uHemi.y, 0.0, 1.0));
  vec3 lit = mix(vColour * (fill + uSun * sun), vColour, vGlow);
  float under = clamp(-vWorld.y * uUnder.x, 0.0, 1.0) * uUnder.y;
  lit = mix(lit, uUnderColour, under);
  vec3 ray = vWorld - uEye;
  vec3 sky = mix(uFogColour, uZenith, smoothstep(uSkyBand.x, uSkyBand.y, normalize(ray).y));
  float f = smoothstep(uFog.x, uFog.y, length(ray)) * mix(1.0, uGlow, vGlow);
  outColour = vec4(mix(lit, sky, f), 1.0);
}
