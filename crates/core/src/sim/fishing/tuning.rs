//! The `fish` and `fishing` sections of `data/tuning.json`: how many fish
//! a match stocks and how they roam and show, and every number of the cast,
//! the bite and the reel. [`crate::tuning::Tuning`] holds them and calls
//! [`check`] with its own checker, so a bad value is refused by name.

use serde::Deserialize;

use crate::map::fish::Tier;
use crate::tuning::Checker;

/// One number per tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerTier {
    pub common: i32,
    pub uncommon: i32,
    pub rare: i32,
    pub legendary: i32,
}

impl PerTier {
    pub fn get(&self, t: Tier) -> i32 {
        match t {
            Tier::Common => self.common,
            Tier::Uncommon => self.uncommon,
            Tier::Rare => self.rare,
            Tier::Legendary => self.legendary,
        }
    }

    fn check(&self, c: &mut Checker, key: &str, lo: i32, hi: i32) {
        for (name, v) in [
            ("common", self.common),
            ("uncommon", self.uncommon),
            ("rare", self.rare),
            ("legendary", self.legendary),
        ] {
            c.range(&format!("{key}.{name}"), v, lo, hi);
        }
    }
}

/// The fish in the lake.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// each kind's count (`count` in `data/fish.json`) moves by up to this
    /// share either way, by the match seed
    pub count_variance_pct: i32,
    /// a fish's length is this share of its kind's base length, at random
    pub length_min_pct: i32,
    pub length_max_pct: i32,
    /// how far a school wanders from where it was placed
    pub roam_radius_mm: i32,
    pub roam_speed_mm_per_s: i32,
    /// a school rests this long between legs of its wander
    pub roam_rest_min_ms: i32,
    pub roam_rest_max_ms: i32,
    /// how fast a fish keeps up with its school
    pub follow_speed_mm_per_s: i32,
    /// a freed or scared fish swims this far off, this fast, then settles
    pub freed_swim_mm: i32,
    pub freed_speed_mm_per_s: i32,
    /// how far from a boat a fish shows under the water: `sight_shallow_mm`
    /// at `sight_shallow_depth_mm` deep or less, `sight_deep_mm` at
    /// `sight_deep_depth_mm` or more, in proportion between
    pub sight_shallow_mm: i32,
    pub sight_deep_mm: i32,
    pub sight_shallow_depth_mm: i32,
    pub sight_deep_depth_mm: i32,
    /// how far from the lure a fish shows to its angler while the line is
    /// in (the underwater camera)
    pub sight_lure_mm: i32,
}

/// The reel minigame. Bar positions are thousandths of the bar's height,
/// from the bottom.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReelTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// the catch zone's height
    pub zone_permille: i32,
    /// thousandths of the bar a second², down while Space is up, up while
    /// it is held
    pub gravity_per_s2: i32,
    pub lift_per_s2: i32,
    pub zone_top_speed_per_s: i32,
    /// the catch meter starts this full
    pub start_pct: i32,
    /// a perfect reel (the fish always in the zone) lands it in this time
    pub perfect_ms: PerTier,
    /// the meter drains from full to empty in this time with the fish out
    pub drain_ms: PerTier,
    /// the fish's marker: its top speed (thousandths of the bar a second),
    /// how long it holds a course, and the chance a new course is a dart
    /// to the far end of the bar
    pub fish_speed_per_s: PerTier,
    pub fish_turn_ms: PerTier,
    pub fish_dart_pct: PerTier,
    /// the reel's ratchet ticks this often while Space lifts the zone
    pub tick_ms: i32,
    /// the depth a fish is pulled up to by the time it lands
    pub surface_depth_mm: i32,
}

/// Casting, the bite, and what a catch is worth.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishingTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    pub cast_max_speed_mm_per_s: i32,
    /// holding Space this long charges a full cast
    pub charge_full_ms: i32,
    pub cast_min_mm: i32,
    pub cast_max_mm: i32,
    pub cast_flight_ms: i32,
    pub sink_mm_per_s: i32,
    /// the lure stops this far over the lake bed
    pub bed_clear_mm: i32,
    /// the boat moving faster than this reels an unhooked line in, and
    /// faster than `snap` breaks a line with a fish on it
    pub drag_reel_in_mm_per_s: i32,
    pub drag_snap_mm_per_s: i32,
    /// a fish this near the lure comes over when its depth matches the
    /// lure's; with the depths `attract_depth_mm` apart or more, only
    /// `attract_far_pct` of the radius
    pub attract_radius_mm: i32,
    pub attract_depth_mm: i32,
    pub attract_far_pct: i32,
    pub approach_speed_mm_per_s: i32,
    /// from the fish reaching the lure to its first nibble
    pub wait_min_ms: i32,
    pub wait_max_ms: i32,
    pub nibbles_min: i32,
    pub nibbles_max: i32,
    pub nibble_gap_min_ms: i32,
    pub nibble_gap_max_ms: i32,
    /// Space within this of the bite hooks the fish
    pub hook_window_ms: i32,
    pub reel: ReelTuning,
    /// a catch's points: its kind's value (`data/fish.json`) times this
    /// percent for its tier, times its length over its kind's base length
    pub tier_value_pct: PerTier,
    /// the High-Quality Camera's multiplier (the items system uses it)
    pub camera_value_pct: i32,
    /// a landed fish is held up this long before the line is free again
    pub landed_ms: i32,
}

/// Every range in the two sections.
pub fn check(c: &mut Checker, f: &FishTuning, g: &FishingTuning) {
    c.range("fish.count_variance_pct", f.count_variance_pct, 0, 90)
        .range("fish.length_min_pct", f.length_min_pct, 10, 1000)
        .range("fish.length_max_pct", f.length_max_pct, f.length_min_pct, 1000)
        .range("fish.roam_radius_mm", f.roam_radius_mm, 0, 100_000)
        .range("fish.roam_speed_mm_per_s", f.roam_speed_mm_per_s, 1, 10_000)
        .range("fish.roam_rest_min_ms", f.roam_rest_min_ms, 0, 60_000)
        .range("fish.roam_rest_max_ms", f.roam_rest_max_ms, f.roam_rest_min_ms, 60_000)
        .range("fish.follow_speed_mm_per_s", f.follow_speed_mm_per_s, 1, 20_000)
        .range("fish.freed_swim_mm", f.freed_swim_mm, 0, 100_000)
        .range("fish.freed_speed_mm_per_s", f.freed_speed_mm_per_s, 1, 20_000)
        .range("fish.sight_shallow_mm", f.sight_shallow_mm, 0, 400_000)
        .range("fish.sight_deep_mm", f.sight_deep_mm, 0, 400_000)
        .range("fish.sight_shallow_depth_mm", f.sight_shallow_depth_mm, 0, 30_000)
        .range(
            "fish.sight_deep_depth_mm",
            f.sight_deep_depth_mm,
            f.sight_shallow_depth_mm + 1,
            30_000,
        )
        .range("fish.sight_lure_mm", f.sight_lure_mm, 0, 100_000);
    c.range("fishing.cast_max_speed_mm_per_s", g.cast_max_speed_mm_per_s, 0, 60_000)
        .range("fishing.charge_full_ms", g.charge_full_ms, 20, 10_000)
        .range("fishing.cast_min_mm", g.cast_min_mm, 0, 100_000)
        .range("fishing.cast_max_mm", g.cast_max_mm, g.cast_min_mm, 100_000)
        .range("fishing.cast_flight_ms", g.cast_flight_ms, 20, 10_000)
        .range("fishing.sink_mm_per_s", g.sink_mm_per_s, 1, 20_000)
        .range("fishing.bed_clear_mm", g.bed_clear_mm, 0, 5000)
        .range("fishing.drag_reel_in_mm_per_s", g.drag_reel_in_mm_per_s, 0, 60_000)
        .range("fishing.drag_snap_mm_per_s", g.drag_snap_mm_per_s, 0, 60_000)
        .range("fishing.attract_radius_mm", g.attract_radius_mm, 0, 100_000)
        .range("fishing.attract_depth_mm", g.attract_depth_mm, 1, 30_000)
        .range("fishing.attract_far_pct", g.attract_far_pct, 0, 100)
        .range("fishing.approach_speed_mm_per_s", g.approach_speed_mm_per_s, 1, 20_000)
        .range("fishing.wait_min_ms", g.wait_min_ms, 0, 60_000)
        .range("fishing.wait_max_ms", g.wait_max_ms, g.wait_min_ms, 60_000)
        .range("fishing.nibbles_min", g.nibbles_min, 0, 10)
        .range("fishing.nibbles_max", g.nibbles_max, g.nibbles_min, 10)
        .range("fishing.nibble_gap_min_ms", g.nibble_gap_min_ms, 20, 10_000)
        .range("fishing.nibble_gap_max_ms", g.nibble_gap_max_ms, g.nibble_gap_min_ms, 10_000)
        .range("fishing.hook_window_ms", g.hook_window_ms, 20, 10_000)
        .range("fishing.camera_value_pct", g.camera_value_pct, 100, 1000)
        .range("fishing.landed_ms", g.landed_ms, 0, 10_000);
    g.tier_value_pct.check(c, "fishing.tier_value_pct", 1, 10_000);
    let r = &g.reel;
    c.range("fishing.reel.zone_permille", r.zone_permille, 50, 900)
        .range("fishing.reel.gravity_per_s2", r.gravity_per_s2, 0, 100_000)
        .range("fishing.reel.lift_per_s2", r.lift_per_s2, 1, 100_000)
        .range("fishing.reel.zone_top_speed_per_s", r.zone_top_speed_per_s, 1, 100_000)
        .range("fishing.reel.start_pct", r.start_pct, 1, 99)
        .range("fishing.reel.tick_ms", r.tick_ms, 20, 10_000)
        .range("fishing.reel.surface_depth_mm", r.surface_depth_mm, 0, 5000);
    r.perfect_ms.check(c, "fishing.reel.perfect_ms", 200, 600_000);
    r.drain_ms.check(c, "fishing.reel.drain_ms", 200, 600_000);
    r.fish_speed_per_s.check(c, "fishing.reel.fish_speed_per_s", 1, 100_000);
    r.fish_turn_ms.check(c, "fishing.reel.fish_turn_ms", 20, 60_000);
    r.fish_dart_pct.check(c, "fishing.reel.fish_dart_pct", 0, 100);
}
