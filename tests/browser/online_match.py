#!/usr/bin/env python3
"""An online match between two tabs through the game page (PLAN.md decision 11).

Step 3 of scripts/check-online.sh. Two tabs, each in its own browser
context, load index.html with `?codes&ice=none`, so they connect by pasted
codes on host candidates alone: no relay, no STUN, and every off-origin
request is aborted and counted as a failure. For each engine it:

- both tabs open Play, then Online;
- A hosts by pasted code and shows an invite code; B pastes it and joins,
  and shows a reply code; A pastes that back;
- B is seated in A's Lobby: A's seats list both names and B's ping; B's
  Start is not offered, and A's Start waits for B;
- A adds an AI player and B's Lobby shows it;
- B presses Ready; A's Start comes on; A presses it;
- both tabs reach Play, and both countdowns run out;
- B holds W, and A sees B's boat leave its spawn
  (`body[data-remote-moved="true"]`, from the core); A holds W and B sees
  A's boat move;
- B's match menu does not stop the online match's time;
- fails on any off-origin request, WebSocket, CSP violation, console error
  or uncaught page error.

WebKit's tabs connect only with loopback candidates (see online.py,
LOOPBACK_ONLY): the pasted codes are rewritten for it.

    python3 tests/browser/online_match.py

`ONLINE_SHOTS=DIR` also saves each tab's Lobby (DIR/lobby-<engine>-<tab>.png),
in Chromium and Firefox: a WebKit screenshot injects a style the CSP refuses.
"""
import engines
import json
import os
import pathlib
import sys

from playwright.sync_api import sync_playwright

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import online as transport  # noqa: E402  (the helpers of the transport check)

WAIT_MS = transport.WAIT_MS
STRINGS = json.loads((pathlib.Path(__file__).resolve().parents[2] / "data" / "strings.json").read_text())
check = transport.check


def by_id(id_):
    return f'[id="{id_}"]'


class Tab:
    def __init__(self, browser, origin, name, seen):
        self.name = name
        self.context = transport.watched_context(browser, origin, seen)
        self.page = self.context.new_page()
        self.page.set_viewport_size({"width": 1920, "height": 1080})
        tag = f"[{name}] "
        self.page.on("console", lambda m: seen["console"].append(tag + m.text) if m.type == "error" else None)
        self.page.on("pageerror", lambda e: seen["errors"].append(tag + str(e)))
        self.page.on("websocket", lambda ws: seen["sockets"].append(ws.url))
        self.page.goto(origin + "/index.html?codes&ice=none")
        self.page.wait_for_selector("body[data-ready='true']", timeout=WAIT_MS)

    def click(self, id_):
        self.page.click(by_id(id_), timeout=WAIT_MS)

    def code(self, id_):
        sel = by_id(id_)
        self.page.wait_for_function(
            "sel => { const e = document.querySelector(sel); return e && e.value; }",
            arg=sel,
            timeout=WAIT_MS,
        )
        return self.page.input_value(sel)

    def fill(self, id_, text):
        self.page.fill(by_id(id_), text)
        # the field sends its text when it loses the focus
        self.page.evaluate("() => document.activeElement && document.activeElement.blur()")

    def screen(self, name):
        try:
            self.page.wait_for_selector(f"body[data-screen='{name}']", timeout=WAIT_MS)
            return True
        except Exception:
            return False

    def enabled(self, id_):
        try:
            self.page.wait_for_selector(by_id(id_) + ":not([disabled])", timeout=WAIT_MS)
            return True
        except Exception:
            return False

    def has_text(self, texts):
        try:
            self.page.wait_for_function(
                "want => { const t = document.getElementById('menus').innerText;"
                " return want.every(w => t.includes(w)); }",
                arg=texts,
                timeout=WAIT_MS,
            )
            return True
        except Exception:
            return False

    def ui2d(self, key):
        """A word of the 2D layer's status (body[data-ui2d]), `key=value`."""
        for word in (self.page.get_attribute("body", "data-ui2d") or "").split(" "):
            if word.startswith(key + "="):
                return word[len(key) + 1:]
        return None

    def ui2d_has(self, word):
        try:
            self.page.wait_for_function(
                "w => (document.body.dataset.ui2d || '').split(' ').includes(w)", arg=word, timeout=WAIT_MS
            )
            return True
        except Exception:
            return False

    def csp(self):
        return self.page.evaluate("() => window.__builderCsp || []")


def run_engine(p, engine, origin):
    transport.report.note(f"\n[{engine}] {origin}/index.html?codes&ice=none")
    seen = {"foreign": [], "console": [], "errors": [], "sockets": []}
    browser, _ = engines.launch(p, engine)
    paste = transport.loopback if engine in transport.LOOPBACK_ONLY else (lambda c: c)
    a = Tab(browser, origin, "A", seen)
    b = Tab(browser, origin, "B", seen)
    tabs = [a, b]

    def finish():
        csp = [x for t in tabs for x in t.csp()]
        check(not csp, f"no Content-Security-Policy violation {csp}")
        check(not seen["foreign"], f"no off-origin request at any point {seen['foreign']}")
        check(not seen["sockets"], f"no WebSocket {seen['sockets']}")
        check(not seen["errors"], f"no uncaught page error {seen['errors']}")
        check(not seen["console"], f"no console error {seen['console']}")
        browser.close()

    for t in tabs:
        t.click("menu.play")
    check(a.screen("play_menu") and b.screen("play_menu"), "Play offers Offline and Online")
    for t in tabs:
        t.click("menu.online")
    check(a.screen("online") and b.screen("online"), "both tabs open the Online screen")
    a.fill("online.name", "Hana Lund")
    b.fill("online.name", "Bea Marsh")

    a.click("online.host_code")
    check(a.screen("online_lobby"), "A hosts and is in the Lobby")
    invite = a.code("online.invite")
    check(invite.startswith("O"), "A shows an invite code")
    b.fill("online.invite", paste(invite))
    b.click("online.join_code")
    check(b.screen("online_lobby"), "B joins and is in the Lobby")
    reply = b.code("online.reply")
    check(reply.startswith("A"), "B shows a reply code")
    a.fill("online.reply", paste(reply))
    a.click("online.connect")

    ok = b.enabled("online.ready")
    check(ok, "B is seated in A's Lobby")
    if not ok:
        return finish()
    check(a.has_text(["Hana Lund", "Bea Marsh"]), "A's seats list both players")
    check(b.has_text(["Hana Lund", "Bea Marsh"]), "B's seats list both players")
    check(b.page.locator(by_id("online.start")).count() == 0, "only the host is offered Start match")
    check(a.page.is_disabled(by_id("online.start")), "A's Start match waits for B's Ready")
    try:
        a.page.wait_for_function(
            "() => [...document.querySelectorAll('table.cells td')].some(td => / ms$/.test(td.textContent))",
            timeout=WAIT_MS,
        )
        pinged = True
    except Exception:
        pinged = False
    check(pinged, "A's Lobby shows B's ping")
    a.page.select_option(by_id("lobby.ai"), "1")
    ai = STRINGS["lobby.ai_player"].split("{")[0]
    check(b.has_text([ai]), "A's AI player shows in B's Lobby")
    # not in WebKit: its screenshot injects a style the page's CSP refuses
    if os.environ.get("ONLINE_SHOTS") and engine != "webkit":
        out = pathlib.Path(os.environ["ONLINE_SHOTS"])
        out.mkdir(parents=True, exist_ok=True)
        for t in tabs:
            t.page.screenshot(path=str(out / f"lobby-{engine}-{t.name}.png"))
    b.click("online.ready")
    ok = a.enabled("online.start")
    check(ok, "A's Start match comes on once B is Ready")
    if not ok:
        return finish()
    a.click("online.start")
    ok = a.screen("play") and b.screen("play")
    check(ok, "both tabs reach Play")
    if not ok:
        return finish()
    # the 3-2-1-Go on both (the HUD's status: countdown=N, then countdown=)
    check(a.ui2d_has("countdown=") and b.ui2d_has("countdown="), "both countdowns run out")

    for t, driver, other in ((a, b, "B's boat"), (b, a, "A's boat")):
        driver.page.keyboard.down("KeyW")
        try:
            t.page.wait_for_selector("body[data-remote-moved='true']", timeout=WAIT_MS)
            moved = True
        except Exception:
            moved = False
        check(moved, f"{t.name} sees {other} move")
        driver.page.keyboard.up("KeyW")

    # the match menu over B: the online match's time runs on
    b.page.keyboard.press("Escape")
    b.page.wait_for_selector(".panel.pause", timeout=WAIT_MS)
    before = b.ui2d("time")
    b.page.wait_for_timeout(1500)
    after = b.ui2d("time")
    check(before != after, f"the online match runs on behind B's menu ({before} -> {after})")
    finish()


def main():
    if not (transport.DIST / "index.html").exists():
        sys.exit("dist/index.html is missing: run scripts/build.sh first")
    server = transport.serve()
    host, port = server.server_address[:2]
    origin = f"http://{host}:{port}"
    with sync_playwright() as p:
        transport.report = transport.progress.Report(transport.ENGINES, ["match"])
        try:
            for i, engine in enumerate(transport.ENGINES, 1):
                transport.report.engine_start(i, engine)
                transport.report.step_start(1, "a match between two tabs")
                run_engine(p, engine, origin)
                transport.report.step_end()
                transport.report.engine_end()
        finally:
            transport.report.close()
    server.shutdown()
    if transport.failures:
        print(f"\nONLINE MATCH CHECK FAILED: {len(transport.failures)} problem(s)")
        for f in transport.failures:
            print("  - " + f)
        sys.exit(1)
    print(f"\nONLINE MATCH CHECK PASSED in {', '.join(transport.ENGINES)}")


if __name__ == "__main__":
    main()
