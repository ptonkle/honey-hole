#!/usr/bin/env python3
"""Transport check: web/online.js between three browser tabs, by pasted code.

Not part of scripts/check.sh, which stays strictly same-origin; run it with
scripts/check-online.sh. It never contacts a relay or a STUN server: the tabs
load web/nettest.html with `?ice=none`, so they connect on host candidates
alone, and every off-origin request is aborted and counted as a failure.

For each engine it:
- opens three tabs, each in its own browser context (its own "browser");
- connects A (host) and B by pasted codes, then A and C, after which C and B
  meet through A (the mesh is completed over the host's data channels);
- checks that every tab sees both others connect, and that each sees A as host;
- sends a burst of numbered messages from every tab, both ways on every pair,
  and checks that each arrives exactly once and in order;
- checks the echo (ping to one peer, pong back from it only);
- closes C and checks that A and B both report it disconnected;
- fails on any off-origin request, WebSocket, CSP violation, console error or
  uncaught page error.

WebKit (Playwright's Linux build) is run with two tabs and loopback
candidates: see LOOPBACK_ONLY.

    python3 tests/browser/online.py

Environment:
    BUILDER_ENGINES   space-separated engines (default "chromium firefox webkit")
"""
import engines
import base64
import functools
import http.server
import os
import pathlib
import re
import sys
import threading
import zlib

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402  (tests/browser/progress.py)

from playwright.sync_api import sync_playwright

ROOT = pathlib.Path(__file__).resolve().parents[2]
DIST = ROOT / "dist"
ENGINES = os.environ.get("BUILDER_ENGINES", "chromium firefox webkit").split()
BURST = 30
WAIT_MS = 20000
# Engines whose tabs cannot resolve each other's mDNS (.local) host
# candidates: Playwright's WebKit on Linux stays at ICE state "new" between
# two pages, in one context or two (measured 2026-10-10), and connects once
# the candidates name 127.0.0.1. The test rewrites the pasted codes for these
# engines, and runs two tabs only, because the third tab's link to B is
# signalled over the data channels, where the test cannot rewrite it.
LOOPBACK_ONLY = {"webkit"}


failures = []


report = None  # tests/browser/progress.py: the bars or the plain lines


def check(ok, what):
    report.check(ok, what)
    if not ok:
        failures.append(what)


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "text/javascript",
        ".json": "application/json",
    }

    def log_message(self, *args):
        pass


def serve():
    handler = functools.partial(Handler, directory=str(DIST))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def watched_context(browser, origin, seen):
    """A browser context whose every off-origin request is aborted and counted."""
    context = browser.new_context()

    def same_origin(url):
        return url.startswith(origin + "/") or url.startswith("blob:" + origin) or url.startswith("data:")

    def route(r):
        if same_origin(r.request.url):
            r.continue_()
        else:
            r.abort()

    context.on("request", lambda r: None if same_origin(r.url) else seen["foreign"].append(r.url))
    context.route("**/*", route)
    context.add_init_script(
        "window.__builderCsp = [];"
        "document.addEventListener('securitypolicyviolation',"
        " e => window.__builderCsp.push(e.violatedDirective + ' ' + e.blockedURI));"
    )
    return context


class Tab:
    def __init__(self, context, origin, name, seen):
        self.name = name
        self.context = context
        self.page = self.context.new_page()
        tag = f"[{name}] "
        self.page.on("console", lambda m: seen["console"].append(tag + m.text) if m.type == "error" else None)
        self.page.on("pageerror", lambda e: seen["errors"].append(tag + str(e)))
        self.page.on("websocket", lambda ws: seen["sockets"].append(ws.url))
        self.page.goto(origin + "/nettest.html?ice=none")
        self.page.wait_for_selector("body[data-ready='true']", timeout=WAIT_MS)

    def code(self, previous=""):
        self.page.wait_for_function(
            "prev => { const v = document.getElementById('my-code').value; return v && v !== prev; }",
            arg=previous,
            timeout=WAIT_MS,
        )
        return self.page.input_value("#my-code")

    def use(self, code):
        self.page.fill("#their-code", code)
        self.page.click("#use-code")

    def me(self):
        self.page.wait_for_function("() => document.getElementById('me').textContent", timeout=WAIT_MS)
        return self.page.text_content("#me")

    def events(self, kind):
        return self.page.evaluate(
            "kind => [...document.querySelectorAll('#log li')]"
            ".filter(li => li.dataset.kind === kind)"
            ".map(li => ({peer: li.dataset.peer, text: li.dataset.text, host: li.dataset.host}))",
            kind,
        )

    def wait_events(self, kind, count):
        try:
            self.page.wait_for_function(
                "([kind, n]) => document.querySelectorAll(`#log li[data-kind='${kind}']`).length >= n",
                arg=[kind, count],
                timeout=WAIT_MS,
            )
            return True
        except Exception:
            return False

    def send(self, text, to=""):
        self.page.fill("#to", to)
        self.page.fill("#text", text)
        self.page.click("#send")

    def status(self):
        return self.page.text_content("#status")

    def csp(self):
        return self.page.evaluate("() => window.__builderCsp || []")


def loopback(code):
    """The same pasted code with mDNS host candidates (`<uuid>.local`) named
    127.0.0.1, as Trystero's own tests do (`mdnsHostFallbackToLoopback`).
    Used only for engines in LOOPBACK_ONLY; the code format is web/online.js's:
    one letter, then the session description, deflate-raw, base64url."""
    body = code[1:]
    raw = zlib.decompress(base64.urlsafe_b64decode(body + "=" * (-len(body) % 4)), -15)
    sdp = re.sub(rb"[0-9a-fA-F-]+\.local", b"127.0.0.1", raw)
    z = zlib.compressobj(wbits=-15)
    packed = z.compress(sdp) + z.flush()
    return code[0] + base64.urlsafe_b64encode(packed).decode().rstrip("=")


def run_engine(p, engine, origin):
    report.note(f"\n[{engine}] {origin}/nettest.html?ice=none")
    seen = {"foreign": [], "console": [], "errors": [], "sockets": []}
    browser, _ = engines.launch(p, engine)
    # Each tab is its own browser context: three separate "browsers".
    names = "AB" if engine in LOOPBACK_ONLY else "ABC"
    tabs = [Tab(watched_context(browser, origin, seen), origin, n, seen) for n in names]
    a, b = tabs[0], tabs[1]
    c = tabs[2] if len(tabs) > 2 else None
    paste = loopback if engine in LOOPBACK_ONLY else (lambda code: code)
    if engine in LOOPBACK_ONLY:
        report.note("  NOTE two tabs only, mDNS candidates pasted as 127.0.0.1 (see LOOPBACK_ONLY)")

    def finish():
        csp = [x for t in tabs if not t.page.is_closed() for x in t.csp()]
        check(not csp, f"no Content-Security-Policy violation {csp}")
        check(not seen["foreign"], f"no off-origin request at any point {seen['foreign']}")
        check(not seen["sockets"], f"no WebSocket {seen['sockets']}")
        check(not seen["errors"], f"no uncaught page error {seen['errors']}")
        check(not seen["console"], f"no console error {seen['console']}")
        browser.close()

    # A hosts; B joins by pasted code.
    a.page.click("#host-code")
    offer1 = a.code()
    check(offer1.startswith("O"), "the host's code is an offer")
    b.use(paste(offer1))
    answer1 = b.code()
    check(answer1.startswith("A"), "the joiner's code is an answer")
    a.use(paste(answer1))
    ok = a.wait_events("connected", 1) and b.wait_events("connected", 1)
    check(ok, f"A and B connect by pasted code (status A={a.status()!r} B={b.status()!r})")
    if not ok:
        return finish()

    # A code pasted into the wrong box is refused by name.
    last = tabs[-1]
    if last is b:
        last = Tab(watched_context(browser, origin, seen), origin, "X", seen)
        tabs.append(last)
    last.page.fill("#their-code", answer1)
    last.page.click("#use-code")
    last.page.wait_for_function("() => document.getElementById('status').textContent", timeout=WAIT_MS)
    check(last.status() == "online.wrong_code", f"an answer pasted as an offer is refused ({last.status()!r})")
    if last.name == "X":
        tabs.pop()
        last.context.close()

    if c:
        # A invites C; C then meets B through A.
        a.page.click("#invite")
        offer2 = a.code(answer1)
        c.use(paste(offer2))
        answer2 = c.code()
        a.use(paste(answer2))
        ok = all(t.wait_events("connected", 2) for t in tabs)
        check(ok, "all three tabs see both others (C met B through the host)")
        if not ok:
            return finish()

    ids = {t.name: t.me() for t in tabs}
    check(len(set(ids.values())) == len(tabs), f"distinct peer ids {ids}")
    for t in tabs:
        got = {e["peer"]: e["host"] for e in t.events("connected")}
        want = {ids[o.name]: ("true" if o is a else "false") for o in tabs if o is not t}
        check(got == want, f"{t.name} sees the others with the right host flag {got}")

    # A burst from every tab, both ways on every pair.
    others = len(tabs) - 1
    for i in range(BURST):
        for t in tabs:
            t.send(f"m {t.name} {i}")
    for t in tabs:
        t.wait_events("message", others * BURST)
    for t in tabs:
        msgs = t.events("message")
        for o in tabs:
            if o is t:
                continue
            got = [e["text"] for e in msgs if e["peer"] == ids[o.name]]
            want = [f"m {o.name} {i}" for i in range(BURST)]
            check(got == want, f"{t.name} got {o.name}'s {BURST} messages once each, in order")

    # The echo: to one peer only, and back from it only.
    target = c if c else a
    b.send("ping 7", to=ids[target.name])
    ok = b.wait_events("message", others * BURST + 1)
    last = b.events("message")[-1] if ok else {}
    want = {"peer": ids[target.name], "text": "pong 7", "host": "false"}
    check(last == want, f"B's ping to {target.name} came back from it {last}")
    if c:
        check(not [e for e in a.events("message") if e["text"].startswith("p")], "A saw neither the ping nor the pong")

    # The last joiner's tab closes: everyone else hears it.
    leaver = tabs[-1]
    stay = tabs[:-1]
    leaver.page.close()
    ok = all(t.wait_events("disconnected", 1) for t in stay)
    gone = [e["peer"] for t in stay for e in t.events("disconnected")]
    check(ok and gone == [ids[leaver.name]] * len(stay), f"{leaver.name}'s leaving is reported by every other tab {gone}")

    if c:
        # A and B still talk after C has gone.
        a.send("after")
        ok = b.wait_events("message", others * BURST + 2)
        check(ok and b.events("message")[-1]["text"] == "after", "A and B still connected after C left")

    finish()


def main():
    global report
    if not (DIST / "nettest.html").exists():
        sys.exit("dist/nettest.html is missing: run scripts/build.sh first")
    server = serve()
    host, port = server.server_address[:2]
    origin = f"http://{host}:{port}"
    with sync_playwright() as p:
        report = progress.Report(ENGINES, ["transport"])
        try:
            for i, engine in enumerate(ENGINES, 1):
                report.engine_start(i, engine)
                report.step_start(1, "transport between tabs")
                run_engine(p, engine, origin)
                report.step_end()
                report.engine_end()
        finally:
            report.close()
    server.shutdown()
    if failures:
        print(f"\nONLINE CHECK FAILED: {len(failures)} problem(s)")
        for f in failures:
            print("  - " + f)
        sys.exit(1)
    print(f"\nONLINE CHECK PASSED in {', '.join(ENGINES)}")


if __name__ == "__main__":
    main()
