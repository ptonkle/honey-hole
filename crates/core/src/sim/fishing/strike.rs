//! A fish at a lure, moving: on the bite it strikes, darting in until its
//! mouth is on the lure, and holds there through the hook window (T4: the
//! bite seen under the water, not only heard). Motion only: the bite's
//! timing is the line's rule (`set_step` in `mod.rs`) and nothing here
//! changes it. The numbers are `fishing.strike` in `data/tuning.json`.
//!
//! The fish's place is its middle, and its mouth is half its length ahead
//! of it (the fish models' origin is their middle, the snout at the front).
//! Every step is in whole millimetres from the tick, so it is the same on
//! every run.

use serde::Deserialize;

use crate::map::{trig, P};
use crate::tuning::Checker;

use super::lake::{self, LakeFish};

/// `fishing.strike` in `data/tuning.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrikeTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// how fast it darts in
    pub speed_mm_per_s: i32,
    /// the gap from its mouth to the lure while it bites (below 0: the
    /// lure inside its mouth)
    pub bite_gap_mm: i32,
}

pub fn check(c: &mut Checker, t: &StrikeTuning) {
    c.range("fishing.strike.speed_mm_per_s", t.speed_mm_per_s, 1, 50_000)
        .range("fishing.strike.bite_gap_mm", t.bite_gap_mm, -1000, 5000);
}

/// The gap from a fish's mouth to `lure`.
pub fn mouth_gap(f: &LakeFish, lure: P) -> i32 {
    f.at.dist(lure) as i32 - f.length_mm / 2
}

/// One tick of `f` moving its mouth to `gap` from `lure` at up to `step` a
/// tick, along the line between them, and facing the lure.
pub fn toward_gap(f: &mut LakeFish, lure: P, gap: i32, step: i32) {
    let want = (f.length_mm / 2 + gap).max(0);
    let d = f.at.dist(lure) as i32;
    // the way from the lure out to the fish (its heading's back, on the lure)
    let out = if d == 0 {
        f.heading + trig::HALF
    } else {
        trig::atan2((f.at.y - lure.y) as i64, (f.at.x - lure.x) as i64)
    };
    let next = crate::gfx::fx::approach(d, want, step.max(1));
    if next != d {
        f.at = lake::ahead(lure, out, next);
    }
    f.heading = trig::wrap(out + trig::HALF);
}

/// The bite: the fish darts its mouth onto the lure and holds it.
pub fn bite(f: &mut LakeFish, lure: P, t: &StrikeTuning) {
    toward_gap(f, lure, t.bite_gap_mm, lake::per_tick(t.speed_mm_per_s));
}
