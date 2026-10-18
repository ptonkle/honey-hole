// Online play's page side: carries text between the core's online session
// and the transport (web/online.js), and does what the core asks of the
// transport. It decides nothing: the core says what to send, to whom, and
// when to host, join or leave.
//
// online.js, and the vendored bundle behind it, load only when the core
// first asks for the transport (a player pressed Host or Join), so the
// default load-and-idle path stays on this origin (R7).
import { ONLINE } from "./online-config.js";

let game = null;
let redraw = null;
let online = null;
let clock = null;

// The worker's tick, started with online play (the default load-and-idle
// path starts no worker: tests/browser/check.py), for a host or joiner
// whose tab goes to the background.
function startClock() {
  if (clock) return;
  try {
    clock = new Worker("./net-clock.js");
    clock.onmessage = pump;
    clock.postMessage(ONLINE.pumpMs);
  } catch (e) {
    clock = setInterval(pump, ONLINE.pumpMs);
  }
}

function onNet(e) {
  game.net_event(e.kind, e.peer || "", e.text || "", !!e.host);
}

async function request(req) {
  startClock();
  try {
    if (!online) online = await import("./online.js");
    if (req.op === "host_room") await online.hostRoom(req.room, onNet);
    else if (req.op === "join_room") await online.joinRoom(req.room, onNet);
    else if (req.op === "host_code") game.net_code("invite", await online.hostByCode(onNet));
    else if (req.op === "invite_code") game.net_code("invite", await online.inviteByCode());
    else if (req.op === "join_code") game.net_code("reply", await online.joinByCode(req.invite, onNet));
    else if (req.op === "accept_reply") await online.acceptAnswer(req.reply);
    else if (req.op === "leave") online.leave();
  } catch (err) {
    game.net_event("error", "", (err && err.message) || "online.signal_failed", false);
  }
}

function pump() {
  const result = JSON.parse(game.net_pump(performance.now()));
  if (!result.ok) return;
  const p = result.ok;
  // the text first: a parting word goes out before the links close
  if (online) {
    p.out.forEach((m) => {
      if (m.to === null) online.broadcast(m.text);
      else online.send(m.to, m.text);
    });
  }
  p.requests.forEach(request);
  if (p.remote_moved !== undefined) document.body.dataset.remoteMoved = String(p.remote_moved);
  if (p.redraw && !document.hidden) redraw();
}

function loop() {
  pump();
  requestAnimationFrame(loop);
}

/** Starts pumping every animation frame; online play adds a worker's tick, which a tab in the background does not slow. */
export function startNet(g, draw) {
  game = g;
  redraw = () => draw(JSON.parse(game.ui()));
  requestAnimationFrame(loop);
}
