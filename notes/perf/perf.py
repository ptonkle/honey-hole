#!/usr/bin/env python3
"""Frame-time probe for Honey Hole in the real page.

    perf.py <repo> <engine> <scenario> <seconds> [out.json]

engine: chromium (GPU through engines.py's Xvfb path) or firefox (software).
scenario: watch  -- ?debug=watch: a Hard AI plays this seat, 3 Hard AI rivals
          fishboy -- 3 Hard AI rivals; this seat drives, then holds the Fish-Boy up

It records each animation frame's time, the time inside each WebAssembly
call (the shim's methods wrapped as served), and the draw calls and
triangles (WebGL wrapped), plus the HUD status each half second to split
the run into driving, fishing, underwater and Fish-Boy stretches. Summarise
with analyse.py; trace.py reads a Chromium trace (TRACE=<seconds in>).
Experiments: NOAA=1 (no MSAA), NOPDB=1, DPR=<ratio>, PATCH=<json of
[file, find, replace]> applied to served files. frameprof.rs is the native
counterpart (copy it into crates/core/examples/ to run it). MEASUREMENTS.md,
"A full match in the browser", has the results; SECOND-ORDER R6.1.
"""
import functools
import http.server
import json
import os
import pathlib
import sys
import threading
import time

from playwright.sync_api import sync_playwright

repo = pathlib.Path(sys.argv[1]).resolve()
engine = sys.argv[2]
scenario = sys.argv[3]
seconds = float(sys.argv[4])
out = sys.argv[5] if len(sys.argv) > 5 else None
sys.path.insert(0, str(repo / "tests/browser"))
import engines  # noqa

DIST = repo / "dist"
SERVER_404 = []


class H(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map,
                      ".wasm": "application/wasm", ".js": "text/javascript", ".json": "application/json"}

    def log_message(self, fmt, *a):
        line = fmt % a
        if " 404 " in line or "404" in str(a[1:2]):
            SERVER_404.append(line)


srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(H, directory=str(DIST)))
threading.Thread(target=srv.serve_forever, daemon=True).start()
base = f"http://127.0.0.1:{srv.server_address[1]}/"

INIT = r"""
(() => {
  const raf = window.requestAnimationFrame.bind(window);
  window.__acc = {};
  window.__cbs = [];
  window.__gl = { calls: 0, tris: 0 };
  window.requestAnimationFrame = (cb) => raf((ts) => {
    const acc0 = Object.assign({}, window.__acc);
    const g0 = window.__gl.calls, t0g = window.__gl.tris;
    const t0 = performance.now();
    cb(ts);
    const t1 = performance.now();
    const w = {};
    for (const k in window.__acc) { const d = window.__acc[k] - (acc0[k] || 0); if (d > 0) w[k] = d; }
    window.__cbs.push([ts, t1 - t0, w, window.__gl.calls - g0, window.__gl.tris - t0g, t0]);
  });
  const P = WebGL2RenderingContext.prototype;
  const da = P.drawArrays, dai = P.drawArraysInstanced;
  P.drawArrays = function (m, f, c) { window.__gl.calls++; window.__gl.tris += c / 3; return da.call(this, m, f, c); };
  P.drawArraysInstanced = function (m, f, c, n) { window.__gl.calls++; window.__gl.tris += c * n / 3; return dai.call(this, m, f, c, n); };
  if (window.__NOAA) {
    const gc2 = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (k, o) { if (k === 'webgl2' && o) o = Object.assign({}, o, { antialias: false }); return gc2.call(this, k, o); };
  }
  if (window.__DPR) Object.defineProperty(window, 'devicePixelRatio', { get: () => window.__DPR });
  if (window.__NOPDB) {
    const gc = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (k, o) { if (k === 'webgl2' && o) o = Object.assign({}, o, { preserveDrawingBuffer: false }); return gc.call(this, k, o); };
  }
  window.__long = [];
  try { new PerformanceObserver((l) => l.getEntries().forEach((e) => window.__long.push([e.startTime, e.duration]))).observe({ entryTypes: ['longtask'] }); } catch (e) {}
})();
"""

WRAP = r"""
;{
  const proto = Game.prototype;
  for (const n of Object.getOwnPropertyNames(proto)) {
    const d = Object.getOwnPropertyDescriptor(proto, n);
    if (n === 'constructor' || typeof d.value !== 'function') continue;
    const f = d.value;
    proto[n] = function (...a) { const t = performance.now(); const r = f.apply(this, a); window.__acc[n] = (window.__acc[n] || 0) + performance.now() - t; if (r && r.byteLength) window.__acc['B:' + n] = (window.__acc['B:' + n] || 0) + r.byteLength / 1e6; else if (typeof r === 'string') window.__acc['S:' + n] = (window.__acc['S:' + n] || 0) + r.length / 1e6; return r; };
  }
}
"""


def route_shim(route):
    resp = route.fetch()
    body = resp.text() + WRAP
    route.fulfill(response=resp, body=body)


with sync_playwright() as p:
    browser, profile = engines.launch(p, engine)
    ctx = browser.new_context(viewport={"width": 1920, "height": 1080}, **profile)
    page = ctx.new_page()
    errors = []
    page.on("console", lambda m: errors.append(f"{m.type}: {m.text}") if m.type in ("error", "warning") else None)
    page.on("response", lambda r: errors.append(f"HTTP {r.status} {r.url}") if r.status >= 400 else None)
    page.on("requestfailed", lambda r: errors.append(f"FAILED {r.url}"))
    if os.environ.get("NOAA") == "1":
        page.add_init_script("window.__NOAA = true;")
    if os.environ.get("DPR"):
        page.add_init_script(f"window.__DPR = {os.environ['DPR']};")
    if os.environ.get("NOPDB") == "1":
        page.add_init_script("window.__NOPDB = true;")
    page.add_init_script(INIT)
    page.route("**/pkg/tool_shim.js*", route_shim)
    if os.environ.get("PATCH"):
        pairs = json.loads(pathlib.Path(os.environ["PATCH"]).read_text())
        for fname in sorted({x[0] for x in pairs}):
            mine = [(a, b) for f, a, b in pairs if f == fname]

            def make(mine):
                def route_app(route):
                    resp = route.fetch()
                    body = resp.text()
                    for a, b in mine:
                        assert a in body, a
                        body = body.replace(a, b)
                    route.fulfill(response=resp, body=body)
                return route_app
            page.route(f"**/{fname}*", make(mine))
    # `?mute` (crates/core/src/app/fast.rs): no sound out while it runs
    # (BUILDER_MUTE=0 to hear it); not `?fast`, which would change the timing measured
    flags = [] if os.environ.get("BUILDER_MUTE") == "0" else ["mute"]
    q = "&".join(flags + (["debug=watch"] if scenario == "watch" else []))
    q = "?" + q if q else ""
    page.goto(base + q)
    page.wait_for_selector("body[data-ready='true']", state="attached", timeout=30000)
    renderer = engines.renderer(page)
    page.click("[id='menu.play']")
    page.click("[id='menu.offline']")
    page.wait_for_selector("body[data-screen='lobby']", state="attached")
    page.select_option("[id='lobby.ai']", "3")
    page.wait_for_selector("[id='lobby.ai.3']", state="attached")
    for k in (1, 2, 3):
        page.select_option(f"[id='lobby.ai.{k}']", "hard")
    time.sleep(0.5)
    page.evaluate("() => { window.__cbs = []; window.__long = []; }")
    page.click("[id='lobby.start']")
    t_start = page.evaluate("() => performance.now()")
    page.wait_for_selector("body[data-screen='play']", state="attached")
    # a timeline of the HUD status each half second
    timeline = []
    t0 = time.time()
    phase = "drive"
    if scenario == "fishboy":
        time.sleep(4)
        page.keyboard.down("KeyW")
    while time.time() - t0 < seconds:
        el = time.time() - t0
        if scenario == "fishboy":
            if phase == "drive" and el > seconds * 0.4:
                page.keyboard.up("KeyW")
                page.keyboard.press("Tab")
                phase = "fishboy"
            elif phase == "fishboy" and el > seconds * 0.6:
                page.keyboard.press("Digit2")
                phase = "fishpedia"
            elif phase == "fishpedia" and el > seconds * 0.8:
                page.keyboard.press("Digit3")
                phase = "catch"
        if os.environ.get("TRACE") and phase != "traced" and el > float(os.environ["TRACE"]):
            browser.start_tracing(page=page, path=os.environ.get("TRACE_OUT", "trace.json"),
                                  categories=["toplevel", "gpu", "viz", "cc", "disabled-by-default-devtools.timeline", "blink", "v8"])
            time.sleep(8)
            browser.stop_tracing()
            phase = "traced"
        st = page.evaluate("() => [performance.now(), document.body.dataset.ui2d || '', document.body.dataset.fishboy || '']")
        timeline.append([st[0], st[1], st[2], phase])
        time.sleep(0.5)
    cbs = page.evaluate("() => window.__cbs")
    longs = page.evaluate("() => window.__long")
    browser.close()

data = {"engine": engine, "renderer": renderer, "scenario": scenario, "t_start": t_start,
        "cbs": cbs, "long": longs, "timeline": timeline, "errors": errors, "server_404": SERVER_404}
if out:
    pathlib.Path(out).write_text(json.dumps(data))
print("server 404s", SERVER_404)
print("renderer", renderer, "callbacks", len(cbs), "errors", errors[:10])
