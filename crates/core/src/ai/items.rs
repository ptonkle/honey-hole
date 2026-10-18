//! The AI and the items: when a seat goes for the buoy, when it uses its
//! item, and its hand on the minigame ([`super::hand`]), all through the
//! same [`SeatInput`] a person makes. Every threshold is `ai.items` in
//! `data/tuning.json`.
//!
//! - **The buoy** ([`buoy_worth`]): it is on every radar at all times. The
//!   brain goes for it when the trip is no longer than `buoy_worth_ms`
//!   (`buoy_worth_held_ms` with an item held, which the new one would
//!   replace), and never with a fish at its lure or on its line, nor while
//!   it hunts or flees.
//! - **Using** ([`want_use`]), Shift when:
//!   - Nitrous: a drive to somewhere `nitrous_min_mm` off, or a hunt
//!     `nitrous_hunt_min_mm` off;
//!   - Teleporter: a hunt's target `teleport_min_mm` off with
//!     `teleport_lead_ms` of its reel left (landing `teleport_short_mm`
//!     short, for the run-in), or a rare or legendary fish an Enhanced
//!     Radar marked that far; held `teleport_explore_after_ms` with neither,
//!     the far water it means to explore;
//!   - Cloaking Tech: on a hunt `cloak_hunt_min_mm` off, and before the
//!     cast at a fish of `cloak_min_tier` (`cloak_before_cast`; else on
//!     hooking one);
//!   - Power Reel: on hooking a fish of `power_reel_min_tier` (the reel
//!     holds while its minigame plays, SECOND-ORDER R5.31), or with
//!     `power_reel_on_hook` false before the cast at one (R4.11);
//!   - Camera: its offer after a catch of `camera_min_tier`, or any catch
//!     with under `camera_any_left_ms` left; Esc keeps it for later;
//!   - Enhanced Radar: exploring with no fish in sight for
//!     `radar_blind_ms`; then it drives to the best fish marked.
//! - **The minigame** and a teleport's spot on the radar: the hand
//!   ([`during`]).

use crate::fishboy::radar::MAP_RECT;
use crate::fishboy::RADAR_MARGIN_MM;
use crate::map::fish::Tier;
use crate::map::P;
use crate::sim::fishing::lake::ahead;
use crate::sim::fishing::Stage;
use crate::sim::input::SeatInput;
use crate::sim::items::Item;
use crate::sim::{ticks, Boat, Mode, TICK_MS};

use super::hand::Hand;
use super::nav::bearing;
use super::senses::Senses;
use super::{Brain, Goal};

/// A brain's items: its hand and its plans.
#[derive(Debug, Clone)]
pub struct Mind {
    pub hand: Hand,
    /// what the hand is on (a minigame's item, or the teleport's choice)
    on: Option<(Item, bool)>,
    /// where a teleport is to go
    pub teleport_to: Option<P>,
    /// the camera's offer was first seen
    offer_seen: Option<u32>,
    /// exploring with no fish in sight since
    blind_since: Option<u32>,
    /// the item held, and since when
    held_since: Option<(Item, u32)>,
    /// Shift was last pressed (the item takes a tick to start)
    pressed_at: Option<u32>,
    /// the radar's square of the map, once worked out
    region: Option<(P, i32)>,
    /// counts, for the playtests and tests: Shift presses by item
    pub shifts: [u32; 6],
}

impl Mind {
    pub fn new(seed: u64, seat: usize) -> Mind {
        Mind {
            hand: Hand::new(seed, seat),
            on: None,
            teleport_to: None,
            offer_seen: None,
            blind_since: None,
            held_since: None,
            pressed_at: None,
            region: None,
            shifts: [0; 6],
        }
    }
}

fn ms(tick_count: u32) -> i64 {
    tick_count as i64 * TICK_MS as i64
}

/// Whether the buoy at `buoy` is worth the trip from `at` now.
pub fn buoy_worth(brain: &Brain, s: &Senses, b: &Boat, buoy: P) -> bool {
    let t = &s.tuning.ai.items;
    if !t.use_items || matches!(brain.goal, Goal::Hunt { .. } | Goal::Flee { .. }) {
        return false;
    }
    // a seat with a fish on, or one at the lure, does not leave
    let stage = s.fishing.stage;
    let quiet_lure = stage == Stage::Set && !s.fishing.biting && !super::fish::fish_near_lure(s, 4000);
    if !(stage == Stage::Idle || quiet_lure) {
        return false;
    }
    let worth = if s.items.held.is_some() || s.items.charging {
        t.buoy_worth_held_ms
    } else {
        t.buoy_worth_ms
    };
    let d = b.at().dist(buoy);
    // the way round islands, at most of top speed, and the start
    let trip = d * 13 / 10 * 1000 / (s.rules.top as i64 * 4 / 5).max(1) + 1500;
    trip <= worth as i64
}

/// The input while the hand is busy: a minigame, or a teleport's spot on
/// the radar. `None` otherwise.
pub fn during(brain: &mut Brain, s: &Senses) -> Option<SeatInput> {
    let tick = s.tick;
    let skill = s.tuning.ai.items.hand.get(brain.difficulty).clone();
    let mind = &mut brain.items;
    if let Some((item, sight)) = &s.items.playing {
        if mind.on != Some((*item, false)) {
            mind.on = Some((*item, false));
            mind.hand.reset(tick, &skill);
        }
        return Some(mind.hand.play(sight, tick, &skill));
    }
    if s.items.choosing {
        if mind.on != Some((Item::Teleporter, true)) {
            mind.on = Some((Item::Teleporter, true));
            mind.hand.reset(tick, &skill);
        }
        let to = mind.teleport_to.unwrap_or(s.map.spawn.at);
        let region = *mind
            .region
            .get_or_insert_with(|| crate::fishboy::radar::region(s.map, RADAR_MARGIN_MM));
        let at = screen_of(region, to);
        return Some(mind.hand.click_at(at, tick, &skill));
    }
    mind.on = None;
    // the camera's offer: a photo for a rare catch, else keep it
    if !s.items.offer || !s.tuning.ai.items.use_items || !s.tuning.ai.items.use_held {
        mind.offer_seen = None;
        return None;
    }
    let t = &s.tuning.ai.items;
    let seen = *mind.offer_seen.get_or_insert(tick);
    if tick < seen + ticks(skill.pause_ms) {
        return None;
    }
    mind.offer_seen = None;
    let mut out = SeatInput::default();
    let rare = s.items.last_catch.is_some_and(|c| c >= t.camera_min_tier.tier());
    if rare || s.items.left_ms < t.camera_any_left_ms {
        out.pressed.item = true;
        mind.shifts[4] += 1;
    } else {
        out.pressed.cancel = true;
    }
    Some(out)
}

/// A map point on the radar's map, in its screen units (the inverse of
/// `radar::map_point`).
pub fn screen_of(region: (P, i32), p: P) -> (i32, i32) {
    let (lo, side) = (region.0, region.1.max(1) as i64);
    let x = MAP_RECT.x as i64 + (p.x - lo.x) as i64 * MAP_RECT.w as i64 / side;
    let y = MAP_RECT.y as i64 + (lo.y as i64 + side - p.y as i64) * MAP_RECT.h as i64 / side;
    (
        (x as i32).clamp(MAP_RECT.x, MAP_RECT.right() - 1),
        (y as i32).clamp(MAP_RECT.y, MAP_RECT.bottom() - 1),
    )
}

/// The best fish an Enhanced Radar marked to go for, if any: the rarest,
/// then the nearest, not where the boat already is.
pub fn mark_to_visit(s: &Senses, at: P) -> Option<P> {
    s.items
        .marks
        .iter()
        .filter(|m| m.at.dist(at) > 20_000)
        .max_by_key(|m| (m.tier, -m.at.dist(at)))
        .map(|m| m.at)
}

/// Adds this tick's item keys to `out`: Shift to use the item, or the
/// camera offer's answer.
pub fn decide(brain: &mut Brain, s: &Senses, b: &Boat, out: &mut SeatInput) {
    let t = &s.tuning.ai.items;
    if !t.use_items || !t.use_held {
        return;
    }
    let tick = s.tick;
    if s.items.offer {
        return;
    }
    // how long it has seen no fish while exploring
    let blind = matches!(brain.goal, Goal::Explore { .. }) && s.fishing.fish.is_empty();
    if blind {
        brain.items.blind_since.get_or_insert(tick);
    } else {
        brain.items.blind_since = None;
    }
    let Some(item) = s.items.held else {
        brain.items.held_since = None;
        return;
    };
    let since = match brain.items.held_since {
        Some((i, t0)) if i == item => t0,
        _ => {
            brain.items.held_since = Some((item, tick));
            tick
        }
    };
    if s.items.playing.is_some() || s.items.choosing || s.items.charging || b.wrecked() {
        return;
    }
    if brain.items.pressed_at.is_some_and(|p| tick < p + 10) {
        return;
    }
    if want_use(brain, s, b, item, since) {
        out.pressed.item = true;
        brain.items.pressed_at = Some(tick);
        let k = Item::ALL.iter().position(|&i| i == item).unwrap_or(0);
        brain.items.shifts[k] += 1;
    }
}

/// Where the hunted `seat` is, as the brain last knew it.
fn hunted_at(brain: &Brain, s: &Senses, seat: usize) -> Option<P> {
    s.rivals
        .iter()
        .find(|r| r.seat == seat)
        .map(|r| r.at)
        .or(brain.known[seat].map(|k| k.0))
}

/// Whether to use `item` (held since tick `since`) now; for a teleport it
/// sets where to.
pub fn want_use(brain: &mut Brain, s: &Senses, b: &Boat, item: Item, since: u32) -> bool {
    let t = &s.tuning.ai.items;
    let at = b.at();
    let stage = s.fishing.stage;
    let reeling_tier = s
        .fishing
        .reel
        .as_ref()
        .filter(|_| stage == Stage::Reeling)
        .map(|r| r.tier);
    let rare_on = |min: Tier| reeling_tier.is_some_and(|tier| tier >= min);
    let driving = b.mode == Mode::Driving && stage == Stage::Idle;
    match item {
        Item::Nitrous => {
            if s.items.nitrous || !driving {
                return false;
            }
            match brain.goal {
                Goal::Hunt { seat, .. } => {
                    hunted_at(brain, s, seat).is_some_and(|p| p.dist(at) >= t.nitrous_hunt_min_mm as i64)
                }
                Goal::Explore { to } | Goal::Buoy { at: to } => to.dist(at) >= t.nitrous_min_mm as i64,
                _ => false,
            }
        }
        Item::Teleporter => {
            if !driving {
                return false;
            }
            let far = t.teleport_min_mm as i64;
            let to = match brain.goal {
                Goal::Hunt { seat, until } => {
                    let left = ms(until.saturating_sub(s.tick));
                    hunted_at(brain, s, seat)
                        .filter(|p| p.dist(at) >= far && left >= t.teleport_lead_ms as i64)
                        .map(|p| ahead(p, bearing(p, at), t.teleport_short_mm))
                }
                Goal::Explore { to } => {
                    let marked = s
                        .items
                        .marks
                        .iter()
                        .filter(|m| m.tier >= Tier::Rare && m.at.dist(at) >= far)
                        .max_by_key(|m| m.tier)
                        .map(|m| ahead(m.at, bearing(m.at, at), s.tuning.ai.cast_stop_mm));
                    let stale = ms(s.tick.saturating_sub(since)) >= t.teleport_explore_after_ms as i64;
                    marked.or((stale && to.dist(at) >= far).then_some(to))
                }
                _ => None,
            };
            brain.items.teleport_to = to;
            to.is_some()
        }
        Item::Cloak => {
            if s.items.cloaked {
                return false;
            }
            let min = t.cloak_min_tier.tier();
            match brain.goal {
                Goal::Hunt { seat, .. } => {
                    hunted_at(brain, s, seat).is_some_and(|p| p.dist(at) >= t.cloak_hunt_min_mm as i64)
                }
                Goal::Fish { fish } if t.cloak_before_cast => at_cast_spot(s, b, fish, min),
                _ => !t.cloak_before_cast && rare_on(min),
            }
        }
        Item::PowerReel => {
            if s.items.power_reel {
                return false;
            }
            let min = t.power_reel_min_tier.tier();
            if t.power_reel_on_hook {
                return rare_on(min);
            }
            // before the cast at a rare fish, at the spot
            match brain.goal {
                Goal::Fish { fish } => at_cast_spot(s, b, fish, min),
                _ => false,
            }
        }
        Item::Camera => false,
        Item::EnhancedRadar => {
            matches!(brain.goal, Goal::Explore { .. })
                && driving
                && s.items.marks.is_empty()
                && brain
                    .items
                    .blind_since
                    .is_some_and(|t0| ms(s.tick.saturating_sub(t0)) >= t.radar_blind_ms as i64)
        }
    }
}

/// At the spot to cast at `fish`, of `min` tier or rarer: stopped in
/// fishing mode with the line in.
fn at_cast_spot(s: &Senses, b: &Boat, fish: u32, min: Tier) -> bool {
    b.mode == Mode::Fishing
        && s.fishing.stage == Stage::Idle
        && s.fishing.fish.iter().any(|f| f.id == fish && f.tier >= min)
}
