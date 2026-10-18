//! Cameras over the lake.
//!
//! - [`boat`]: the third-person driving camera. It stands behind and above
//!   a target (the boat), looking at a point over it; the mouse's yaw and
//!   pitch orbit it round the target. The match passes those in from its
//!   input; the camera keeps the eye clear of the ground and the water.
//! - [`fly_over`]: a free camera looking at a point from a yaw, a pitch and
//!   a distance, for the overview and screenshots.
//!
//! Angles are the renderer's (`gfx::geom`): a yaw of 0 looks along `+z`
//! and a positive yaw turns toward `+x`; [`super::yaw_of_heading`] turns a
//! map heading into one. Pitch is how far the camera looks down.

use super::look::CameraLook;
use super::{mm, Lake};
use crate::gfx::fx::{deg, ONE};
use crate::gfx::geom::{facing, v3, V3};
use crate::gfx::render::Camera;

/// How many points along the pivot-to-eye line are tested for land.
const LAND_STEPS: i32 = 24;

/// The boat camera for a target at `target` (world, on the water) whose
/// boat faces `boat_yaw`. `yaw` turns the camera round the target from
/// straight behind; `pitch` adds to the look's downward pitch.
///
/// The eye orbits a pivot over the boat; it looks at a point ahead of the
/// boat and low over the water, so the boat sits below the middle of the
/// screen and the water ahead shows above it. Land between the pivot and
/// the eye (an island's shoulder behind the boat) pulls the eye in toward
/// the pivot, down to `min_back_mm`; nearer than that it rises over the
/// land instead. It never sinks under the look's clearance.
pub fn boat(lake: &Lake, look: &CameraLook, target: V3, boat_yaw: i32, yaw: i32, pitch: i32) -> Camera {
    let turn = boat_yaw + yaw;
    let pivot = target.add(v3(0, mm(look.pivot_mm), 0));
    let down = deg(look.pitch_deg) + pitch;
    let orbit = facing(turn, -down);
    let want = pivot.sub(orbit.scale(mm(look.back_mm)));
    let eye = clear(lake, look, pull_in(lake, look, pivot, want));
    let aim = target
        .add(facing(turn, 0).scale(mm(look.ahead_mm)))
        .add(v3(0, mm(look.aim_mm), 0));
    Camera {
        eye,
        fwd: aim.sub(eye).norm(),
        fov: deg(look.fov_deg),
        near: mm(look.near_mm),
        far: mm(look.far_mm),
    }
}

/// The eye on the line from `pivot` to `want`, pulled in to just before
/// the first point where the land comes within the clearance of the line;
/// no nearer the pivot than `min_back_mm`.
fn pull_in(lake: &Lake, look: &CameraLook, pivot: V3, want: V3) -> V3 {
    let clear = mm(look.clear_mm);
    let blocked = |p: V3| p.y < mm(lake.ground_mm(lake.map_point(p))) + clear;
    let least = (look.min_back_mm as i64 * ONE as i64 / look.back_mm.max(1) as i64) as i32;
    for k in 1..=LAND_STEPS {
        let t = k * ONE / LAND_STEPS;
        if blocked(pivot.lerp(want, t)) {
            let ok = ((k - 1) * ONE / LAND_STEPS).max(least).min(ONE);
            return pivot.lerp(want, ok);
        }
    }
    want
}

/// A camera `dist` (world metres) from `centre`, looking at it from the
/// yaw `yaw` and `pitch` down.
pub fn fly_over(look: &CameraLook, centre: V3, yaw: i32, pitch: i32, dist: i32) -> Camera {
    let fwd = facing(yaw, -pitch);
    Camera {
        eye: centre.sub(fwd.scale(dist)),
        fwd,
        fov: deg(look.fov_deg),
        near: mm(look.near_mm),
        far: mm(look.far_mm),
    }
}

/// An eye moved up, if it must be, to the look's clearance over the
/// ground and the water.
pub fn clear(lake: &Lake, look: &CameraLook, eye: V3) -> V3 {
    let ground = lake.ground_mm(lake.map_point(eye)).max(0);
    let lowest = mm(ground + look.clear_mm);
    v3(eye.x, eye.y.max(lowest), eye.z)
}
