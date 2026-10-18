#!/usr/bin/env python3
"""Browser check: the built page in Chromium, Firefox and WebKit.

For each engine it serves dist/ on 127.0.0.1, opens index.html, waits for the
page to set <body data-ready="true">, lets it idle, and then runs the project's
own interactions from tests/browser/interactions.py. The run fails on:

- an uncaught page error;
- a console error;
- any request to another origin, during load, idle or the interactions;
- a Content-Security-Policy violation (a blocked attempt is still an attempt);
- a request for a file the site does not have (the server answered 404), even
  one the browser makes by itself, such as an icon;
- a WebSocket, a popup, a worker or a service worker opened by the page;
- a failed check inside the interactions.

What a pass establishes is what happened on that path, in those engines, on
that run. It does not show that no feature can ever reach the network: a
feature the interactions never start is a feature this check never saw.

    python3 tests/browser/check.py

In a terminal it draws one progress bar per engine (tests/browser/progress.py,
with `rich` installed); in a log, the plain PASS/FAIL lines.

Environment:
    BUILDER_ENGINES   space-separated engines (default "chromium firefox webkit");
                      add "webkit-iphone" for WebKit as an iPhone 15 with touch.
                      CI may run "chromium" alone (D7); run all three locally
                      before a push.
    BUILDER_IDLE      seconds to idle after load (default 10)
    BUILDER_STEPS     space-separated interaction steps to run after `boot`
                      (default: every step; scripts/quick-check.sh sets it)
    BUILDER_HEADLESS  1: Chromium headless even where it could draw on the
                      GPU in an invisible Xvfb window (tests/browser/engines.py)
    BUILDER_FAST      0: the players' timing, not the core's fast mode (`?fast`:
                      no countdown, "Lines up", reveal or Fish-Boy raise to wait
                      through; rule timings are never shortened)
    BUILDER_MUTE      0: let the page's sounds be heard (`?mute` by default)
    BUILDER_ORIGIN    check a deployed origin instead of serving dist/, e.g.
                      https://example.github.io/my-tool  (no trailing slash)
    CHECK_PLAIN       1: the plain lines even in a terminal
"""
import functools
import http.server
import importlib.util
import os
import pathlib
import sys
import threading

from playwright.sync_api import sync_playwright

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import engines  # noqa: E402  (tests/browser/engines.py)
import progress  # noqa: E402  (tests/browser/progress.py)

ROOT = pathlib.Path(__file__).resolve().parents[2]
DIST = ROOT / "dist"
ENGINES = os.environ.get("BUILDER_ENGINES", "chromium firefox webkit").split()
IDLE_MS = int(float(os.environ.get("BUILDER_IDLE", "10")) * 1000)

failures = []
report = None  # tests/browser/progress.py: the bars or the plain lines
# what the local server could not serve: "<status> <path>" (SECOND-ORDER R6.10)
missing = []


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

    def send_error(self, code, message=None, explain=None):
        missing.append(f"{code} {self.path}")
        super().send_error(code, message, explain)


def serve():
    handler = functools.partial(Handler, directory=str(DIST))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def load_interactions():
    path = pathlib.Path(__file__).with_name("interactions.py")
    spec = importlib.util.spec_from_file_location("interactions", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def run_engine(p, engine, base, interactions):
    report.note(f"\n[{engine}] {base}/")
    report.step_start(1, "load", f"load and {IDLE_MS // 1000} s idle")
    scheme_host = base.split("/", 3)
    origin = "/".join(scheme_host[:3])
    seen = {"foreign": [], "console": [], "errors": [], "sockets": [], "popups": [], "workers": []}

    def same_origin(url):
        return url.startswith(origin + "/") or url.startswith("blob:" + origin) or url.startswith("data:")

    browser, profile = engines.launch(p, engine)
    context = browser.new_context(accept_downloads=True, **profile)
    context.on("request", lambda r: None if same_origin(r.url) else seen["foreign"].append(r.url))

    def route(r):
        if same_origin(r.request.url):
            r.continue_()
        else:
            r.abort()

    context.route("**/*", route)
    # A request the page's CSP blocks never becomes a request event, so the
    # violation itself is recorded and reported.
    context.add_init_script(
        "window.__builderCsp = [];"
        "document.addEventListener('securitypolicyviolation',"
        " e => window.__builderCsp.push(e.violatedDirective + ' ' + e.blockedURI));"
    )
    page = context.new_page()
    missing.clear()
    page.on("console", lambda m: seen["console"].append(m.text) if m.type == "error" else None)
    page.on("response", lambda r: missing.append(f"{r.status} {r.url}") if r.status >= 400 else None)
    page.on("pageerror", lambda e: seen["errors"].append(str(e)))
    page.on("websocket", lambda ws: seen["sockets"].append(ws.url))
    page.on("worker", lambda w: seen["workers"].append(w.url))
    page.on("popup", lambda pg: seen["popups"].append(pg.url))

    # with the test flags (`?fast&mute`; BUILDER_FAST=0, BUILDER_MUTE=0 turn them off)
    page.goto(interactions.address(base + "/index.html"))
    page.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
    check(True, "page reported ready")
    report.note(f"  WebGL: {engines.renderer(page)}")
    page.wait_for_timeout(IDLE_MS)
    check(not seen["foreign"], f"no off-origin request during load and {IDLE_MS // 1000}s idle {seen['foreign']}")
    report.step_end()

    interactions.run(page, check, engine, report, first=2)

    report.step_start(len(report.steps), "final checks")
    csp = page.evaluate("() => window.__builderCsp || []")
    check(not csp, f"no Content-Security-Policy violation {csp}")
    sw = page.evaluate("() => navigator.serviceWorker ? navigator.serviceWorker.getRegistrations().then(r => r.length) : 0")
    check(not seen["errors"], f"no uncaught page error {seen['errors']}")
    check(not seen["console"], f"no console error {seen['console']}")
    check(not missing, f"no request for a missing file {sorted(set(missing))}")
    check(not seen["foreign"], f"no off-origin request at any point {seen['foreign']}")
    check(not seen["sockets"], f"no WebSocket {seen['sockets']}")
    check(not seen["popups"], f"no popup {seen['popups']}")
    check(not seen["workers"] and sw == 0, f"no worker or service worker {seen['workers']} sw={sw}")
    report.step_end()
    browser.close()


def main():
    origin = os.environ.get("BUILDER_ORIGIN", "").rstrip("/")
    server = None
    if not origin:
        if not (DIST / "index.html").exists():
            sys.exit("dist/ is missing: run scripts/build.sh first")
        server = serve()
        host, port = server.server_address[:2]
        origin = f"http://{host}:{port}"
    interactions = load_interactions()
    global report
    steps = ["load"] + [n for n, _ in interactions.chosen_steps()] + ["final checks"]
    report = progress.Report(ENGINES, steps)
    try:
        with sync_playwright() as p:
            for i, engine in enumerate(ENGINES, 1):
                report.engine_start(i, engine)
                run_engine(p, engine, origin, interactions)
                report.engine_end()
    finally:
        report.close()
    if server:
        server.shutdown()
    if failures:
        print(f"\nBROWSER CHECK FAILED: {len(failures)} problem(s)")
        for f in failures:
            print("  - " + f)
        sys.exit(1)
    print(f"\nBROWSER CHECK PASSED in {', '.join(ENGINES)}")


if __name__ == "__main__":
    main()
