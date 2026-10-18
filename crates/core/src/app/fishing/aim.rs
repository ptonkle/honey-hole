//! Aiming from the boat camera (the person, 2026-10-18: "a bit of a
//! disconnect between where you see the fish in the water vs where you
//! want to cast to"; SECOND-ORDER R7.70-R7.72).
//!
//! - **The aim marker**: while Space is held a ring sits on the water where
//!   the cast will land (`FishingView::aim`, worked out by the same
//!   `cast_landing` the cast uses), sliding out as the charge grows.
//! - **The fish** are drawn straight over their true map spot, just under
//!   the surface, so the low chase camera sees them where they are. Drawn
//!   at their depth (or at the old 45% of it) a fish looked metres nearer
//!   the boat than it is: a point `d` under the water shows on screen over
//!   the surface spot `d` times the view's slope nearer the camera (about
//!   5 times at 15 m from the camera, 11 at 30 m). Their depth still shows
//!   as the water's tint, as deep as `fish_depth_pct` of their depth.
//!
//! The numbers are `fishing` in `data/look.json`: `fish_float_mm`,
//! `fish_float_pct` and the `aim` section.

use serde::Deserialize;

use crate::gfx::fx::{self, ONE};
use crate::gfx::geom::{v3, V3};
use crate::gfx::render;
use crate::map::P;
use crate::scene::look::UnderLook as WaterTint;
use crate::scene::{mm, Lake};

/// `data/look.json`'s `fishing.aim`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AimLook {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// the ring's radius to the middle of its band, the band's width, and
    /// how high over the waves it lies
    pub radius_mm: i32,
    pub width_mm: i32,
    pub lift_mm: i32,
    pub colour: [i32; 3],
    /// how unlit (percent), so it reads on the water in any light
    pub glow_pct: i32,
}

/// The ring's segments.
const SEGMENTS: i32 = 32;

impl AimLook {
    /// The aim marker round the map point `at`, lying on the waves as the
    /// page draws them at `clock_ms`, as triangles appended to `out`.
    pub fn marker(&self, out: &mut Vec<i32>, lake: &Lake, at: P, clock_ms: u64) {
        let (inner, outer) = (
            (self.radius_mm - self.width_mm / 2).max(0),
            self.radius_mm + self.width_mm / 2,
        );
        let lift = mm(self.lift_mm);
        let on_water = |p: V3| lake.surface_at(lake.map_point(p), clock_ms) + lift;
        ring(
            out,
            lake.world(at, 0),
            [mm(inner), mm(outer)],
            render::rgb(self.colour),
            self.glow_pct * ONE / 100,
            on_water,
        );
    }
}

/// A ring round `c` from `radii[0]` to `radii[1]` (world units), facing up,
/// each point at the height `y` gives for it.
pub fn ring(
    out: &mut Vec<i32>,
    c: V3,
    radii: [i32; 2],
    colour: render::Rgb,
    glow: i32,
    y: impl Fn(V3) -> i32,
) {
    let up = v3(0, ONE, 0);
    let at = |i: i32, rad: i32| {
        let a = i * (fx::TURN / SEGMENTS);
        let p = c.add(v3(fx::mul(rad, fx::cos(a)), 0, fx::mul(rad, fx::sin(a))));
        V3 { y: y(p), ..p }
    };
    let [inner, outer] = radii;
    for i in 0..SEGMENTS {
        let (a0, a1, b0, b1) = (at(i, inner), at(i + 1, inner), at(i, outer), at(i + 1, outer));
        render::push_tri_glow(out, [a0, b0, b1], up, colour, glow);
        render::push_tri_glow(out, [a0, b1, a1], up, colour, glow);
    }
}

/// How deep (mm) the boat camera draws a fish whose drawn length is
/// `drawn_length_mm`: `float_mm`, plus `float_pct` of its length so a big
/// fish's back stays under the wave troughs.
pub fn drawn_depth_mm(float_mm: i32, float_pct: i32, drawn_length_mm: i32) -> i32 {
    float_mm + (drawn_length_mm as i64 * float_pct as i64 / 100) as i32
}

impl crate::app::App {
    /// How much further down the boat camera looks with `me` in fishing
    /// mode (`fishing.camera_tilt_deg`, SECOND-ORDER R7.75): steep enough
    /// that the angler's back hides little of the water a cast reaches.
    pub(in crate::app) fn fishing_tilt(&self, me: Option<&crate::sim::view::BoatView>) -> i32 {
        let fishing = me.is_some_and(|b| b.mode == crate::sim::Mode::Fishing);
        match (fishing, self.look.as_ref().and_then(|l| l.fishing.as_ref())) {
            (true, Some(l)) => fx::deg(l.camera_tilt_deg),
            _ => 0,
        }
    }

    /// Under `?debug=aimfish`, once the match has run a tick (after
    /// `debug_move`): a common fish resting 1.5 m down, 13 m out from this
    /// seat's bow 30 degrees to the right, and 1 m right of that line, for
    /// the aim's screenshots.
    pub(in crate::app) fn debug_aim_fish(&mut self) {
        let Some(p) = self.play.as_mut().filter(|p| p.game.tick > 0) else {
            return;
        };
        let Some(i) = self.debug_flags.iter().position(|f| f == "aimfish") else {
            return;
        };
        self.debug_flags.remove(i);
        use crate::sim::fishing::lake::{ahead, FishState, LakeFish, School};
        let Some(b) = p.game.boat(p.seat) else {
            return;
        };
        let r = &p.game.rules;
        let bow = crate::sim::fishing::bow(b, r.half_length, r.half_beam);
        let heading = b.heading;
        // a twelfth of a turn (30 degrees) right of the bow, clear of the
        // angler in the chase camera's view
        let way = heading - crate::map::trig::TURN / 12;
        let at = ahead(ahead(bow, way, 13_000), way - crate::map::trig::QUARTER, 1000);
        let fi = &mut p.game.fishing;
        let Some(kind) = fi
            .roster
            .0
            .iter()
            .position(|k| k.tier == crate::map::fish::Tier::Common)
        else {
            return;
        };
        fi.schools.push(School {
            at,
            home: at,
            target: at,
            rest_until: u32::MAX,
        });
        fi.fish.push(LakeFish {
            id: fi.fish.len() as u32,
            kind,
            tier: fi.roster.0[kind].tier,
            length_mm: fi.roster.0[kind].base_length_mm * 6 / 5,
            at,
            depth_mm: 1500,
            home_depth_mm: 1500,
            heading,
            school: fi.schools.len() - 1,
            offset: crate::map::P::new(0, 0),
            state: FishState::Roaming,
            phase_ms: 0,
        });
    }
}

/// The water's tint (`scene.frag`'s `under`, ONE whole) on what lies
/// `depth_mm` under the surface.
pub fn tint(w: &WaterTint, depth_mm: i32) -> i32 {
    let share = (depth_mm.max(0) as i64 * ONE as i64 / w.clear_mm.max(1) as i64).min(ONE as i64);
    (share * w.strength_pct as i64 / 100) as i32
}

/// Turns posed vertices `tris` (drawn `drawn_mm` deep) toward the water's
/// colour as though they lay `look_mm` deep: the shader tints them for
/// `drawn_mm` already, so only the rest is added here.
pub fn deepen(tris: &mut [i32], w: &WaterTint, drawn_mm: i32, look_mm: i32) {
    let k = (tint(w, look_mm) - tint(w, drawn_mm)).max(0);
    if k == 0 {
        return;
    }
    let to = render::rgb(w.colour);
    for v in tris.chunks_mut(render::VERTEX_VALUES) {
        for c in 0..3 {
            v[6 + c] += fx::mul(to[c] - v[6 + c], k);
        }
    }
}
