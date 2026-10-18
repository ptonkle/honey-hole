//! What a player sees of a minigame, as plain values: the targets on the
//! canvas, where they are going, and the progress a person reads off the
//! screen. The AI players play from this and nothing else (`ai::hand`),
//! moving the pointer and the button through a seat's input as a person
//! would. Nothing hidden is in it: the Enhanced Radar's sweet spots are
//! not, only how blurred each slider leaves the picture.

use super::shape::Pt;
use super::{Game, Minigame, State};
use crate::ui2d::Rect;

/// A free atom of the Nitrous minigame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtomSeen {
    pub nitrogen: bool,
    pub at: Pt,
    /// thousandths of a unit a tick, as its motion shows
    pub vel: [i32; 2],
}

/// A fish of the camera's minigame in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlyerSeen {
    pub at: Pt,
    /// thousandths of a unit a tick
    pub vel: [i32; 2],
}

/// One slider of the Enhanced Radar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliderSeen {
    /// its track's x, and its knob's y
    pub x: i32,
    pub knob_y: i32,
    /// how much of the picture's fault it leaves, 0 (sharp) to 1000
    pub fault: i32,
}

/// The screen of a minigame, canvas units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sight {
    /// the banner is up, or it is over: nothing to do
    Wait,
    /// circle the reel clockwise, outside its dead zone
    Reel { centre: Pt, radius: i32, dead_zone: i32 },
    /// the boat's cells still drawn, and the eraser's side
    Cloak { left: Vec<Pt>, eraser: i32 },
    /// the free atoms, the one held (nitrogen or not), the bins (Nitrogen,
    /// Oxygen) and how near a press must be to take an atom
    Atoms {
        free: Vec<AtomSeen>,
        held: Option<bool>,
        bins: [Rect; 2],
        reach: i32,
    },
    /// each circle and the ticks until its ring meets its edge (below 0
    /// after); a click inside within `window` ticks of it is a hit
    Beats {
        circles: Vec<(Pt, i32)>,
        radius: i32,
        window: i32,
    },
    /// the fish in flight, gravity (thousandths a tick a tick) and how near
    /// a click must be
    Camera {
        fish: Vec<FlyerSeen>,
        gravity: i32,
        reach: i32,
        canvas: [i32; 2],
    },
    /// the three sliders, their tracks' ends (top, bottom y), and whether
    /// the picture has snapped sharp
    Focus {
        sliders: Vec<SliderSeen>,
        top: i32,
        bottom: i32,
        snapped: bool,
    },
}

impl Minigame {
    /// What is on the screen now.
    pub fn sight(&self) -> Sight {
        if self.state != State::Playing || self.in_banner() {
            return Sight::Wait;
        }
        match &self.game {
            Game::Reel(g) => Sight::Reel {
                centre: g.centre,
                radius: g.t.radius,
                dead_zone: g.t.dead_zone,
            },
            Game::Cloak(g) => Sight::Cloak {
                left: g.left_cells(),
                eraser: g.t.eraser,
            },
            Game::Atoms(g) => Sight::Atoms {
                free: g
                    .atoms
                    .iter()
                    .filter(|a| a.at == super::atoms::Where::Free)
                    .map(|a| AtomSeen {
                        nitrogen: a.nitrogen,
                        at: a.pos(),
                        vel: [a.vx, a.vy],
                    })
                    .collect(),
                held: g.holding().map(|k| g.atoms[k].nitrogen),
                bins: g.bins,
                reach: g.t.atom_radius + g.t.grab_slop,
            },
            Game::Beats(g) => Sight::Beats {
                circles: g
                    .circles
                    .iter()
                    .map(|c| (c.at, g.meets(c) as i32 - g.now as i32))
                    .collect(),
                radius: g.t.radius,
                window: crate::sim::ticks(g.t.window_ms) as i32,
            },
            Game::Camera(g) => Sight::Camera {
                fish: g
                    .fish
                    .iter()
                    .map(|f| FlyerSeen {
                        at: f.pos(),
                        vel: [f.vx, f.vy],
                    })
                    .collect(),
                gravity: g.gravity(),
                reach: g.reach(),
                canvas: self.canvas,
            },
            Game::Focus(g) => Sight::Focus {
                sliders: (0..super::focus::SLIDERS)
                    .map(|k| SliderSeen {
                        x: super::focus::TRACK_X[k],
                        knob_y: super::focus::Focus::track_y(g.value[k]),
                        fault: g.fault(k),
                    })
                    .collect(),
                top: super::focus::TRACK_TOP,
                bottom: super::focus::TRACK_BOTTOM,
                snapped: g.snapped.is_some(),
            },
        }
    }
}
