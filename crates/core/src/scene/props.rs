//! The props: where each stands, and their meshes baked for instancing.
//!
//! The map places the trees and the clumps of reeds, and the spawn's dock
//! point. The scene adds what the map leaves to the look: the dock itself
//! at the shore behind the spawn, a cabin up from it and more cabins along
//! other flat shores, lily pads off the reed beds, rocks along the shore,
//! boulders in the shallows and fallen logs in the woods. Each draws from
//! hashes of the map's seed and the spot, so a map always looks the same.
//!
//! A swaying or bobbing model is sampled at `frames` poses through its loop
//! and baked as its mean pose and the parts of its motion that follow the
//! loop's cosine and sine (its first harmonic, which is the whole of a sway
//! made of sines). The renderer adds them back at each instance's own
//! phase (SECOND-ORDER R1.104), so every copy of a model is one instanced
//! draw however its copies are phased.

use super::look::LakeLook;
use super::{Lake, Prop, PropMesh};
use crate::gfx::fx::{self, ONE};
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
    /// thousandths of the model's height, when it is stretched up and
    /// down apart from `scale` (a stem reaching from the bed to the
    /// surface); 0: as `scale`
    pub stretch: i32,
}

const DOCK: u64 = 0x646f_636b;
const CABIN: u64 = 0x6361_6269;
const TREE: u64 = 0x7472_6565;
const REED: u64 = 0x7265_6564;
const LILY: u64 = 0x6c69_6c79;
const ROCK: u64 = 0x726f_636b;
const LOG: u64 = 0x6c6f_6773;
const WEED: u64 = 0x7765_6564;
const BED: u64 = 0x6265_6473;

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
    q.dist(P::new(
        a.x + (abx * t / 1024) as i32,
        a.y + (aby * t / 1024) as i32,
    ))
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
    let hs: Vec<i32> = (0..8)
        .map(|k| lake.ground_mm(toward(p, k * TURN / 8, r)))
        .collect();
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
            stretch: 0,
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
        put(
            &mut out,
            &p.cabin,
            at,
            lowest(lake, at, 2600),
            facing,
            1000,
            h(map, CABIN, at.x, at.y),
        );
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
        put(
            &mut out,
            model,
            t.at,
            lake.ground_mm(t.at),
            turn,
            t.scale,
            k >> 24,
        );
    }

    // reeds, and lily pads off them
    for (r, patch) in map.reeds.iter().enumerate() {
        for c in &patch.clumps {
            let k = h(map, REED, c.x, c.y);
            let scale = 800 + (k % 400) as i32;
            put(
                &mut out,
                &p.reeds,
                *c,
                0,
                ((k >> 12) % TURN as u64) as i32,
                scale,
                k >> 28,
            );
            stems(&mut out, lake, &p.reed_stems, *c, ((k >> 12) % TURN as u64) as i32, scale);
        }
        // weeds on the bed round the reed bed
        let weeds = patch.clumps.len() as i32 * p.weeds_pct / 100;
        let reach = patch.radius_mm * p.weed_reach_pct / 100;
        for w in 0..weeds {
            let k = h(map, WEED, r as i32, w);
            let q = P::new(
                patch.at.x + (k % (2 * reach.max(1) as u64)) as i32 - reach,
                patch.at.y + ((k >> 20) % (2 * reach.max(1) as u64)) as i32 - reach,
            );
            weed(&mut out, lake, p, q, k);
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
            put(
                &mut out,
                &p.lily_pads,
                q,
                0,
                ((k >> 44) % TURN as u64) as i32,
                scale,
                k >> 8,
            );
            stems(&mut out, lake, &p.lily_stems, q, ((k >> 44) % TURN as u64) as i32, scale);
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
        // under the water: weeds in the shallows, rocks and logs on the bed
        if cell.water() {
            let b = h(map, BED, c.x, c.y);
            let q = jitter(b, map.cell_mm / 2 - 200);
            let depth = -lake.ground_mm(q);
            if cell == Cell::Shallow && ((b >> 50) % 1000) < p.weed_permille as u64 {
                weed(&mut out, lake, p, q, b >> 3);
            }
            if depth >= p.bed_min_mm && !p.bed_rocks.is_empty() && ((b % 1000) as i32) < p.bed_rock_permille {
                let model = &p.bed_rocks[((b >> 40) % p.bed_rocks.len() as u64) as usize];
                put(&mut out, model, q, lake.ground_mm(q) - 150, turn, scale * 3 / 4, b);
            } else if depth >= p.bed_min_mm && ((b % 1000) as i32) < p.bed_rock_permille + p.bed_log_permille {
                put(&mut out, &p.bed_log, q, lake.ground_mm(q) - 120, turn, scale.min(1100), b);
            }
        }
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

/// A stretched stem model from the bed under `at` up to the surface, if
/// the water there is deep enough to show one.
fn stems(out: &mut Vec<Placed>, lake: &Lake, model: &str, at: P, heading: i32, scale: i32) {
    let bed = lake.ground_mm(at);
    if bed < -100 {
        out.push(Placed {
            model: model.to_string(),
            at,
            height_mm: bed,
            heading,
            scale,
            hash: 0,
            stretch: -bed,
        });
    }
}

/// One weed on the bed at `q`, if the water there is in the look's depths;
/// sized to stay well under the surface.
fn weed(out: &mut Vec<Placed>, lake: &Lake, p: &super::look::PropsLook, q: P, k: u64) {
    let bed = lake.ground_mm(q);
    if !(p.weed_depth_mm[0]..=p.weed_depth_mm[1]).contains(&-bed) || p.weeds.is_empty() {
        return;
    }
    let model = &p.weeds[(k % p.weeds.len() as u64) as usize];
    // the tallest weed is about 1.5 m; keep it under weed_fill_pct of the depth
    let most = (-bed * p.weed_fill_pct / 100 * 1000 / 1500).max(200);
    let scale = (700 + ((k >> 12) % 600) as i32).min(most);
    out.push(Placed {
        model: model.clone(),
        at: q,
        height_mm: bed - 40,
        heading: ((k >> 24) % TURN as u64) as i32,
        scale,
        hash: k >> 30,
        stretch: 0,
    });
}

/// How much each cell's ground is forest floor (0..=ONE): the trees within
/// two cells of it, against the look's count for a whole forest; and the
/// land past the trees' reach, which only the overview sees, as forest
/// seen from above.
pub fn forest_cells(map: &Map, placed: &[Placed], look: &LakeLook) -> Vec<i32> {
    let n = map.n;
    let mut count = vec![0i32; n * n];
    let trees: Vec<&String> = look
        .props
        .pine
        .iter()
        .chain(&look.props.birch)
        .chain(&look.props.maple)
        .collect();
    for p in placed.iter().filter(|p| trees.contains(&&p.model)) {
        if let Some(i) = map.index(p.at) {
            count[i] += 1;
        }
    }
    (0..n * n)
        .map(|i| {
            let (x, y) = ((i % n) as i32, (i / n) as i32);
            let mut sum = 0;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let (a, b) = (x + dx, y + dy);
                    if a >= 0 && b >= 0 && a < n as i32 && b < n as i32 {
                        sum += count[(b * n as i32 + a) as usize];
                    }
                }
            }
            if map.cells[i].water() {
                return 0;
            }
            let trees = sum * ONE / look.ground.forest_trees.max(1);
            let reach = map.setting.params().tree_reach_mm;
            let far = (map.shore[i] - reach * 3 / 4) as i64 * ONE as i64 / (reach / 4).max(1) as i64;
            super::terrain::smooth(trees.max(far as i32))
        })
        .collect()
}

/// The animation a prop plays in place: its sway, else its bob.
fn idle(m: &Model) -> Option<Anim> {
    [Anim::Sway, Anim::Bob]
        .into_iter()
        .find(|a| m.animation(*a).is_some())
}

/// A swaying model's loop as three blocks of vertices: the mean pose, and
/// the motion along the loop's cosine and along its sine, from `samples`
/// equally spaced poses (each a whole triangle list). Positions and normals
/// are decomposed; colours and glows are the first pose's.
fn harmonics(samples: &[Vec<i32>]) -> Vec<i32> {
    let k = samples.len() as i64;
    let len = samples[0].len();
    let mut out = vec![0i32; len * 3];
    let turns: Vec<(i64, i64)> = (0..k)
        .map(|i| {
            let a = (TURN as i64 * i / k) as i32;
            (fx::cos(a) as i64, fx::sin(a) as i64)
        })
        .collect();
    for v in 0..len {
        let moving = v % render::VERTEX_VALUES < 6;
        if !moving {
            for block in 0..3 {
                out[block * len + v] = samples[0][v];
            }
            continue;
        }
        let (mut mean, mut c, mut s) = (0i64, 0i64, 0i64);
        for (i, sample) in samples.iter().enumerate() {
            let x = sample[v] as i64;
            mean += x;
            c += x * turns[i].0;
            s += x * turns[i].1;
        }
        out[v] = (mean / k) as i32;
        out[len + v] = (2 * c / (k * ONE as i64)) as i32;
        out[2 * len + v] = (2 * s / (k * ONE as i64)) as i32;
    }
    out
}

/// Bakes each model the props use, appending it to `out`, and gives every
/// prop its mesh, transform and phase.
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
                let anim = idle(m).filter(|_| look.props.frames > 1);
                let ms = anim.and_then(|a| m.animation(a)).map_or(1, |a| a.ms);
                let first = render::vertex_count(out) as i32;
                let colours = m.colours([0; 3]);
                let pose = |t: u32| {
                    let mut tris = Vec::new();
                    m.draw(
                        &m.transforms(&m.poses(anim, t)),
                        &Affine::IDENTITY,
                        &colours,
                        &mut tris,
                    );
                    tris
                };
                let still = pose(0);
                let verts = render::vertex_count(&still) as i32;
                if anim.is_some() {
                    let frames = look.props.frames;
                    let samples: Vec<Vec<i32>> = (0..frames).map(|k| pose(ms * k / frames)).collect();
                    out.extend(harmonics(&samples));
                } else {
                    out.extend(still);
                }
                meshes.push(PropMesh {
                    model: p.model.clone(),
                    first,
                    verts,
                    moving: anim.is_some(),
                    ms,
                    radius: m.ball().1,
                });
                meshes.len() - 1
            }
        };
        let mesh = &meshes[mi];
        let place = if p.stretch > 0 {
            let (s, t) = (p.scale * ONE / 1000, p.stretch * ONE / 1000);
            Affine::translate(lake.world(p.at, p.height_mm))
                .then(&Affine::rot_y(super::yaw_of_heading(p.heading)))
                .then(&Affine::scale(crate::gfx::geom::v3(s, t, s)))
        } else {
            lake.place(p.at, p.height_mm, p.heading, p.scale)
        };
        let size = p.scale.max(p.stretch);
        props.push(Prop {
            mesh: mi,
            place,
            at: place.t,
            radius: (mesh.radius as i64 * size as i64 / 1000) as i32 + super::mm(2000),
            phase_ms: (p.hash % mesh.ms as u64) as u32,
        });
    }
    (meshes, props)
}
