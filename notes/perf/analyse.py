#!/usr/bin/env python3
"""Summarise perf.py output: frame percentiles, wasm time, draws, by stretch."""
import bisect
import json
import sys
from collections import defaultdict

d = json.load(open(sys.argv[1]))
skip_ms = float(sys.argv[2]) if len(sys.argv) > 2 else 1000.0
frames = defaultdict(lambda: {"js": 0.0, "w": defaultdict(float), "calls": 0, "tris": 0})
for ts, dur, w, calls, tris, _t0 in d["cbs"]:
    f = frames[ts]
    f["js"] += dur
    for k, v in w.items():
        f["w"][k] += v
    f["calls"] += calls
    f["tris"] += tris
tss = sorted(frames)
start = d["t_start"] + skip_ms
tl = d["timeline"]
tl_t = [x[0] for x in tl]


def stretch(ts):
    i = max(0, bisect.bisect_right(tl_t, ts) - 1)
    if not tl:
        return "all"
    _, status, fb, phase = tl[i]
    words = dict(w.split("=", 1) for w in status.split() if "=" in w)
    if fb and fb not in ("down", "off", "0", ""):
        return "fishboy"
    if words.get("fishing") not in (None, "", "ready", "charging", "flying"):
        return "underwater"
    if words.get("mode") == "fishing":
        return "fishing"
    return "driving"


def pct(xs, p):
    xs = sorted(xs)
    if not xs:
        return float("nan")
    k = min(len(xs) - 1, int(round(p / 100 * (len(xs) - 1))))
    return xs[k]


groups = defaultdict(list)
for a, b in zip(tss, tss[1:]):
    if a < start:
        continue
    groups["all"].append((b - a, frames[b]))
    groups[stretch(b)].append((b - a, frames[b]))

CORE = ["advance", "build_frame", "build_ui2d", "ui", "vertices", "instances", "view", "ui2d_values",
        "ui2d_strings", "pass_vertices", "inset_vertices", "inset_instances", "sound", "net_pump", "act", "resize"]
print(f"{d['engine']} {d['scenario']}  renderer: {d['renderer']}")
print(f"{'stretch':11} {'frames':>6} {'fps':>5} {'med':>6} {'p95':>6} {'p99':>6} {'worst':>6} {'>25':>4} {'>50':>4} | {'js med':>6} {'js p99':>6} {'wasm med':>8} {'wasm p99':>8} {'wasm max':>8} | calls tris(k)")
for name, fs in sorted(groups.items(), key=lambda x: -len(x[1])):
    dt = [x[0] for x in fs]
    js = [x[1]["js"] for x in fs]
    wasm = [sum(v for k, v in x[1]["w"].items() if ":" not in k) for x in fs]
    calls = [x[1]["calls"] for x in fs]
    tris = [x[1]["tris"] for x in fs]
    tot = sum(dt) / 1000
    print(f"{name:11} {len(fs):6} {len(fs)/tot:5.1f} {pct(dt,50):6.1f} {pct(dt,95):6.1f} {pct(dt,99):6.1f} {max(dt):6.1f} "
          f"{sum(1 for x in dt if x > 25):4} {sum(1 for x in dt if x > 50):4} | {pct(js,50):6.2f} {pct(js,99):6.2f} "
          f"{pct(wasm,50):8.2f} {pct(wasm,99):8.2f} {max(wasm):8.2f} | {pct(calls,50):.0f} ({min(calls)}-{max(calls)}) "
          f"{pct(tris,50)/1000:.0f} ({min(tris)/1000:.0f}-{max(tris)/1000:.0f})")
print("per wasm call, all frames (median / p99 / max ms; B: MB returned, S: M chars):")
fs = groups["all"]
for k in sorted({k for x in fs for k in x[1]["w"]}, key=lambda k: -sum(x[1]["w"].get(k, 0) for x in fs)):
    v = [x[1]["w"].get(k, 0.0) for x in fs]
    if max(v) < 0.05:
        continue
    print(f"  {k:22} {pct(v,50):6.2f} {pct(v,99):6.2f} {max(v):7.2f}")
slow = sorted(((x[0], x[1]) for x in fs), key=lambda x: -x[0])[:8]
print("slowest frames: interval ms, js ms, top wasm")
for dt, f in slow:
    top = sorted(f["w"].items(), key=lambda x: -x[1])[:3]
    print(f"  {dt:6.1f} js {f['js']:6.2f} " + ", ".join(f"{k} {v:.2f}" for k, v in top))
print("long tasks after start:", [(round(a), round(b)) for a, b in d["long"] if a > start][:20])
print("errors:", d["errors"][:10])
