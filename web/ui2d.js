// The 2D layer's replayer. The core sends a draw list each frame: numbers
// (each op's code and arguments) and strings (colours, fonts, alignments,
// text, image ids) that ops name by index. This file looks each code up by
// name in the core's numbers and draws it on the overlay canvas; it holds no
// layout, size or colour of its own (crates/core/src/ui2d/mod.rs).
//
// A "hole" clears its rectangle, so the model the WebGL renderer drew there
// shows through; a "bitmap" draws an image the core made, fetched once and
// kept under its id.

export function makeOverlay(canvas, N, fetchBitmap) {
  const ctx = canvas.getContext("2d");
  const images = {};

  function image(id) {
    if (!images[id]) images[id] = fetchBitmap(id);
    return images[id];
  }

  // each op reads its own arguments, in order, from the list
  let nums = null;
  let strs = null;
  let i = 0;
  const a = () => nums[i++];
  const s = () => strs[nums[i++]];

  function paint(fill, stroke, width) {
    if (fill) {
      ctx.fillStyle = fill;
      ctx.fill();
    }
    if (stroke) {
      ctx.strokeStyle = stroke;
      ctx.lineWidth = width;
      ctx.stroke();
    }
  }

  const ops = {
    layer() {
      ctx.setTransform(a(), 0, 0, a(), a(), a());
    },
    fill_rect() {
      ctx.fillStyle = s();
      ctx.fillRect(a(), a(), a(), a());
    },
    stroke_rect() {
      ctx.strokeStyle = s();
      ctx.lineWidth = a();
      ctx.strokeRect(a(), a(), a(), a());
    },
    round_rect() {
      const fill = s();
      const stroke = s();
      const width = a();
      ctx.beginPath();
      ctx.roundRect(a(), a(), a(), a(), a());
      paint(fill, stroke, width);
    },
    line() {
      ctx.strokeStyle = s();
      ctx.lineWidth = a();
      ctx.beginPath();
      ctx.moveTo(a(), a());
      ctx.lineTo(a(), a());
      ctx.stroke();
    },
    circle() {
      const fill = s();
      const stroke = s();
      const width = a();
      ctx.beginPath();
      const x = a();
      const y = a();
      const r = a();
      ctx.moveTo(x + r, y);
      ctx.arc(x, y, r, 0, N.full_turn);
      paint(fill, stroke, width);
    },
    arc() {
      ctx.strokeStyle = s();
      ctx.lineWidth = a();
      ctx.beginPath();
      ctx.arc(a(), a(), a(), a(), a());
      ctx.stroke();
    },
    poly() {
      const fill = s();
      const stroke = s();
      const width = a();
      const n = a();
      ctx.beginPath();
      for (let k = 0; k < n; k++) ctx.lineTo(a(), a());
      ctx.closePath();
      paint(fill, stroke, width);
    },
    radial() {
      const inner = s();
      const outer = s();
      const x = a();
      const y = a();
      const r0 = a();
      const r1 = a();
      const g = ctx.createRadialGradient(x, y, r0, x, y, r1);
      g.addColorStop(0, inner || "transparent");
      g.addColorStop(1, outer || "transparent");
      ctx.fillStyle = g;
      ctx.beginPath();
      ctx.arc(x, y, r1, 0, N.full_turn);
      ctx.moveTo(x + r0, y);
      ctx.arc(x, y, r0, 0, N.full_turn, true);
      ctx.fill("evenodd");
    },
    text() {
      ctx.fillStyle = s();
      ctx.font = s();
      ctx.textAlign = s();
      const x = a();
      const y = a();
      ctx.fillText(s(), x, y);
    },
    clip() {
      ctx.save();
      ctx.beginPath();
      ctx.rect(a(), a(), a(), a());
      ctx.clip();
    },
    unclip() {
      ctx.restore();
    },
    bitmap() {
      const img = image(s());
      ctx.drawImage(img, a(), a(), a(), a());
    },
    hole() {
      ctx.clearRect(a(), a(), a(), a());
    },
    dash() {
      const n = a();
      const d = [];
      for (let k = 0; k < n; k++) d.push(a());
      ctx.setLineDash(d);
    },
    alpha() {
      ctx.globalAlpha = a();
    },
  };

  // code -> op, from the core's names
  const byCode = [];
  Object.keys(N.ops).forEach((name) => {
    byCode[N.ops[name]] = ops[name];
  });

  return {
    // a canvas given a size is cleared, so only a new size is set (the
    // page asks to resize on every view model)
    resize(width, height) {
      if (canvas.width === width && canvas.height === height) return;
      canvas.width = width;
      canvas.height = height;
    },
    draw(values, strings) {
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.globalAlpha = 1;
      ctx.setLineDash([]);
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      ctx.textBaseline = "middle";
      ctx.imageSmoothingEnabled = false;
      nums = values;
      strs = strings;
      i = 0;
      while (i < nums.length) byCode[a()]();
    },
  };
}

// An image from the core's RGBA bytes, drawn onto a canvas of its own.
export function bitmapFrom(rgba, size) {
  const [w, h] = size;
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  if (w && h) c.getContext("2d").putImageData(new ImageData(new Uint8ClampedArray(rgba), w, h), 0, 0);
  return c;
}
