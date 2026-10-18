//! A fish at a lure, moving. Once it has reached the lure it hangs back
//! with its mouth `rest_gap_mm` off; `lead_ms` before each nibble it darts
//! in until its mouth touches the lure, and after it backs off again (T1:
//! "fish should hit and back off of bait when biting"). On the bite it
//! strikes, darting in until the lure is in its mouth, and holds there
//! through the hook window (T4: the bite seen under the water, not only
//! heard). Motion only: the nibbles' and the bite's timing is the line's
//! rule (`set_step` in `mod.rs`) and nothing here changes it. The numbers
//! are `fishing.strike` in `data/tuning.json`.
//!
//! The fish's place is its middle, and its mouth is half its length ahead
//! of it (the fish models' origin is their middle, the snout at the front).
//! Every step is in whole millimetres from the tick, so it is the same on
//! every run.

use serde::Deserialize;

use crate::map::{trig, P};
use crate::tuning::Checker;

use super::lake::{self, LakeFish};
use super::Interest;
use crate::sim::ticks;

/// `fishing.strike` in `data/tuning.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrikeTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// how fast it darts in, and backs off
    pub speed_mm_per_s: i32,
    pub back_mm_per_s: i32,
    /// it darts in this long before each nibble and the bite
    pub lead_ms: i32,
    /// the gap from its mouth to the lure as it waits, as it nibbles, and
    /// while it bites (below 0: the lure inside its mouth)
    pub rest_gap_mm: i32,
    pub touch_gap_mm: i32,
    pub bite_gap_mm: i32,
}

pub fn check(c: &mut Checker, t: &StrikeTuning) {
    c.range("fishing.strike.speed_mm_per_s", t.speed_mm_per_s, 1, 50_000)
        .range("fishing.strike.back_mm_per_s", t.back_mm_per_s, 1, 50_000)
        .range("fishing.strike.lead_ms", t.lead_ms, 0, 10_000)
        .range("fishing.strike.rest_gap_mm", t.rest_gap_mm, -1000, 5000)
        .range("fishing.strike.touch_gap_mm", t.touch_gap_mm, -1000, 5000)
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

/// One tick of a fish that has reached `lure` (`int`): biting, it strikes
/// and holds; `lead_ms` before its next nibble (or the bite), it darts in
/// to touch; otherwise it backs off to wait.
pub fn at_lure(f: &mut LakeFish, lure: P, int: &Interest, tick: u32, t: &StrikeTuning) {
    let dart = lake::per_tick(t.speed_mm_per_s);
    if int.biting_until.is_some() {
        toward_gap(f, lure, t.bite_gap_mm, dart);
    } else if tick + ticks(t.lead_ms) >= int.next {
        toward_gap(f, lure, t.touch_gap_mm, dart);
    } else {
        toward_gap(f, lure, t.rest_gap_mm, lake::per_tick(t.back_mm_per_s));
    }
}
