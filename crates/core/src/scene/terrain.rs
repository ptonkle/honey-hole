//! The terrain and the water surface: flat-shaded triangles over the map's
//! grid, two to a cell.
//!
//! The terrain's corners take the mean height of the cells round them, so
//! the shoreline is where the triangles cross the water's surface (y = 0),
//! not the cells' square edges. A cell is split along one diagonal or the
//! other by a checkerboard, which keeps the facets from all leaning one way.
//! Each face takes one colour from its height, its slope and the trees round
//! it: lake bed, beach, rock, forest floor or grass.

use super::look::LakeLook;
use super::{mm, Lake};
use crate::gfx::fx::{self, ONE};
use crate::gfx::geom::{v3, V3};
use crate::gfx::render::{self, Rgb};
use crate::map::noise;
use crate::map::rng::hash2;
use crate::map::{Map, P};
use crate::model::newell;

/// Each corner's height: the mean of the up to four cells that meet there.
pub fn corner_heights(map: &Map) -> Vec<i32> {
    let n = map.n;
    let mut out = Vec::with_capacity((n + 1) * (n + 1));
    for j in 0..=n {
        for i in 0..=n {
            let (mut sum, mut count) = (0i64, 0i64);
            for (ci, cj) in [(i.wrapping_sub(1), j.wrapping_sub(1)), (i, j.wrapping_sub(1)), (i.wrapping_sub(1), j), (i, j)] {
                if ci < n && cj < n {
                    sum += map.ground[cj * n + ci] as i64;
                    count += 1;
                }
            }
            out.push((sum / count) as i32);
        }
    }
    out
}

/// Whether a cell's diagonal runs from its south-west corner to its
/// north-east one (or else from south-east to north-west).
fn rising(i: usize, j: usize) -> bool {
    (i + j) % 2 == 0
}

/// The height (mm) on the drawn triangles at a map point.
pub fn height_at(lake: &Lake, p: P) -> i32 {
    let n = lake.map.n;
    let c = lake.map.cell_mm as i64;
    let max = n as i64 * c - 1;
    let (x, y) = ((p.x as i64).clamp(0, max), (p.y as i64).clamp(0, max));
    let (i, j) = ((x / c) as usize, (y / c) as usize);
    let (fx, fy) = (x % c, y % c);
    let h = |a: usize, b: usize| lake.heights[b * (n + 1) + a] as i64;
    let (h00, h10, h01, h11) = (h(i, j), h(i + 1, j), h(i, j + 1), h(i + 1, j + 1));
    let v = if rising(i, j) {
        if fx >= fy {
            h00 + (h10 - h00) * fx / c + (h11 - h10) * fy / c
        } else {
            h00 + (h11 - h01) * fx / c + (h01 - h00) * fy / c
        }
    } else if fx + fy <= c {
        h00 + (h10 - h00) * fx / c + (h01 - h00) * fy / c
    } else {
        h11 + (h01 - h11) * (c - fx) / c + (h10 - h11) * (c - fy) / c
    };
    v as i32
}

/// The two triangles of a cell, as corner (i, j) offsets.
fn cell_tris(i: usize, j: usize) -> [[(usize, usize); 3]; 2] {
    let (a, b, c, d) = ((i, j), (i + 1, j), (i, j + 1), (i + 1, j + 1));
    if rising(i, j) {
        [[a, b, d], [a, d, c]]
    } else {
        [[a, b, c], [b, d, c]]
    }
}

/// A triangle wound counter-clockwise seen from above (the shaders light
/// a face's front by its normal), and its unit normal, which faces up.
fn upward(p: [V3; 3]) -> ([V3; 3], V3) {
    let n = p[1].sub(p[0]).cross(p[2].sub(p[0]));
    if n.y < 0 {
        ([p[0], p[2], p[1]], n.neg().norm())
    } else {
        (p, n.norm())
    }
}

pub fn mix(a: Rgb, b: Rgb, t: i32) -> Rgb {
    [0, 1, 2].map(|k| fx::lerp(a[k], b[k], t))
}

/// 3t² − 2t³ of `t` clamped to 0..=ONE.
pub fn smooth(t: i32) -> i32 {
    let t = t.clamp(0, ONE);
    let t2 = fx::mul(t, t);
    3 * t2 - 2 * fx::mul(t2, t)
}

/// A colour brightened or darkened by up to `pct` percent, by `hash`.
fn jitter(c: Rgb, pct: i32, hash: u64) -> Rgb {
    let span = 2 * pct + 1;
    let k = 1000 + ((hash % span as u64) as i32 - pct) * 10;
    c.map(|v| (v * k / 1000).clamp(0, ONE))
}

/// The terrain's face colour.
struct Paint<'a> {
    look: &'a LakeLook,
    seed: u64,
}

impl Paint<'_> {
    fn face(&self, heights: [i32; 3], normal: V3, forest: bool, at: P, id: u64) -> Rgb {
        let g = &self.look.ground;
        let rgb = render::rgb;
        let mean = (heights[0] + heights[1] + heights[2]) / 3;
        let low = heights.iter().copied().min().unwrap_or(0);
        let base = if mean < 0 {
            let t = smooth((-mean as i64 * ONE as i64 / g.bed_deep_mm as i64) as i32);
            mix(rgb(g.bed_shallow), rgb(g.bed_deep), t)
        } else if low < g.sand_top_mm && mean < g.sand_top_mm {
            rgb(g.sand)
        } else if normal.y < g.rock_slope_pct * ONE / 100 {
            rgb(g.rock)
        } else if forest {
            rgb(g.forest)
        } else {
            let bump = noise::fractal(self.seed ^ 0x6d65_6164, at.x, at.y, g.meadow_mm, 2);
            mix(rgb(g.grass), rgb(g.meadow), smooth((bump + 1024) * ONE / 2048))
        };
        jitter(base, g.jitter_pct, hash2(self.seed ^ 0x6a69_7474, id as i32, (id >> 32) as i32))
    }
}

/// The terrain, the lake bed and the skirt round the map's edge, appended
/// to `out`. Returns its first vertex and count. `forest` marks the cells
/// with trees in or next to them.
pub fn ground(lake: &Lake, forest: &[bool], out: &mut Vec<i32>) -> [i32; 2] {
    let map = &lake.map;
    let n = map.n;
    let c = map.cell_mm;
    let first = render::vertex_count(out) as i32;
    let paint = Paint {
        look: &lake.look,
        seed: map.seed as u64,
    };
    let corner = |(i, j): (usize, usize)| {
        let h = lake.heights[j * (n + 1) + i];
        (lake.world(P::new(i as i32 * c, j as i32 * c), h), h)
    };
    for j in 0..n {
        for i in 0..n {
            for (k, tri) in cell_tris(i, j).iter().enumerate() {
                let pts = tri.map(corner);
                let (p, normal) = upward(pts.map(|q| q.0));
                let at = P::new(i as i32 * c + c / 2, j as i32 * c + c / 2);
                let id = ((j * n + i) * 2 + k) as u64;
                let colour = paint.face(pts.map(|q| q.1), normal, forest[j * n + i], at, id);
                render::push_tri(out, p, normal, colour);
            }
        }
    }
    // the skirt: the edge drops away, so the overview shows a block of land
    let skirt = render::rgb(lake.look.ground.skirt);
    let drop = |q: V3| v3(q.x, q.y - mm(lake.look.ground.skirt_mm), q.z);
    let edges: Vec<[(usize, usize); 2]> = (0..n)
        .flat_map(|k| {
            [
                [(k, 0), (k + 1, 0)],
                [(k + 1, n), (k, n)],
                [(0, k + 1), (0, k)],
                [(n, k), (n, k + 1)],
            ]
        })
        .collect();
    for [a, b] in edges {
        let (pa, pb) = (corner(a).0, corner(b).0);
        for q in [[pa, pb, drop(pb)], [pa, drop(pb), drop(pa)]] {
            render::push_tri(out, q, newell(&q), skirt);
        }
    }
    [first, render::vertex_count(out) as i32 - first]
}

/// The water's surface over every cell whose lowest corner is under the
/// look's shore overlap, at y = 0 (the shader lifts the waves), coloured by
/// the depth beneath. Returns its first vertex and count.
pub fn water(lake: &Lake, out: &mut Vec<i32>) -> [i32; 2] {
    let map = &lake.map;
    let n = map.n;
    let c = map.cell_mm;
    let w = &lake.look.water;
    let (shallow, deep) = (render::rgb(w.shallow), render::rgb(w.deep));
    let first = render::vertex_count(out) as i32;
    for j in 0..n {
        for i in 0..n {
            for tri in cell_tris(i, j) {
                let hs = tri.map(|(a, b)| lake.heights[b * (n + 1) + a]);
                if hs.iter().copied().min().unwrap_or(0) >= w.shore_overlap_mm {
                    continue;
                }
                let depth = (-(hs[0] + hs[1] + hs[2]) / 3).max(0);
                let t = smooth((depth as i64 * ONE as i64 / w.deep_mm as i64) as i32);
                let (p, _) = upward(tri.map(|(a, b)| lake.world(P::new(a as i32 * c, b as i32 * c), 0)));
                render::push_tri(out, p, V3::UP, mix(shallow, deep, t));
            }
        }
    }
    [first, render::vertex_count(out) as i32 - first]
}
