//! The props: where each stands, and their meshes baked for instancing.
//!
//! The map places the trees and the clumps of reeds, and the spawn's dock
//! point. The scene adds what the map leaves to the look: the dock itself
//! at the shore behind the spawn, a cabin up from it and more cabins along
//! other flat shores, lily pads off the reed beds, rocks along the shore,
//! boulders in the shallows and fallen logs in the woods. Each draws from
//! hashes of the map's seed and the spot, so a map always looks the same.
//!
//! A swaying or bobbing model is baked at `frames` poses through its loop;
//! the renderer blends each instance between two of them, at the instance's
//! own phase (SECOND-ORDER R1.104).

use super::look::LakeLook;
use super::{Lake, Prop, PropMesh};
use crate::gfx::geom::Affine;
use crate::gfx::render;
use crate::map::rng::hash2;
use crate::map::trig::{self, TURN};
use crate::map::{Cell, Map, TreeKind, P};
use crate::model::{Anim, Model};

/// One prop to stand on the map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    pub model: String,
    pub at: P,
    pub height_mm: i32,
    pub heading: i32,
    /// thousandths of the model's size
    pub scale: i32,
    /// a hash for its phase in its loop
    pub hash: u64,
}

const DOCK: u64 = 0x646f_636b;
const CABIN: u64 = 0x6361_6269;
const TREE: u64 = 0x7472_6565;
const REED: u64 = 0x7265_6564;
const LILY: u64 = 0x6c69_6c79;
const ROCK: u64 = 0x726f_636b;
const LOG: u64 = 0x6c6f_6773;

fn h(map: &Map, salt: u64, a: i32, b: i32) -> u64 {
    hash2(map.seed as u64 ^ salt, a, b)
}

/// A point `d` mm from `p` toward `heading`.
fn toward(p: P, heading: i32, d: i32) -> P {
    let (c, s) = (trig::cos(heading) as i64, trig::sin(heading) as i64);
    P::new(
        p.x + (c * d as i64 / trig::ONE as i64) as i32,
        p.y + (s * d as i64 / trig::ONE as i64) as i32,
    )
}

fn heading_to(from: P, to: P) -> i32 {
    trig::atan2((to.y - from.y) as i64, (to.x - from.x) as i64)
}

/// Distance from `q` to the segment `a`-`b`, in mm.
fn segment_dist(q: P, a: P, b: P) -> i64 {
    let (abx, aby) = ((b.x - a.x) as i64, (b.y - a.y) as i64);
    let len2 = abx * abx + aby * aby;
    if len2 == 0 {
        return q.dist(a);
    }
    let t = (((q.x - a.x) as i64 * abx + (q.y - a.y) as i64 * aby) * 1024 / len2).clamp(0, 1024);
    q.dist(P::new(a.x + (abx * t / 1024) as i32, a.y + (aby * t / 1024) as i32))
}

/// The lowest ground within `r` of a point: a cabin set there sinks into a
/// slope rather than floating over it.
fn lowest(lake: &Lake, p: P, r: i32) -> i32 {
    (0..8)
        .map(|k| lake.ground_mm(toward(p, k * TURN / 8, r)))
        .chain([lake.ground_mm(p)])
        .min()
        .unwrap_or(0)
}

/// How uneven the ground is within `r` of a point (highest minus lowest).
fn unevenness(lake: &Lake, p: P, r: i32) -> i32 {
    let hs: Vec<i32> = (0..8).map(|k| lake.ground_mm(toward(p, k * TURN / 8, r))).collect();
    hs.iter().max().unwrap_or(&0) - hs.iter().min().unwrap_or(&0)
}

/// The way down to the water from a land point: the lowest of 16 headings
/// 15 m out.
fn downhill(lake: &Lake, p: P) -> i32 {
    (0..16)
        .map(|k| k * TURN / 16)
        .min_by_key(|&a| lake.ground_mm(toward(p, a, 15_000)))
        .unwrap_or(0)
}

/// Every prop of the map, as the look names them.
pub fn place(lake: &Lake, look: &LakeLook) -> Vec<Placed> {
    let map = &lake.map;
    let p = &look.props;
    let mut out = Vec::new();
    let put = |out: &mut Vec<Placed>, model: &str, at: P, height_mm: i32, heading: i32, scale: i32, hash| {
        out.push(Placed {
            model: model.to_string(),
            at,
            height_mm,
            heading,
            scale,
            hash,
        })
    };

    // the dock: from the shore behind the spawn, out toward it
    let spawn = map.spawn;
    let out_dir = heading_to(spawn.dock, spawn.at);
    let mut dock = spawn.dock;
    for k in -40..80 {
        let q = toward(spawn.dock, out_dir, k * 500);
        if lake.ground_mm(q) < 0 {
            dock = toward(q, out_dir, -600);
            break;
        }
    }
    let dock_end = toward(dock, out_dir, 4800);
    put(&mut out, &p.dock, dock, 0, out_dir, 1000, h(map, DOCK, 0, 0));

    // cabins: one up from the dock, the rest on the flattest shores far from it
    let mut cabins = Vec::new();
    if p.cabins > 0 {
        let at = toward(dock, out_dir, -p.cabin_back_mm);
        cabins.push((at, out_dir));
    }
    let n = map.n;
    let mut spots: Vec<(i32, P)> = (0..n * n)
        .filter(|&i| map.cells[i] == Cell::Land)
        .filter(|&i| (p.cabin_back_mm - 2000..=p.cabin_back_mm + 4000).contains(&map.shore[i]))
        .filter(|&i| i % 3 == 0)
        .map(|i| map.centre(i))
        .map(|c| (unevenness(lake, c, 3000), c))
        .collect();
    spots.sort_by_key(|&(u, c)| (u / 300, h(map, CABIN, c.x, c.y)));
    for (_, c) in spots {
        if cabins.len() >= p.cabins as usize {
            break;
        }
        if cabins.iter().any(|(o, _)| o.dist(c) < 150_000) || c.dist(spawn.at) < 150_000 {
            continue;
        }
        cabins.push((c, downhill(lake, c)));
    }
    for &(at, facing) in &cabins {
        put(&mut out, &p.cabin, at, lowest(lake, at, 2600), facing, 1000, h(map, CABIN, at.x, at.y));
    }
    let clear = |q: P| {
        cabins.iter().any(|(c, _)| c.dist(q) < p.cabin_clear_mm as i64)
            || segment_dist(q, toward(dock, out_dir, -p.cabin_back_mm), dock_end) < p.dock_clear_mm as i64
    };

    // the map's trees
    for t in &map.trees {
        if clear(t.at) {
            continue;
        }
        let list = match t.kind {
            TreeKind::Pine => &p.pine,
            TreeKind::Birch => &p.birch,
            TreeKind::Maple => &p.maple,
        };
        let k = h(map, TREE, t.at.x, t.at.y);
        let model = &list[(k % list.len() as u64) as usize];
        let turn = ((k >> 16) % TURN as u64) as i32;
        put(&mut out, model, t.at, lake.ground_mm(t.at), turn, t.scale, k >> 24);
    }

    // reeds, and lily pads off them
    for (r, patch) in map.reeds.iter().enumerate() {
        for c in &patch.clumps {
            let k = h(map, REED, c.x, c.y);
            let scale = 800 + (k % 400) as i32;
            put(&mut out, &p.reeds, *c, 0, ((k >> 12) % TURN as u64) as i32, scale, k >> 28);
        }
        let want = patch.clumps.len() as i32 * p.lily_pct / 100;
        let mut placed = 0;
        for tries in 0..want * 12 {
            if placed >= want {
                break;
            }
            let k = h(map, LILY, r as i32, tries);
            let reach = patch.radius_mm * 3 / 2;
            let q = P::new(
                patch.at.x + (k % (2 * reach as u64)) as i32 - reach,
                patch.at.y + ((k >> 20) % (2 * reach as u64)) as i32 - reach,
            );
            let depth = -lake.ground_mm(q);
            if !(p.lily_depth_mm[0]..=p.lily_depth_mm[1]).contains(&depth)
                || patch.clumps.iter().any(|c| c.dist(q) < 1500)
            {
                continue;
            }
            let scale = 700 + ((k >> 40) % 600) as i32;
            put(&mut out, &p.lily_pads, q, 0, ((k >> 44) % TURN as u64) as i32, scale, k >> 8);
            placed += 1;
        }
    }

    // rocks on the shore, boulders in the shallows, logs in the woods
    for i in 0..n * n {
        let c = map.centre(i);
        let k = h(map, ROCK, c.x, c.y);
        let roll = (k % 1000) as i32;
        let jitter = |k: u64, r: i32| {
            P::new(
                c.x + ((k >> 10) % (2 * r as u64)) as i32 - r,
                c.y + ((k >> 22) % (2 * r as u64)) as i32 - r,
            )
        };
        let turn = ((k >> 34) % TURN as u64) as i32;
        let scale = 700 + ((k >> 50) % 600) as i32;
        let cell = map.cells[i];
        if !cell.water() && map.shore[i] <= map.cell_mm && roll < p.shore_rocks_permille {
            let q = jitter(k, map.cell_mm / 3);
            if !clear(q) {
                put(&mut out, &p.shore_rocks, q, lake.ground_mm(q), turn, scale, k);
            }
        } else if cell == Cell::Shallow && roll < p.boulder_permille {
            let q = jitter(k, map.cell_mm / 3);
            if !clear(q) {
                put(&mut out, &p.boulder, q, lake.ground_mm(q), turn, scale, k);
            }
        } else if !cell.water() && (6_000..40_000).contains(&map.shore[i]) {
            let s = h(map, LOG, c.x, c.y);
            if ((s % 1000) as i32) < p.log_permille {
                let q = jitter(s, map.cell_mm / 3);
                if !clear(q) {
                    put(&mut out, &p.log, q, lake.ground_mm(q), turn, scale.min(1100), s);
                }
            }
        }
    }
    out
}

/// The cells with at least the look's count of trees in them or next to
/// them: their ground is forest floor.
pub fn forest_cells(map: &Map, placed: &[Placed], look: &LakeLook) -> Vec<bool> {
    let n = map.n;
    let mut count = vec![0i32; n * n];
    let trees: Vec<&String> = look.props.pine.iter().chain(&look.props.birch).chain(&look.props.maple).collect();
    for p in placed.iter().filter(|p| trees.contains(&&p.model)) {
        if let Some(i) = map.index(p.at) {
            count[i] += 1;
        }
    }
    (0..n * n)
        .map(|i| {
            let (x, y) = ((i % n) as i32, (i / n) as i32);
            let mut sum = 0;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (a, b) = (x + dx, y + dy);
                    if a >= 0 && b >= 0 && a < n as i32 && b < n as i32 {
                        sum += count[(b * n as i32 + a) as usize];
                    }
                }
            }
            !map.cells[i].water() && sum >= look.ground.forest_trees
        })
        .collect()
}

/// The animation a prop plays in place: its sway, else its bob.
fn idle(m: &Model) -> Option<Anim> {
    [Anim::Sway, Anim::Bob].into_iter().find(|a| m.animation(*a).is_some())
}

/// Bakes each model the props use, appending its frames to `out`, and
/// gives every prop its mesh, transform and phase.
pub fn bake(
    lake: &Lake,
    models: &[Model],
    placed: &[Placed],
    look: &LakeLook,
    out: &mut Vec<i32>,
) -> (Vec<PropMesh>, Vec<Prop>) {
    let mut meshes: Vec<PropMesh> = Vec::new();
    let mut props = Vec::with_capacity(placed.len());
    for p in placed {
        let mi = match meshes.iter().position(|m| m.model == p.model) {
            Some(i) => i,
            None => {
                let m = models
                    .iter()
                    .find(|m| m.id == p.model)
                    .expect("Lake::build checked every model is loaded");
                let anim = idle(m);
                let (frames, ms) = match anim.and_then(|a| m.animation(a)) {
                    Some(a) => (look.props.frames, a.ms),
                    None => (1, 1),
                };
                let first = render::vertex_count(out) as i32;
                let colours = m.colours([0; 3]);
                for k in 0..frames {
                    let t = m.transforms(&m.poses(anim, ms * k / frames));
                    m.draw(&t, &Affine::IDENTITY, &colours, out);
                }
                let verts = (render::vertex_count(out) as i32 - first) / frames as i32;
                meshes.push(PropMesh {
                    model: p.model.clone(),
                    first,
                    verts,
                    frames,
                    ms,
                    radius: m.ball().1,
                });
                meshes.len() - 1
            }
        };
        let mesh = &meshes[mi];
        let place = lake.place(p.at, p.height_mm, p.heading, p.scale);
        props.push(Prop {
            mesh: mi,
            place,
            at: place.t,
            radius: (mesh.radius as i64 * p.scale as i64 / 1000) as i32 + super::mm(2000),
            phase_ms: (p.hash % mesh.ms as u64) as u32,
        });
    }
    (meshes, props)
}
