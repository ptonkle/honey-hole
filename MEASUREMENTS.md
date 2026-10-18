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

## Not measured

Devices, browsers and conditions no measurement here covers.
