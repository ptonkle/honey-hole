#version 300 es
precision highp float;
// The overcast sky: the fog's colour at the horizon, turning to the
// zenith's overhead, by the height of the view's direction through this
// point of the screen (the camera's axes and the tangents of its half
// field of view come from the core).
in vec2 vScreen;
uniform vec3 uFogColour;
uniform vec3 uZenith;
uniform vec2 uSkyBand;
uniform vec3 uCamRight;
uniform vec3 uCamUp;
uniform vec3 uCamFwd;
uniform vec2 uCamTan;
out vec4 outColour;
void main() {
  vec3 ray = normalize(uCamFwd + uCamRight * vScreen.x * uCamTan.x + uCamUp * vScreen.y * uCamTan.y);
  outColour = vec4(mix(uFogColour, uZenith, smoothstep(uSkyBand.x, uSkyBand.y, ray.y)), 1.0);
}
