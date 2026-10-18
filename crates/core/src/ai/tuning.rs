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

/// The section's range checks. `ram_wreck_pct` and `land_wreck_pct` are the
/// match's: a ram must be driven fast enough to wreck, and careful driving
/// slow enough to bounce.
pub fn check(c: &mut Checker, a: &AiTuning, ram_wreck_pct: i32, land_wreck_pct: i32) {
    c.range("ai.path_cell_mm", a.path_cell_mm, 1000, 32_000)
        .range("ai.path_clear_mm", a.path_clear_mm, 0, 40_000)
        .range("ai.path_roomy_mm", a.path_roomy_mm, a.path_clear_mm, 80_000)
        .range("ai.waypoint_mm", a.waypoint_mm, 500, 40_000)
        .range("ai.look_ahead_mm", a.look_ahead_mm, 1000, 200_000)
        .range("ai.near_land_mm", a.near_land_mm, 0, 40_000)
        .range("ai.careful_speed_pct", a.careful_speed_pct, 5, land_wreck_pct - 1)
        .range("ai.brake_use_pct", a.brake_use_pct, 10, 100)
        .range("ai.ram_speed_pct", a.ram_speed_pct, ram_wreck_pct, 100)
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
    check_skill(c, "easy", &a.skill.easy);
    check_skill(c, "normal", &a.skill.normal);
    check_skill(c, "hard", &a.skill.hard);
}
