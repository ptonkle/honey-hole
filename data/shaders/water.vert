#version 300 es
// One vertex of the lake's surface, flat at y = 0 from the core and lifted
// here by two travelling waves (uWave: k along x, k along z, height, phase;
// the core advances the phase with the clock). Its colour is the depth's.
in vec3 aPos;
in vec3 aColour;
uniform mat4 uVP;
uniform vec4 uWave1;
uniform vec4 uWave2;
out vec3 vWorld;
out vec3 vColour;
void main() {
  vec3 p = aPos;
  p.y += uWave1.z * sin(dot(p.xz, uWave1.xy) + uWave1.w) + uWave2.z * sin(dot(p.xz, uWave2.xy) + uWave2.w);
  vWorld = p;
  vColour = aColour;
  gl_Position = uVP * vec4(p, 1.0);
}
