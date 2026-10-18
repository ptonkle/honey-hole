//! `data/tuning.json`: every gameplay number that may change, parsed and
//! checked once when the page starts (PLAN.md, contract 6).
//!
//! The file is in people's units, each named in its key (`_ms`, `_mm`,
//! `_mm_per_s`, `_deg`, `_deg_per_s`, `_pct`); the systems turn them into
//! ticks and angle units where they use them (`sim::Rules`). A missing key, an
//! unknown key or a value out of its range is refused by name, so a typo
//! cannot quietly fall back to a default.
//!
//! **Adding a section** (fishing, items, radar, fish counts and values): add
//! a `#[serde(deny_unknown_fields)]` struct for it below, a field for it in
//! [`Tuning`], its object in `data/tuning.json` (with a `_note` saying what
//! each key means), and its range checks in [`Tuning::check`] through
//! [`Checker`]. `tuning_file_is_complete_and_valid` in
//! `crates/core/tests/data_files.rs` then holds the shipped file to it, and
//! [`tests::sample`] gives tests a valid copy to change.

use serde::Deserialize;

/// The whole file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    #[serde(rename = "match")]
    pub game: MatchTuning,
    pub boat: BoatTuning,
    pub wreck: WreckTuning,
    /// the fish in the lake and the fishing (`sim::fishing::tuning`)
    pub fish: crate::sim::fishing::tuning::FishTuning,
    pub fishing: crate::sim::fishing::tuning::FishingTuning,
    /// the Fish-Boy's radar: the ping (`fishboy/radar.rs`)
    pub radar: RadarTuning,
    /// the item buoy and the items' effects (`sim::items::tuning`)
    pub items: crate::sim::items::tuning::ItemsTuning,
    /// the AI players (`ai/tuning.rs`)
    pub ai: crate::ai::tuning::AiTuning,
    /// the match's flow: the countdown, the popups, the end
    /// (`crates/core/src/flow/`)
    pub flow: FlowTuning,
    /// the item minigames (`minigame/tuning.rs`)
    pub minigames: crate::minigame::tuning::MinigameTuning,
    /// the sound engine's section, read and checked by `sound::Bank::parse`
    #[serde(default)]
    pub audio: serde_json::Value,
}

/// The match: its length, its seats and the spawn.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// the default match length; the setup may choose another
    pub length_ms: i32,
    /// what the match setup offers
    pub length_choices_ms: Vec<i32>,
    /// the default time between item buoys; the setup may choose another
    pub buoy_interval_ms: i32,
    pub buoy_interval_choices_ms: Vec<i32>,
    /// lobby slots (each has a player colour)
    pub seats: i32,
    /// the gap between neighbouring boats at the spawn
    pub spawn_spread_mm: i32,
}

/// The boat's handling.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoatTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    pub top_speed_mm_per_s: i32,
    /// W from rest to top speed
    pub accel_ms: i32,
    pub reverse_speed_mm_per_s: i32,
    /// S from rest to full reverse
    pub reverse_accel_ms: i32,
    /// S while moving forward: top speed to rest
    pub brake_ms: i32,
    /// no throttle: top speed to rest
    pub coast_ms: i32,
    /// sideways slip: top speed to none
    pub side_drag_ms: i32,
    pub turn_deg_per_s: i32,
    pub turn_at_rest_pct: i32,
    pub turn_full_at_pct: i32,
    pub swivel_deg_per_s: i32,
    pub swivel_limit_deg: i32,
    pub half_length_mm: i32,
    pub half_beam_mm: i32,
    pub nitrous: NitrousTuning,
}

/// What nitrous multiplies, in percent of the plain figure.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NitrousTuning {
    pub accel_pct: i32,
    pub top_speed_pct: i32,
    pub turn_pct: i32,
}

/// Rams, grounding, bumps and the respawn.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WreckTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    pub ram_wreck_pct: i32,
    pub land_wreck_pct: i32,
    pub stall_ms: i32,
    pub respawn_ms: i32,
    pub ghost_ms: i32,
    pub bounce_pct: i32,
    pub scrape_keep_pct: i32,
    pub bump_pct: i32,
}

/// The radar's ping.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadarTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// from a ping's start to the next one allowed, per player
    pub ping_cooldown_ms: i32,
    /// a sweep's crossing of the whole map
    pub ping_sweep_ms: i32,
}

/// The match's flow (`crates/core/src/flow/`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// the 3-2-1-Go before a match, the boats held still
    pub countdown_ms: i32,
    /// how long a popup stays up
    pub popup_ms: i32,
    /// at the end: the horn and "Lines up" before the postgame
    pub end_hold_ms: i32,
    /// the HUD's key hints show for this long of match time, then fade
    pub hints_ms: i32,
}

/// Collects range checks; the first failure names its key.
pub struct Checker {
    problem: Option<String>,
}

impl Checker {
    pub fn new() -> Checker {
        Checker { problem: None }
    }

    /// `key` must lie in `lo..=hi`.
    pub fn range(&mut self, key: &str, v: i32, lo: i32, hi: i32) -> &mut Checker {
        if self.problem.is_none() && !(lo..=hi).contains(&v) {
            self.problem = Some(format!(
                "The tuning value {key} is {v}; it must be from {lo} to {hi}."
            ));
        }
        self
    }

    /// A rule that must hold, with its own message.
    pub fn rule(&mut self, ok: bool, message: &str) -> &mut Checker {
        if self.problem.is_none() && !ok {
            self.problem = Some(format!("The tuning file breaks a rule: {message}"));
        }
        self
    }

    pub fn done(&self) -> Result<(), String> {
        match &self.problem {
            Some(p) => Err(p.clone()),
            None => Ok(()),
        }
    }
}

impl Default for Checker {
    fn default() -> Checker {
        Checker::new()
    }
}

/// An hour, the longest time any value may name.
const HOUR_MS: i32 = 3_600_000;

impl Tuning {
    pub fn check(&self) -> Result<(), String> {
        let (m, b, w) = (&self.game, &self.boat, &self.wreck);
        let mut c = Checker::new();
        c.range("match.length_ms", m.length_ms, 1000, HOUR_MS)
            .rule(
                !m.length_choices_ms.is_empty(),
                "match.length_choices_ms lists no length.",
            )
            .rule(
                m.length_choices_ms.contains(&m.length_ms),
                "match.length_choices_ms must hold match.length_ms.",
            )
            .range("match.buoy_interval_ms", m.buoy_interval_ms, 1000, HOUR_MS)
            .rule(
                m.buoy_interval_choices_ms.contains(&m.buoy_interval_ms),
                "match.buoy_interval_choices_ms must hold match.buoy_interval_ms.",
            )
            // the player colours are four (data/look.json)
            .range("match.seats", m.seats, 1, 4)
            .range(
                "match.spawn_spread_mm",
                m.spawn_spread_mm,
                2 * b.half_beam_mm,
                20_000,
            );
        for &v in &m.length_choices_ms {
            c.range("match.length_choices_ms", v, 1000, HOUR_MS);
        }
        for &v in &m.buoy_interval_choices_ms {
            c.range("match.buoy_interval_choices_ms", v, 1000, HOUR_MS);
        }
        c.range("boat.top_speed_mm_per_s", b.top_speed_mm_per_s, 1000, 60_000)
            .range("boat.accel_ms", b.accel_ms, 100, 60_000)
            .range(
                "boat.reverse_speed_mm_per_s",
                b.reverse_speed_mm_per_s,
                100,
                b.top_speed_mm_per_s,
            )
            .range("boat.reverse_accel_ms", b.reverse_accel_ms, 100, 60_000)
            .range("boat.brake_ms", b.brake_ms, 100, 60_000)
            .range("boat.coast_ms", b.coast_ms, 100, 600_000)
            .range("boat.side_drag_ms", b.side_drag_ms, 20, 60_000)
            .range("boat.turn_deg_per_s", b.turn_deg_per_s, 1, 720)
            .range("boat.turn_at_rest_pct", b.turn_at_rest_pct, 0, 100)
            .range("boat.turn_full_at_pct", b.turn_full_at_pct, 1, 100)
            .range("boat.swivel_deg_per_s", b.swivel_deg_per_s, 1, 720)
            .range("boat.swivel_limit_deg", b.swivel_limit_deg, 0, 180)
            .range("boat.half_length_mm", b.half_length_mm, 0, 10_000)
            .range("boat.half_beam_mm", b.half_beam_mm, 100, 5_000)
            .range("boat.nitrous.accel_pct", b.nitrous.accel_pct, 100, 1000)
            .range("boat.nitrous.top_speed_pct", b.nitrous.top_speed_pct, 100, 400)
            .range("boat.nitrous.turn_pct", b.nitrous.turn_pct, 100, 1000)
            .range("wreck.ram_wreck_pct", w.ram_wreck_pct, 1, 400)
            .range("wreck.land_wreck_pct", w.land_wreck_pct, 1, 400)
            .range("wreck.stall_ms", w.stall_ms, 0, 60_000)
            .range("wreck.respawn_ms", w.respawn_ms, 0, 60_000)
            .range("wreck.ghost_ms", w.ghost_ms, 0, 60_000)
            .range("wreck.bounce_pct", w.bounce_pct, 0, 100)
            .range("wreck.scrape_keep_pct", w.scrape_keep_pct, 0, 100)
            .range("wreck.bump_pct", w.bump_pct, 0, 100);
        let f = &self.flow;
        // the countdown's beeps are cues for 3, 2 and 1 (data/sounds.json)
        c.range("flow.countdown_ms", f.countdown_ms, 0, 3000)
            .range("flow.popup_ms", f.popup_ms, 200, 30_000)
            .range("flow.end_hold_ms", f.end_hold_ms, 0, 60_000)
            .range("flow.hints_ms", f.hints_ms, 0, HOUR_MS);
        crate::sim::fishing::tuning::check(&mut c, &self.fish, &self.fishing);
        let r = &self.radar;
        c.range("radar.ping_sweep_ms", r.ping_sweep_ms, 20, 60_000)
            .range("radar.ping_cooldown_ms", r.ping_cooldown_ms, 20, 600_000)
            .rule(
                r.ping_sweep_ms <= r.ping_cooldown_ms,
                "radar.ping_sweep_ms must end before radar.ping_cooldown_ms.",
            );
        crate::ai::tuning::check(&mut c, &self.ai);
        self.minigames.check(&mut c);
        crate::sim::items::tuning::check(&mut c, &self.items);
        c.done()
    }
}

/// Reads and checks the tuning file.
pub fn parse(json: &str) -> Result<Tuning, String> {
    let t: Tuning = serde_json::from_str(json).map_err(|e| format!("The tuning file does not read: {e}."))?;
    t.check()?;
    Ok(t)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A valid tuning file for tests: the shipped one's shape, with round
    /// numbers. Tests change a copy of it; their expected values are worked
    /// from these.
    pub fn sample_json() -> serde_json::Value {
        serde_json::json!({
            "match": {
                "length_ms": 480000,
                "length_choices_ms": [240000, 480000],
                "buoy_interval_ms": 60000,
                "buoy_interval_choices_ms": [60000],
                "seats": 4,
                "spawn_spread_mm": 5000
            },
            "boat": {
                "top_speed_mm_per_s": 16000,
                "accel_ms": 5000,
                "reverse_speed_mm_per_s": 4000,
                "reverse_accel_ms": 2000,
                "brake_ms": 2000,
                "coast_ms": 6000,
                "side_drag_ms": 800,
                "turn_deg_per_s": 100,
                "turn_at_rest_pct": 30,
                "turn_full_at_pct": 40,
                "swivel_deg_per_s": 120,
                "swivel_limit_deg": 135,
                "half_length_mm": 1000,
                "half_beam_mm": 850,
                "nitrous": {"accel_pct": 200, "top_speed_pct": 150, "turn_pct": 150}
            },
            "wreck": {
                "ram_wreck_pct": 80,
                "land_wreck_pct": 50,
                "stall_ms": 3000,
                "respawn_ms": 2000,
                "ghost_ms": 3000,
                "bounce_pct": 30,
                "scrape_keep_pct": 70,
                "bump_pct": 50
            },
            "radar": {
                "ping_cooldown_ms": 5000,
                "ping_sweep_ms": 3000
            },
            // no countdown, so a test's first tick moves the boats; the
            // countdown's own tests set one
            "flow": {
                "countdown_ms": 0,
                "popup_ms": 2000,
                "end_hold_ms": 3000,
                "hints_ms": 60000
            },
            "fish": fish_sample(),
            "fishing": fishing_sample(),
            "items": items_sample(),
            "minigames": crate::minigame::tuning::tests::sample_json(),
            "ai": ai_sample()
        })
    }

    /// The items section, round numbers for tests: a 5 s warning with 5
    /// beats a second apart, a 3 m pickup, equal weights, a 60 s cloak, a
    /// 20 s Power Reel at 200%, 15 s of nitrous and a 5 s teleport charge.
    pub fn items_sample() -> serde_json::Value {
        serde_json::json!({
            "warning_ms": 5000,
            "beats": 5,
            "beat_ms": 1000,
            "pickup_radius_mm": 3000,
            "unclaimed_moves": true,
            "weights": {"cloak": 1, "power_reel": 1, "nitrous": 1, "teleporter": 1,
                        "camera": 1, "enhanced_radar": 1},
            "cloak_ms": 60000,
            "power_reel_ms": 20000,
            "power_reel_pct": 200,
            "nitrous_ms": 15000,
            "teleport_charge_ms": 5000,
            "teleport_clear_mm": 6000,
            "camera_prompt_ms": 8000
        })
    }

    /// The AI section, for tests: the shipped file's numbers.
    pub fn ai_sample() -> serde_json::Value {
        let file: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/tuning.json")).unwrap();
        file["ai"].clone()
    }

    /// The fish section, round numbers for tests.
    pub fn fish_sample() -> serde_json::Value {
        serde_json::json!({
            "count_variance_pct": 20,
            "length_min_pct": 100,
            "length_max_pct": 150,
            "roam_radius_mm": 10000,
            "roam_speed_mm_per_s": 400,
            "roam_rest_min_ms": 1000,
            "roam_rest_max_ms": 4000,
            "follow_speed_mm_per_s": 800,
            "freed_swim_mm": 12000,
            "freed_speed_mm_per_s": 3000,
            "sight_shallow_mm": 40000,
            "sight_deep_mm": 25000,
            "sight_shallow_depth_mm": 1000,
            "sight_deep_depth_mm": 12000,
            "sight_lure_mm": 15000
        })
    }

    /// The fishing section, round numbers for tests: a 1 s full charge
    /// (50 ticks) from 4 m to 24 m, a 1 s flight, a 1 m/s sink.
    pub fn fishing_sample() -> serde_json::Value {
        serde_json::json!({
            "cast_max_speed_mm_per_s": 1000,
            "charge_full_ms": 1000,
            "cast_min_mm": 4000,
            "cast_max_mm": 24000,
            "cast_flight_ms": 1000,
            "sink_mm_per_s": 1000,
            "bed_clear_mm": 200,
            "drag_reel_in_mm_per_s": 3000,
            "drag_snap_mm_per_s": 5000,
            "attract_radius_mm": 6000,
            "attract_depth_mm": 3000,
            "attract_far_pct": 30,
            "approach_speed_mm_per_s": 1000,
            "wait_min_ms": 2000,
            "wait_max_ms": 6000,
            "nibbles_min": 1,
            "nibbles_max": 3,
            "nibble_gap_min_ms": 600,
            "nibble_gap_max_ms": 1400,
            "hook_window_ms": 760,
            "reel": {
                "zone_permille": 280,
                "gravity_per_s2": 2500,
                "lift_per_s2": 2500,
                "zone_top_speed_per_s": 1400,
                "start_pct": 30,
                "perfect_ms": {"common": 3000, "uncommon": 5000, "rare": 10000, "legendary": 20000},
                "drain_ms": {"common": 5000, "uncommon": 4500, "rare": 4000, "legendary": 3500},
                "fish_speed_per_s": {"common": 400, "uncommon": 600, "rare": 800, "legendary": 1000},
                "fish_turn_ms": {"common": 1400, "uncommon": 1100, "rare": 900, "legendary": 700},
                "fish_dart_pct": {"common": 5, "uncommon": 15, "rare": 25, "legendary": 40},
                "tick_ms": 100,
                "surface_depth_mm": 300
            },
            "tier_value_pct": {"common": 100, "uncommon": 200, "rare": 300, "legendary": 500},
            "camera_value_pct": 150,
            "landed_ms": 2000
        })
    }

    pub fn sample() -> Tuning {
        parse(&sample_json().to_string()).unwrap()
    }

    #[test]
    fn a_missing_key_is_refused_by_name() {
        let mut v = sample_json();
        v["boat"].as_object_mut().unwrap().remove("coast_ms");
        let e = parse(&v.to_string()).unwrap_err();
        assert!(e.contains("coast_ms"), "{e}");
    }

    #[test]
    fn an_unknown_key_is_refused_by_name() {
        let mut v = sample_json();
        v["wreck"]["ram_pct"] = 80.into();
        let e = parse(&v.to_string()).unwrap_err();
        assert!(e.contains("ram_pct"), "{e}");
    }

    #[test]
    fn a_value_out_of_range_is_refused_by_name() {
        let mut v = sample_json();
        v["boat"]["top_speed_mm_per_s"] = 0.into();
        assert_eq!(
            parse(&v.to_string()).unwrap_err(),
            "The tuning value boat.top_speed_mm_per_s is 0; it must be from 1000 to 60000."
        );
        let mut v = sample_json();
        v["match"]["seats"] = 5.into();
        assert!(parse(&v.to_string()).unwrap_err().contains("match.seats"));
        let mut v = sample_json();
        v["match"]["length_ms"] = 300000.into();
        assert!(parse(&v.to_string()).unwrap_err().contains("length_choices_ms"));
    }
}
