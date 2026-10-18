#version 300 es
// The sky: one triangle over the whole screen (its corners from the core),
// drawn behind everything. vScreen is the point's place on the screen,
// -1 to 1 each way.
in vec2 aCorner;
out vec2 vScreen;
void main() {
  vScreen = aCorner;
  gl_Position = vec4(aCorner, 0.0, 1.0);
}
