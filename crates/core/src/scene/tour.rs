//! The camera tour: a loop over open water from the spawn past each island
//! and reed bed and back, for the Lake preview. It keeps to water at least
//! [`ROOM_MM`] from land, so a boat drawn on it never runs aground.

use super::Lake;
use crate::map::trig;
use crate::map::{Map, P};

/// The tour keeps this far from land.
pub const ROOM_MM: i32 = 10_000;
/// It passes islands and reeds about this far off.
const PASS_MM: (i32, i32) = (14_000, 30_000);
/// Each point is the mean of this many on either side, three times over.
const SMOOTH: usize = 4;

/// The breadth-first way over roomy water from cell `a` to cell `b`, as
/// cells, or none.
fn way(map: &Map, open: &[bool], a: usize, b: usize) -> Option<Vec<usize>> {
    let n = map.n;
    let mut from = vec![usize::MAX; n * n];
    from[a] = a;
    let mut queue = std::collections::VecDeque::from([a]);
    while let Some(c) = queue.pop_front() {
        if c == b {
            let mut out = vec![b];
            let mut k = b;
            while k != a {
                k = from[k];
                out.push(k);
            }
            out.reverse();
            return Some(out);
        }
        let (x, y) = ((c % n) as i32, (c / n) as i32);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let (u, v) = (x + dx, y + dy);
            if u < 0 || v < 0 || u >= n as i32 || v >= n as i32 {
                continue;
            }
            let d = (v * n as i32 + u) as usize;
            if open[d] && from[d] == usize::MAX {
                from[d] = c;
                queue.push_back(d);
            }
        }
    }
    None
}

/// The roomy water cell nearest `t` that lies `PASS_MM` from land.
fn pass_by(map: &Map, open: &[bool], t: P) -> Option<usize> {
    (0..map.n * map.n)
        .filter(|&i| open[i] && (PASS_MM.0..=PASS_MM.1).contains(&map.shore[i]))
        .min_by_key(|&i| map.centre(i).dist2(t))
}

/// The tour's points (mm) and the length to each.
pub fn path(map: &Map) -> (Vec<P>, Vec<i64>) {
    let open: Vec<bool> = (0..map.n * map.n)
        .map(|i| map.cells[i].water() && map.shore[i] >= ROOM_MM)
        .collect();
    let Some(start) = map.index(map.spawn.at).filter(|&i| open[i]).or_else(|| pass_by(map, &open, map.spawn.at))
    else {
        return (vec![map.spawn.at], vec![0]);
    };
    let mut stops: Vec<usize> = map
        .islands
        .iter()
        .map(|i| i.at)
        .chain(map.reeds.iter().map(|r| r.at))
        .filter_map(|t| pass_by(map, &open, t))
        .collect();
    // nearest first, from the spawn
    let mut cells = vec![start];
    let mut here = start;
    while !stops.is_empty() {
        let k = (0..stops.len())
            .min_by_key(|&k| map.centre(stops[k]).dist2(map.centre(here)))
            .unwrap_or(0);
        let next = stops.remove(k);
        if let Some(w) = way(map, &open, here, next) {
            cells.extend_from_slice(&w[1..]);
            here = next;
        }
    }
    if let Some(w) = way(map, &open, here, start) {
        cells.extend_from_slice(&w[1..]);
    }
    let mut pts: Vec<P> = cells.iter().map(|&c| map.centre(c)).collect();
    // smooth the corners of the grid away, keeping the loop closed
    let len = pts.len();
    if len > 2 * SMOOTH + 1 {
        for _ in 0..3 {
            pts = (0..len)
                .map(|i| {
                    let (mut x, mut y) = (0i64, 0i64);
                    for k in 0..=2 * SMOOTH {
                        let q = pts[(i + len + k - SMOOTH) % len];
                        x += q.x as i64;
                        y += q.y as i64;
                    }
                    let w = (2 * SMOOTH + 1) as i64;
                    P::new((x / w) as i32, (y / w) as i32)
                })
                .collect();
        }
    }
    let mut at = vec![0i64];
    for k in 1..pts.len() {
        at.push(at[k - 1] + pts[k].dist(pts[k - 1]));
    }
    (pts, at)
}

/// Where the tour's boat is `ms` in at the look's tour speed, and its
/// heading (map angle units).
pub fn pose(lake: &Lake, ms: u64) -> (P, i32) {
    let pts = &lake.tour;
    let total = *lake.tour_at.last().unwrap_or(&0);
    if pts.len() < 2 || total == 0 {
        return (lake.map.spawn.at, lake.map.spawn.heading);
    }
    let speed = lake.look.camera.tour_mm_per_s as i64;
    let at = |s: i64| {
        let s = s.rem_euclid(total);
        let k = lake.tour_at.partition_point(|&a| a <= s).clamp(1, pts.len() - 1);
        let (a, b) = (pts[k - 1], pts[k]);
        let span = (lake.tour_at[k] - lake.tour_at[k - 1]).max(1);
        let t = s - lake.tour_at[k - 1];
        P::new(
            a.x + ((b.x - a.x) as i64 * t / span) as i32,
            a.y + ((b.y - a.y) as i64 * t / span) as i32,
        )
    };
    let s = ms as i64 * speed / 1000;
    let (here, behind, ahead) = (at(s), at(s - 6000), at(s + 6000));
    let heading = trig::atan2((ahead.y - behind.y) as i64, (ahead.x - behind.x) as i64);
    (here, heading)
}
