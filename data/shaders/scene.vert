#version 300 es
// One vertex of a triangle list already placed in the world by the core
// (the frame's posed models, the static terrain): its position, its face's
// flat normal, its face's colour and its glow (0 lit, 1 unlit).
in vec3 aPos;
in vec3 aNormal;
in vec3 aColour;
in float aGlow;
uniform mat4 uVP;
out vec3 vWorld;
out vec3 vNormal;
out vec3 vColour;
out float vGlow;
void main() {
  vWorld = aPos;
  vNormal = aNormal;
  vColour = aColour;
  vGlow = aGlow;
  gl_Position = uVP * vec4(aPos, 1.0);
}
