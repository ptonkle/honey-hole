//! Getting about: a path over a coarse grid of the map, and the steering
//! that follows it without grounding.
//!
//! **The grid** ([`Nav`]) has cells of `ai.path_cell_mm` (a whole number of
//! map cells). A cell is open when every map cell in it is water at least
//! `ai.path_clear_mm` from land; water nearer land than `ai.path_roomy_mm`
//! costs a path more, so paths keep to open water and squeeze through a
//! channel only when they must. [`Nav::path`] is A* over the open cells
//! (eight neighbours, no cut corners), then pulled straight wherever the
//! water between two points is `ai.path_roomy_mm` from land ([`clear_line`]).
//!
//! **Steering** ([`steer`]) turns toward the next waypoint and sets the
//! throttle for a wanted speed: slower for a sharp turn, for a stop, and
//! under `ai.careful_speed_pct` of top speed with land within
//! `ai.look_ahead_mm` ahead (along the bow and along its way through the
//! water) or within `ai.near_land_mm` at all, which is under the speed that
//! grounds a boat.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::gfx::fx;
use crate::map::trig::{self, isqrt};
use crate::map::{Map, P};
use crate::sim::input::SeatInput;
use crate::sim::{Boat, Rules};

use super::tuning::AiTuning;

/// The coarse grid of open water.
#[derive(Debug, Clone)]
pub struct Nav {
    /// cells per side
    pub n: usize,
    pub cell_mm: i32,
    pub open: Vec<bool>,
    /// a step's cost into a cell, in percent of its length
    pub cost: Vec<i32>,
    /// the clearance a straight stretch of path needs
    clear_mm: i32,
}

/// Whether the water from `a` to `b` is clear: every point along it, each
/// 2 m, is water at least `clear_mm` from land (by its map cell).
pub fn clear_line(map: &Map, a: P, b: P, clear_mm: i32) -> bool {
    let d = a.dist(b);
    let steps = (d / 2000).max(1);
    for k in 0..=steps {
        let p = P::new(
            a.x + ((b.x - a.x) as i64 * k / steps) as i32,
            a.y + ((b.y - a.y) as i64 * k / steps) as i32,
        );
        match map.index(p) {
            Some(i) if map.cells[i].water() && map.shore[i] >= clear_mm => {}
            _ => return false,
        }
    }
    true
}

/// Whether land lies between `a` and `b` (sight across the lake), each 4 m.
pub fn land_between(map: &Map, a: P, b: P) -> bool {
    let d = a.dist(b);
    let steps = (d / 4000).max(1);
    (1..steps).any(|k| {
        let p = P::new(
            a.x + ((b.x - a.x) as i64 * k / steps) as i32,
            a.y + ((b.y - a.y) as i64 * k / steps) as i32,
        );
        !map.wet(p)
    })
}

/// How far ahead along `heading` the first land is, up to `reach` (each
/// 2 m), or `reach` when there is none.
pub fn land_ahead(map: &Map, from: P, heading: i32, reach: i32) -> i32 {
    let mut d = 2000;
    while d <= reach {
        if !map.wet(crate::sim::fishing::lake::ahead(from, heading, d)) {
            return d;
        }
        d += 2000;
    }
    reach
}

impl Nav {
    pub fn new(map: &Map, t: &AiTuning) -> Nav {
        let k = (t.path_cell_mm / map.cell_mm).max(1) as usize;
        let n = map.n.div_ceil(k);
        let mut open = vec![false; n * n];
        let mut cost = vec![100; n * n];
        for cy in 0..n {
            for cx in 0..n {
                let mut ok = true;
                let mut near = i32::MAX;
                for fy in cy * k..(cy * k + k).min(map.n) {
                    for fx_ in cx * k..(cx * k + k).min(map.n) {
                        let i = fy * map.n + fx_;
                        if !map.cells[i].water() || map.shore[i] < t.path_clear_mm {
                            ok = false;
                        }
                        near = near.min(map.shore[i]);
                    }
                }
                let c = cy * n + cx;
                open[c] = ok;
                if near < t.path_roomy_mm {
                    cost[c] = 300;
                }
            }
        }
        Nav {
            n,
            cell_mm: k as i32 * map.cell_mm,
            open,
            cost,
            clear_mm: t.path_roomy_mm,
        }
    }

    fn cell_of(&self, p: P) -> Option<usize> {
        if p.x < 0 || p.y < 0 {
            return None;
        }
        let (x, y) = ((p.x / self.cell_mm) as usize, (p.y / self.cell_mm) as usize);
        (x < self.n && y < self.n).then_some(y * self.n + x)
    }

    pub fn centre(&self, c: usize) -> P {
        P::new(
            (c % self.n) as i32 * self.cell_mm + self.cell_mm / 2,
            (c / self.n) as i32 * self.cell_mm + self.cell_mm / 2,
        )
    }

    /// The open cell nearest `p`, within a few cells.
    pub fn nearest_open(&self, p: P) -> Option<usize> {
        let c = self.cell_of(p)?;
        if self.open[c] {
            return Some(c);
        }
        let (x, y) = ((c % self.n) as i32, (c / self.n) as i32);
        let mut best: Option<(i64, usize)> = None;
        for r in 1..=4i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let (tx, ty) = (x + dx, y + dy);
                    if tx < 0 || ty < 0 || tx >= self.n as i32 || ty >= self.n as i32 {
                        continue;
                    }
                    let j = ty as usize * self.n + tx as usize;
                    let d = self.centre(j).dist2(p);
                    if self.open[j] && best.is_none_or(|(b, _)| d < b) {
                        best = Some((d, j));
                    }
                }
            }
            if best.is_some() {
                break;
            }
        }
        best.map(|(_, j)| j)
    }

    /// Whether a point is in an open cell.
    pub fn open_at(&self, p: P) -> bool {
        self.cell_of(p).is_some_and(|c| self.open[c])
    }

    /// The waypoints from `from` to `to`, ending at `to` itself, or `None`
    /// when no open water joins them.
    pub fn path(&self, map: &Map, from: P, to: P) -> Option<Vec<P>> {
        if clear_line(map, from, to, self.clear_mm) {
            return Some(vec![to]);
        }
        let start = self.nearest_open(from)?;
        let goal = self.nearest_open(to)?;
        let n = self.n;
        let mut g = vec![i32::MAX; n * n];
        let mut came = vec![u32::MAX; n * n];
        let mut heap = BinaryHeap::new();
        let h = |c: usize| {
            let (dx, dy) = (
                ((c % n) as i32 - (goal % n) as i32).abs(),
                ((c / n) as i32 - (goal / n) as i32).abs(),
            );
            (dx.max(dy) * 100 + dx.min(dy) * 41) * self.cell_mm / 100
        };
        g[start] = 0;
        heap.push(Reverse((h(start), start as u32)));
        while let Some(Reverse((_, c))) = heap.pop() {
            let c = c as usize;
            if c == goal {
                break;
            }
            let (x, y) = ((c % n) as i32, (c / n) as i32);
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                let (tx, ty) = (x + dx, y + dy);
                if tx < 0 || ty < 0 || tx >= n as i32 || ty >= n as i32 {
                    continue;
                }
                let j = ty as usize * n + tx as usize;
                if !self.open[j] {
                    continue;
                }
                if dx != 0
                    && dy != 0
                    && !(self.open[y as usize * n + tx as usize] && self.open[ty as usize * n + x as usize])
                {
                    continue;
                }
                let step = if dx != 0 && dy != 0 { 141 } else { 100 };
                let ng = g[c] + step * self.cell_mm / 100 * self.cost[j] / 100;
                if ng < g[j] {
                    g[j] = ng;
                    came[j] = c as u32;
                    heap.push(Reverse((ng + h(j), j as u32)));
                }
            }
        }
        if g[goal] == i32::MAX {
            return None;
        }
        let mut cells = vec![goal];
        while *cells.last()? != start {
            cells.push(came[*cells.last()?] as usize);
        }
        cells.reverse();
        let mut raw: Vec<P> = cells.iter().map(|&c| self.centre(c)).collect();
        raw.push(to);
        // pulled straight: from each anchor, the farthest point in clear water
        let mut out = Vec::new();
        let mut at = from;
        let mut i = 0;
        while i < raw.len() {
            let mut j = i;
            while j + 1 < raw.len() && clear_line(map, at, raw[j + 1], self.clear_mm) {
                j += 1;
            }
            out.push(raw[j]);
            at = raw[j];
            i = j + 1;
        }
        Some(out)
    }
}

/// `a` percent of `v`.
pub fn pct(v: i32, a: i32) -> i32 {
    (v as i64 * a as i64 / 100) as i32
}

/// The bearing from one point to another, in map angle units.
pub fn bearing(from: P, to: P) -> i32 {
    trig::atan2((to.y - from.y) as i64, (to.x - from.x) as i64)
}

/// The fastest speed (mm/s) from which the boat stops within `d` mm,
/// braking at `decel` mm/s², down to `floor` at no distance.
pub fn stop_speed(d: i32, decel: i32, floor: i32) -> i32 {
    let v2 = floor as i64 * floor as i64 + 2 * decel as i64 * d.max(0) as i64;
    isqrt(v2).min(i32::MAX as i64) as i32
}

/// The boat's braking, mm/s², at the share the AI uses.
pub fn decel(r: &Rules, t: &AiTuning) -> i32 {
    // r.brake is mm/s lost a tick
    pct(r.brake * crate::sim::TICKS_PER_S, t.brake_use_pct)
}

/// Turns toward `to` and holds the throttle for `want` mm/s, slower for a
/// sharp turn and near land ahead (unless `reckless`, for a ram's run-in).
/// Writes A, D, W and S into `out`.
pub fn steer(map: &Map, r: &Rules, t: &AiTuning, b: &Boat, to: P, want: i32, reckless: bool, out: &mut SeatInput) {
    let at = b.at();
    let err = fx::wrap(bearing(at, to) - b.heading);
    let dead = r.turn.max(1);
    if err > dead {
        out.held.left = true;
    } else if err < -dead {
        out.held.right = true;
    }
    let mut want = want;
    let turn_slow = pct(r.top, 45);
    if err.abs() > trig::TURN / 8 {
        want = want.min(turn_slow);
    }
    if !reckless {
        let mut ahead = land_ahead(map, at, b.heading, t.look_ahead_mm);
        // and along the way it is really going, which a turn leaves behind
        let v = b.vel();
        if b.speed_mm_per_s() > 2000 {
            let way = trig::atan2(v[1] as i64, v[0] as i64);
            ahead = ahead.min(land_ahead(map, at, way, t.look_ahead_mm));
        }
        // by the shore itself, the hull's side would scrape it
        let near = map.index(at).is_none_or(|i| map.shore[i] < t.near_land_mm);
        if near {
            ahead = 0;
        }
        if ahead < t.look_ahead_mm {
            let careful = pct(r.top, t.careful_speed_pct);
            want = want.min(stop_speed(ahead - 8000, decel(r, t), careful));
        }
    }
    let speed = b.forward_speed();
    if speed < want - 300 {
        out.held.forward = true;
    } else if speed > want + 800 && speed > 0 {
        out.held.back = true;
    }
}
