#version 300 es
// One vertex of an instanced prop (a tree, a reed, a rock), in the model's
// own space. A swaying model is baked by the core as its mean pose (aPos)
// and its motion along its loop's cosine and sine (aPosCos, aPosSin);
// aLoop is the cosine and sine of where this instance is in its loop, so
// each copy sways at its own phase. A still model's motion is zero. The
// instance's transform (three columns and a translation) places it.
in vec3 aPos;
in vec3 aNormal;
in vec3 aColour;
in float aGlow;
in vec3 aPosCos;
in vec3 aNormalCos;
in vec3 aPosSin;
in vec3 aNormalSin;
in vec3 aC0;
in vec3 aC1;
in vec3 aC2;
in vec3 aT;
in vec2 aLoop;
uniform mat4 uVP;
out vec3 vWorld;
out vec3 vNormal;
out vec3 vColour;
out float vGlow;
void main() {
  vec3 p = aPos + aPosCos * aLoop.x + aPosSin * aLoop.y;
  vec3 n = aNormal + aNormalCos * aLoop.x + aNormalSin * aLoop.y;
  vWorld = aC0 * p.x + aC1 * p.y + aC2 * p.z + aT;
  vNormal = aC0 * n.x + aC1 * n.y + aC2 * n.z;
  vColour = aColour;
  vGlow = aGlow;
  gl_Position = uVP * vec4(vWorld, 1.0);
}
