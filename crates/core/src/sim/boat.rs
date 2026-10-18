//! One player's boat: where it is, how it moves, and its mode.
//!
//! **Driving mode.** W pushes the boat up to top speed in `boat.accel_ms`;
//! S brakes (`brake_ms` from top speed to rest), then backs up slowly to
//! `reverse_speed`; with neither, the boat coasts to rest in `coast_ms`.
//! A and D turn, faster the faster the boat goes: `turn_at_rest_pct` of the
//! full rate at rest, rising in proportion to the full rate at
//! `turn_full_at_pct` of top speed. Sideways slip (after a turn or a
//! bounce) dies away in `side_drag_ms`.
//!
//! **Fishing mode.** Q switches instantly and keeps the boat's momentum; the
//! throttle is off, so it coasts to rest, and A and D swivel the cast
//! (`swivel`) instead of steering. The cast goes the way the seat's camera
//! looks (`SeatInput::look`) turned by the swivel, at most `swivel_limit`
//! either side of the bow (`cast_yaw`, SECOND-ORDER R7.74).
//!
//! Each tick runs `steer` (heading, or the cast's swivel), then the move and
//! its collisions (`collide`), then `throttle` (the speed for the next tick).
//! Collisions are judged on the speed the boat carried into the tick.

use serde::Serialize;

use super::input::SeatInput;
use super::Rules;
use crate::gfx::fx;
use crate::map::P;

/// 1.0 for the boat's directions (Q16, from `gfx::fx`).
pub const ONE: i32 = fx::ONE;

/// Micrometres a millimetre: boats move in µm so a slow speed still moves
/// them each tick (a mm/s for a 20 ms tick is 20 µm).
pub const UM: i64 = 1000;

/// `a / b` rounded to the nearest, halves away from zero.
pub fn round_div(a: i64, b: i64) -> i64 {
    if (a < 0) == (b < 0) {
        (a + b / 2) / b
    } else {
        (a - b / 2) / b
    }
}

/// The unit vector of a heading, as (east, north) in Q16.
pub fn dir(heading: i32) -> (i32, i32) {
    (fx::cos(heading), fx::sin(heading))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Driving,
    Fishing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Boat {
    pub seat: usize,
    /// the middle of the hull, in µm (east, north)
    pub pos_um: [i64; 2],
    /// µm/s (east, north), so a turn's rounding does not wear the speed
    /// down; read and set it in mm/s with `vel` and `set_vel`
    pub vel_um: [i32; 2],
    /// where the bow points
    pub heading: i32,
    pub mode: Mode,
    /// the casting direction, from the bow (left positive): the camera's
    /// look turned by the swivel, set each tick in fishing mode
    pub cast_yaw: i32,
    /// A and D's turn of the cast from the camera's look (fishing mode)
    pub swivel: i32,
    /// wrecked; comes back at the spawn on this tick
    pub wrecked_until: Option<u32>,
    /// ghosted before this tick: passes through boats, cannot wreck or be
    /// wrecked by one
    pub ghost_until: u32,
    /// the motor is stalled before this tick (it rammed a boat)
    pub stall_until: u32,
    /// **Hook for the items system.** While true the boat accelerates
    /// faster, reaches a higher top speed and turns more sharply (the
    /// tuning's `boat.nitrous` percents). Ram and grounding speeds are still
    /// judged against the plain top speed.
    pub nitrous: bool,
    /// **Hook for the Fish-Boy and minigames.** While true the layer above
    /// has the keys: the throttle is released, so the boat coasts, and A, D
    /// and Q do nothing to the boat.
    pub captured: bool,
    /// last tick's place and heading, for drawing between ticks
    pub prev_um: [i64; 2],
    pub prev_heading: i32,
}

impl Boat {
    pub fn new(seat: usize, at: P, heading: i32) -> Boat {
        let pos = [at.x as i64 * UM, at.y as i64 * UM];
        Boat {
            seat,
            pos_um: pos,
            vel_um: [0, 0],
            heading,
            mode: Mode::Driving,
            cast_yaw: 0,
            swivel: 0,
            wrecked_until: None,
            ghost_until: 0,
            stall_until: 0,
            nitrous: false,
            captured: false,
            prev_um: pos,
            prev_heading: heading,
        }
    }

    /// The middle of the hull, in mm.
    pub fn at(&self) -> P {
        P::new((self.pos_um[0] / UM) as i32, (self.pos_um[1] / UM) as i32)
    }

    pub fn set_at(&mut self, p: P) {
        self.pos_um = [p.x as i64 * UM, p.y as i64 * UM];
    }

    pub fn wrecked(&self) -> bool {
        self.wrecked_until.is_some()
    }

    pub fn ghosted(&self, tick: u32) -> bool {
        tick < self.ghost_until
    }

    pub fn stalled(&self, tick: u32) -> bool {
        tick < self.stall_until
    }

    /// The velocity in mm/s (east, north), rounded.
    pub fn vel(&self) -> [i32; 2] {
        self.vel_um.map(|v| round_div(v as i64, UM) as i32)
    }

    pub fn set_vel(&mut self, v: [i32; 2]) {
        self.vel_um = v.map(|c| (c as i64 * UM) as i32);
    }

    fn fwd_um(&self) -> i64 {
        let (c, s) = dir(self.heading);
        (self.vel_um[0] as i64 * c as i64 + self.vel_um[1] as i64 * s as i64) / ONE as i64
    }

    fn side_um(&self) -> i64 {
        let (c, s) = dir(self.heading);
        (-(self.vel_um[0] as i64) * s as i64 + self.vel_um[1] as i64 * c as i64) / ONE as i64
    }

    /// Speed along the bow, mm/s (negative backing up).
    pub fn forward_speed(&self) -> i32 {
        round_div(self.fwd_um(), UM) as i32
    }

    /// Speed across the bow, mm/s (positive to the left).
    pub fn side_speed(&self) -> i32 {
        round_div(self.side_um(), UM) as i32
    }

    /// The boat's speed over the water, mm/s, whichever way it moves (the
    /// fishing system allows a cast only below its own limit).
    pub fn speed_mm_per_s(&self) -> i32 {
        let v = self.vel_um;
        round_div(crate::map::trig::hypot(v[0] as i64, v[1] as i64), UM) as i32
    }

    fn set_motion(&mut self, fwd: i64, side: i64) {
        let (c, s) = dir(self.heading);
        let (c, s) = (c as i64, s as i64);
        // over the direction's own length squared, which is not quite one,
        // so splitting the velocity and joining it again changes nothing
        let norm2 = c * c + s * s;
        self.vel_um = [
            ((fwd * c - side * s) * ONE as i64 / norm2) as i32,
            ((fwd * s + side * c) * ONE as i64 / norm2) as i32,
        ];
    }

    pub(crate) fn remember(&mut self) {
        self.prev_um = self.pos_um;
        self.prev_heading = self.heading;
    }

    /// `a` percent of `v`.
    fn pct(v: i32, a: i32) -> i32 {
        (v as i64 * a as i64 / 100) as i32
    }

    /// Top speed now, with nitrous if it is on.
    pub fn top(&self, r: &Rules) -> i32 {
        if self.nitrous {
            Self::pct(r.top, r.nitrous_top_pct)
        } else {
            r.top
        }
    }

    /// Turning (driving) or swivelling the cast (fishing), from A and D.
    pub(crate) fn steer(&mut self, r: &Rules, i: &SeatInput) {
        if self.captured {
            return;
        }
        let way = i.down("left") as i32 - i.down("right") as i32;
        if self.mode == Mode::Fishing {
            let limit = r.swivel_limit;
            self.swivel = (self.swivel + way * r.swivel).clamp(-limit, limit);
            // the camera's look as an angle either side of the bow
            let look = fx::wrap(i.look);
            self.cast_yaw = (look + self.swivel).clamp(-limit, limit);
            return;
        }
        if way == 0 {
            return;
        }
        match self.mode {
            Mode::Driving => {
                let speed = self.forward_speed().abs().min(r.turn_full_at);
                let pct = r.turn_at_rest_pct
                    + ((100 - r.turn_at_rest_pct) as i64 * speed as i64 / r.turn_full_at.max(1) as i64)
                        as i32;
                let mut rate = Self::pct(r.turn, pct);
                if self.nitrous {
                    rate = Self::pct(rate, r.nitrous_turn_pct);
                }
                self.heading = crate::map::trig::wrap(self.heading + way * rate);
            }
            Mode::Fishing => {}
        }
    }

    /// The speed for the next tick: throttle, brake, reverse or coast, and
    /// the sideways slip dying away.
    pub(crate) fn throttle(&mut self, r: &Rules, i: &SeatInput, tick: u32) {
        if self.wrecked() {
            return;
        }
        // in µm/s: each rate is mm/s a tick, times a thousand
        let k = UM as i32;
        let mut fwd = self.fwd_um() as i32;
        let mut side = self.side_um() as i32;
        let motor = self.mode == Mode::Driving && !self.captured && !self.stalled(tick);
        let (w, s) = (motor && i.down("forward"), motor && i.down("back"));
        let top = self.top(r) * k;
        let accel = k * if self.nitrous {
            Self::pct(r.accel, r.nitrous_accel_pct)
        } else {
            r.accel
        };
        let (coast, brake, reverse, reverse_accel) =
            (r.coast * k, r.brake * k, r.reverse * k, r.reverse_accel * k);
        fwd = if w && !s {
            if fwd > top {
                fx::approach(fwd, top, coast)
            } else {
                fx::approach(fwd, top, accel)
            }
        } else if s && !w {
            if fwd > 0 {
                fx::approach(fwd, 0, brake)
            } else {
                fx::approach(fwd, -reverse, reverse_accel)
            }
        } else {
            fx::approach(fwd, 0, coast)
        };
        side = fx::approach(side, 0, r.side_drag * k);
        self.set_motion(fwd as i64, side as i64);
    }

    pub(crate) fn wreck(&mut self, respawn_at: u32) {
        self.wrecked_until = Some(respawn_at);
        self.vel_um = [0, 0];
        self.nitrous = false;
    }

    pub(crate) fn respawn(&mut self, at: P, heading: i32, ghost_until: u32) {
        let seat = self.seat;
        let captured = self.captured;
        *self = Boat::new(seat, at, heading);
        self.ghost_until = ghost_until;
        self.captured = captured;
    }
}
