// The page draws what the core returns and decides nothing.
//
// The core describes the screen (its name, panels, actions and the
// glossary's definitions), and this file builds the DOM from that
// description without knowing what any panel means. Every button sends its
// action name and value back through `act`, the core's one door for
// settings, screens and flags. (After Peter's Mandible.)
import init, { start, act } from "./pkg/tool_shim.js";

const el = (id) => document.getElementById(id);
const make = (tag, text) => {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  return node;
};

let state = null;
let glossary = "";
let hints = [];

function call(fn, ...args) {
  return JSON.parse(fn(...args));
}

// A label whose text is a glossary term shows the definition on hover.
function mark(node) {
  const hint = hints.find((h) => h.term === node.textContent);
  if (hint) {
    node.dataset.term = hint.term;
    node.title = hint.definition;
  }
}

function panel(p) {
  const box = make("section");
  box.id = p.id;
  if (p.heading) box.append(make("h2", p.heading));
  if (p.text) box.append(make("p", p.text));
  (p.actions || []).forEach((a) => {
    const button = make("button", a.label);
    button.disabled = !a.enabled;
    button.addEventListener("click", () => draw(call(act, JSON.stringify(state), glossary, a.name, a.value)));
    box.append(button);
  });
  return box;
}

function draw(result) {
  if (result.error) {
    el("message").textContent = result.error;
    return;
  }
  const ui = result.ok;
  state = ui.state;
  hints = ui.hints;
  document.title = ui.title;
  el("title").textContent = ui.title;
  el("panels").replaceChildren(...ui.panels.map(panel));
  document.querySelectorAll("#panels h2, #panels button").forEach(mark);
  el("message").textContent = "";
  document.body.dataset.screen = ui.screen;
}

async function main() {
  await init();
  const strings = await (await fetch("data/strings.json")).text();
  glossary = await (await fetch("data/glossary.md")).text();
  draw(call(start, strings, glossary));
  document.body.dataset.ready = "true";
}

main();
