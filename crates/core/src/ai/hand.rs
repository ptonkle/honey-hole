//! The AI's hand on the mouse: it plays an item's minigame, or clicks the
//! radar for a teleport, by moving the pointer and the main button through
//! [`SeatInput`] as a person would. It sees only the minigame's screen
//! ([`Sight`]): the Enhanced Radar's sweet spots are found by sliding each
//! knob and watching the picture clear, not read.
//!
//! Each difficulty's hand (`ai.items.hand`) moves the pointer up to
//! `speed_per_s`, waits `pause_ms` after each grab, drop, click or photo,
//! misses its aim by up to `aim_noise` and a closing ring's moment by up to
//! `timing_noise_ms`, and circles the Power Reel at `reel_permille_per_s`
//! of a turn a second. Easy is slow and loose; Hard is quick and sure.
//!
//! - **Power Reel:** out to the drawn reel, then round it clockwise.
//! - **Cloaking Tech:** the button held, always to the nearest part of the
//!   boat still drawn.
//! - **Nitrous:** to the nearest free atom (where it will be), press, carry
//!   it to its own bin, let go.
//! - **Teleporter:** to the circle whose ring meets it next; click as it
//!   meets, give or take the timing's miss.
//! - **Camera:** to the nearest fish in sight (where it will be), click.
//! - **Enhanced Radar:** take a slider, slide it the way the picture
//!   clears, past its sharpest, back to the sharpest, let go; the next. A
//!   slider still blurred after all three is tried again.

use crate::map::rng::Rng;
use crate::map::trig;
use crate::minigame::shape::{dist2, Pt};
use crate::minigame::sight::Sight;
use crate::sim::input::SeatInput;
use crate::sim::{ticks, TICKS_PER_S};

use super::tuning::Hand as HandSkill;

/// Where the pointer starts, canvas units (the middle of 1600 x 900).
const START: Pt = (800, 450);
/// A slider whose fault is above this after all three are set is tried
/// again (the snap's 10% is a fault of 222 at the shipped falloff).
const SLIDER_OK: i32 = 150;

/// The Enhanced Radar's plan: the slider in hand and how its sweep goes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Focus {
    settled: [bool; 3],
    slider: Option<usize>,
    /// 0 to the knob, 1 sweeping, 2 back to the sharpest
    phase: u8,
    /// the sweep's way along the track (+1 down the screen)
    dir: i32,
    reversed: bool,
    first: Option<i32>,
    /// the sharpest seen, and where
    best: Option<(i32, i32)>,
    /// the pointer's y last tick, whose fault this tick shows
    last_y: Option<i32>,
}

/// One AI seat's hand.
#[derive(Debug, Clone)]
pub struct Hand {
    rng: Rng,
    pub at: Option<Pt>,
    down: bool,
    pause_until: u32,
    /// the aim's miss for the present target
    miss: Pt,
    /// the reel's angle, once on the reel
    angle: Option<i32>,
    focus: Focus,
    /// the ring's timing miss for the circle aimed at, ticks
    beat: Option<(Pt, i32)>,
}

impl Hand {
    pub fn new(seed: u64, seat: usize) -> Hand {
        Hand {
            rng: Rng::stream(seed, 0xB1 + seat as u64),
            at: None,
            down: false,
            pause_until: 0,
            miss: (0, 0),
            angle: None,
            focus: Focus::default(),
            beat: None,
        }
    }

    /// Lets go and forgets the last game (a new minigame or choice begins).
    pub fn reset(&mut self, tick: u32, skill: &HandSkill) {
        self.at = None;
        self.down = false;
        self.angle = None;
        self.focus = Focus::default();
        self.beat = None;
        self.pause_until = tick + ticks(skill.pause_ms);
    }

    fn step_len(skill: &HandSkill) -> i32 {
        (skill.speed_per_s / TICKS_PER_S).max(1)
    }

    fn draw_miss(&mut self, skill: &HandSkill) {
        let n = skill.aim_noise;
        self.miss = (self.rng.range(-n, n), self.rng.range(-n, n));
    }

    /// The pointer moved toward `to` by at most `step`.
    fn toward(from: Pt, to: Pt, step: i32) -> Pt {
        let (dx, dy) = ((to.0 - from.0) as i64, (to.1 - from.1) as i64);
        let d = trig::hypot(dx, dy);
        if d <= step as i64 {
            return to;
        }
        (
            from.0 + (dx * step as i64 / d) as i32,
            from.1 + (dy * step as i64 / d) as i32,
        )
    }

    /// The seat's input for the hand as it now is, `was` the button last tick.
    fn input(&self, was: bool) -> SeatInput {
        let mut out = SeatInput {
            pointer: self.at.map(|(x, y)| [x, y]),
            ..SeatInput::default()
        };
        out.held.click = self.down;
        out.pressed.click = self.down && !was;
        out.released.click = !self.down && was;
        out
    }

    /// A click at `target` (the radar's map for a teleport): there, then
    /// the button. True in the input's tick that clicks.
    pub fn click_at(&mut self, target: Pt, tick: u32, skill: &HandSkill) -> SeatInput {
        let was = self.down;
        let at = self.at.unwrap_or(START);
        self.down = false;
        if tick < self.pause_until {
            self.at = Some(at);
            return self.input(was);
        }
        let next = Self::toward(at, target, Self::step_len(skill));
        self.at = Some(next);
        if next == target && !was {
            self.down = true;
        }
        self.input(was)
    }

    /// One tick of a minigame.
    pub fn play(&mut self, sight: &Sight, tick: u32, skill: &HandSkill) -> SeatInput {
        let was = self.down;
        let at = *self.at.get_or_insert(START);
        if tick < self.pause_until || matches!(sight, Sight::Wait) {
            // the Cloak's eraser stays down through a pause
            if !matches!(sight, Sight::Cloak { .. }) {
                self.down = false;
            }
            return self.input(was);
        }
        let step = Self::step_len(skill);
        match sight {
            Sight::Wait => {}
            Sight::Reel {
                centre,
                radius,
                dead_zone,
            } => {
                let r = (*radius).max(dead_zone + 30);
                match self.angle {
                    None => {
                        let start = (centre.0 + r, centre.1);
                        let next = Self::toward(at, start, step);
                        self.at = Some(next);
                        if next == start {
                            self.angle = Some(0);
                        }
                    }
                    Some(a) => {
                        let per_tick = (trig::TURN as i64 * skill.reel_permille_per_s as i64
                            / 1000
                            / TICKS_PER_S as i64) as i32;
                        let a = a + per_tick.min(trig::TURN / 3);
                        self.angle = Some(a);
                        let n = skill.aim_noise;
                        let rr = r + self.rng.range(-n, n);
                        self.at = Some((
                            centre.0 + rr * trig::cos(a) / trig::ONE,
                            centre.1 + rr * trig::sin(a) / trig::ONE,
                        ));
                    }
                }
            }
            Sight::Cloak { left, .. } => {
                self.down = true;
                if let Some(&cell) = left.iter().min_by_key(|&&c| dist2(c, at)) {
                    self.at = Some(Self::toward(at, cell, step));
                }
            }
            Sight::Atoms {
                free,
                held,
                bins,
                reach,
            } => match held {
                Some(nitrogen) => {
                    let bin = bins[if *nitrogen { 0 } else { 1 }];
                    let mid = (bin.x + bin.w / 2 + self.miss.0, bin.y + bin.h / 2 + self.miss.1);
                    let next = Self::toward(at, mid, step);
                    self.at = Some(next);
                    self.down = true;
                    if next == mid {
                        self.down = false;
                        self.pause(tick, skill);
                    }
                }
                None => {
                    if self.down {
                        // the press caught nothing: let go and look again
                        self.down = false;
                        self.draw_miss(skill);
                        return self.input(was);
                    }
                    let lead = |a: &crate::minigame::sight::AtomSeen| {
                        let eta = (trig::hypot((a.at.0 - at.0) as i64, (a.at.1 - at.1) as i64) / step as i64)
                            .min(40) as i32;
                        (a.at.0 + a.vel[0] * eta / 1000, a.at.1 + a.vel[1] * eta / 1000)
                    };
                    if let Some(a) = free.iter().min_by_key(|a| dist2(lead(a), at)) {
                        let aim = lead(a);
                        let aim = (aim.0 + self.miss.0, aim.1 + self.miss.1);
                        let next = Self::toward(at, aim, step);
                        self.at = Some(next);
                        let r = (*reach as i64) * 2 / 3;
                        let seen = (a.at.0 + self.miss.0, a.at.1 + self.miss.1);
                        if dist2(next, seen) <= r * r {
                            self.down = true;
                        }
                    }
                }
            },
            Sight::Beats {
                circles,
                radius,
                window,
            } => {
                self.down = false;
                let next_up = circles
                    .iter()
                    .filter(|(_, t)| *t >= -*window)
                    .min_by_key(|(_, t)| *t)
                    .copied();
                let Some((c, ttm)) = next_up else {
                    return self.input(was);
                };
                let err = match self.beat {
                    Some((p, e)) if p == c => e,
                    _ => {
                        let n = ticks(skill.timing_noise_ms) as i32;
                        let e = self.rng.range(-n, n);
                        self.beat = Some((c, e));
                        self.draw_miss(skill);
                        e
                    }
                };
                let aim = (c.0 + self.miss.0, c.1 + self.miss.1);
                let next = Self::toward(at, aim, step);
                self.at = Some(next);
                let r = *radius as i64;
                if !was && ttm <= err && dist2(next, c) <= r * r {
                    self.down = true;
                    self.beat = None;
                }
            }
            Sight::Camera {
                fish,
                gravity,
                reach,
                canvas,
            } => {
                self.down = false;
                let on = |p: Pt| (0..canvas[0]).contains(&p.0) && (0..canvas[1]).contains(&p.1);
                let lead = |f: &crate::minigame::sight::FlyerSeen| {
                    let eta =
                        (trig::hypot((f.at.0 - at.0) as i64, (f.at.1 - at.1) as i64) / step as i64).min(40);
                    (
                        f.at.0 + (f.vel[0] as i64 * eta / 1000) as i32,
                        f.at.1 + ((f.vel[1] as i64 * eta + *gravity as i64 * eta * eta / 2) / 1000) as i32,
                    )
                };
                let best = fish
                    .iter()
                    .filter(|f| on(f.at) && on(lead(f)))
                    .min_by_key(|f| dist2(lead(f), at));
                let Some(f) = best else {
                    return self.input(was);
                };
                let aim = lead(f);
                let next = Self::toward(at, (aim.0 + self.miss.0, aim.1 + self.miss.1), step);
                self.at = Some(next);
                let seen = (f.at.0 + self.miss.0, f.at.1 + self.miss.1);
                let r = *reach as i64 * 2 / 3;
                if !was && dist2(next, seen) <= r * r {
                    self.down = true;
                    self.pause(tick, skill);
                }
            }
            Sight::Focus {
                sliders,
                top,
                bottom,
                snapped,
            } => {
                if *snapped {
                    self.down = false;
                    return self.input(was);
                }
                self.focus_step(sliders, *top, *bottom, at, step, tick, skill);
            }
        }
        self.input(was)
    }

    fn pause(&mut self, tick: u32, skill: &HandSkill) {
        self.pause_until = tick + ticks(skill.pause_ms);
        self.draw_miss(skill);
    }

    #[allow(clippy::too_many_arguments)]
    fn focus_step(
        &mut self,
        sliders: &[crate::minigame::sight::SliderSeen],
        top: i32,
        bottom: i32,
        at: Pt,
        step: i32,
        tick: u32,
        skill: &HandSkill,
    ) {
        let f = &mut self.focus;
        if f.slider.is_none() {
            // all set and still blurred: try again the ones off
            if f.settled.iter().all(|s| *s) {
                for (k, s) in sliders.iter().enumerate().take(3) {
                    f.settled[k] = s.fault <= SLIDER_OK;
                }
            }
            f.slider = (0..sliders.len().min(3)).find(|&k| !f.settled[k]);
            f.phase = 0;
            f.best = None;
            f.first = None;
            f.reversed = false;
            f.last_y = None;
        }
        let Some(k) = f.slider else {
            self.down = false;
            return;
        };
        let s = sliders[k];
        match f.phase {
            0 => {
                self.down = false;
                let knob = (s.x, s.knob_y);
                let next = Self::toward(at, knob, step);
                self.at = Some(next);
                if next == knob {
                    self.down = true;
                    let mid = (top + bottom) / 2;
                    let f = &mut self.focus;
                    f.dir = if s.knob_y > mid { -1 } else { 1 };
                    f.phase = 1;
                }
            }
            1 => {
                self.down = true;
                // the fault seen now is for last tick's place
                if let Some(y) = f.last_y {
                    let first = *f.first.get_or_insert(s.fault);
                    if f.best.is_none_or(|(b, _)| s.fault < b) {
                        f.best = Some((s.fault, y));
                    }
                    let best = f.best.map_or(1000, |b| b.0);
                    let worse = s.fault > first + 50 && best >= first;
                    let past = best < first && s.fault > best + 100;
                    let end = (f.dir < 0 && y <= top) || (f.dir > 0 && y >= bottom);
                    if worse && !f.reversed {
                        f.dir = -f.dir;
                        f.reversed = true;
                    } else if past || (end && best < 1000) {
                        f.phase = 2;
                        self.draw_miss(skill);
                        return;
                    } else if end {
                        f.dir = -f.dir;
                        f.reversed = true;
                    }
                }
                let sweep = (step / 2).max(1);
                let y = (at.1 + self.focus.dir * sweep).clamp(top, bottom);
                self.at = Some((s.x, y));
                self.focus.last_y = Some(y);
            }
            _ => {
                self.down = true;
                let y = f.best.map_or(s.knob_y, |b| b.1) + self.miss.1 / 2;
                let to = (s.x, y.clamp(top, bottom));
                let next = Self::toward(at, to, (step / 2).max(1));
                self.at = Some(next);
                if next == to {
                    self.down = false;
                    let f = &mut self.focus;
                    f.settled[k] = true;
                    f.slider = None;
                    self.pause_until = tick + ticks(skill.pause_ms) / 2;
                }
            }
        }
    }
}
