//! Where an item buoy goes: open water about as far by boat from every
//! player.
//!
//! Distances are boat trips over water (around islands and points), not
//! straight lines. A point is fair when the longest trip and the shortest
//! differ by at most [`tolerance`] of their mean: 8 m, or 5% of the mean
//! when that is more. Of the fair points, those nearest [`TARGET_MM`] away
//! win (so the buoy is a race, not a gift), and the seed picks among the
//! closest few. With nobody fair (a player cut off, say) the fairest
//! point wins.

use super::grid::{self, FAR};
use super::rng::Rng;
use super::{Map, P};

/// The buoy's usual trip length from the players.
pub const TARGET_MM: i32 = 150_000;
/// No buoy nearer than this to any player.
pub const NEAR_MM: i32 = 60_000;
/// Water this far from land counts as open.
pub const OPEN_MM: i32 = 12_000;

/// The largest fair spread for a mean trip, in mm.
pub fn tolerance(mean_mm: i32) -> i32 {
    (mean_mm / 20).max(8000)
}

/// The buoy's place for players at `players`; `None` with no players or
/// no open water.
pub fn fair_point(map: &Map, players: &[P], seed: u64) -> Option<P> {
    if players.is_empty() {
        return None;
    }
    let n = map.n;
    let water: Vec<bool> = map.cells.iter().map(|c| c.water()).collect();
    let fields: Vec<Vec<i32>> = players
        .iter()
        .filter_map(|&p| nearest_water(map, p))
        .map(|i| grid::distances(n, map.cell_mm, &[i], &water))
        .collect();
    if fields.is_empty() {
        return None;
    }
    // (unfairness beyond the tolerance, distance from the target, cell)
    let mut scored: Vec<(i32, i32, usize)> = Vec::new();
    for i in 0..n * n {
        if !water[i] || map.shore[i] < OPEN_MM {
            continue;
        }
        let ds: Vec<i32> = fields.iter().map(|f| f[i]).collect();
        if ds.contains(&FAR) {
            continue;
        }
        let lo = *ds.iter().min().unwrap_or(&0);
        let hi = *ds.iter().max().unwrap_or(&0);
        if lo < NEAR_MM {
            continue;
        }
        let mean = (ds.iter().map(|&d| d as i64).sum::<i64>() / ds.len() as i64) as i32;
        let over = (hi - lo - tolerance(mean)).max(0);
        scored.push((over, (mean - TARGET_MM).abs(), i));
    }
    scored.sort();
    let best = scored.first()?;
    let close: Vec<&(i32, i32, usize)> = scored
        .iter()
        .take_while(|s| s.0 == best.0 && s.1 <= best.1 + 10_000)
        .take(12)
        .collect();
    let pick = close[Rng::stream(seed, 21).below(close.len() as u32) as usize];
    Some(map.centre(pick.2))
}

/// The water cell nearest a point (a player on a dock still counts).
fn nearest_water(map: &Map, p: P) -> Option<usize> {
    if let Some(i) = map.index(p).filter(|&i| map.cells[i].water()) {
        return Some(i);
    }
    (0..map.n * map.n)
        .filter(|&i| map.cells[i].water())
        .min_by_key(|&i| map.centre(i).dist2(p))
}
