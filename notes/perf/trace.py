import json
import sys
from collections import defaultdict

t = json.load(open(sys.argv[1]))
ev = t["traceEvents"] if isinstance(t, dict) else t
names = {}
for e in ev:
    if e.get("ph") == "M" and e.get("name") == "thread_name":
        names[(e["pid"], e["tid"])] = e["args"]["name"]
pnames = {}
for e in ev:
    if e.get("ph") == "M" and e.get("name") == "process_name":
        pnames[e["pid"]] = e["args"]["name"]
ts = [e["ts"] for e in ev if "ts" in e and e.get("ph") in ("X", "B")]
span = (max(ts) - min(ts)) / 1e6
busy = defaultdict(float)
top = defaultdict(lambda: defaultdict(float))
for e in ev:
    if e.get("ph") == "X" and "dur" in e:
        th = (e["pid"], e["tid"])
        nm = names.get(th, "?")
        if e.get("cat", "").startswith("toplevel") or e["name"] in ("ThreadControllerImpl::RunTask", "RunTask"):
            busy[th] += e["dur"] / 1e3
        top[th][e["name"]] += e["dur"] / 1e3
print(f"span {span:.1f} s")
for th, b in sorted(busy.items(), key=lambda x: -x[1])[:12]:
    print(f"{pnames.get(th[0], th[0])!s:20} {names.get(th, '?'):28} busy {b / span / 10:5.1f}%  ({b / span:.1f} ms/s)")
    for n, d in sorted(top[th].items(), key=lambda x: -x[1])[:6]:
        print(f"      {n:50} {d / span:7.1f} ms/s")
