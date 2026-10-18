//! Making a map from a setting and a seed.
//!
//! 1. The lake: an ellipse of random size, stretch and tilt, with a few
//!    smaller stretched ellipses (arms) joined to its sides; each outline is
//!    wobbled by two to six waves (bays and points) and roughened by value
//!    noise (coves). A field gives each point a rough signed distance to that
//!    shore, in mm, positive on land.
//! 2. The spawn: at one end of the long axis, the setting's distance off the
//!    shore, facing back along the axis; the dock is the shore behind it.
//! 3. Islands: small wobbly blobs dropped where the field says the water is
//!    deep enough round them, away from the shore, each other and the
//!    spawn's lane.
//! 4. Clean-up: cut-off pockets of water too small to matter become marsh
//!    (land), and specks of land become water. Bigger pockets stay, for the
//!    validator to reject.
//! 5. Depth and height: depth grows with the distance from the shore, with
//!    a rough bed; land rises away from the water into hills.
//! 6. Reeds in the shallows, mostly in coves; trees on the land near water,
//!    in clumps, birches by the shore and pines most of all.
//!
//! Each step draws from its own stream of the seed, so changing one step's
//! use of chance leaves the earlier steps of every seed as they were.

use super::grid;
use super::noise;
use super::rng::Rng;
use super::trig::{self, HALF, ONE, TURN};
use super::{Map, Params, ReedPatch, Setting, Spawn, Tree, TreeKind, P};

/// Stream names, one per step.
const LAKE: u64 = 1;
const ISLANDS: u64 = 2;
const RAGGED: u64 = 3;
const BED: u64 = 4;
const REEDS: u64 = 5;
const TREES: u64 = 6;
const SIDE: u64 = 7;

/// An outline: an ellipse with lobes, as a field in mm.
struct Blob {
    at: P,
    half_long: i32,
    half_short: i32,
    tilt: i32,
    /// (lobes, amplitude in thousandths, phase)
    waves: Vec<(i32, i32, i32)>,
}

impl Blob {
    /// Roughly how far outside the outline a point is, in mm (negative inside).
    fn outside(&self, p: P) -> i32 {
        let dx = (p.x - self.at.x) as i64;
        let dy = (p.y - self.at.y) as i64;
        let (c, s) = (trig::cos(self.tilt) as i64, trig::sin(self.tilt) as i64);
        let u = (dx * c + dy * s) / ONE as i64;
        let v = (-dx * s + dy * c) / ONE as i64;
        let uu = u * 1000 / self.half_long as i64;
        let vv = v * 1000 / self.half_short as i64;
        let rho = trig::hypot(uu, vv);
        let theta = trig::atan2(vv, uu);
        let wobble: i64 = self
            .waves
            .iter()
            .map(|&(k, amp, phase)| amp as i64 * trig::cos(k * theta + phase) as i64 / ONE as i64)
            .sum();
        let edge = 1000 + wobble;
        let scale = (self.half_long as i64 + self.half_short as i64) / 2;
        ((rho - edge) * scale / 1000) as i32
    }
}

/// The map for a setting and a seed. The same pair always gives the same map.
pub fn generate(setting: Setting, seed: u32) -> Map {
    let p = setting.params();
    let s = seed as u64;
    let n = p.cells;
    let size = n as i32 * p.cell_mm;

    // 1. the lake: a main body and a few lobes joined to it (arms and bays)
    let mut r = Rng::stream(s, LAKE);
    let half_long = r.range(p.half_length_mm.0, p.half_length_mm.1);
    let half_short = (half_long as i64 * 1000 / r.range(p.aspect.0, p.aspect.1) as i64) as i32;
    let jitter = 14_000;
    let waves = |r: &mut Rng, scale: i32| -> Vec<(i32, i32, i32)> {
        (2..=6)
            .map(|k| {
                let most = p.wobble[k as usize - 2] * scale / 1000;
                (k, r.range(most / 3, most), r.range(0, TURN - 1))
            })
            .collect()
    };
    let lake = Blob {
        at: P::new(
            size / 2 + r.range(-jitter, jitter),
            size / 2 + r.range(-jitter, jitter),
        ),
        half_long,
        half_short,
        tilt: r.range(0, TURN - 1),
        waves: waves(&mut r, 1000),
    };
    let mut lobes: Vec<Blob> = Vec::new();
    let mut used: Vec<i32> = Vec::new();
    for _ in 0..r.range(p.lobes.0, p.lobes.1) {
        // a lobe sits on the main body's outline, away from the long axis ends
        // (one of which holds the spawn) and from the other lobes
        let mut angle = 0;
        for _try in 0..20 {
            angle = r.range(0, TURN - 1);
            let off_axis = (trig::sin(angle - lake.tilt) as i64).abs() > (ONE / 3) as i64;
            let apart = used
                .iter()
                .all(|&u| (trig::cos(angle - u) as i64) < (ONE / 2) as i64);
            if off_axis && apart {
                break;
            }
        }
        used.push(angle);
        let (c, sn) = (
            trig::cos(angle - lake.tilt) as i64,
            trig::sin(angle - lake.tilt) as i64,
        );
        let reach = r.range(800, 1050) as i64;
        let u = c * lake.half_long as i64 / ONE as i64 * reach / 1000;
        let v = sn * lake.half_short as i64 / ONE as i64 * reach / 1000;
        let (tc, ts) = (trig::cos(lake.tilt) as i64, trig::sin(lake.tilt) as i64);
        let at = P::new(
            lake.at.x + ((u * tc - v * ts) / ONE as i64) as i32,
            lake.at.y + ((u * ts + v * tc) / ONE as i64) as i32,
        );
        let long = r.range(p.lobe_half_mm.0, p.lobe_half_mm.1);
        lobes.push(Blob {
            at,
            half_long: long,
            half_short: (long as i64 * 1000 / r.range(1300, 2400) as i64) as i32,
            // roughly outward, so the lobe reads as an arm of the lake
            tilt: angle + r.range(-TURN / 10, TURN / 10),
            waves: waves(&mut r, 1400),
        });
    }
    let ragged_seed = Rng::stream(s, RAGGED).next_u64();
    let border = |q: P| {
        let edge = q.x.min(q.y).min(size - q.x).min(size - q.y);
        (p.border_mm - edge) * 3
    };
    let shore_field = |q: P| {
        let rough = noise::fractal(ragged_seed, q.x, q.y, p.ragged_wave_mm, 3);
        let body = lobes.iter().map(|b| b.outside(q)).fold(lake.outside(q), i32::min);
        (body + rough * p.ragged_mm / 1024).max(border(q))
    };

    // 2. the spawn, at one end of the long axis
    let way = if Rng::stream(s, SIDE).chance(500) {
        lake.tilt
    } else {
        lake.tilt + HALF
    };
    let (ux, uy) = (trig::cos(way) as i64, trig::sin(way) as i64);
    let along = |t: i32| {
        P::new(
            lake.at.x + (ux * t as i64 / ONE as i64) as i32,
            lake.at.y + (uy * t as i64 / ONE as i64) as i32,
        )
    };
    let mut spawn_at = lake.at;
    let mut dock = lake.at;
    let mut t = 0;
    while t < size {
        let q = along(t);
        let f = shore_field(q);
        if f > 0 {
            dock = q;
            break;
        }
        if f <= -p.spawn_offshore_mm {
            spawn_at = q;
        }
        t += 2000;
    }
    let spawn = Spawn {
        at: spawn_at,
        heading: trig::wrap(way + HALF),
        dock,
    };
    // the lane ahead of the spawn, which islands keep out of
    let lane_end = {
        let (hx, hy) = (trig::cos(spawn.heading) as i64, trig::sin(spawn.heading) as i64);
        let len = p.spawn_lane_mm.0 as i64 + 40_000;
        P::new(
            spawn_at.x + (hx * len / ONE as i64) as i32,
            spawn_at.y + (hy * len / ONE as i64) as i32,
        )
    };

    // 3. islands
    let mut r = Rng::stream(s, ISLANDS);
    let wanted = r.range(p.island_tries.0, p.island_tries.1);
    let mut islands: Vec<(Blob, i32)> = Vec::new();
    for _ in 0..wanted {
        for _try in 0..80 {
            let radius = r.range(p.island_radius_mm.0, p.island_radius_mm.1);
            let at = P::new(r.range(0, size - 1), r.range(0, size - 1));
            if shore_field(at) > -(radius + p.island_room_mm) {
                continue;
            }
            if at.dist(spawn_at) < (radius + 70_000) as i64
                || segment_dist(at, spawn_at, lane_end) < (radius + 30_000) as i64
            {
                continue;
            }
            if islands
                .iter()
                .any(|(b, rad)| at.dist(b.at) < (radius + rad + p.island_room_mm) as i64)
            {
                continue;
            }
            let stretch = r.range(1000, 1700);
            let blob = Blob {
                at,
                half_long: radius,
                half_short: (radius as i64 * 1000 / stretch as i64) as i32,
                tilt: r.range(0, TURN - 1),
                waves: (2..=4)
                    .map(|k| (k, r.range(40, 220 / (k - 1)), r.range(0, TURN - 1)))
                    .collect(),
            };
            islands.push((blob, radius));
            break;
        }
    }
    let land_at = |q: P| {
        if shore_field(q) > 0 {
            return true;
        }
        // an island's outline stays within twice its radius (the wobble and
        // stretch add at most half), so farther cells skip it
        islands.iter().any(|(b, radius)| {
            let reach = 2 * *radius as i64;
            q.dist2(b.at) <= reach * reach && {
                let rough = noise::value(ragged_seed ^ 0x5157, q.x, q.y, 9000) * 2500 / 1024;
                b.outside(q) + rough < 0
            }
        })
    };

    let cell_mm = p.cell_mm;
    let mut water: Vec<bool> = (0..n * n)
        .map(|i| {
            !land_at(P::new(
                (i % n) as i32 * cell_mm + cell_mm / 2,
                (i / n) as i32 * cell_mm + cell_mm / 2,
            ))
        })
        .collect();

    // 4. clean-up
    let spawn_cell = (spawn_at.y / cell_mm) as usize * n + (spawn_at.x / cell_mm) as usize;
    let (wet, count) = grid::regions(n, &water, false);
    let mut sizes = vec![0usize; count as usize + 1];
    for &l in &wet {
        sizes[l as usize] += 1;
    }
    let main = wet[spawn_cell];
    for i in 0..n * n {
        let l = wet[i];
        if l != 0 && l != main && sizes[l as usize] <= p.pocket_fill_cells {
            water[i] = false;
        }
    }
    let dry: Vec<bool> = water.iter().map(|w| !w).collect();
    let (land, count) = grid::regions(n, &dry, true);
    let mut sizes = vec![0usize; count as usize + 1];
    let mut edge = vec![false; count as usize + 1];
    for (i, &l) in land.iter().enumerate() {
        sizes[l as usize] += 1;
        let (x, y) = (i % n, i / n);
        if x == 0 || y == 0 || x == n - 1 || y == n - 1 {
            edge[l as usize] = true;
        }
    }
    for i in 0..n * n {
        let l = land[i] as usize;
        if l != 0 && !edge[l] && sizes[l] <= p.speck_cells {
            water[i] = true;
        }
    }

    let mut map = Map::from_water(setting, seed, n, cell_mm, &water, spawn);

    // 5. depth and height
    let bed = Rng::stream(s, BED).next_u64();
    for (i, &wet) in water.iter().enumerate() {
        let c = map.centre(i);
        if wet {
            let rough = noise::fractal(bed, c.x, c.y, 60_000, 2) * 1500 / 1024;
            let near = map.shore[i].min(12_000);
            // roughen less right by the shore, so the shallows stay shallow
            map.ground[i] = (map.ground[i] + rough * near / 12_000).min(-250);
        } else {
            let hills = (noise::fractal(bed ^ 0x4111, c.x, c.y, 160_000, 3) + 1024) * p.hill_mm / 2048;
            let ramp = map.shore[i].min(60_000);
            map.ground[i] += hills * ramp / 60_000;
        }
    }
    map.classify();

    // 6. reeds and trees
    map.reeds = reeds(&map, p, &mut Rng::stream(s, REEDS));
    map.trees = trees(
        &map,
        p,
        &mut Rng::stream(s, TREES),
        Rng::stream(s, TREES ^ 0x77).next_u64(),
    );
    map
}

/// Distance from `q` to the segment `a`-`b`.
fn segment_dist(q: P, a: P, b: P) -> i64 {
    let (abx, aby) = ((b.x - a.x) as i64, (b.y - a.y) as i64);
    let (aqx, aqy) = ((q.x - a.x) as i64, (q.y - a.y) as i64);
    let len2 = abx * abx + aby * aby;
    if len2 == 0 {
        return q.dist(a);
    }
    let t = ((aqx * abx + aqy * aby) * 1024 / len2).clamp(0, 1024);
    let c = P::new(a.x + (abx * t / 1024) as i32, a.y + (aby * t / 1024) as i32);
    q.dist(c)
}

/// How much land is round a cell: high in a cove.
fn cove(map: &Map, i: usize) -> i32 {
    let n = map.n as i32;
    let (x, y) = ((i % map.n) as i32, (i / map.n) as i32);
    let mut land = 0;
    for dy in -6..=6 {
        for dx in -6..=6 {
            let (tx, ty) = (x + dx, y + dy);
            if tx >= 0 && ty >= 0 && tx < n && ty < n && !map.cells[(ty * n + tx) as usize].water() {
                land += 1;
            }
        }
    }
    land
}

fn reeds(map: &Map, p: &Params, r: &mut Rng) -> Vec<ReedPatch> {
    let n = map.n;
    let candidates: Vec<usize> = (0..n * n)
        .filter(|&i| map.cells[i] == super::Cell::Shallow && (4000..=12_000).contains(&map.shore[i]))
        .collect();
    let mut out: Vec<ReedPatch> = Vec::new();
    if candidates.is_empty() {
        return out;
    }
    let wanted = r.range(p.reed_patches.0, p.reed_patches.1);
    for _ in 0..wanted {
        let mut best: Option<(i32, P)> = None;
        for _ in 0..40 {
            let i = candidates[r.below(candidates.len() as u32) as usize];
            let at = map.centre(i);
            if at.dist(map.spawn.at) < 50_000 || out.iter().any(|o| at.dist(o.at) < 70_000) {
                continue;
            }
            let score = cove(map, i) + r.range(0, 20);
            if best.is_none_or(|(b, _)| score > b) {
                best = Some((score, at));
            }
        }
        let Some((_, at)) = best else { continue };
        let radius = r.range(p.reed_radius_mm.0, p.reed_radius_mm.1);
        let mut clumps = Vec::new();
        for _ in 0..radius / 450 {
            let q = P::new(at.x + r.range(-radius, radius), at.y + r.range(-radius, radius));
            if q.dist2(at) > radius as i64 * radius as i64 {
                continue;
            }
            if let Some(j) = map.index(q) {
                if map.cells[j].water() && map.shore[j] <= 14_000 {
                    clumps.push(q);
                }
            }
        }
        if clumps.len() >= 3 {
            out.push(ReedPatch {
                at,
                radius_mm: radius,
                clumps,
            });
        }
    }
    out
}

fn trees(map: &Map, p: &Params, r: &mut Rng, forest: u64) -> Vec<Tree> {
    let sp = p.tree_spacing_mm;
    let steps = map.size_mm() / sp;
    let mut out = Vec::new();
    for gy in 0..steps {
        for gx in 0..steps {
            let at = P::new(gx * sp + r.range(0, sp - 1), gy * sp + r.range(0, sp - 1));
            let pick = r.below(1000);
            let scale = r.range(800, 1300);
            let Some(i) = map.index(at) else { continue };
            let from_water = map.shore[i];
            if map.cells[i].water() || from_water < 2500 || from_water > p.tree_reach_mm {
                continue;
            }
            let island = map.island[i] != 0;
            let dense = noise::fractal(forest, at.x, at.y, 90_000, 2);
            let keep = if island {
                850
            } else {
                let near = if from_water < 15_000 { 100 } else { 0 };
                (450 + dense * 550 / 1024 + near).clamp(60, 950)
            };
            if !r.chance(keep as u32) {
                continue;
            }
            let by_shore = from_water < 20_000;
            let kind = match (island, by_shore) {
                (true, _) if pick < 800 => TreeKind::Pine,
                (true, _) => TreeKind::Birch,
                (false, true) if pick < 400 => TreeKind::Birch,
                (false, true) if pick < 850 => TreeKind::Pine,
                (false, false) if pick < 550 => TreeKind::Pine,
                (false, false) if pick < 850 => TreeKind::Maple,
                (false, false) => TreeKind::Birch,
                _ => TreeKind::Maple,
            };
            out.push(Tree { at, kind, scale });
        }
    }
    out
}
