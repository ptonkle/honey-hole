#version 300 es
// The sky: one triangle over the whole screen (its corners from the core),
// on the far plane, so drawn after the solid world it fills only what
// nothing covers. vScreen is the point's place on the screen, -1 to 1 each
// way.
in vec2 aCorner;
out vec2 vScreen;
void main() {
  vScreen = aCorner;
  gl_Position = vec4(aCorner, 1.0, 1.0);
}
