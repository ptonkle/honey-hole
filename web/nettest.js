// The transport test page: web/online.js wired to a log and an echo. Driven
// by tests/browser/online.py, and by hand for the room-code path.
//
// Echo: a message "ping X" is answered "pong X" to its sender only.
import * as online from "./online.js";

const el = (id) => document.getElementById(id);
let hosting = false;

function onEvent(ev) {
  const li = document.createElement("li");
  li.dataset.kind = ev.kind;
  li.dataset.peer = ev.peer || "";
  li.dataset.text = ev.text || "";
  li.dataset.host = ev.host ? "true" : "false";
  li.textContent = [ev.kind, ev.peer, ev.text, ev.host ? "(host)" : ""].join(" ");
  el("log").append(li);
  if (ev.kind === "message" && ev.text.startsWith("ping ")) {
    online.send(ev.peer, "pong " + ev.text.slice("ping ".length));
  }
}

function status(text) {
  el("status").textContent = text;
}

async function run(fn) {
  try {
    await fn();
    el("me").textContent = online.myId() || "";
  } catch (e) {
    status(String(e && e.message ? e.message : e));
  }
}

el("host-code").addEventListener("click", () =>
  run(async () => {
    hosting = true;
    el("my-code").value = "";
    el("my-code").value = await online.hostByCode(onEvent);
  }),
);

el("invite").addEventListener("click", () =>
  run(async () => {
    el("my-code").value = "";
    el("my-code").value = await online.inviteByCode();
  }),
);

el("use-code").addEventListener("click", () =>
  run(async () => {
    const code = el("their-code").value;
    if (hosting) {
      await online.acceptAnswer(code);
      status("answer accepted");
    } else {
      el("my-code").value = "";
      el("my-code").value = await online.joinByCode(code, onEvent);
    }
  }),
);

el("host-room").addEventListener("click", () =>
  run(async () => {
    hosting = true;
    await online.hostRoom(el("room").value, onEvent);
  }),
);

el("join-room").addEventListener("click", () =>
  run(async () => {
    hosting = false;
    await online.joinRoom(el("room").value, onEvent);
  }),
);

el("send").addEventListener("click", () => {
  const to = el("to").value;
  if (to) online.send(to, el("text").value);
  else online.broadcast(el("text").value);
});

el("leave").addEventListener("click", () => {
  online.leave();
  status("left");
});

document.body.dataset.ready = "true";
