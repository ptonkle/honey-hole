// The page draws what the core returns and decides nothing.
//
// The core describes the screen (its name, panels and their items), and this
// file builds the DOM from that description without knowing what any panel
// means. Every control sends its action name and value back through `act`,
// the core's one door for buttons, settings, keys and screens; the core
// answers with the next view model. No state lives here: what is drawn is
// what the core last returned.
import init, { Game } from "./pkg/tool_shim.js";
import { makeRenderer } from "./gl.js";
import { makeOverlay, bitmapFrom } from "./ui2d.js";
import { startAudio } from "./audio.js";

const el = (id) => document.getElementById(id);
const make = (tag, cls, text) => {
  const node = document.createElement(tag);
  if (cls) node.className = cls;
  if (text !== undefined) node.textContent = text;
  return node;
};

let game = null;
let ui = null;
let panelsText = "";

// ---- talking to the core ----

// What each menu element does, by its id: { click, change, input, keydown }
// handlers, made afresh with each build. The menus listen once, on their
// box, and look the target up here, so an element kept across a rebuild
// (drawMenus patches the old tree to match the new one) does the latest
// thing.
let handlers = {};

function on(id, type, fn) {
  if (!handlers[id]) handlers[id] = {};
  handlers[id][type] = fn;
}

function dispatch(e) {
  for (let t = e.target; t && t.id !== "menus"; t = t.parentElement) {
    const h = t.id && handlers[t.id] && handlers[t.id][e.type];
    if (h) {
      h(t, e);
      return;
    }
  }
}

function send(name, value) {
  draw(JSON.parse(game.act(name, value)));
}

// ---- items, one builder per type ----

function button(item) {
  const b = make("button", item.quiet ? "button quiet" : "button", item.text);
  b.type = "button";
  b.id = item.id;
  b.disabled = item.disabled === true;
  if (item.badge) b.append(make("span", "badge", item.badge));
  on(item.id, "click", () => send(item.id, ""));
  return b;
}

function slider(item) {
  const wrap = make("div", "field slider");
  const label = make("label", "", item.label);
  const input = make("input");
  input.type = "range";
  input.id = item.id;
  input.setAttribute("min", item.min);
  input.setAttribute("max", item.max);
  input.setAttribute("step", item.step);
  input.value = item.value;
  label.htmlFor = item.id;
  on(item.id, "change", (t) => send(item.id, t.value));
  const shown = make("span", "slider-value", item.text);
  shown.id = item.id + ".text";
  wrap.append(label, input, shown);
  return wrap;
}

function toggle(item) {
  const wrap = make("label", "field toggle");
  const input = make("input");
  input.type = "checkbox";
  input.id = item.id;
  input.checked = item.value;
  on(item.id, "change", (t) => send(item.id, t.checked ? "on" : "off"));
  wrap.append(input, make("span", "", item.label));
  return wrap;
}

function select(item) {
  const wrap = make("div", "field");
  const label = make("label", "", item.label);
  const input = make("select");
  item.options.forEach((o) => {
    const opt = make("option", "", o.text);
    opt.value = o.value;
    opt.disabled = o.disabled === true;
    input.append(opt);
  });
  input.value = item.value;
  input.id = item.id;
  label.htmlFor = item.id;
  on(item.id, "change", (t) => send(item.id, t.value));
  wrap.append(label, input);
  return wrap;
}

function text(item) {
  const wrap = make("div", "field text");
  const label = make("label", "", item.label);
  const input = make("input");
  input.type = "text";
  input.id = item.id;
  input.value = item.value;
  input.maxLength = item.max;
  if (item.placeholder) input.placeholder = item.placeholder;
  label.htmlFor = item.id;
  on(item.id, "change", (t) => send(item.id, t.value));
  wrap.append(label, input);
  return wrap;
}

function list(item) {
  const table = make("table", "pairs");
  item.rows.forEach((r) => {
    const tr = make("tr");
    tr.append(make("th", "", r[0]), make("td", "", r[1]));
    table.append(tr);
  });
  return table;
}

function table(item) {
  const box = make("table", "cells");
  const head = make("tr");
  item.head.forEach((h) => head.append(make("th", "", h)));
  box.append(head);
  item.rows.forEach((r) => {
    const tr = make("tr", r.marked ? "marked" : "");
    r.cells.forEach((c) => tr.append(make("td", "", c)));
    box.append(tr);
  });
  return box;
}

// Each version, its date and its lines (a minor version drawn smaller).
function notes(item) {
  const box = make("div", "notes");
  box.id = item.id;
  item.entries.forEach((e) => {
    const sec = make("section", e.minor ? "note minor" : "note");
    const head = make("h3", "note-head");
    head.append(make("span", "note-version", e.version), make("span", "note-date", e.date));
    const ul = make("ul");
    e.lines.forEach((l) => ul.append(make("li", "", l)));
    sec.append(head, ul);
    box.append(sec);
  });
  return box;
}

// A bar the core fills: the share filled, and any zone and marker on it,
// as CSS lengths the stylesheet lays out across or up.
function meter(item) {
  const wrap = make("div", item.vertical ? "field meter vertical" : "field meter");
  wrap.id = item.id;
  const bar = make("div", "meter-bar");
  const fill = make("div", "meter-fill");
  fill.style.setProperty("--at", item.fill + "%");
  bar.append(fill);
  if (item.zone) {
    const zone = make("div", "meter-zone");
    zone.style.setProperty("--from", item.zone[0] + "%");
    zone.style.setProperty("--to", item.zone[1] + "%");
    bar.append(zone);
  }
  if (item.marker !== undefined) {
    const mark = make("div", "meter-marker");
    mark.style.setProperty("--at", item.marker + "%");
    bar.append(mark);
  }
  wrap.append(make("span", "meter-label", item.label), bar, make("span", "meter-text", item.text));
  return wrap;
}

const builders = { button, slider, toggle, select, text, list, table, notes, meter };

// The panels as the core gives them, built as a fresh tree off the page.
function buildMenus() {
  handlers = {};
  const box = make("div");
  (ui.panels || []).forEach((p) => {
    const panel = make("section", "panel " + p.kind);
    if (p.heading) {
      const h = make("h2", "", p.heading);
      // a heading the core makes clickable
      if (p.click) {
        h.id = p.click;
        on(p.click, "click", () => send(p.click, ""));
      }
      panel.append(h);
    }
    if (p.sub) panel.append(make("p", "sub", p.sub));
    p.items.forEach((item) => panel.append(builders[item.type](item)));
    box.append(panel);
  });
  return box;
}

// ---- patching: keep every element that is still there ----

// Two nodes are the same element if they have the same tag and id.
function same(a, b) {
  return a.nodeType === b.nodeType && a.nodeName === b.nodeName && (a.nodeType !== Node.ELEMENT_NODE || a.id === b.id);
}

// Makes `old` (on the page) match `fresh` (just built), keeping each element
// that is still there: a rebuilt element under the mouse between press and
// release loses the click, a rebuilt select closes while open, and a rebuilt
// control loses the focus.
function patch(old, fresh) {
  if (old.nodeType !== Node.ELEMENT_NODE) {
    if (old.nodeValue !== fresh.nodeValue) old.nodeValue = fresh.nodeValue;
    return;
  }
  [...old.attributes].forEach((a) => {
    if (!fresh.hasAttribute(a.name)) old.removeAttribute(a.name);
  });
  // a style goes across through the CSSOM: the page's CSP refuses style
  // attributes written as text (a meter's lengths)
  [...fresh.attributes].forEach((a) => {
    if (a.name === "style") {
      if (old.style.cssText !== fresh.style.cssText) old.style.cssText = fresh.style.cssText;
    } else if (old.getAttribute(a.name) !== a.value) old.setAttribute(a.name, a.value);
  });
  patchChildren(old, fresh);
  // the live state, after the children (a select's value needs its options)
  if (old.tagName === "INPUT" || old.tagName === "SELECT" || old.tagName === "TEXTAREA") {
    if (old.value !== fresh.value) old.value = fresh.value;
    if (old.checked !== fresh.checked) old.checked = fresh.checked;
  }
  if (old.disabled !== undefined && old.disabled !== fresh.disabled) old.disabled = fresh.disabled;
}

function patchChildren(old, fresh) {
  const want = [...fresh.childNodes];
  want.forEach((f, i) => {
    const o = old.childNodes[i];
    if (!o) old.append(f);
    else if (same(o, f)) patch(o, f);
    else old.replaceChild(f, o);
  });
  while (old.childNodes.length > want.length) old.lastChild.remove();
}

// An open dropdown: the browser closes a select's list when the page around
// it changes, so the menus wait while one is open, and catch up when it
// closes (a choice, Escape, Enter or Tab, a click anywhere, or the focus
// leaving it).
let openSelect = null;

function selectClosed() {
  if (!openSelect) return;
  openSelect = null;
  drawMenus();
}

function watchSelects() {
  document.addEventListener("pointerdown", (e) => {
    const s = e.target.closest && e.target.closest("#menus select");
    if (s && s !== openSelect) openSelect = s;
    else selectClosed();
  }, true);
  // after the change is sent (a redraw first would put back the old value)
  document.addEventListener("change", () => setTimeout(selectClosed, 0), true);
  document.addEventListener("focusout", (e) => {
    if (e.target === openSelect) selectClosed();
  }, true);
  document.addEventListener("keydown", (e) => {
    if (!openSelect) {
      if (e.target.matches && e.target.matches("#menus select") && (e.key === " " || e.key === "ArrowDown" && e.altKey)) openSelect = e.target;
    } else if (e.key === "Escape" || e.key === "Enter" || e.key === "Tab") {
      setTimeout(selectClosed, 0);
    }
  }, true);
}

function drawMenus() {
  if (openSelect) {
    if (openSelect.isConnected) return;
    openSelect = null;
  }
  const text = JSON.stringify(ui.panels || []);
  if (text === panelsText) return;
  panelsText = text;
  const menus = el("menus");
  const kept = document.activeElement;
  const focused = kept && menus.contains(kept) ? kept.id : "";
  const caret = focused && kept.selectionStart !== undefined ? kept.selectionStart : null;
  patchChildren(menus, buildMenus());
  // an element that had to be replaced gets the focus back (and a text
  // field its caret); when it is gone (a new screen), the screen's first
  // control takes the focus, so the keyboard carries on from there
  // (while the core captures the keys, no control takes the focus: Space
  // is the cast, not a click)
  if (!focused || kept.isConnected || ui.capture) return;
  const again = el(focused) || menus.querySelector("button, input, select");
  if (!again) return;
  again.focus();
  if (caret !== null && again.setSelectionRange && again.type === "text") again.setSelectionRange(caret, caret);
}

// ---- drawing the view model ----

function draw(result) {
  if (result.error) {
    el("message").textContent = result.error;
    return;
  }
  ui = result.ok;
  document.title = ui.title;
  document.body.dataset.screen = ui.screen;
  document.body.dataset.fishboy = ui.fishboy;
  // the Fish-Boy up takes the pointer: the page lets it go so it can point
  document.body.dataset.pointer = ui.pointer ? "on" : "";
  // the 2D layer draws its own cursor (the camera's viewfinder)
  document.body.dataset.cursor = ui.hide_cursor ? "none" : "";
  if (ui.pointer && document.pointerLockElement) document.exitPointerLock();
  document.documentElement.dataset.textSize = ui.text_size;
  const version = el("version");
  if (version.textContent !== ui.version) version.textContent = ui.version;
  version.disabled = !ui.version_opens;
  el("message").textContent = "";
  drawMenus();
  // the canvas has its size only while it shows
  if (ui.scene && renderer) resize();
}

// ---- input ----

// A key typed into a text control belongs to that control.
function typing(e) {
  const t = e.target;
  return t.tagName === "SELECT" || t.tagName === "TEXTAREA" || (t.tagName === "INPUT" && t.type === "text");
}

function bindInput() {
  ["click", "change", "input", "keydown"].forEach((type) => el("menus").addEventListener(type, dispatch));
  el("version").addEventListener("click", () => send("version", ""));
  // every key goes to the core, which decides what it does; while the core
  // captures input (a match), the browser does nothing with it (Tab, Space)
  window.addEventListener("keydown", (e) => {
    if (ui && ui.capture) e.preventDefault();
    if (e.repeat || typing(e)) return;
    send("key", e.code);
  });
  window.addEventListener("keyup", (e) => {
    if (typing(e)) return;
    send("key.up", e.code);
  });
  window.addEventListener("blur", () => send("keys.release", ""));
  const scene = el("scene");
  window.addEventListener("mousemove", (e) => {
    if (ui && ui.capture) send("mouse.move", e.movementX + "," + e.movementY);
  });
  scene.addEventListener("mousedown", (e) => {
    if (!ui || !ui.capture) return;
    if (document.pointerLockElement !== scene && scene.requestPointerLock) scene.requestPointerLock();
    send("mouse.down", String(e.button));
  });
  window.addEventListener("mouseup", (e) => {
    if (ui && ui.capture) send("mouse.up", String(e.button));
  });
  scene.addEventListener("contextmenu", (e) => {
    if (ui && ui.capture) e.preventDefault();
  });
  watchSelects();
  // the pointer over the 2D layer, in CSS pixels (the core turns it into
  // its own units)
  const overlay = el("overlay");
  const point = (name) => (e) => send(name, e.offsetX + " " + e.offsetY);
  overlay.addEventListener("pointermove", point("pointer.move"));
  overlay.addEventListener("pointerdown", point("pointer.down"));
  overlay.addEventListener("pointerup", point("pointer.up"));
}

// ---- the 3D scene ----

let renderer = null;
let overlay = null;
let passLayout = null;

// The canvas's size goes to the core, which says how many pixels to draw.
function resize() {
  const canvas = el("scene");
  const [w, h] = game.resize(canvas.clientWidth, canvas.clientHeight, window.devicePixelRatio);
  if (canvas.width !== w || canvas.height !== h) renderer.resize(w, h);
  if (overlay) overlay.resize(w, h);
}

// The static world the renderer holds, by the core's stamp.
let worldStamp = 0;

// Each frame: the clock to the core, and, when the core says the screen
// has a scene, its draw data to the renderer; the static world goes up
// again only when the core's stamp for it changes.
function frame(now) {
  if (renderer && ui && ui.scene) {
    game.advance(now);
    game.build_frame();
    const stamp = game.world_stamp();
    if (stamp !== worldStamp) {
      renderer.world(game.world_vertices());
      worldStamp = stamp;
    }
    renderer.frame(game.view(), game.vertices(), game.vertex_count(), game.statics(), game.groups(), game.instances(),
      game.ghost_view(), game.ghost_vertices(), game.ghost_vertex_count());
    // the picture-in-picture, when the core gives one
    const inset = game.inset_rect();
    if (inset.length) {
      renderer.inset(inset, game.inset_border(), game.inset_view(), game.inset_vertices(), game.inset_vertex_count(),
        game.inset_statics(), game.inset_groups(), game.inset_instances());
    }
    // the 2D layer: its models and the Fish-Boy through the renderer,
    // then the draw list on the overlay
    game.build_ui2d();
    const passes = game.pass_count();
    for (let k = 0; k < passes; k++) {
      renderer.pass(passLayout, game.pass_values(k), game.pass_vertices(k), game.pass_vertex_count(k));
    }
    const list = JSON.parse(game.ui2d_strings());
    overlay.draw(game.ui2d_values(), list.strings);
    document.body.dataset.ui2d = list.status;
    // a match's clock moves: its view model is asked for each frame
    if (ui.live) draw(JSON.parse(game.ui()));
    if (ui.panels.length) document.body.dataset.drawn = ui.panels[0].heading;
  }
  requestAnimationFrame(frame);
}

async function main() {
  await init();
  const strings = await (await fetch("data/strings.json")).text();
  const notes = await (await fetch("data/patch-notes.json")).text();
  const models = await (await fetch("data/models.json")).text();
  const look = await (await fetch("data/look.json")).text();
  const ui2dLook = await (await fetch("data/ui2d.json")).text();
  const fish = await (await fetch("data/fish.json")).text();
  const flow = await (await fetch("data/flow.json")).text();
  const shaders = {
    vertex: await (await fetch("data/shaders/scene.vert")).text(),
    fragment: await (await fetch("data/shaders/scene.frag")).text(),
    props: await (await fetch("data/shaders/props.vert")).text(),
    waterVertex: await (await fetch("data/shaders/water.vert")).text(),
    waterFragment: await (await fetch("data/shaders/water.frag")).text(),
    skyVertex: await (await fetch("data/shaders/sky.vert")).text(),
    skyFragment: await (await fetch("data/shaders/sky.frag")).text(),
  };
  const sounds = await (await fetch("data/sounds.json")).text();
  const tuning = await (await fetch("data/tuning.json")).text();
  game = new Game(strings, notes);
  bindInput();
  draw(JSON.parse(game.load_models(models, look)));
  send("data.ui2d", ui2dLook);
  send("data.fish", fish);
  send("data.flow", flow);
  const ui2dNumbers = JSON.parse(game.ui2d_numbers());
  passLayout = ui2dNumbers.pass;
  overlay = makeOverlay(el("overlay"), ui2dNumbers, (id) => bitmapFrom(game.bitmap_rgba(id), game.bitmap_size(id)));
  // the match's files, each checked by the core (refused by name)
  const files = [
    ["data.tuning", tuning],
    ["data.controls", await (await fetch("data/controls.json")).text()],
    ["data.names", await (await fetch("data/names.json")).text()],
    ["data.maps", await (await fetch("data/maps/michigan-lake.json")).text()],
    ["data.fish", await (await fetch("data/fish.json")).text()],
  ];
  files.forEach(([name, text]) => {
    const result = JSON.parse(game.act(name, text));
    if (result.error) el("message").textContent = result.error;
  });
  draw(JSON.parse(game.load_sounds(sounds, tuning)));
  // debug flags from the page's address (`?debug=fishing`), for checks
  const debug = new URLSearchParams(location.search).get("debug");
  if (debug) send("debug.flags", debug);
  startAudio(game);
  renderer = makeRenderer(el("scene"), JSON.parse(game.numbers()), shaders);
  if (!renderer) send("scene.unavailable", "");
  window.addEventListener("resize", () => renderer && resize());
  requestAnimationFrame(frame);
  document.body.dataset.ready = "true";
}

main();
