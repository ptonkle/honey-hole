//! Power Reel: a reel drawn as a circle; the pointer circles it clockwise
//! `turns` times. The angle swept round the middle is summed tick by tick
//! (and pointer move by pointer move, through `Minigame::point`).
//! Anticlockwise does not count: it winds back the turn in progress, never
//! a finished one, so wiggling back and forth gains nothing. The pointer
//! within `dead_zone` of the middle counts for nothing and starts afresh.

use super::shape::Pt;
use super::tuning::ReelTuning;
use super::{cue, Ctx, Input, Paint};
use crate::map::trig::{self, HALF, QUARTER, TURN};
use crate::ui2d::theme::c;
use crate::ui2d::{Rect, Rgba};

#[derive(Debug, Clone)]
pub struct Reel {
    pub t: ReelTuning,
    pub centre: Pt,
    /// clockwise angle swept, never below `banked`
    pub swept: i64,
    /// whole turns finished, in angle units
    pub banked: i64,
    last: Option<i32>,
}

/// An angle difference brought into -HALF..HALF: the short way round.
fn signed(d: i32) -> i32 {
    let d = trig::wrap(d);
    if d > HALF {
        d - TURN
    } else {
        d
    }
}

impl Reel {
    pub fn new(t: &ReelTuning, canvas: [i32; 2]) -> Reel {
        Reel {
            t: t.clone(),
            centre: (canvas[0] / 2, canvas[1] / 2 + 20),
            swept: 0,
            banked: 0,
            last: None,
        }
    }

    pub fn turns_done(&self) -> i32 {
        (self.banked / TURN as i64) as i32
    }

    fn done(&self) -> bool {
        self.swept >= self.t.turns as i64 * TURN as i64
    }

    /// One pointer sample.
    pub fn sample(&mut self, ctx: &mut Ctx, at: Option<Pt>) {
        let Some(p) = at else {
            self.last = None;
            return;
        };
        let (dx, dy) = ((p.0 - self.centre.0) as i64, (p.1 - self.centre.1) as i64);
        let dz = self.t.dead_zone as i64;
        if dx * dx + dy * dy < dz * dz {
            self.last = None;
            return;
        }
        // with y down the screen, the angle grows clockwise
        let a = trig::atan2(dy, dx);
        if let Some(l) = self.last {
            self.swept = (self.swept + signed(a - l) as i64).max(self.banked);
            let goal = self.t.turns as i64 * TURN as i64;
            while self.swept >= self.banked + TURN as i64 && self.banked < goal {
                self.banked += TURN as i64;
                if self.banked < goal {
                    ctx.cues.push(cue::TICK);
                }
            }
        }
        self.last = Some(a);
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &Input) -> bool {
        self.sample(ctx, input.pointer);
        self.done()
    }

    pub fn draw(&self, p: &mut Paint) {
        let look = &p.look.power_reel;
        let [w, h] = p.canvas;
        p.list.fill_rect(c(look.background), Rect::new(0, 0, w, h));
        let r = self.t.radius;
        let o = self.centre;
        // the turning direction: a dashed ring with arrowheads, clockwise
        let ring = r + 70;
        p.list.dash(&[26, 22]);
        p.list.circle(Rgba::NONE, c(look.arrow).alpha(60), 6, o, ring);
        p.list.dash(&[]);
        for k in 0..4 {
            let a = k * QUARTER + QUARTER / 2;
            let tip = at_angle(o, ring, a + TURN / 40);
            let back = at_angle(o, ring, a - TURN / 40);
            let out = at_angle(o, ring + 22, a - TURN / 40);
            let inn = at_angle(o, ring - 22, a - TURN / 40);
            p.list.poly(c(look.arrow), Rgba::NONE, 0, &[tip, out, back, inn]);
        }
        // the spool, its line, the spokes turning with the sweep
        let spin = (self.swept % TURN as i64) as i32;
        p.list.circle(c(look.rim), Rgba::NONE, 0, o, r);
        p.list.circle(c(look.spool), Rgba::NONE, 0, o, r * 86 / 100);
        for k in 0..6 {
            p.list
                .circle(Rgba::NONE, c(look.line), 3, o, r * (50 + k * 6) / 100);
        }
        for k in 0..3 {
            let a = spin + k * TURN / 3;
            p.list.line(
                c(look.rim),
                14,
                at_angle(o, r * 22 / 100, a),
                at_angle(o, r * 84 / 100, a),
            );
        }
        // the handle on its arm, then the hub over the arm's root
        let knob = at_angle(o, r + 4, spin - QUARTER);
        p.list.line(c(look.rim), 18, o, knob);
        p.list.circle(c(look.handle), c(look.rim), 5, knob, 34);
        p.list.circle(c(look.hub), c(look.rim), 6, o, r * 22 / 100);
        // the turns done, as pips under the reel: the only progress shown
        // (the spokes turn with the hand; no rim arc and no bottom bar)
        let n = self.t.turns;
        let gap = 46;
        let x0 = o.0 - (n - 1) * gap / 2;
        for k in 0..n {
            let on = k < self.turns_done();
            let fill = if on { look.pip_on } else { look.pip_off };
            p.list
                .circle(c(fill), Rgba::NONE, 0, (x0 + k * gap, o.1 + ring + 50), 14);
        }
    }
}

/// The point `r` from `o` at angle `a` (clockwise on screen from the right).
fn at_angle(o: Pt, r: i32, a: i32) -> Pt {
    (
        o.0 + r * trig::cos(a) / trig::ONE,
        o.1 + r * trig::sin(a) / trig::ONE,
    )
}
