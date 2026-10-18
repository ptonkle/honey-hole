//! A stand-in for the match, so the Fish-Boy has something live to show
//! until the match core (`crates/core/src/sim/`) lands: Burt Lake from one
//! of its checked seeds, your boat and three rivals circling open water,
//! one of them cloaking now and then, an item buoy at a fair point, and a
//! fish caught every so often. It fills the same plain view structs the
//! match will ([`RadarView`], [`FishpediaRow`], [`CatchRow`]); once the
//! match fills them, this file goes.

use crate::gfx::fx::{self, TURN};
use crate::map::rng::hash2;
use crate::map::{self, Map, Setting, P};
use crate::roster::Roster;

use super::fishpedia::{CatchRow, FishpediaRow};
use super::radar::{BoatView, Ping, RadarTuning, RadarView, TICK_MS};

/// The demo lake: the first checked Burt Lake seed.
pub const SEED: u32 = 0;
/// Each boat circles a point of open water at least this far from land.
const OPEN_MM: i32 = 45_000;
/// A fish is caught somewhere on the lake every this many ticks; every
/// other one is yours.
const CATCH_EVERY: i64 = 600;
/// The rival in slot 3 is cloaked for the second half of each of these.
const CLOAK_EVERY: i64 = 1500;

/// One boat's circle: its middle, radius (mm), speed (mm a tick) and way
/// round.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Circle {
    at: P,
    r: i32,
    speed: i32,
    clockwise: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Demo {
    pub map: Map,
    pub tick: i64,
    /// the clock at tick 0, in µs
    pub start_us: i64,
    pub ping: Ping,
    circles: Vec<Circle>,
    pub buoy: Option<P>,
    /// the starting counts, by roster order
    counts: Vec<(String, u32)>,
    pub names: Vec<String>,
}

/// The middles of open water: each the open cell farthest from those
/// already picked, starting from `from`.
fn spread(map: &Map, from: P, count: usize) -> Vec<P> {
    let open: Vec<P> = (0..map.cells.len())
        .filter(|&i| map.cells[i].water() && map.shore[i] >= OPEN_MM)
        .map(|i| map.centre(i))
        .collect();
    let mut picked = vec![from];
    let mut out = Vec::new();
    for _ in 0..count {
        let best = open
            .iter()
            .max_by_key(|p| picked.iter().map(|q| p.dist2(*q)).min().unwrap_or(0));
        if let Some(&p) = best {
            picked.push(p);
            out.push(p);
        }
    }
    out
}

impl Demo {
    /// `names`: the four players' names, yours first.
    pub fn new(roster: &Roster, names: Vec<String>, start_us: i64) -> Demo {
        let map = map::generate(Setting::MichiganLake, SEED);
        let spawn = map.spawn.at;
        // yours: the open water nearest the spawn
        let mine = (0..map.cells.len())
            .filter(|&i| map.cells[i].water() && map.shore[i] >= OPEN_MM)
            .map(|i| map.centre(i))
            .min_by_key(|p| p.dist2(spawn))
            .unwrap_or(spawn);
        let mut middles = vec![mine];
        middles.extend(spread(&map, mine, 3));
        let circles: Vec<Circle> = middles
            .iter()
            .enumerate()
            .map(|(k, &at)| Circle {
                at,
                r: if k == 0 { 28_000 } else { 42_000 },
                speed: if k == 0 { 110 } else { 150 + 20 * k as i32 },
                clockwise: k % 2 == 1,
            })
            .collect();
        let players: Vec<P> = middles.clone();
        let buoy = map::buoy::fair_point(&map, &players, 7);
        let spread_counts = [3u32, 2, 4, 3, 3, 5, 5, 5, 5, 8, 7, 8, 7];
        let counts = roster
            .0
            .iter()
            .zip(spread_counts.iter().cycle())
            .map(|(k, &n)| (k.id.clone(), n))
            .collect();
        Demo {
            map,
            tick: 0,
            start_us,
            ping: Ping::default(),
            circles,
            buoy,
            counts,
            names,
        }
    }

    /// Runs the demo up to the clock (µs). A long gap (a hidden tab)
    /// skips ahead rather than running every tick.
    pub fn advance(&mut self, clock_us: i64, tuning: &RadarTuning) {
        let target = (clock_us - self.start_us) / (TICK_MS * 1000);
        if target - self.tick > 500 {
            self.tick = target - 500;
        }
        while self.tick < target {
            self.tick += 1;
            let view = self.view();
            self.ping.step(self.tick, &view, tuning);
        }
    }

    fn boat(&self, k: usize) -> BoatView {
        let c = self.circles[k];
        // the angle round the circle: speed over radius, in turns
        let travelled = c.speed as i64 * self.tick;
        let turn = (travelled * TURN as i64 / (c.r as i64 * 6283 / 1000)) as i32;
        let a = if c.clockwise { -turn } else { turn } + k as i32 * TURN / 5;
        let at = P::new(
            c.at.x + fx::mul(fx::cos(a), c.r),
            c.at.y + fx::mul(fx::sin(a), c.r),
        );
        let heading = a + if c.clockwise { -TURN / 4 } else { TURN / 4 };
        BoatView {
            at,
            heading: heading.rem_euclid(TURN),
            slot: k,
            name: self.names.get(k).cloned().unwrap_or_default(),
            cloaked: k == 3 && (self.tick / (CLOAK_EVERY / 2)) % 2 == 1,
        }
    }

    pub fn view(&self) -> RadarView {
        RadarView {
            me: 0,
            boats: (0..self.circles.len()).map(|k| self.boat(k)).collect(),
            buoy: self.buoy,
        }
    }

    /// Starts your ping, if its cooldown allows.
    pub fn ping(&mut self, tuning: &RadarTuning) -> bool {
        let view = self.view();
        let size = self.map.size_mm();
        let started = self.ping.start(self.tick, &view, size, tuning);
        if started {
            self.ping.step(self.tick, &view, tuning);
        }
        started
    }

    /// The catches so far: (kind index, yours, length mm).
    fn catches(&self, roster: &Roster) -> Vec<(usize, bool, i32)> {
        let mut left: Vec<u32> = self.counts.iter().map(|c| c.1).collect();
        let mut out = Vec::new();
        for n in 0..self.tick / CATCH_EVERY {
            let h = hash2(SEED as u64, n as i32, 3);
            let k = (h % left.len().max(1) as u64) as usize;
            if left[k] == 0 {
                continue;
            }
            left[k] -= 1;
            let base = roster.0.get(k).map_or(300, |f| f.base_length_mm);
            let length = base * (85 + (h >> 20) % 50) as i32 / 100;
            out.push((k, n % 2 == 0, length));
        }
        out
    }

    pub fn fishpedia(&self, roster: &Roster) -> Vec<FishpediaRow> {
        let caught = self.catches(roster);
        roster
            .0
            .iter()
            .enumerate()
            .map(|(k, f)| {
                let start = self.counts.get(k).map_or(0, |c| c.1);
                let gone = caught.iter().filter(|c| c.0 == k).count() as u32;
                FishpediaRow {
                    kind: f.id.clone(),
                    tier: f.tier,
                    left: start.saturating_sub(gone),
                }
            })
            .collect()
    }

    pub fn my_catches(&self, roster: &Roster) -> Vec<CatchRow> {
        self.catches(roster)
            .into_iter()
            .filter(|c| c.1)
            .filter_map(|(k, _, length_mm)| {
                roster.0.get(k).map(|f| CatchRow {
                    kind: f.id.clone(),
                    length_mm,
                })
            })
            .collect()
    }
}
