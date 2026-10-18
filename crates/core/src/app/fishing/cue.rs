//! The bite seen, not only heard (T4: "need a cue when the fish takes the
//! bait other than sound"). No words (KISS): over the water the bobber
//! plunges under with a splash and rings spreading from it, and the line snaps
//! taut and twitches; under it the fish strikes the lure (its motion is the
//! fishing sim's, `sim/fishing/strike.rs`) in a burst of bubbles. Each
//! part is over well inside the 0.75 s hook window, so it reads in time.
//!
//! The numbers are `fishing.bite` in `data/look.json`; the drawing reads
//! only the line's `bite_ms` from the view.

use serde::Deserialize;

use crate::gfx::fx::{self, ONE};
use crate::gfx::geom::{v3, V3};
use crate::gfx::render::{self, Camera};
use crate::scene::mm;

/// `data/look.json`'s `fishing.bite`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BiteLook {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// the bobber is pulled this far under, reaching it in `plunge_in_ms`
    pub plunge_mm: i32,
    pub plunge_in_ms: i32,
    /// so many rings, `ring_gap_ms` apart, each spreading from
    /// `ring_from_mm` to `ring_to_mm` out from the bobber in `ring_ms` and thinning
    /// from `ring_width_mm` to nothing; `ring_lift_mm` over the water
    pub rings: i32,
    pub ring_gap_ms: i32,
    pub ring_ms: i32,
    pub ring_from_mm: i32,
    pub ring_to_mm: i32,
    pub ring_width_mm: i32,
    pub ring_lift_mm: i32,
    pub ring: [i32; 3],
    pub ring_glow_pct: i32,
    /// so many drops thrown up from the bobber, `splash_mm` across, out
    /// to `splash_out_mm` and up to `splash_up_mm` and down in `splash_ms`
    pub splash: i32,
    pub splash_ms: i32,
    pub splash_mm: i32,
    pub splash_out_mm: i32,
    pub splash_up_mm: i32,
    /// so many bubbles from the lure, `bubble_mm` across, spread over
    /// `bubble_spread_mm` and rising at `bubble_rise_mm_per_s` for
    /// `bubble_ms`
    pub bubbles: i32,
    pub bubble_ms: i32,
    pub bubble_mm: i32,
    pub bubble_spread_mm: i32,
    pub bubble_rise_mm_per_s: i32,
    pub bubble: [i32; 3],
    /// the line's middle shakes sideways up to `twitch_mm`, once each way
    /// every `twitch_period_ms`, dying away over `twitch_ms`
    pub twitch_ms: i32,
    pub twitch_mm: i32,
    pub twitch_period_ms: i32,
}

/// The ring's segments.
const SEGMENTS: i32 = 32;

impl BiteLook {
    /// How far under the water the bobber is, `ms` into the bite.
    pub fn plunge_mm(&self, ms: u32) -> i32 {
        let k = (ms as i32).min(self.plunge_in_ms.max(1));
        self.plunge_mm * k / self.plunge_in_ms.max(1)
    }

    /// The line's sideways shake at its middle, mm, `ms` into the bite.
    pub fn twitch_mm(&self, ms: u32) -> i32 {
        let (ms, span) = (ms as i32, self.twitch_ms.max(1));
        if ms >= span {
            return 0;
        }
        let period = self.twitch_period_ms.max(1);
        let wave = fx::sin(((ms % period) as i64 * fx::TURN as i64 / period as i64) as i32);
        (self.twitch_mm as i64 * wave as i64 * (span - ms) as i64 / (ONE as i64 * span as i64)) as i32
    }

    /// The splash rings round `centre` (on the water), `ms` into the bite,
    /// as flat triangles appended to `out`.
    pub fn rings(&self, out: &mut Vec<i32>, centre: V3, ms: u32) {
        let colour = render::rgb(self.ring);
        let glow = self.ring_glow_pct * ONE / 100;
        let up = v3(0, ONE, 0);
        let c = centre.add(v3(0, mm(self.ring_lift_mm), 0));
        for k in 0..self.rings {
            let age = ms as i32 - k * self.ring_gap_ms;
            let span = self.ring_ms.max(1);
            if !(0..span).contains(&age) {
                continue;
            }
            let r = self.ring_from_mm + (self.ring_to_mm - self.ring_from_mm) * age / span;
            let w = self.ring_width_mm * (span - age) / span;
            let (inner, outer) = (mm((r - w / 2).max(0)), mm(r + w / 2));
            let at = |i: i32, rad: i32| {
                let a = i * (fx::TURN / SEGMENTS);
                c.add(v3(fx::mul(rad, fx::cos(a)), 0, fx::mul(rad, fx::sin(a))))
            };
            for i in 0..SEGMENTS {
                let (a0, a1, b0, b1) = (at(i, inner), at(i + 1, inner), at(i, outer), at(i + 1, outer));
                render::push_tri_glow(out, [a0, b0, b1], up, colour, glow);
                render::push_tri_glow(out, [a0, b1, a1], up, colour, glow);
            }
        }
    }

    /// The bubbles bursting from the lure at `lure`, `ms` into the bite, as
    /// small diamonds turned to `cam`, appended to `out`. Each has its own
    /// spot round the lure from its number, so the burst is the same on
    /// every run.
    pub fn bubbles(&self, out: &mut Vec<i32>, lure: V3, cam: &Camera, ms: u32) {
        let span = self.bubble_ms.max(1);
        if ms as i32 >= span {
            return;
        }
        let colour = render::rgb(self.bubble);
        let rise = self.bubble_rise_mm_per_s as i64 * ms as i64 / 1000;
        // they shrink as they go
        let size = mm(self.bubble_mm * (2 * span - ms as i32) / (2 * span));
        let spread = self.bubble_spread_mm as u64;
        for i in 0..self.bubbles {
            let h = crate::map::rng::hash2(0x6269_7465, i, 0);
            let off = |shift: u32| ((h >> shift) % (2 * spread + 1)) as i32 - spread as i32;
            // each rises at its own pace, from two thirds to all of it
            let pace = 67 + (h >> 48) as i64 % 34;
            let lift = off(40) / 2 + (rise * pace / 100) as i32;
            let p = lure.add(v3(mm(off(0)), mm(lift), mm(off(20))));
            // never above the surface
            if p.y > -size {
                continue;
            }
            diamond(out, p, cam, size, colour);
        }
    }

    /// The splash thrown up round the bobber at `centre` (on the water),
    /// `ms` into the bite: `splash` drops flying out to `splash_out_mm` and
    /// up to `splash_up_mm` and back down in `splash_ms`, `splash_mm`
    /// across and shrinking, in the rings' colour, turned to `cam`. It
    /// reads from the low boat camera, which sees the rings edge on.
    pub fn splash(&self, out: &mut Vec<i32>, centre: V3, cam: &Camera, ms: u32) {
        let span = self.splash_ms.max(1);
        let ms = ms as i32;
        if ms >= span {
            return;
        }
        let colour = render::rgb(self.ring);
        let size = mm(self.splash_mm * (span - ms / 2) / span);
        for i in 0..self.splash {
            let h = crate::map::rng::hash2(0x7370_6c61, i, 0);
            // round the bobber, each at its own reach and height
            let a = i * (fx::TURN / self.splash.max(1)) + (h % 4096) as i32;
            let reach = self.splash_out_mm * (60 + (h >> 16) as i32 % 41) / 100;
            let top = self.splash_up_mm * (60 + (h >> 32) as i32 % 41) / 100;
            let out_mm = reach * ms / span;
            let up_mm =
                (4 * top as i64 * ms as i64 * (span - ms) as i64 / (span as i64 * span as i64)) as i32;
            let p = centre.add(v3(
                fx::mul(mm(out_mm), fx::cos(a)),
                mm(up_mm),
                fx::mul(mm(out_mm), fx::sin(a)),
            ));
            diamond(out, p, cam, size, colour);
        }
    }
}

/// A small diamond at `p`, `size` across, turned to `cam`, fully lit.
fn diamond(out: &mut Vec<i32>, p: V3, cam: &Camera, size: i32, colour: render::Rgb) {
    let fwd = cam.fwd.norm();
    let right = crate::gfx::geom::right_of(fwd);
    let up = right.cross(fwd);
    let (r, u) = (right.scale(size / 2), up.scale(size / 2));
    let n = fwd.neg();
    render::push_tri_glow(out, [p.sub(r), p.sub(u), p.add(r)], n, colour, ONE);
    render::push_tri_glow(out, [p.sub(r), p.add(r), p.add(u)], n, colour, ONE);
}
