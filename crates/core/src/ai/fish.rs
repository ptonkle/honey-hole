//! Fishing: choosing a fish, stopping near it, the cast, the lure's depth,
//! the wait, the hook and the reel bar.
//!
//! - **Choosing** ([`choose`]): of the fish in sight, the most points for
//!   the time it would take (the drive, the bite, the reel by tier); not a
//!   fish on a line, by another seat's bobber, or failed at
//!   `ai.tries_per_fish` times. The current fish is kept unless another is
//!   half as good again.
//! - **Stopping**: it drives to `ai.cast_stop_mm` short of the fish, brakes,
//!   and switches to fishing mode.
//! - **Casting**: it swivels toward the fish and holds Space for the charge
//!   that reaches it, both off by up to the skill's aim and charge noise.
//! - **The lure** stops at the fish's depth; with nothing coming for
//!   `ai.lure_wait_ms` the line comes in. Space never goes down before the
//!   bite, so it never scares the fish.
//! - **The hook**: Space after the skill's reaction time from the bite
//!   showing.
//! - **The reel** ([`reel_hold`]): it reads the fish's marker the skill's lag
//!   late, aims off by a wandering noise, and holds Space while the catch
//!   zone rises slower than it wants.

use crate::fishboy::radar::TICK_MS;
use crate::gfx::fx;
use crate::map::P;
use crate::sim::fishing::view::{FishSeen, ReelView};
use crate::sim::fishing::{bow, Stage};
use crate::sim::input::SeatInput;
use crate::sim::{ticks, Boat, Mode};

use super::nav::{self, bearing};
use super::senses::Senses;
use super::{Brain, Goal};

/// What a fish in sight would be worth landed.
pub fn worth(s: &Senses, f: &FishSeen) -> i32 {
    let Some(k) = s.roster.get(&f.kind) else {
        return 0;
    };
    crate::sim::fishing::value(
        k.value,
        k.base_length_mm,
        f.length_mm,
        s.tuning.fishing.tier_value_pct.get(f.tier),
    )
}

/// A fish's points per minute from here: its worth over the drive, the
/// bite's wait and the reel.
pub fn rate(s: &Senses, at: P, f: &FishSeen) -> i64 {
    let d = at.dist(P::new(f.x_mm, f.y_mm));
    let drive_ms = d * 1000 / (s.rules.top as i64 * 3 / 4).max(1) + 2000;
    let ms = drive_ms + 8000 + s.tuning.ai.rival_reel_ms.get(f.tier) as i64;
    worth(s, f) as i64 * 60_000 / ms.max(1)
}

/// Some visible fish is within `mm` of the seat's lure.
pub fn fish_near_lure(s: &Senses, mm: i64) -> bool {
    let Some(l) = s.fishing.lines.iter().find(|l| l.seat == s.seat) else {
        return false;
    };
    let bob = P::new(l.bobber_x_mm, l.bobber_y_mm);
    s.fishing.fish.iter().any(|f| P::new(f.x_mm, f.y_mm).dist2(bob) <= mm * mm)
}

/// The best fish in sight to go for, if any.
pub fn choose(brain: &Brain, s: &Senses, at: P, current: Option<u32>) -> Option<u32> {
    let limit = s.tuning.ai.tries_per_fish;
    let taken = |f: &FishSeen| {
        let p = P::new(f.x_mm, f.y_mm);
        s.fishing.lines.iter().any(|l| {
            l.seat != s.seat
                && matches!(l.stage, Stage::Sinking | Stage::Set | Stage::Reeling)
                && p.dist2(P::new(l.bobber_x_mm, l.bobber_y_mm)) <= 8000 * 8000
        })
    };
    let mut best: Option<(i64, u32)> = None;
    let mut kept: Option<i64> = None;
    for f in &s.fishing.fish {
        if f.panic || brain.tried(f.id) >= limit || taken(f) {
            continue;
        }
        let r = rate(s, at, f);
        if Some(f.id) == current {
            kept = Some(r);
        }
        if best.is_none_or(|(b, _)| r > b) {
            best = Some((r, f.id));
        }
    }
    match (best, kept, current) {
        (Some((b, _)), Some(k), Some(c)) if b <= k * 3 / 2 => Some(c),
        (Some((_, id)), _, _) => Some(id),
        _ => None,
    }
}

/// Where the line would go out to reach `to` from the boat as it is: the
/// swivel from the bow and the reach, if the cast can make it.
fn cast_to(s: &Senses, b: &Boat, to: P) -> Option<(i32, i32)> {
    let t = &s.tuning.fishing;
    let rod = bow(b, s.rules.half_length, s.rules.half_beam);
    let d = rod.dist(to) as i32;
    if d < t.cast_min_mm || d > t.cast_max_mm - 1000 {
        return None;
    }
    let yaw = fx::wrap(bearing(rod, to) - b.heading);
    if yaw.abs() > s.rules.swivel_limit {
        return None;
    }
    if nav::land_between(s.map, rod, to) {
        return None;
    }
    Some((yaw, d))
}

/// Works the line toward the fish `fish`: drive, stop, cast, wait, hook, reel.
pub fn act(brain: &mut Brain, s: &Senses, nav: &nav::Nav, b: &Boat, fish: u32, out: &mut SeatInput) {
    if s.fishing.stage != Stage::Idle {
        tend_line(brain, s, b, out);
        return;
    }
    let Some(f) = s.fishing.fish.iter().find(|f| f.id == fish) else {
        brain.goal = Goal::Explore { to: b.at() };
        return;
    };
    let fp = P::new(f.x_mm, f.y_mm);
    let castable = cast_to(s, b, fp);
    let speed = b.speed_mm_per_s();
    let at = b.at();
    let stop_mm = s.tuning.ai.cast_stop_mm;
    if let Some((yaw, d)) = castable.filter(|_| at.dist(fp) <= stop_mm as i64 + 6000) {
        if b.mode == Mode::Driving {
            if b.forward_speed() > 400 {
                out.held.back = true;
            } else {
                out.pressed.mode = true;
            }
            return;
        }
        if speed > s.tuning.fishing.cast_max_speed_mm_per_s - 300 {
            return;
        }
        // the swivel, then the cast
        let step = s.rules.swivel;
        let off = yaw + brain.aim_off();
        let gap = fx::wrap(off - b.cast_yaw);
        if gap > step / 2 {
            out.held.left = true;
            return;
        }
        if gap < -step / 2 {
            out.held.right = true;
            return;
        }
        start_cast(brain, s, d, f.depth_mm, out);
        return;
    }
    // too far, or the wrong way: go nearer
    if b.mode == Mode::Fishing {
        out.pressed.mode = true;
        return;
    }
    let away = if at == fp { b.heading + crate::map::trig::HALF } else { bearing(fp, at) };
    let mut stop_at = crate::sim::fishing::lake::ahead(fp, away, stop_mm);
    if !nav.open_at(stop_at) || nav::land_between(s.map, stop_at, fp) {
        stop_at = nav.nearest_open(stop_at).map_or(fp, |c| nav.centre(c));
    }
    if at.dist(stop_at) < 6000 && speed < 1500 {
        brain.stalled += 1;
        if brain.stalled > ticks(4000) {
            brain.try_failed(fish);
            brain.stalled = 0;
        }
    }
    brain.drive_to(s, nav, b, stop_at, s.rules.top, true, out);
}

impl Brain {
    /// The aim's miss for this cast, drawn when the cast is planned.
    /// One miss per cast, held while it swivels.
    fn aim_off(&mut self) -> i32 {
        if let Some(a) = self.aim {
            return a;
        }
        let n = self.skill.aim_noise_deg * crate::map::trig::TURN / 360;
        let a = self.rng.range(-n, n);
        self.aim = Some(a);
        a
    }
}

/// Space down for the charge that reaches `d` mm, give or take the skill's
/// noise, and the lure's depth to stop at.
fn start_cast(brain: &mut Brain, s: &Senses, d: i32, depth: i32, out: &mut SeatInput) {
    let t = &s.tuning.fishing;
    let noise = brain.skill.charge_noise_pct;
    let d = d + (d as i64 * brain.rng.range(-noise, noise) as i64 / 100) as i32;
    let full = ticks(t.charge_full_ms).max(1) as i64;
    let power = ((d - t.cast_min_mm).max(0) as i64 * 1000 / (t.cast_max_mm - t.cast_min_mm).max(1) as i64).min(1000);
    let hold = ((power * full + 999) / 1000).max(1) as u32;
    out.pressed.cast = true;
    out.held.cast = true;
    brain.cast_release = Some(s.tick + hold);
    brain.aim = None;
    brain.lure_depth = depth;
    brain.lure_set = None;
    brain.hook_at = None;
    brain.reeling_in = false;
}

/// The line once it is out: the charge, the lure's stop, the wait, the
/// hook and the reel.
pub fn tend_line(brain: &mut Brain, s: &Senses, b: &Boat, out: &mut SeatInput) {
    let _ = b;
    let v = &s.fishing;
    let me = v.lines.iter().find(|l| l.seat == s.seat);
    match v.stage {
        Stage::Charging => {
            if brain.cast_release.is_some_and(|t| s.tick < t) {
                out.held.cast = true;
            } else {
                brain.cast_release = None;
            }
        }
        Stage::Sinking => {
            let Some(l) = me else { return };
            let sink = crate::sim::fishing::lake::per_tick(s.tuning.fishing.sink_mm_per_s);
            if l.lure_depth_mm + sink >= brain.lure_depth {
                out.pressed.cast = true;
                out.released.cast = true;
            }
        }
        Stage::Set => {
            let set = *brain.lure_set.get_or_insert(s.tick);
            if v.biting {
                let at = *brain.hook_at.get_or_insert_with(|| {
                    let ms = brain.rng.range(brain.skill.reaction_min_ms, brain.skill.reaction_max_ms);
                    s.tick + (ms / TICK_MS as i32) as u32
                });
                if s.tick >= at {
                    out.pressed.cast = true;
                    out.released.cast = true;
                }
                return;
            }
            let tug = me.is_some_and(|l| l.tug);
            let quiet = !tug && !fish_near_lure(s, 2500);
            let waited = s.tick >= set + ticks(s.tuning.ai.lure_wait_ms);
            if brain.reeling_in || (quiet && waited) {
                if !brain.reeling_in {
                    if let Goal::Fish { fish } = brain.goal {
                        brain.try_failed(fish);
                    }
                }
                brain.reeling_in = true;
                if s.tick % 5 == 0 {
                    out.pressed.cast = true;
                    out.released.cast = true;
                }
            }
        }
        Stage::Reeling => {
            brain.hook_at = None;
            if let Some(r) = &v.reel {
                let tick = s.tick;
                if reel_hold(brain, r, tick) {
                    out.held.cast = true;
                }
            }
        }
        Stage::Idle | Stage::Flying | Stage::Landed => {}
    }
}

/// Whether to hold Space this tick on the reel bar. It sees the fish's
/// marker the skill's lag late, aims off by a wandering noise, and lifts
/// while the catch zone's middle, a fifth of a second on at its present
/// speed, is under the marker.
pub fn reel_hold(brain: &mut Brain, r: &ReelView, tick: u32) -> bool {
    let lag = (ticks(brain.skill.reel_lag_ms) as usize).min(63);
    let h = &mut brain.reel;
    let k = (h.seen % 64) as usize;
    h.marks[k] = r.fish;
    h.seen += 1;
    let n = h.seen as usize;
    let mark = |back: usize| if n > back { h.marks[(n - 1 - back) % 64] } else { r.fish };
    // what it saw, carried on by the drift it saw then (over 3 ticks)
    let drift = (mark(lag) - mark(lag + 3)) / 3;
    let seen = mark(lag) + drift * lag as i32;
    if tick >= h.offset_until {
        let noise = brain.skill.reel_noise_permille;
        h.offset = brain.rng.range(-noise, noise);
        h.offset_until = tick + ticks(500);
    }
    let h = &mut brain.reel;
    let v = if h.zone_prev < 0 { 0 } else { (r.zone - h.zone_prev) * crate::sim::TICKS_PER_S };
    h.zone_prev = r.zone;
    let mid = r.zone + r.zone_h / 2 + v / 5;
    mid < seen + h.offset
}
