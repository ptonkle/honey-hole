//! What one seat sees of the fishing: the fish it can see under the water,
//! and every seat's line (the bobbers and lines show to everyone; the
//! charge and the reel bar are the seat's own). The drawing, the radar and
//! the AI read this, never the fish list itself, so no one sees a fish the
//! rules hide.
//!
//! A fish shows within `sight_shallow_mm` of the seat's boat when it swims
//! at `sight_shallow_depth_mm` or shallower, fading in proportion to
//! `sight_deep_mm` at `sight_deep_depth_mm` and deeper; with the seat's
//! line in the water, also within `sight_lure_mm` of its lure (the
//! underwater camera). The fish on the seat's own line always shows.

use serde::Serialize;

use crate::map::fish::Tier;
use crate::sim::{Match, TICK_MS};

use super::lake::FishState;
use super::reel::{BAR, FULL};
use super::tuning::FishTuning;
use super::{bow, charge_permille, Stage};

/// One fish in sight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FishSeen {
    pub id: u32,
    /// the roster id, which is its model's id
    pub kind: String,
    pub tier: Tier,
    pub x_mm: i32,
    pub y_mm: i32,
    pub depth_mm: i32,
    pub length_mm: i32,
    /// its kind's base length (the model's own length)
    pub base_length_mm: i32,
    pub heading: i32,
    /// how far into its swim loop, ms
    pub swim_ms: u32,
    /// on a line: the panicked swim
    pub panic: bool,
}

/// One seat's line as anyone sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LineView {
    pub seat: usize,
    pub stage: Stage,
    /// the bobber, mm, and how high over the water (in its flight)
    pub bobber_x_mm: i32,
    pub bobber_y_mm: i32,
    pub bobber_up_mm: i32,
    pub lure_depth_mm: i32,
    /// a fish is nibbling or biting now (the bobber dips)
    pub tug: bool,
}

/// The reel bar, in thousandths of the bar from the bottom.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReelView {
    pub zone: i32,
    pub zone_h: i32,
    pub fish: i32,
    /// the catch meter, thousandths
    pub meter: i32,
    pub in_zone: bool,
    /// 0 to 1000, for the line's whine
    pub tension: i32,
    pub tier: Tier,
}

/// What one seat sees.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FishingView {
    pub seat: usize,
    pub fish: Vec<FishSeen>,
    pub lines: Vec<LineView>,
    /// this seat's own line
    pub stage: Stage,
    /// the charge, 0 to 1000, while Space is held
    pub charge: i32,
    pub reel: Option<ReelView>,
    /// a fish is biting: Space hooks it
    pub biting: bool,
    /// the seat's boat is too fast to cast
    pub too_fast: bool,
    /// the fish on the line, or being held up
    pub on_line: Option<FishSeen>,
}

/// How far a fish this deep shows from a boat.
pub fn sight_mm(t: &FishTuning, depth_mm: i32) -> i32 {
    let (lo, hi) = (t.sight_shallow_depth_mm, t.sight_deep_depth_mm);
    let d = depth_mm.clamp(lo, hi) - lo;
    t.sight_shallow_mm + ((t.sight_deep_mm - t.sight_shallow_mm) as i64 * d as i64 / (hi - lo) as i64) as i32
}

impl Match {
    /// What `seat` sees of the fishing.
    pub fn fishing_view(&self, seat: usize) -> FishingView {
        let fi = &self.fishing;
        let ms = self.tick.wrapping_mul(TICK_MS as u32);
        let me = self.boat(seat);
        let own = fi.lines.get(seat);
        let seen = |id: usize| {
            let f = &fi.fish[id];
            let kind = fi.roster.0.get(f.kind);
            FishSeen {
                id: f.id,
                kind: kind.map_or(String::new(), |k| k.id.clone()),
                tier: f.tier,
                x_mm: f.at.x,
                y_mm: f.at.y,
                depth_mm: f.depth_mm,
                length_mm: f.length_mm,
                base_length_mm: kind.map_or(f.length_mm, |k| k.base_length_mm),
                heading: f.heading,
                swim_ms: ms.wrapping_add(f.phase_ms),
                panic: matches!(f.state, FishState::Hooked { .. }),
            }
        };
        let fish = fi
            .fish
            .iter()
            .enumerate()
            .filter(|(_, f)| f.in_lake())
            .filter(|(_, f)| {
                if f.state == (FishState::Hooked { seat }) {
                    return true;
                }
                let near_boat = me.is_some_and(|b| {
                    let s = sight_mm(&fi.fish_tuning, f.depth_mm) as i64;
                    !b.wrecked() && f.at.dist2(b.at()) <= s * s
                });
                let near_lure = own.is_some_and(|l| {
                    let s = fi.fish_tuning.sight_lure_mm as i64;
                    l.in_water() && f.at.dist2(l.bobber) <= s * s
                });
                near_boat || near_lure
            })
            .map(|(k, _)| seen(k))
            .collect();
        let r = &self.rules;
        let lines = fi
            .lines
            .iter()
            .enumerate()
            .filter(|(_, l)| !matches!(l.stage, Stage::Idle | Stage::Charging))
            .map(|(s, l)| {
                let (at, up) = if l.stage == Stage::Flying {
                    // along the cast, in an arc a fifth as high as it is long
                    let span = (l.lands - l.flew).max(1) as i64;
                    let done = (self.tick.saturating_sub(l.flew) as i64).min(span);
                    let x = l.from.x + ((l.to.x - l.from.x) as i64 * done / span) as i32;
                    let y = l.from.y + ((l.to.y - l.from.y) as i64 * done / span) as i32;
                    let arc = l.length_mm as i64 / 5 * 4 * done * (span - done) / (span * span);
                    (crate::map::P::new(x, y), arc as i32)
                } else if l.stage == Stage::Landed {
                    let b = self.boat(s);
                    (b.map_or(l.bobber, |b| bow(b, r.half_length, r.half_beam)), 0)
                } else {
                    (l.bobber, 0)
                };
                LineView {
                    seat: s,
                    stage: l.stage,
                    bobber_x_mm: at.x,
                    bobber_y_mm: at.y,
                    bobber_up_mm: up,
                    lure_depth_mm: l.lure_depth_mm,
                    tug: l
                        .interest
                        .is_some_and(|i| i.biting_until.is_some() || (i.arrived && self.tick + 5 >= i.next)),
                }
            })
            .collect();
        let stage = own.map_or(Stage::Idle, |l| l.stage);
        let charge = match own {
            Some(l) if l.stage == Stage::Charging => charge_permille(&fi.tuning, self.tick - l.since),
            _ => 0,
        };
        let reel = own.and_then(|l| l.reel.as_ref()).map(|r| ReelView {
            zone: r.zone / (BAR / 1000),
            zone_h: r.zone_h / (BAR / 1000),
            fish: r.marker / (BAR / 1000),
            meter: r.meter / (FULL / 1000),
            in_zone: r.in_zone(),
            tension: r.tension(),
            tier: r.tier,
        });
        let on_line = own.and_then(|l| l.fish).map(seen);
        FishingView {
            seat,
            fish,
            lines,
            stage,
            charge,
            reel,
            biting: own.is_some_and(|l| l.interest.is_some_and(|i| i.biting_until.is_some())),
            too_fast: me.is_some_and(|b| b.speed_mm_per_s() > fi.tuning.cast_max_speed_mm_per_s),
            on_line,
        }
    }
}
