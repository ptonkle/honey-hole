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
use crate::gfx::fx::deg;
use crate::gfx::geom::{facing, v3, V3};
use crate::gfx::render::Camera;

/// The boat camera for a target at `target` (world, on the water) whose
/// boat faces `boat_yaw`. `yaw` turns the camera round the target from
/// straight behind; `pitch` adds to the look's downward pitch.
pub fn boat(lake: &Lake, look: &CameraLook, target: V3, boat_yaw: i32, yaw: i32, pitch: i32) -> Camera {
    let aim = target.add(v3(0, mm(look.aim_mm), 0));
    let down = deg(look.pitch_deg) + pitch;
    let back = facing(boat_yaw + yaw, -down);
    let eye = aim
        .sub(back.scale(mm(look.back_mm)))
        .add(v3(0, mm(look.up_mm - look.aim_mm), 0));
    let eye = clear(lake, look, eye);
    Camera {
        eye,
        fwd: aim.sub(eye).norm(),
        fov: deg(look.fov_deg),
        near: mm(look.near_mm),
        far: mm(look.far_mm),
    }
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
