//! The `items` section of `data/tuning.json`: the item buoy's clock and
//! pickup, the draw of an item, and each effect's length and size. The
//! minigames' own numbers are the `minigames` section (`minigame::tuning`).
//! [`crate::tuning::Tuning`] holds it and calls [`check`] with its checker,
//! so a bad value is refused by name.

use serde::Deserialize;

use super::Item;
use crate::tuning::Checker;

/// One weight per item: an item is drawn with its weight's share of the
/// total (all equal draws each a sixth of the time).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Weights {
    pub cloak: i32,
    pub power_reel: i32,
    pub nitrous: i32,
    pub teleporter: i32,
    pub camera: i32,
    pub enhanced_radar: i32,
}

impl Weights {
    pub fn get(&self, item: Item) -> i32 {
        match item {
            Item::Cloak => self.cloak,
            Item::PowerReel => self.power_reel,
            Item::Nitrous => self.nitrous,
            Item::Teleporter => self.teleporter,
            Item::Camera => self.camera,
            Item::EnhancedRadar => self.enhanced_radar,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemsTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// the warning popup comes this long before a buoy spawns
    pub warning_ms: i32,
    /// the countdown's beats, `beat_ms` apart, the last as the buoy spawns
    pub beats: i32,
    pub beat_ms: i32,
    /// a boat whose middle comes this near the buoy's takes its item
    pub pickup_radius_mm: i32,
    /// an unclaimed buoy moves to a new fair point when the next is due
    /// (false: it stays where it is, and no new one comes until it is taken)
    pub unclaimed_moves: bool,
    pub weights: Weights,
    pub cloak_ms: i32,
    pub power_reel_ms: i32,
    /// the reel's power while the Power Reel runs, percent of plain
    pub power_reel_pct: i32,
    pub nitrous_ms: i32,
    pub teleport_charge_ms: i32,
    /// a teleport lands at least this far from land
    pub teleport_clear_mm: i32,
    /// the camera's offer after a catch stays up this long, then the
    /// camera is kept for later
    pub camera_prompt_ms: i32,
    /// a fish on the line waits while the seat plays an item's minigame or
    /// picks a teleport's spot: the reel bar holds still (R4.11; false: it
    /// fights on with no Space, and the line may break)
    pub reel_holds_in_minigame: bool,
}

/// An hour, the longest time any value may name.
const HOUR_MS: i32 = 3_600_000;

pub fn check(c: &mut Checker, t: &ItemsTuning) {
    c.range("items.beats", t.beats, 0, 20)
        .range("items.beat_ms", t.beat_ms, 20, 10_000)
        .range("items.warning_ms", t.warning_ms, 0, 600_000)
        .rule(
            t.warning_ms >= (t.beats - 1).max(0) * t.beat_ms,
            "items.warning_ms must leave room for the countdown's beats.",
        )
        .range("items.pickup_radius_mm", t.pickup_radius_mm, 100, 50_000)
        .range("items.cloak_ms", t.cloak_ms, 20, HOUR_MS)
        .range("items.power_reel_ms", t.power_reel_ms, 20, HOUR_MS)
        .range("items.power_reel_pct", t.power_reel_pct, 100, 1000)
        .range("items.nitrous_ms", t.nitrous_ms, 20, HOUR_MS)
        .range("items.teleport_charge_ms", t.teleport_charge_ms, 20, 60_000)
        .range("items.teleport_clear_mm", t.teleport_clear_mm, 0, 50_000)
        .range("items.camera_prompt_ms", t.camera_prompt_ms, 20, 600_000);
    let mut total = 0;
    for item in Item::ALL {
        let w = t.weights.get(item);
        c.range(&format!("items.weights.{}", item.key()), w, 0, 1000);
        total += w.max(0);
    }
    c.rule(total > 0, "items.weights must give some item a weight.");
}
