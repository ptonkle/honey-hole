//! The `ai` section of `data/tuning.json`: how the AI players drive, fish,
//! hunt and evade, and what each difficulty changes. [`crate::tuning::Tuning`]
//! holds it and calls [`check`] with its own checker, so a bad value is
//! refused by name.

use serde::Deserialize;

use crate::sim::Difficulty;
use crate::tuning::Checker;

/// What one difficulty sets.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skill {
    /// from the bite showing to Space, at random between these
    pub reaction_min_ms: i32,
    pub reaction_max_ms: i32,
    /// how stale its read of the fish's marker on the reel bar is
    pub reel_lag_ms: i32,
    /// how far off (thousandths of the bar) its aim on the marker wanders
    pub reel_noise_permille: i32,
    /// the cast's direction and length miss by up to these
    pub aim_noise_deg: i32,
    pub charge_noise_pct: i32,
    /// 0 to 100: how readily it hunts a rival's hook
    pub aggression_pct: i32,
    /// the chance it drops its line and flees a rival coming fast
    pub evade_pct: i32,
    /// it raises the Fish-Boy to ping about this often
    pub ping_every_ms: i32,
}

/// The skills by difficulty.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skills {
    pub easy: Skill,
    pub normal: Skill,
    pub hard: Skill,
}

impl Skills {
    pub fn get(&self, d: Difficulty) -> &Skill {
        match d {
            Difficulty::Easy => &self.easy,
            Difficulty::Normal => &self.normal,
            Difficulty::Hard => &self.hard,
        }
    }
}

/// One number per tier (common, uncommon, rare, legendary).
pub use crate::sim::fishing::tuning::PerTier;

/// The whole `ai` section.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// the path-finding grid's cell, a whole number of map cells
    pub path_cell_mm: i32,
    /// a path cell is open when every map cell in it is water at least this
    /// far from land
    pub path_clear_mm: i32,
    /// water nearer land than this costs a path more
    pub path_roomy_mm: i32,
    /// the A* path searches the crew runs a tick, between all its players
    /// (one waits a tick when more need one at once: `ai::nav`)
    pub path_searches_per_tick: i32,
    /// a waypoint is reached within this
    pub waypoint_mm: i32,
    /// with land this near ahead the boat keeps to careful_speed_pct of top
    /// speed, under the grounding speed
    pub look_ahead_mm: i32,
    /// and with land this near on any side
    pub near_land_mm: i32,
    pub careful_speed_pct: i32,
    /// slowing for a stop or a bend, at this share of the boat's braking
    pub brake_use_pct: i32,
    /// a ram is driven at this share of top speed or more
    pub ram_speed_pct: i32,
    /// the straight run-in before a ram
    pub ram_runup_mm: i32,
    /// it stops this far from the fish it means to cast at
    pub cast_stop_mm: i32,
    /// with no fish coming to the lure this long, the line comes in
    pub lure_wait_ms: i32,
    /// a fish it has failed at this many times is left alone
    pub tries_per_fish: i32,
    /// it hears a rival's motor this far off (the sound's 5 s at top speed)
    pub hear_mm: i32,
    /// a rival closing faster than this share of top speed is a threat
    pub threat_speed_pct: i32,
    /// a threat arriving within this is dodged
    pub flee_within_ms: i32,
    /// what a wreck costs in fishing time (the trip back from the spawn)
    pub respawn_cost_ms: i32,
    /// it flees for this long, then fishes again
    pub flee_ms: i32,
    /// what it expects a rival's reel of each tier to take
    pub rival_reel_ms: PerTier,
    /// the points a second it reckons its own fishing earns, against which a
    /// hunt is weighed
    pub fishing_rate_per_min: i32,
    /// how long a ping's sighting stays worth chasing
    pub sighting_keep_ms: i32,
    pub skill: Skills,
    /// the buoy, the items and their minigames (`ai::items`, `ai::hand`)
    pub items: ItemsAi,
}

/// A tier named in the tuning (`"rare"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TierName {
    Common,
    Uncommon,
    Rare,
    Legendary,
}

impl TierName {
    pub fn tier(self) -> crate::map::fish::Tier {
        use crate::map::fish::Tier;
        match self {
            TierName::Common => Tier::Common,
            TierName::Uncommon => Tier::Uncommon,
            TierName::Rare => Tier::Rare,
            TierName::Legendary => Tier::Legendary,
        }
    }
}

/// How one difficulty's hand plays a minigame through the pointer.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hand {
    /// the pointer moves up to this fast, canvas units a second
    pub speed_per_s: i32,
    /// it waits this long after each grab, drop, click or photo, and
    /// before its first move
    pub pause_ms: i32,
    /// its aim is off by up to this, canvas units
    pub aim_noise: i32,
    /// its click on a closing ring is early or late by up to this
    pub timing_noise_ms: i32,
    /// it circles the Power Reel this many thousandths of a turn a second
    pub reel_permille_per_s: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hands {
    pub easy: Hand,
    pub normal: Hand,
    pub hard: Hand,
}

impl Hands {
    pub fn get(&self, d: Difficulty) -> &Hand {
        match d {
            Difficulty::Easy => &self.easy,
            Difficulty::Normal => &self.normal,
            Difficulty::Hard => &self.hard,
        }
    }
}

/// When the AI goes for the buoy and uses each item.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemsAi {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// false: the AI never goes for a buoy nor uses an item (for playtests)
    pub use_items: bool,
    /// false: it fetches buoys but never uses what they give (to measure
    /// what the trips cost)
    pub use_held: bool,
    /// a buoy is worth a trip of up to this long with no item held, and of
    /// up to `buoy_worth_held_ms` holding one (a new one replaces it)
    pub buoy_worth_ms: i32,
    pub buoy_worth_held_ms: i32,
    /// Nitrous on a drive at least this far, or on a hunt at least this far
    pub nitrous_min_mm: i32,
    pub nitrous_hunt_min_mm: i32,
    /// the Teleporter to a hunt's target or a marked fish at least this
    /// far, when the rival's reel has `teleport_lead_ms` left (the
    /// minigame and the charge); it lands `teleport_short_mm` short of a
    /// hunted rival, for the run-in
    pub teleport_min_mm: i32,
    pub teleport_lead_ms: i32,
    pub teleport_short_mm: i32,
    /// held this long with no hunt or marked fish, it teleports to the far
    /// water it means to explore
    pub teleport_explore_after_ms: i32,
    /// Cloaking Tech on a hunt `cloak_hunt_min_mm` off or more, and for a
    /// fish of `cloak_min_tier` or rarer: before the cast at it
    /// (`cloak_before_cast`), else on hooking it
    pub cloak_hunt_min_mm: i32,
    pub cloak_min_tier: TierName,
    pub cloak_before_cast: bool,
    /// the Power Reel on a fish of this tier or rarer: at the hook
    /// (`power_reel_on_hook`, safe while the reel holds for the minigame),
    /// else before the cast at one
    pub power_reel_min_tier: TierName,
    pub power_reel_on_hook: bool,
    /// the camera's photo for a catch of this tier or rarer, or for any
    /// catch with under `camera_any_left_ms` of the match left
    pub camera_min_tier: TierName,
    pub camera_any_left_ms: i32,
    /// Enhanced Radar after this long exploring with no fish in sight
    pub radar_blind_ms: i32,
    pub hand: Hands,
}

fn check_skill(c: &mut Checker, key: &str, s: &Skill) {
    let k = |n: &str| format!("ai.skill.{key}.{n}");
    c.range(&k("reaction_min_ms"), s.reaction_min_ms, 0, 5000)
        .range(&k("reaction_max_ms"), s.reaction_max_ms, s.reaction_min_ms, 5000)
        .range(&k("reel_lag_ms"), s.reel_lag_ms, 0, 2000)
        .range(&k("reel_noise_permille"), s.reel_noise_permille, 0, 500)
        .range(&k("aim_noise_deg"), s.aim_noise_deg, 0, 90)
        .range(&k("charge_noise_pct"), s.charge_noise_pct, 0, 100)
        .range(&k("aggression_pct"), s.aggression_pct, 0, 100)
        .range(&k("evade_pct"), s.evade_pct, 0, 100)
        .range(&k("ping_every_ms"), s.ping_every_ms, 1000, 600_000);
}

/// The section's range checks. Whatever these say, the AI drives carefully
/// under the match's grounding speed and rams over its wrecking speed
/// (`nav::careful`, `hunt::ram_speed`).
pub fn check(c: &mut Checker, a: &AiTuning) {
    c.range("ai.path_cell_mm", a.path_cell_mm, 1000, 32_000)
        .range("ai.path_clear_mm", a.path_clear_mm, 0, 40_000)
        .range("ai.path_roomy_mm", a.path_roomy_mm, a.path_clear_mm, 80_000)
        .range("ai.path_searches_per_tick", a.path_searches_per_tick, 1, 16)
        .range("ai.waypoint_mm", a.waypoint_mm, 500, 40_000)
        .range("ai.look_ahead_mm", a.look_ahead_mm, 1000, 200_000)
        .range("ai.near_land_mm", a.near_land_mm, 0, 40_000)
        .range("ai.careful_speed_pct", a.careful_speed_pct, 5, 100)
        .range("ai.brake_use_pct", a.brake_use_pct, 10, 100)
        .range("ai.ram_speed_pct", a.ram_speed_pct, 1, 100)
        .range("ai.ram_runup_mm", a.ram_runup_mm, 5000, 200_000)
        .range("ai.cast_stop_mm", a.cast_stop_mm, 2000, 40_000)
        .range("ai.lure_wait_ms", a.lure_wait_ms, 1000, 120_000)
        .range("ai.tries_per_fish", a.tries_per_fish, 1, 20)
        .range("ai.hear_mm", a.hear_mm, 0, 400_000)
        .range("ai.threat_speed_pct", a.threat_speed_pct, 1, 100)
        .range("ai.flee_ms", a.flee_ms, 0, 60_000)
        .range("ai.flee_within_ms", a.flee_within_ms, 0, 60_000)
        .range("ai.respawn_cost_ms", a.respawn_cost_ms, 0, 600_000)
        .range("ai.fishing_rate_per_min", a.fishing_rate_per_min, 0, 10_000)
        .range("ai.sighting_keep_ms", a.sighting_keep_ms, 0, 600_000);
    for (name, v) in [
        ("common", a.rival_reel_ms.common),
        ("uncommon", a.rival_reel_ms.uncommon),
        ("rare", a.rival_reel_ms.rare),
        ("legendary", a.rival_reel_ms.legendary),
    ] {
        c.range(&format!("ai.rival_reel_ms.{name}"), v, 0, 600_000);
    }
    let i = &a.items;
    c.range("ai.items.buoy_worth_ms", i.buoy_worth_ms, 0, 600_000)
        .range("ai.items.buoy_worth_held_ms", i.buoy_worth_held_ms, 0, 600_000)
        .range("ai.items.nitrous_min_mm", i.nitrous_min_mm, 0, 2_000_000)
        .range(
            "ai.items.nitrous_hunt_min_mm",
            i.nitrous_hunt_min_mm,
            0,
            2_000_000,
        )
        .range("ai.items.teleport_min_mm", i.teleport_min_mm, 0, 2_000_000)
        .range("ai.items.cloak_hunt_min_mm", i.cloak_hunt_min_mm, 0, 2_000_000)
        .range("ai.items.teleport_lead_ms", i.teleport_lead_ms, 0, 600_000)
        .range("ai.items.teleport_short_mm", i.teleport_short_mm, 0, 200_000)
        .range(
            "ai.items.teleport_explore_after_ms",
            i.teleport_explore_after_ms,
            0,
            3_600_000,
        )
        .range("ai.items.camera_any_left_ms", i.camera_any_left_ms, 0, 3_600_000)
        .range("ai.items.radar_blind_ms", i.radar_blind_ms, 0, 600_000);
    for (key, h) in [
        ("easy", &i.hand.easy),
        ("normal", &i.hand.normal),
        ("hard", &i.hand.hard),
    ] {
        let k = |n: &str| format!("ai.items.hand.{key}.{n}");
        c.range(&k("speed_per_s"), h.speed_per_s, 100, 50_000)
            .range(&k("pause_ms"), h.pause_ms, 0, 5000)
            .range(&k("aim_noise"), h.aim_noise, 0, 200)
            .range(&k("timing_noise_ms"), h.timing_noise_ms, 0, 2000)
            // under half a turn a tick, or the reel would read it backwards
            .range(&k("reel_permille_per_s"), h.reel_permille_per_s, 100, 20_000);
    }
    check_skill(c, "easy", &a.skill.easy);
    check_skill(c, "normal", &a.skill.normal);
    check_skill(c, "hard", &a.skill.hard);
}
