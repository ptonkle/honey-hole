#version 300 es
// One vertex of the frame's triangle list, already placed in the world by
// the core: its position, its face's flat normal and its face's colour.
in vec3 aPos;
in vec3 aNormal;
in vec3 aColour;
uniform mat4 uVP;
out vec3 vWorld;
out vec3 vNormal;
out vec3 vColour;
void main() {
  vWorld = aPos;
  vNormal = aNormal;
  vColour = aColour;
  gl_Position = uVP * vec4(aPos, 1.0);
}
