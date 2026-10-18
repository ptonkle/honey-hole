//! The reel minigame, Stardew Valley style: a vertical bar with the fish's
//! marker darting up and down it, and the angler's catch zone, which Space
//! lifts and gravity pulls down. The catch meter fills while the marker is
//! inside the zone and drains while it is out; the fish is landed when the
//! meter is full, and the line breaks only when it is empty.
//!
//! Units: positions on the bar run from 0 (bottom) to [`BAR`] (top), a
//! millionth of the bar each (the tuning names thousandths); the meter runs
//! from 0 to [`FULL`].
//!
//! A perfect reel, the marker always in the zone, fills the meter from
//! `start_pct` to full in the tier's `perfect_ms`, so each tier's reel time
//! is one tuning value. The marker's speed, how long it keeps a course and
//! how often it darts to the far end grow with the tier: bigger, rarer fish
//! fight harder.

use serde::Serialize;

use crate::map::fish::Tier;
use crate::map::rng::Rng;
use crate::sim::{ticks, TICKS_PER_S};

use super::tuning::ReelTuning;

/// The bar's height.
pub const BAR: i32 = 1_000_000;
/// The meter when full.
pub const FULL: i32 = 1_000_000;

/// What one tick of the reel came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Fighting,
    Landed,
    Broke,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reel {
    pub tier: Tier,
    /// the zone's bottom, and its height
    pub zone: i32,
    pub zone_h: i32,
    /// the zone's speed, millionths of the bar a second (up positive)
    pub zone_v: i32,
    /// the fish's marker, where it is heading, and until when
    pub marker: i32,
    pub target: i32,
    pub course_until: u32,
    pub meter: i32,
    /// the marker never moves and stays near the top (a test hook)
    pub tame: bool,
    /// ticks reeled
    pub ticks: u32,
}

impl Reel {
    pub fn new(tier: Tier, t: &ReelTuning, tame: bool) -> Reel {
        let zone_h = t.zone_permille * (BAR / 1000);
        let marker = if tame { BAR * 9 / 10 } else { BAR / 2 };
        Reel {
            tier,
            zone: (BAR / 2 - zone_h / 2).clamp(0, BAR - zone_h),
            zone_h,
            zone_v: 0,
            marker,
            target: marker,
            course_until: 0,
            meter: FULL / 100 * t.start_pct,
            tame,
            ticks: 0,
        }
    }

    /// The marker is inside the zone.
    pub fn in_zone(&self) -> bool {
        (self.zone..=self.zone + self.zone_h).contains(&self.marker)
    }

    /// The meter's gain a tick with the fish in the zone, at `power_pct` of
    /// the plain reel (the Power Reel doubles it).
    pub fn fill_per_tick(&self, t: &ReelTuning, power_pct: i32) -> i32 {
        let need = (FULL - FULL / 100 * t.start_pct) as i64;
        let n = ticks(t.perfect_ms.get(self.tier)).max(1) as i64;
        let plain = (need + n - 1) / n;
        (plain * power_pct as i64 / 100).max(1) as i32
    }

    pub fn drain_per_tick(&self, t: &ReelTuning) -> i32 {
        let n = ticks(t.drain_ms.get(self.tier)).max(1) as i64;
        ((FULL as i64 + n - 1) / n) as i32
    }

    /// One tick: the zone under Space (`lift`) or gravity, the fish's
    /// marker on its course, and the meter.
    pub fn step(&mut self, t: &ReelTuning, rng: &mut Rng, tick: u32, lift: bool, power_pct: i32) -> Outcome {
        self.ticks += 1;
        // the zone: millionths of the bar per second, gained per tick
        let per_tick = |per_s2: i32| per_s2 * (BAR / 1000) / TICKS_PER_S;
        let top = t.zone_top_speed_per_s * (BAR / 1000);
        self.zone_v = if lift {
            self.zone_v + per_tick(t.lift_per_s2)
        } else {
            self.zone_v - per_tick(t.gravity_per_s2)
        }
        .clamp(-top, top);
        self.zone += self.zone_v / TICKS_PER_S;
        if self.zone <= 0 {
            self.zone = 0;
            self.zone_v = self.zone_v.max(0);
        }
        if self.zone >= BAR - self.zone_h {
            self.zone = BAR - self.zone_h;
            self.zone_v = self.zone_v.min(0);
        }

        // the fish: a new course when it gets there or its time is up
        if !self.tame {
            if self.marker == self.target || tick >= self.course_until {
                let span = t.fish_turn_ms.get(self.tier);
                self.course_until = tick + ticks(rng.range(span / 2, span * 3 / 2)).max(1);
                self.target = if rng.chance(t.fish_dart_pct.get(self.tier) as u32 * 10) {
                    // a dart to the far end of the bar
                    if self.marker > BAR / 2 {
                        rng.range(0, BAR / 4)
                    } else {
                        rng.range(BAR * 3 / 4, BAR)
                    }
                } else {
                    (self.marker + rng.range(-BAR * 3 / 10, BAR * 3 / 10)).clamp(0, BAR)
                };
            }
            let step = t.fish_speed_per_s.get(self.tier) * (BAR / 1000) / TICKS_PER_S;
            self.marker = crate::gfx::fx::approach(self.marker, self.target, step.max(1));
        }

        if self.in_zone() {
            self.meter = (self.meter + self.fill_per_tick(t, power_pct)).min(FULL);
        } else {
            self.meter = (self.meter - self.drain_per_tick(t)).max(0);
        }
        if self.meter >= FULL {
            Outcome::Landed
        } else if self.meter <= 0 {
            Outcome::Broke
        } else {
            Outcome::Fighting
        }
    }

    /// How hard the line pulls, 0 to 1000, for the line's whine: steady
    /// with the fish in the zone, rising toward a break as the meter
    /// empties with it out.
    pub fn tension(&self) -> i32 {
        if self.in_zone() {
            350
        } else {
            350 + (650 * (FULL - self.meter) as i64 / FULL as i64) as i32
        }
    }
}
