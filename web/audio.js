// The synth: plays what the core says to play, with WebAudio, and decides
// nothing.
//
// Each frame the core hands over (game.sound()) the master volume, the cues
// to start (each a list of layers: a waveform or the noise buffer, a gain
// schedule, a frequency schedule, an optional filter and wobble, all times
// in seconds from now and every value final), and the loops running this
// frame with their settings. A loop the core stops naming fades out. The
// recipes and every number come from data/sounds.json and data/tuning.json
// through the core (crates/core/src/sound/); this file holds none.
//
// Browsers start sound only after a click or a key press, so the
// AudioContext starts on the first one. There is no panning: the brief
// rules out directional sound.

let game = null;
let ctx = null;
let master = null;
let noise = null;
const voices = new Map();

// For the browser check: the context's state and how many layers have been
// built.
let built = 0;

function mark(name, value) {
  document.body.dataset[name] = value;
}

function wake() {
  if (!ctx) {
    const Context = window.AudioContext || window.webkitAudioContext;
    if (!Context) {
      mark("audio", "unavailable");
      return;
    }
    ctx = new Context();
    master = ctx.createGain();
    master.gain.value = 0;
    // a safety net against clipping when many sounds play at once
    const limiter = ctx.createDynamicsCompressor();
    master.connect(limiter);
    limiter.connect(ctx.destination);
    const samples = game.sound_noise();
    noise = ctx.createBuffer(1, samples.length, ctx.sampleRate);
    noise.copyToChannel(samples, 0);
  }
  if (ctx.state === "suspended") ctx.resume();
  mark("audio", "started");
}

// ---- building blocks ----

function source(wave) {
  if (wave === "noise") {
    const s = ctx.createBufferSource();
    s.buffer = noise;
    s.loop = true;
    return s;
  }
  const o = ctx.createOscillator();
  o.type = wave;
  return o;
}

// A parameter's schedule: { m: the AudioParam method, t: seconds, v }.
function schedule(param, points, from) {
  points.forEach((p) => param[p.m](p.v, from + p.t));
}

// A slow oscillator added to `param` (a pitch wobble in cents, a volume
// throb in gain).
function lfo(rate, depth, param) {
  const o = ctx.createOscillator();
  const d = ctx.createGain();
  o.frequency.value = rate;
  d.gain.value = depth;
  o.connect(d);
  d.connect(param);
  return { o, d };
}

// src -> (filter) -> gain -> master
function chain(src, filterType) {
  const g = ctx.createGain();
  g.gain.value = 0;
  let out = src;
  let filter = null;
  if (filterType) {
    filter = ctx.createBiquadFilter();
    filter.type = filterType;
    out.connect(filter);
    out = filter;
  }
  out.connect(g);
  g.connect(master);
  built += 1;
  mark("audioLayers", built);
  return { g, filter };
}

// ---- cues ----

function playLayer(l, now) {
  const src = source(l.wave);
  const { g, filter } = chain(src, l.filter && l.filter.type);
  schedule(g.gain, l.gain, now);
  if (l.freq) schedule(src.frequency, l.freq, now);
  if (filter) {
    filter.Q.value = l.filter.q;
    schedule(filter.frequency, l.filter.freq, now);
  }
  const start = now + l.start;
  const stop = now + l.stop;
  if (l.wobble) {
    const w = lfo(l.wobble.rate, l.wobble.cents, src.detune);
    w.o.start(start);
    w.o.stop(stop);
  }
  src.start(start);
  src.stop(stop);
  src.onended = () => g.disconnect();
}

function playCue(cue, now) {
  cue.layers.forEach((l) => playLayer(l, now));
  mark("lastSound", cue.id);
}

// ---- loops ----

function startVoice(v) {
  const now = ctx.currentTime;
  const parts = v.layers.map((l) => {
    const src = source(l.wave);
    const { g, filter } = chain(src, l.filter && l.filter.type);
    const part = { src, g, filter, wobble: null, throb: null };
    if (l.freq) src.frequency.setValueAtTime(l.freq, now);
    if (filter) {
      filter.frequency.setValueAtTime(l.filter.freq, now);
      filter.Q.setValueAtTime(l.filter.q, now);
    }
    if (l.wobble) part.wobble = lfo(l.wobble.rate, l.wobble.cents, src.detune);
    if (l.throb) part.throb = lfo(l.throb.rate, l.throb.amount, g.gain);
    [src, part.wobble && part.wobble.o, part.throb && part.throb.o].forEach((o) => o && o.start(now));
    return part;
  });
  return { parts };
}

function setVoice(voice, v, smooth) {
  const now = ctx.currentTime;
  const glide = (param, value) => param.setTargetAtTime(value, now, smooth);
  v.layers.forEach((l, i) => {
    const p = voice.parts[i];
    if (!p) return;
    glide(p.g.gain, l.gain);
    if (l.freq) glide(p.src.frequency, l.freq);
    if (p.filter) {
      glide(p.filter.frequency, l.filter.freq);
      glide(p.filter.Q, l.filter.q);
    }
    if (p.throb) {
      glide(p.throb.o.frequency, l.throb.rate);
      glide(p.throb.d.gain, l.throb.amount);
    }
    if (p.wobble) glide(p.wobble.o.frequency, l.wobble.rate);
  });
}

function stopVoice(voice, fade) {
  const now = ctx.currentTime;
  const end = now + fade;
  voice.parts.forEach((p) => {
    [p.g.gain, p.throb && p.throb.d.gain].forEach((param) => {
      if (!param) return;
      param.cancelScheduledValues(now);
      param.setValueAtTime(param.value, now);
      param.linearRampToValueAtTime(0, end);
    });
    [p.src, p.wobble && p.wobble.o, p.throb && p.throb.o].forEach((o) => o && o.stop(end));
    p.src.onended = () => p.g.disconnect();
  });
}

function syncVoices(frame) {
  const named = new Set();
  frame.voices.forEach((v) => {
    named.add(v.key);
    if (!voices.has(v.key)) voices.set(v.key, startVoice(v));
    setVoice(voices.get(v.key), v, frame.smooth);
  });
  voices.forEach((voice, key) => {
    if (named.has(key)) return;
    stopVoice(voice, frame.fade);
    voices.delete(key);
  });
  mark("audioVoices", [...voices.keys()].join(" "));
}

// ---- each frame ----

function play(frame) {
  const now = ctx.currentTime;
  master.gain.setTargetAtTime(frame.master, now, frame.smooth);
  frame.cues.forEach((cue) => playCue(cue, now));
  syncVoices(frame);
}

function tick(now) {
  game.advance(now);
  // taken every frame, so a cue from before sound could start is dropped,
  // not played late
  const result = JSON.parse(game.sound());
  if (result.ok && ctx && ctx.state !== "closed") play(result.ok);
  requestAnimationFrame(tick);
}

export function startAudio(theGame) {
  game = theGame;
  ["pointerdown", "keydown"].forEach((type) => window.addEventListener(type, wake, true));
  mark("audio", "waiting");
  requestAnimationFrame(tick);
}
