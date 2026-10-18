#version 300 es
// One vertex of an instanced prop (a tree, a reed, a rock). The mesh is in
// the model's own space, baked by the core at several poses of its sway:
// this vertex in this instance's frame (aPos) and the next (aPosB), blended
// by the instance's aBlend. The instance's transform (three columns and a
// translation) places it in the world.
in vec3 aPos;
in vec3 aNormal;
in vec3 aColour;
in float aGlow;
in vec3 aPosB;
in vec3 aNormalB;
in vec3 aC0;
in vec3 aC1;
in vec3 aC2;
in vec3 aT;
in float aBlend;
uniform mat4 uVP;
out vec3 vWorld;
out vec3 vNormal;
out vec3 vColour;
out float vGlow;
void main() {
  vec3 p = mix(aPos, aPosB, aBlend);
  vec3 n = mix(aNormal, aNormalB, aBlend);
  vWorld = aC0 * p.x + aC1 * p.y + aC2 * p.z + aT;
  vNormal = aC0 * n.x + aC1 * n.y + aC2 * n.z;
  vColour = aColour;
  vGlow = aGlow;
  gl_Position = uVP * vec4(vWorld, 1.0);
}
