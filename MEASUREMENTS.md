# Measurements

Stage 4, recorded by the agent before building and again at handoff. `B2 Require step-by-step justification`.

The state measured rather than recalled, so later changes can be compared with it. A probe prints numbers and asserts loose bounds at most; record its output here with the command that produced it.

| What | Command | Result | When |
| --- | --- | --- | --- |
| Toolchain | `rustc --version; wasm-bindgen --version` | | |
| Core tests | `./scripts/count-tests.sh` | | |
| Full check | `./scripts/check.sh` | | |
| WebAssembly size | printed by `./scripts/build.sh` | | |
| Breaks caught | `python3 scripts/break_tests.py` | | |

## The lake (world drawing)

`cargo run --release --example lake -- 0 3`, 2026-10-17, branch `worktree-agent-a2ff84d9ac75f0c6e`: seed 0 builds in 41 ms (native, release): 75,264 terrain triangles, 16,258 water, 2,526 props of 11 models, 11.2 MB static buffer. At 1920x1080 with the fog on: 98,000 to 124,000 triangles and 9 to 15 draw calls a frame (the terrain and water are about 91,000 of them); with the fog off, up to 343,000 triangles. The core builds a frame in 0.1 to 0.25 ms.

Frame rate, Chromium 1920x1080 on this machine's GPU (AMD Radeon through WSL's D3D12, ANGLE), 3 s each: tour 59.9, reeds 59.9, island 60.0, spawn 60.0, dock 59.6, overview without fog 60.2 fps (capped at 60). A match on the lake (Play, Start match, the boat camera at the spawn): 60.2 fps. Before the sways were baked as one harmonic (SECOND-ORDER R2.31): tour 46.2, reeds 46.9 fps at 65 to 69 draw calls.

## The AI players

`cargo run --release --example playtest -- 48 1`, 2026-10-18, branch `ai`: 0.9 µs per AI player per tick on average (native, release; four AI players, 48 eight-minute matches, 1,152,000 ticks), against a budget of 300 µs. Path searches make the costly ticks: 52 ticks took the four AI players over 1 ms together, the slowest 6 ms. The match step itself took 2.9 µs a tick. Not measured in WebAssembly, which is usually a few times slower than native.

## A full match in the browser

2026-10-18, branch `worktree-agent-a9dc849f7fd273052` (SECOND-ORDER R6.1 to R6.12). `notes/perf/perf.py <repo> chromium watch 150`: the page at 1920x1080, Play, Offline, three Hard AI rivals, `?debug=watch` handing this seat to a fourth Hard AI, so the camera follows it driving, fishing (underwater, with the picture-in-picture) and raising the Fish-Boy; `fishboy` instead drives this seat with W, then holds the Fish-Boy up on each tab. Frames from 1 s after Start match. Chromium draws on the GPU through `tests/browser/engines.py` (ANGLE, D3D12 on an AMD Radeon through WSL, an invisible Xvfb window); Firefox in software (llvmpipe) is the worst case. "Core" is the time inside the WebAssembly calls a frame (advance, frame building, the 2D list, the view model, sound). `notes/perf/analyse.py` summarises.

This machine is shared: other agents' builds keep the load at 8 to 15 on 16 cores, and the late frames follow the load more than the build (R6.1). The load average is beside each run; before and after were run back to back.

| Run | Load | Frames | Median ms | p95 | p99 | Worst | Over 25 / 50 ms | Page JS median / p99 ms | Core median / p99 / worst ms | Draw calls | Triangles |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Before, Chromium, watch 150 s | 10 | 8,229 | 16.7 | 33.3 | 33.4 | 83.4 | 700 / 6 | 4.0 / 17.0 | 2.9 / 14.2 / 31.1 | 12 to 37 | 101k to 245k |
| Before, Chromium, Fish-Boy 60 s | 10 | 3,175 | 16.7 | 33.3 | 33.4 | 66.7 | 361 / 3 | 3.2 / 15.6 | 2.5 / 10.0 / 26.7 | 7 to 24 | 104k to 115k |
| Before, Firefox software, watch 120 s | 10 | 3,455 | 33.3 | 67.1 | 84.1 | 133.8 | 2,595 / 451 | 8 / 27 | 3 / 13 / 39 | 12 to 25 | 109k to 232k |
| After, Chromium, watch 150 s | 0.2 | 8,912 | 16.7 | 16.7 | 16.8 | 49.9 | 52 / 0 | 1.6 / 3.3 | 1.3 / 2.5 / 9.6 | 9 to 28 | 102k to 218k |
| After, Chromium, Fish-Boy 60 s | 6 | 3,552 | 16.7 | 16.8 | 16.8 | 50.0 | 22 / 0 | 1.9 / 4.8 | 1.4 / 3.8 / 9.0 | 12 to 27 | 104k to 116k |
| After, Firefox software, watch 120 s | 14 | 3,271 | 33.4 | 50.6 | 66.9 | 165.8 | 2,902 / 486 | 5 / 15 | 2 / 7 / 15 | 7 to 32 | 103k to 233k |
| Before (42f2a02), back to back, run 1 | 8.6 | 8,358 | 16.7 | 33.3 | 33.4 | 50.1 | 595 / 2 | 3.7 / 15.5 | 2.8 / 11.6 / 23.1 | 7 to 34 | 104k to 240k |
| After, back to back, run 1 | 9.8 | 8,867 | 16.7 | 16.8 | 16.8 | 33.5 | 85 / 0 | 2.1 / 4.9 | 1.6 / 3.8 / 10.6 | 10 to 29 | 103k to 226k |
| Before (42f2a02), back to back, run 2 | 5.7 | 8,740 | 16.7 | 16.8 | 33.3 | 33.5 | 225 / 0 | 4.2 / 6.1 | 3.6 / 5.3 / 7.5 | 12 to 27 | 107k to 230k |
| After, back to back, run 2 | 1 to 7 | 8,113 | 16.7 | 33.3 | 33.4 | 83.4 | 802 / 5 | 1.7 / 9.6 | 1.3 / 6.6 / 34.4 | 7 to 27 | 107k to 224k |

By stretch, the after run at load 0.2: driving 59.9 fps, p99 16.8 ms; underwater 60.0 fps, p99 16.8 ms, 0 frames over 25 ms (before: 53.9 fps, 381 of 3,405 over 25 ms); Fish-Boy 57.8 fps, p99 33.4 ms (46 of 1,194 over 25 ms); fishing at the bow 60.0 fps. Underwater the triangles double: the picture-in-picture draws the boat's view again.

The target (60 fps median, p99 under 25 ms, no frame over 50 ms after the first second) is met on the GPU path with the machine idle and in one of two loaded back-to-back runs; the other loaded run (after, run 2, whose load rose from 1 to 7 during it) missed it. The page's own work is down by half or more in every run: its JavaScript p99 from 15 to 17 ms to 3 to 10 ms, the core's p99 from 10 to 14 ms to 2.5 to 6.6 ms. Firefox in software stays at about 30 fps, bound by llvmpipe's drawing (its JavaScript is 5 ms a frame).

Natively (`notes/perf/frameprof.rs`, 150 s watched match, release): frame building 579 to 235 µs a frame, the picture-in-picture 359 to 186 µs, the 2D list 51 to 45 µs, the view model 8 µs, sound 21 µs; triangles handed to the page 1,575 to 649 KB a frame (R6.5).

Chromium's GPU thread, from a trace of 8 s underwater (`TRACE=20`): 863 ms busy a second before, about 600 to 750 after; a page of two blank full-screen canvases takes 510, nearly all of it the present step. Leaving each draw out in turn: the water 194 ms a second, the props 75, the terrain 24; MSAA about 135 (R6.11).

The AI (R6.2 to R6.4): `cargo run --release --example playtest -- 48 1` after the search budget: 0.8 µs per AI player per tick; 6 ticks in 1,152,000 timed over 1 ms (the slowest 12 ms, a scheduler slice on the loaded machine, not work). Counted, the worst tick of four Hard AIs is one path search of 3,148 steps (about 250 µs natively). `four_hard_ais_run_at_most_one_path_search_a_tick` guards it.

## Not measured

Devices, browsers and conditions no measurement here covers.
