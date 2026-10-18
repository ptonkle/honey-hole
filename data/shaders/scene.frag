#version 300 es
precision highp float;
// Flat low-poly light: one sun, a sky-to-ground fill blended by which way
// the face points, a tint toward the water's colour for what lies under the
// surface (y below 0), and distance fog that fades to the sky behind it.
// An unlit (glowing) colour skips the light and takes only part of the fog.
// Under the water (the core sets the strengths; above it they are 0): the
// ripples' caustic net plays on faces that look up (uCaustic.z), the fog
// is the murk with its light shafts (as sky.frag), and what floats on the
// surface seen from below is a dark shape against the light (uSilhouette:
// the height it starts at and its strength, toward uShadow).
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
uniform vec3 uDeep;
uniform vec2 uDeepBand;
uniform vec4 uShaft;
uniform vec4 uShaftLook;
uniform vec3 uShaftDir;
uniform vec3 uSheen;
uniform vec4 uRipple1;
uniform vec4 uRipple2;
uniform vec4 uRipple3;
uniform vec4 uCaustic;
uniform vec2 uSilhouette;
uniform vec3 uShadow;
uniform vec3 uSunDir;
uniform vec3 uSun;
uniform vec3 uSky;
uniform vec3 uGround;
uniform vec2 uHemi;
uniform vec2 uUnder;
uniform vec3 uUnderColour;
uniform float uGlow;
out vec4 outColour;
vec3 murk(vec3 d) {
  vec3 c = mix(uFogColour, uZenith, smoothstep(uSkyBand.x, uSkyBand.y, d.y));
  c = mix(c, uDeep, smoothstep(uDeepBand.x, uDeepBand.y, -d.y));
  if (uShaftLook.x <= 0.0) return c;
  vec3 a = normalize(cross(uShaftDir, vec3(1.0, 0.0, 0.0)));
  vec3 b = cross(uShaftDir, a);
  float turn = atan(dot(d, b), dot(d, a));
  float rays = max(sin(turn * uShaft.x + uShaft.z) + sin(turn * uShaft.y + uShaft.w) - 1.0, 0.0);
  float fade = smoothstep(uShaftLook.z, uShaftLook.w, dot(d, uShaftDir));
  return c + uSheen * pow(rays, uShaftLook.y) * uShaftLook.x * fade;
}
// The caustic net: bright thin lines where three ripples cancel.
float caustic(vec2 p) {
  float s = sin(dot(p, uRipple1.xy) + uRipple1.z) + sin(dot(p, uRipple2.xy) + uRipple2.z) + sin(dot(p, uRipple3.xy) + uRipple3.z);
  return pow(clamp(1.0 - abs(s) * uCaustic.x, 0.0, 1.0), uCaustic.y);
}
void main() {
  vec3 n = normalize(vNormal);
  // either winding: a face seen from behind is lit on the side seen
  if (!gl_FrontFacing) n = -n;
  float sun = max(dot(n, uSunDir), 0.0);
  vec3 fill = mix(uGround, uSky, clamp(n.y * uHemi.x + uHemi.y, 0.0, 1.0));
  vec3 lit = mix(vColour * (fill + uSun * sun), vColour, vGlow);
  if (uCaustic.z > 0.0 && vWorld.y < 0.0) {
    lit += uSheen * caustic(vWorld.xz) * uCaustic.z * clamp(n.y, 0.0, 1.0) * (1.0 - vGlow);
  }
  float under = clamp(-vWorld.y * uUnder.x, 0.0, 1.0) * uUnder.y;
  lit = mix(lit, uUnderColour, under);
  if (uSilhouette.y > 0.0 && vWorld.y > uSilhouette.x) {
    lit = mix(lit, uShadow, uSilhouette.y);
  }
  vec3 ray = vWorld - uEye;
  vec3 sky = murk(normalize(ray));
  float f = smoothstep(uFog.x, uFog.y, length(ray)) * mix(1.0, uGlow, vGlow);
  outColour = vec4(mix(lit, sky, f), 1.0);
}
