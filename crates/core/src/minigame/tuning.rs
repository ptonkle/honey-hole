//! The minigames' numbers: `data/tuning.json`'s `minigames` section.
//!
//! Sizes and places are in the minigames' own canvas units (`canvas`, 1600
//! x 900, the Fish-Boy screen's), speeds in units a second (`_per_s`),
//! times in milliseconds (`_ms`), shares in percent (`_pct`) or thousandths
//! (`_permille`). Each minigame turns them into ticks where it uses them.

use serde::Deserialize;

use crate::tuning::Checker;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinigameTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// the virtual canvas every minigame lays itself out on
    pub canvas: [i32; 2],
    /// the one-line instruction shows alone for this long before play
    pub banner_ms: i32,
    pub cloak: CloakTuning,
    pub power_reel: ReelTuning,
    pub nitrous: AtomsTuning,
    pub teleporter: BeatsTuning,
    pub camera: CameraTuning,
    pub enhanced_radar: FocusTuning,
}

/// Cloaking Tech: erase an overhead boat.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloakTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// the picture of the boat, centred
    pub picture: [i32; 2],
    /// the coverage grid over the picture
    pub cols: i32,
    pub rows: i32,
    /// the eraser's side
    pub eraser: i32,
    /// done when this share of the boat is erased
    pub done_pct: i32,
    /// a tick sounds each time this much more is erased
    pub tick_every_pct: i32,
}

/// Power Reel: circle the reel clockwise.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReelTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    pub turns: i32,
    /// the reel's radius, drawn
    pub radius: i32,
    /// the pointer this close to the middle counts for nothing
    pub dead_zone: i32,
}

/// Nitrous: drag atoms into their bins.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtomsTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    pub nitrogen: i32,
    pub oxygen: i32,
    pub atom_radius: i32,
    pub speed_min_per_s: i32,
    pub speed_max_per_s: i32,
    /// a press this far outside an atom still picks it up
    pub grab_slop: i32,
    /// an atom dropped in the wrong bin flies back out this fast
    pub bounce_out_per_s: i32,
    /// the bins' height along the bottom
    pub bin_h: i32,
}

/// Teleporter: click circles as their ring closes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeatsTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    pub hits: i32,
    pub radius: i32,
    /// the ring starts this big, percent of the circle's radius
    pub ring_start_pct: i32,
    /// from a circle's appearing to its ring meeting its edge
    pub approach_ms: i32,
    /// a click this close to the meeting, either side, is a hit
    pub window_ms: i32,
    pub spawn_every_ms: i32,
    /// circles keep this far from the canvas's edges
    pub margin: i32,
    /// and this far from each other where they can
    pub min_gap: i32,
    /// a hit's or a miss's mark fades over this long
    pub feedback_ms: i32,
}

/// High-Quality Camera: click fish flying in from the edges.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    pub photos: i32,
    pub spawn_min_ms: i32,
    pub spawn_max_ms: i32,
    pub max_fish: i32,
    /// a fish's length
    pub fish_len: i32,
    /// sideways throws: their speed across
    pub speed_min_per_s: i32,
    pub speed_max_per_s: i32,
    /// the pull down on a fish in flight, units a second a second
    pub gravity_per_s2: i32,
    /// a click this near a fish's middle, percent of its half length,
    /// photographs it
    pub hit_pct: i32,
    /// the flash after a photo
    pub flash_ms: i32,
}

/// Enhanced Radar: three sliders bring a fish into focus.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FocusTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// within this share of the slider's length of every sweet spot, the
    /// picture snaps sharp
    pub snap_within_pct: i32,
    /// and stays sharp this long before the minigame is done
    pub snap_ms: i32,
    /// sweet spots keep this far from the slider's ends (thousandths)
    pub sweet_margin_permille: i32,
    /// each slider starts at least this far from its sweet spot
    pub start_gap_permille: i32,
    /// this far from the sweet spot, or further, a slider's blur is whole
    pub falloff_permille: i32,
    /// slider one: the blur's spread at its widest
    pub blur_max: i32,
    /// slider two: the pixels' size at their largest
    pub pixel_max: i32,
    /// slider three: the double image's offset at its widest
    pub ghost_max: i32,
}

/// An hour, the longest any minigame time may be.
const HOUR_MS: i32 = 3_600_000;

impl MinigameTuning {
    /// Range checks, added to the tuning file's own.
    pub fn check(&self, c: &mut Checker) {
        let [w, h] = self.canvas;
        let side = w.min(h);
        c.range("minigames.canvas[0]", w, 100, 10_000)
            .range("minigames.canvas[1]", h, 100, 10_000)
            .range("minigames.banner_ms", self.banner_ms, 0, 10_000);
        let k = &self.cloak;
        c.range("minigames.cloak.picture[0]", k.picture[0], 10, w)
            .range("minigames.cloak.picture[1]", k.picture[1], 10, h)
            .range("minigames.cloak.cols", k.cols, 2, 400)
            .range("minigames.cloak.rows", k.rows, 2, 400)
            .range("minigames.cloak.eraser", k.eraser, 1, side)
            .range("minigames.cloak.done_pct", k.done_pct, 1, 100)
            .range("minigames.cloak.tick_every_pct", k.tick_every_pct, 1, 100);
        let r = &self.power_reel;
        c.range("minigames.power_reel.turns", r.turns, 1, 100)
            .range("minigames.power_reel.radius", r.radius, 10, side / 2)
            .range("minigames.power_reel.dead_zone", r.dead_zone, 0, r.radius);
        let a = &self.nitrous;
        c.range("minigames.nitrous.nitrogen", a.nitrogen, 0, 40)
            .range("minigames.nitrous.oxygen", a.oxygen, 0, 40)
            .rule(a.nitrogen + a.oxygen > 0, "minigames.nitrous needs an atom.")
            .range("minigames.nitrous.atom_radius", a.atom_radius, 4, side / 8)
            .range("minigames.nitrous.speed_min_per_s", a.speed_min_per_s, 0, 10_000)
            .range(
                "minigames.nitrous.speed_max_per_s",
                a.speed_max_per_s,
                a.speed_min_per_s,
                10_000,
            )
            .range("minigames.nitrous.grab_slop", a.grab_slop, 0, side / 4)
            .range(
                "minigames.nitrous.bounce_out_per_s",
                a.bounce_out_per_s,
                1,
                10_000,
            )
            .range("minigames.nitrous.bin_h", a.bin_h, 4 * a.atom_radius, h / 3);
        let b = &self.teleporter;
        c.range("minigames.teleporter.hits", b.hits, 1, 100)
            .range("minigames.teleporter.radius", b.radius, 10, side / 4)
            .range("minigames.teleporter.ring_start_pct", b.ring_start_pct, 101, 1000)
            .range("minigames.teleporter.approach_ms", b.approach_ms, 100, 10_000)
            .range("minigames.teleporter.window_ms", b.window_ms, 0, b.approach_ms)
            .range(
                "minigames.teleporter.spawn_every_ms",
                b.spawn_every_ms,
                40,
                10_000,
            )
            .range("minigames.teleporter.margin", b.margin, b.radius, side / 2 - 1)
            .range("minigames.teleporter.min_gap", b.min_gap, 0, side)
            .range("minigames.teleporter.feedback_ms", b.feedback_ms, 0, 10_000);
        let m = &self.camera;
        c.range("minigames.camera.photos", m.photos, 1, 100)
            .range("minigames.camera.spawn_min_ms", m.spawn_min_ms, 40, HOUR_MS)
            .range(
                "minigames.camera.spawn_max_ms",
                m.spawn_max_ms,
                m.spawn_min_ms,
                HOUR_MS,
            )
            .range("minigames.camera.max_fish", m.max_fish, 1, 40)
            .range("minigames.camera.fish_len", m.fish_len, 20, side / 2)
            .range("minigames.camera.speed_min_per_s", m.speed_min_per_s, 1, 10_000)
            .range(
                "minigames.camera.speed_max_per_s",
                m.speed_max_per_s,
                m.speed_min_per_s,
                10_000,
            )
            .range("minigames.camera.gravity_per_s2", m.gravity_per_s2, 0, 20_000)
            .range("minigames.camera.hit_pct", m.hit_pct, 10, 400)
            .range("minigames.camera.flash_ms", m.flash_ms, 0, 10_000);
        let f = &self.enhanced_radar;
        c.range(
            "minigames.enhanced_radar.snap_within_pct",
            f.snap_within_pct,
            1,
            50,
        )
        .range("minigames.enhanced_radar.snap_ms", f.snap_ms, 0, 10_000)
        .range(
            "minigames.enhanced_radar.sweet_margin_permille",
            f.sweet_margin_permille,
            0,
            400,
        )
        .range(
            "minigames.enhanced_radar.start_gap_permille",
            f.start_gap_permille,
            f.snap_within_pct * 10 + 1,
            500,
        )
        .range(
            "minigames.enhanced_radar.falloff_permille",
            f.falloff_permille,
            1,
            1000,
        )
        .range("minigames.enhanced_radar.blur_max", f.blur_max, 0, side / 4)
        .range("minigames.enhanced_radar.pixel_max", f.pixel_max, 0, side / 4)
        .range("minigames.enhanced_radar.ghost_max", f.ghost_max, 0, side / 4);
    }
}

#[cfg(test)]
pub mod tests {
    /// A valid section for tests, with round numbers the minigames' tests
    /// work their expected values from.
    pub fn sample_json() -> serde_json::Value {
        serde_json::json!({
            "canvas": [1600, 900],
            "banner_ms": 1000,
            "cloak": {
                "picture": [1200, 600],
                "cols": 60,
                "rows": 30,
                "eraser": 80,
                "done_pct": 98,
                "tick_every_pct": 20
            },
            "power_reel": { "turns": 5, "radius": 220, "dead_zone": 40 },
            "nitrous": {
                "nitrogen": 8,
                "oxygen": 4,
                "atom_radius": 34,
                "speed_min_per_s": 150,
                "speed_max_per_s": 250,
                "grab_slop": 16,
                "bounce_out_per_s": 600,
                "bin_h": 200
            },
            "teleporter": {
                "hits": 10,
                "radius": 70,
                "ring_start_pct": 300,
                "approach_ms": 1000,
                "window_ms": 160,
                "spawn_every_ms": 700,
                "margin": 120,
                "min_gap": 220,
                "feedback_ms": 400
            },
            "camera": {
                "photos": 10,
                "spawn_min_ms": 500,
                "spawn_max_ms": 1000,
                "max_fish": 4,
                "fish_len": 160,
                "speed_min_per_s": 300,
                "speed_max_per_s": 500,
                "gravity_per_s2": 500,
                "hit_pct": 100,
                "flash_ms": 200
            },
            "enhanced_radar": {
                "snap_within_pct": 10,
                "snap_ms": 1000,
                "sweet_margin_permille": 100,
                "start_gap_permille": 250,
                "falloff_permille": 400,
                "blur_max": 40,
                "pixel_max": 60,
                "ghost_max": 90
            }
        })
    }
}
