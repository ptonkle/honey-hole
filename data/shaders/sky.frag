#version 300 es
precision highp float;
// The sky, or under the water the murk: the fog's colour at the horizon,
// turning to the zenith's overhead and to the deep colour looking down,
// by the height of the view's direction through this point of the screen
// (the camera's axes and the tangents of its half field of view come from
// the core). Under the water, soft light shafts fan from the light's
// direction (uShaft: two stripe counts round it and their phases;
// uShaftLook: strength, sharpness, and where they fade in toward the light).
// Above the water the deep colour is the fog's and the shafts' strength 0.
in vec2 vScreen;
uniform vec3 uFogColour;
uniform vec3 uZenith;
uniform vec2 uSkyBand;
uniform vec3 uDeep;
uniform vec2 uDeepBand;
uniform vec4 uShaft;
uniform vec4 uShaftLook;
uniform vec3 uShaftDir;
uniform vec3 uSheen;
uniform vec3 uCamRight;
uniform vec3 uCamUp;
uniform vec3 uCamFwd;
uniform vec2 uCamTan;
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
void main() {
  vec3 ray = normalize(uCamFwd + uCamRight * vScreen.x * uCamTan.x + uCamUp * vScreen.y * uCamTan.y);
  outColour = vec4(murk(ray), 1.0);
}
