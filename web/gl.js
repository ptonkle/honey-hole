// The WebGL 2 renderer. It uploads what the core returns and draws it;
// every size, stride, offset and uniform slot comes from the core's
// numbers(), so this file holds no layout of its own. (The machinery
// follows Ape Race's gl.js; the look is the core's.)
//
// Each frame, in order: the sky; the static terrain (uploaded once per
// world); the props, one instanced draw per group the core lists; the
// frame's own posed triangles; and last the water, blended over them.

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

// A program and a setter for the view values it reads.
function program(gl, N, vertexSource, fragmentSource) {
  const p = link(gl, vertexSource, fragmentSource);
  const uVP = gl.getUniformLocation(p, "uVP");
  const uniforms = N.uniforms
    .map(([name, size, at, call]) => ({ loc: gl.getUniformLocation(p, name), size, at, call }))
    .filter((u) => u.loc);
  return {
    p,
    use(view) {
      gl.useProgram(p);
      gl.uniformMatrix4fv(uVP, false, view.subarray(...N.matrix));
      uniforms.forEach((u) => gl[u.call](u.loc, view.subarray(u.at, u.at + u.size)));
    },
    // the attributes this program reads, of a list of [name, size, offset]
    attributes(list) {
      return list.map(([name, size, offset]) => ({ loc: gl.getAttribLocation(p, name), size, offset })).filter((a) => a.loc >= 0);
    },
  };
}

// Points attributes at the bound buffer, `base` bytes in.
function point(gl, attributes, stride, base, divisor) {
  attributes.forEach((a) => {
    gl.enableVertexAttribArray(a.loc);
    gl.vertexAttribPointer(a.loc, a.size, gl.FLOAT, false, stride, base + a.offset);
    gl.vertexAttribDivisor(a.loc, divisor);
  });
}

// Returns null when the browser has no WebGL 2; the page then tells the core.
export function makeRenderer(canvas, N, shaders) {
  const gl = canvas.getContext("webgl2", { antialias: true, preserveDrawingBuffer: true });
  if (!gl) return null;

  const scene = program(gl, N, shaders.vertex, shaders.fragment);
  const props = program(gl, N, shaders.props, shaders.fragment);
  const water = program(gl, N, shaders.waterVertex, shaders.waterFragment);
  const sky = program(gl, N, shaders.skyVertex, shaders.skyFragment);
  gl.enable(gl.DEPTH_TEST);
  gl.disable(gl.CULL_FACE);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

  const stride = N.vertex.stride;
  const dynamic = gl.createBuffer();
  const fixed = gl.createBuffer();
  const perInstance = gl.createBuffer();
  const corners = gl.createBuffer();

  // the frame's posed triangles, and the static terrain, through the scene program
  const dynamicVao = gl.createVertexArray();
  gl.bindVertexArray(dynamicVao);
  gl.bindBuffer(gl.ARRAY_BUFFER, dynamic);
  point(gl, scene.attributes(N.vertex.attributes), stride, 0, 0);
  const fixedVao = gl.createVertexArray();
  gl.bindVertexArray(fixedVao);
  gl.bindBuffer(gl.ARRAY_BUFFER, fixed);
  point(gl, scene.attributes(N.vertex.attributes), stride, 0, 0);
  // the water, from the same static buffer
  const waterVao = gl.createVertexArray();
  gl.bindVertexArray(waterVao);
  point(gl, water.attributes(N.vertex.attributes), stride, 0, 0);
  // the props: pointed afresh for each group
  const propsVao = gl.createVertexArray();
  const propPose = props.attributes(N.vertex.attributes);
  const propCos = props.attributes(N.motion_cos);
  const propSin = props.attributes(N.motion_sin);
  const propI = props.attributes(N.instance.attributes);
  // the sky's triangle
  const skyVao = gl.createVertexArray();
  gl.bindVertexArray(skyVao);
  gl.bindBuffer(gl.ARRAY_BUFFER, corners);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(N.sky), gl.STATIC_DRAW);
  point(gl, sky.attributes([["aCorner", N.sky_size, 0]]), 0, 0, 0);
  gl.bindVertexArray(null);

  return {
    resize(width, height) {
      canvas.width = width;
      canvas.height = height;
      gl.viewport(0, 0, width, height);
    },
    // the static world: the terrain, the water and the prop meshes
    world(vertices) {
      gl.bindBuffer(gl.ARRAY_BUFFER, fixed);
      gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STATIC_DRAW);
    },
    // view: the core's view values; vertices: the posed triangles;
    // statics: the terrain's and the water's ranges; groups and instances:
    // the props to draw
    frame(view, vertices, count, statics, groups, instances) {
      const [r, g, b] = view.subarray(...N.fog_colour);
      gl.clearColor(r, g, b, 1);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      if (!view.length) return;
      const [terrainFirst, terrainCount, waterFirst, waterCount] = statics;

      gl.disable(gl.DEPTH_TEST);
      sky.use(view);
      gl.bindVertexArray(skyVao);
      gl.drawArrays(gl.TRIANGLES, 0, N.sky.length / N.sky_size);
      gl.enable(gl.DEPTH_TEST);

      scene.use(view);
      gl.bindVertexArray(fixedVao);
      gl.drawArrays(gl.TRIANGLES, terrainFirst, terrainCount);

      props.use(view);
      gl.bindVertexArray(propsVao);
      gl.bindBuffer(gl.ARRAY_BUFFER, perInstance);
      gl.bufferData(gl.ARRAY_BUFFER, instances, gl.STREAM_DRAW);
      for (let at = 0; at < groups.length; at += N.group) {
        const [pose, cos, sin, verts, first, many] = groups.subarray(at, at + N.group);
        gl.bindBuffer(gl.ARRAY_BUFFER, fixed);
        point(gl, propPose, stride, pose, 0);
        point(gl, propCos, stride, cos, 0);
        point(gl, propSin, stride, sin, 0);
        gl.bindBuffer(gl.ARRAY_BUFFER, perInstance);
        point(gl, propI, N.instance.stride, first, 1);
        gl.drawArraysInstanced(gl.TRIANGLES, 0, verts, many);
      }

      if (count) {
        scene.use(view);
        gl.bindVertexArray(dynamicVao);
        gl.bindBuffer(gl.ARRAY_BUFFER, dynamic);
        gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STREAM_DRAW);
        gl.drawArrays(gl.TRIANGLES, 0, count);
      }

      water.use(view);
      gl.bindVertexArray(waterVao);
      gl.enable(gl.BLEND);
      gl.depthMask(false);
      gl.drawArrays(gl.TRIANGLES, waterFirst, waterCount);
      gl.depthMask(true);
      gl.disable(gl.BLEND);
      gl.bindVertexArray(null);
    },
    // One more frame drawn into a rectangle of the canvas over what is
    // there (the fishing's picture-in-picture), inside a white border:
    // rectangles are [x, y, width, height] in pixels from the bottom left.
    inset(rect, border, ...frame) {
      gl.enable(gl.SCISSOR_TEST);
      gl.scissor(...border);
      gl.clearColor(1, 1, 1, 1);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.viewport(...rect);
      gl.scissor(...rect);
      this.frame(...frame);
      gl.disable(gl.SCISSOR_TEST);
      gl.viewport(0, 0, canvas.width, canvas.height);
    },
    // One more drawing into part of the buffer (a model in a 2D box, or
    // the Fish-Boy over the world): its rectangle, whether to clear its
    // colour, and its own view values, laid out as the 2D numbers' `pass`
    // says (crates/core/src/ui2d/mod.rs). Its triangles go through the
    // scene program, as the frame's posed ones do.
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
      if (count) {
        scene.use(values.subarray(...P.view));
        gl.bindVertexArray(dynamicVao);
        gl.bindBuffer(gl.ARRAY_BUFFER, dynamic);
        gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STREAM_DRAW);
        gl.drawArrays(gl.TRIANGLES, 0, count);
        gl.bindVertexArray(null);
      }
      gl.disable(gl.SCISSOR_TEST);
      gl.viewport(0, 0, canvas.width, canvas.height);
    },
  };
}
