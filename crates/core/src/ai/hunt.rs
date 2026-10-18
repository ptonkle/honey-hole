//! Hunting and evading: whether a rival's hook is worth a ram, the ram
//! itself, and whether to drop the line and flee a rival coming fast.
//!
//! - **The hunt** ([`worth_hunting`]): a rival's hook pops up. The brain
//!   goes only if it can arrive before the rival's reel ends (`ai.rival_reel_ms`
//!   by tier, less the time since the hook), and if the fish's points,
//!   scaled by its aggression, outweigh what its own fishing
//!   (`ai.fishing_rate_per_min`) would earn in the trip there and back. A
//!   rammed angler's fish swims free close by, so a hunt can end in a catch.
//! - **The ram** ([`chase`]): a path toward where it last knew the rival;
//!   within `ai.ram_runup_mm` with clear water between, straight at it at
//!   full throttle. A run-in under `ai.ram_speed_pct` of top speed close in
//!   turns away for another run-up.
//! - **Evading** ([`should_flee`]): a rival in sight or earshot closing
//!   faster than `ai.threat_speed_pct` of top speed, arriving before its own
//!   reel would end, is a threat; the brain drops the line and flees by its
//!   skill's `evade_pct` chance.

use crate::map::fish::Tier;
use crate::map::trig;
use crate::map::P;
use crate::sim::fishing::lake::ahead;
use crate::sim::fishing::Stage;
use crate::sim::input::SeatInput;
use crate::sim::{Boat, Mode};

use super::nav::{self, bearing, pct, Nav};
use super::senses::{RivalSeen, Senses};
use super::tuning::Skill;
use super::Brain;

/// What a fish of this kind is worth at its middle length.
pub fn kind_worth(s: &Senses, kind: &str, tier: Tier) -> i32 {
    let Some(k) = s.roster.get(kind) else {
        return 0;
    };
    let t = &s.tuning.fish;
    let mid = k.base_length_mm as i64 * (t.length_min_pct + t.length_max_pct) as i64 / 200;
    crate::sim::fishing::value(k.value, k.base_length_mm, mid as i32, s.tuning.fishing.tier_value_pct.get(tier))
}

/// The time to drive `d` mm from rest at a ram's speed, ms, allowing for
/// the way round islands.
pub fn eta_ms(s: &Senses, d: i64) -> i64 {
    let speed = pct(s.rules.top, s.tuning.ai.ram_speed_pct).max(1) as i64;
    d * 13 / 10 * 1000 / speed + 2500
}

/// Whether a rival's hook of `kind` (`tier`), `since_ms` ago, at `rival`, is
/// worth hunting from `at`.
pub fn worth_hunting(
    s: &Senses,
    skill: &Skill,
    at: P,
    rival: P,
    kind: &str,
    tier: Tier,
    since_ms: i32,
) -> bool {
    let t = &s.tuning.ai;
    if s.fishing.stage == Stage::Reeling || skill.aggression_pct == 0 {
        return false;
    }
    let eta = eta_ms(s, at.dist(rival));
    let left = t.rival_reel_ms.get(tier) as i64 - since_ms as i64;
    if eta >= left {
        return false;
    }
    // denying the rival the fish, and half a chance of catching it after
    let gain = kind_worth(s, kind, tier) as i64 * skill.aggression_pct as i64 * 3 / 200;
    let cost = t.fishing_rate_per_min as i64 * 2 * eta / 60_000;
    gain > cost
}

/// The point a rival's ram would come from, if `r` is a threat to `b`.
pub fn should_flee(s: &Senses, b: &Boat, r: &RivalSeen) -> Option<P> {
    let t = &s.tuning.ai;
    if r.wrecked || r.ghosted {
        return None;
    }
    let at = b.at();
    let d = at.dist(r.at);
    if d == 0 || d > t.hear_mm as i64 {
        return None;
    }
    // its speed toward us
    let (ux, uy) = ((at.x - r.at.x) as i64, (at.y - r.at.y) as i64);
    let closing = (r.vel[0] as i64 * ux + r.vel[1] as i64 * uy) / d;
    if closing < pct(s.rules.top, t.threat_speed_pct) as i64 {
        return None;
    }
    let eta = d * 1000 / closing.max(1);
    if let (Stage::Reeling, Some(rv)) = (s.fishing.stage, s.fishing.reel.as_ref()) {
        let need = (1000 - rv.meter).max(0) as i64;
        let perfect = s.tuning.fishing.reel.perfect_ms.get(rv.tier) as i64;
        let start = s.tuning.fishing.reel.start_pct as i64 * 10;
        let left = need * perfect * 3 / 2 / (1000 - start).max(1);
        if left < eta {
            return None;
        }
    }
    Some(r.at)
}

/// Somewhere to flee to: open water away from `from`.
pub fn flee_point(s: &Senses, nav: &Nav, at: P, from: P) -> P {
    let away = if at == from { 0 } else { bearing(from, at) };
    let eighth = trig::TURN / 8;
    for turn in [0, 1, -1, 2, -2, 3, -3] {
        let q = ahead(at, away + turn * eighth, 80_000);
        if nav.open_at(q) && nav.path(s.map, at, q).is_some() {
            return q;
        }
    }
    s.map.spawn.at
}

/// One tick of a hunt for `seat`'s boat.
pub fn chase(brain: &mut Brain, s: &Senses, nav: &Nav, b: &Boat, seat: usize, out: &mut SeatInput) {
    if b.mode == Mode::Fishing {
        // the line comes in: Q drops whatever is on it
        out.pressed.mode = true;
        return;
    }
    let t = &s.tuning.ai;
    let at = b.at();
    let seen = s.rivals.iter().find(|r| r.seat == seat);
    let Some((where_, vel, _)) = seen.map(|r| (r.at, r.vel, s.tick)).or(brain.known[seat]) else {
        brain.goal = super::Goal::Explore { to: at };
        return;
    };
    if let Some(u) = brain.runup {
        if at.dist(u) < t.waypoint_mm as i64 || seen.is_none() {
            brain.runup = None;
        } else {
            brain.drive_to(s, nav, b, u, s.rules.top, false, out);
            return;
        }
    }
    let d = at.dist(where_);
    // where it will be when the ram arrives
    let lead_ms = d * 1000 / s.rules.top.max(1) as i64;
    let aim = P::new(
        where_.x + (vel[0] as i64 * lead_ms / 1000) as i32,
        where_.y + (vel[1] as i64 * lead_ms / 1000) as i32,
    );
    let straight = seen.is_some() && d < t.ram_runup_mm as i64 && !nav::land_between(s.map, at, aim);
    if !straight {
        brain.drive_to(s, nav, b, aim, s.rules.top, false, out);
        return;
    }
    let ram = pct(s.rules.top, t.ram_speed_pct);
    let speed = b.forward_speed();
    let off = crate::gfx::fx::wrap(bearing(at, aim) - b.heading).abs();
    if d < 20_000 && (speed < pct(s.rules.top, s.tuning.wreck.ram_wreck_pct) || off > trig::TURN / 12) {
        // too slow or off line this close: round again
        let back = ahead(where_, bearing(where_, at), t.ram_runup_mm * 4 / 5);
        brain.runup = Some(if nav.open_at(back) {
            back
        } else {
            nav.nearest_open(back).map_or(back, |c| nav.centre(c))
        });
        brain.path.clear();
        return;
    }
    // land beyond the target does not slow the run-in
    let beyond = nav::land_ahead(s.map, at, b.heading, t.look_ahead_mm) as i64 > d + 6000;
    nav::steer(s.map, s.rules, t, b, aim, ram.max(s.rules.top), beyond, out);
}
