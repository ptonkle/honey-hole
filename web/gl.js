// The WebGL 2 renderer. It uploads what the core returns and draws it;
// every size, stride, offset and uniform slot comes from the core's
// numbers(), so this file holds no layout of its own. (The machinery
// follows Ape Race's gl.js; the look is the core's.)

function compile(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    throw new Error(gl.getShaderInfoLog(shader));
  }
  return shader;
}

function link(gl, vertexSource, fragmentSource) {
  const program = gl.createProgram();
  gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, vertexSource));
  gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, fragmentSource));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(program));
  }
  return program;
}

// Returns null when the browser has no WebGL 2; the page then tells the core.
export function makeRenderer(canvas, N, shaders) {
  const gl = canvas.getContext("webgl2", { antialias: true, preserveDrawingBuffer: true });
  if (!gl) return null;

  const program = link(gl, shaders.vertex, shaders.fragment);
  gl.useProgram(program);
  gl.enable(gl.DEPTH_TEST);
  gl.disable(gl.CULL_FACE);

  const vao = gl.createVertexArray();
  gl.bindVertexArray(vao);
  const buffer = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
  N.vertex.attributes.forEach(([name, size, offset]) => {
    const loc = gl.getAttribLocation(program, name);
    gl.enableVertexAttribArray(loc);
    gl.vertexAttribPointer(loc, size, gl.FLOAT, false, N.vertex.stride, offset);
  });

  const uVP = gl.getUniformLocation(program, "uVP");
  const uniforms = N.uniforms.map(([name, size, at, call]) => ({ loc: gl.getUniformLocation(program, name), size, at, call }));

  function draw(view, vertices, count) {
    if (!count) return;
    gl.uniformMatrix4fv(uVP, false, view.subarray(...N.matrix));
    uniforms.forEach((u) => gl[u.call](u.loc, view.subarray(u.at, u.at + u.size)));
    gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STREAM_DRAW);
    gl.drawArrays(gl.TRIANGLES, 0, count);
  }

  return {
    resize(width, height) {
      canvas.width = width;
      canvas.height = height;
      gl.viewport(0, 0, width, height);
    },
    // view: the core's view values; vertices: the triangle list
    frame(view, vertices, count) {
      const [r, g, b] = view.subarray(...N.fog_colour);
      gl.clearColor(r, g, b, 1);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      draw(view, vertices, count);
    },
    // One more drawing into part of the buffer (a model in a 2D box, or
    // the Fish-Boy over the world): its rectangle, whether to clear its
    // colour, and its own view values, laid out as the 2D numbers' `pass`
    // says (crates/core/src/ui2d/mod.rs).
    pass(P, values, vertices, count) {
      const rect = values.subarray(...P.rect);
      gl.enable(gl.SCISSOR_TEST);
      gl.viewport(...rect);
      gl.scissor(...rect);
      if (values[P.clear]) {
        const [r, g, b] = values.subarray(...P.colour);
        gl.clearColor(r, g, b, 1);
        gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      } else {
        gl.clear(gl.DEPTH_BUFFER_BIT);
      }
      draw(values.subarray(...P.view), vertices, count);
      gl.disable(gl.SCISSOR_TEST);
      gl.viewport(0, 0, canvas.width, canvas.height);
    },
  };
}
