//! The match as plain data for drawing: where each boat is between two
//! ticks, its mode and state, and the time left. The world renderer, the
//! HUD and the radar read this; none of them read the match's insides.

use serde::Serialize;

use super::boat::{Mode, UM};
use super::{Match, Occupant, SEAT_COLOURS};
use crate::gfx::fx;
use crate::map::trig;

/// One boat, interpolated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BoatView {
    pub seat: usize,
    /// the seat's player colour name ("blue", ...), for `data/look.json`
    pub colour: &'static str,
    pub name: String,
    pub human: bool,
    /// the middle of the hull, mm (east, north)
    pub x_mm: i32,
    pub y_mm: i32,
    /// where the bow points (65536 a turn, 0 east, a quarter north)
    pub heading: i32,
    pub mode: Mode,
    /// the casting direction, from the bow
    pub cast_yaw: i32,
    /// mm/s over the water, and along the bow (negative backing up)
    pub speed_mm_per_s: i32,
    pub forward_mm_per_s: i32,
    /// waiting to respawn: draw a wreck or nothing
    pub wrecked: bool,
    /// draw see-through
    pub ghosted: bool,
    pub stalled: bool,
    pub nitrous: bool,
}

/// What the screen shows of the match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorldView {
    pub tick: u32,
    pub remaining_ms: i32,
    pub ended: bool,
    pub paused: bool,
    pub map_seed: u32,
    /// one per taken seat, in seat order
    pub boats: Vec<BoatView>,
}

impl Match {
    /// The match `alpha` of the way (Q16, 0 to `fx::ONE`) from the last
    /// tick to the next: each boat between where it was and where it is.
    pub fn view(&self, alpha: i32) -> WorldView {
        let t = alpha.clamp(0, fx::ONE) as i64;
        let lerp = |a: i64, b: i64| a + (b - a) * t / fx::ONE as i64;
        let boats = self
            .boats
            .iter()
            .flatten()
            .map(|b| {
                let x = lerp(b.prev_um[0], b.pos_um[0]) / UM;
                let y = lerp(b.prev_um[1], b.pos_um[1]) / UM;
                let turn = fx::wrap(b.heading - b.prev_heading) as i64;
                let heading = trig::wrap(b.prev_heading + (turn * t / fx::ONE as i64) as i32);
                let set = &self.setup.seats[b.seat];
                BoatView {
                    seat: b.seat,
                    colour: SEAT_COLOURS[b.seat],
                    name: set.name.clone(),
                    human: set.occupant == Occupant::Human,
                    x_mm: x as i32,
                    y_mm: y as i32,
                    heading,
                    mode: b.mode,
                    cast_yaw: b.cast_yaw,
                    speed_mm_per_s: b.speed_mm_per_s(),
                    forward_mm_per_s: b.forward_speed(),
                    wrecked: b.wrecked(),
                    ghosted: b.ghosted(self.tick),
                    stalled: b.stalled(self.tick),
                    nitrous: b.nitrous,
                }
            })
            .collect();
        WorldView {
            tick: self.tick,
            remaining_ms: self.remaining_ms(),
            ended: self.ended,
            paused: self.paused(),
            map_seed: self.map.seed,
            boats,
        }
    }
}
