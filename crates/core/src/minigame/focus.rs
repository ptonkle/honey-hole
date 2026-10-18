//! Enhanced Radar, on the Fish-Boy: a blurred picture of a fish and three
//! vertical sliders (`brief/radar-minigame.png`), each with a hidden sweet
//! spot drawn from the seed. Each slider clears one fault in the picture as
//! it nears its spot: the first a blur (offset copies laid over each other),
//! the second coarse pixels, the third a double image. Within
//! `snap_within_pct` of every spot the picture snaps sharp for `snap_ms`,
//! then the minigame is done.
//!
//! Slider values are thousandths of the track, 0 at the bottom.

use super::shape::{self, Pt};
use super::tuning::FocusTuning;
use super::{cue, ticks_of, Ctx, Input, Paint};
use crate::map::rng::{mix, Rng};
use crate::map::trig;
use crate::ui2d::theme::c;
use crate::ui2d::{Align, Rect, Rgba};

pub const SLIDERS: usize = 3;

/// The picture's frame and the sliders' tracks.
const FRAME: Rect = Rect::new(60, 110, 1060, 690);
const TRACK_X: [i32; SLIDERS] = [1250, 1380, 1510];
const TRACK_TOP: i32 = 150;
const TRACK_BOTTOM: i32 = 760;
/// A press this far either side of a track takes its slider.
const GRAB_HALF_W: i32 = 60;
const KNOB: [i32; 2] = [96, 40];
/// Tries for a start clear of the sweet spot before taking a fixed one.
const START_TRIES: i32 = 16;
/// Pixels smaller than this are drawn as the smooth outline.
const PIXEL_MIN: i32 = 10;
/// The blur's offset copies.
const BLUR_COPIES: i32 = 6;

#[derive(Debug, Clone)]
pub struct Focus {
    pub t: FocusTuning,
    pub sweet: [i32; SLIDERS],
    pub value: [i32; SLIDERS],
    grabbed: Option<usize>,
    /// the tick the picture snapped sharp
    pub snapped: Option<u32>,
    pub now: u32,
}

impl Focus {
    pub fn new(t: &FocusTuning, _canvas: [i32; 2], rng: &mut Rng) -> Focus {
        let m = t.sweet_margin_permille;
        let gap = t.start_gap_permille;
        let mut sweet = [0; SLIDERS];
        let mut value = [0; SLIDERS];
        for k in 0..SLIDERS {
            let s = rng.range(m, 1000 - m);
            let mut v = if s < 500 { s + gap } else { s - gap };
            for _ in 0..START_TRIES {
                let try_v = rng.range(0, 1000);
                if (try_v - s).abs() >= gap {
                    v = try_v;
                    break;
                }
            }
            sweet[k] = s;
            value[k] = v;
        }
        Focus {
            t: t.clone(),
            sweet,
            value,
            grabbed: None,
            snapped: None,
            now: 0,
        }
    }

    /// Each slider's distance from its spot, thousandths.
    pub fn off(&self, k: usize) -> i32 {
        (self.value[k] - self.sweet[k]).abs()
    }

    /// How many sliders are within the snap.
    pub fn within(&self) -> i32 {
        let near = self.t.snap_within_pct * 10;
        (0..SLIDERS).filter(|&k| self.off(k) <= near).count() as i32
    }

    /// A slider's fault, 0 (sharp) to 1000 (at its worst).
    pub fn fault(&self, k: usize) -> i32 {
        if self.snapped.is_some() {
            return 0;
        }
        (self.off(k) * 1000 / self.t.falloff_permille.max(1)).min(1000)
    }

    pub fn track_y(v: i32) -> i32 {
        TRACK_BOTTOM - v * (TRACK_BOTTOM - TRACK_TOP) / 1000
    }

    /// The value at pointer height `y`.
    pub fn value_at(y: i32) -> i32 {
        ((TRACK_BOTTOM - y) * 1000 / (TRACK_BOTTOM - TRACK_TOP)).clamp(0, 1000)
    }

    /// The point a pointer would press to take slider `k` at value `v`.
    #[cfg(test)]
    pub fn knob_at(k: usize, v: i32) -> Pt {
        (TRACK_X[k], Self::track_y(v))
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &Input) -> bool {
        self.now += 1;
        if let Some(at) = self.snapped {
            return self.now >= at + ticks_of(self.t.snap_ms);
        }
        if input.pressed {
            if let Some((x, y)) = input.pointer {
                let on_track = y >= TRACK_TOP - KNOB[1] && y <= TRACK_BOTTOM + KNOB[1];
                self.grabbed = (0..SLIDERS).find(|&k| (x - TRACK_X[k]).abs() <= GRAB_HALF_W && on_track);
            }
        }
        if let (Some(k), Some((_, y))) = (self.grabbed, input.pointer) {
            self.value[k] = Self::value_at(y);
        }
        if input.released || !input.held {
            self.grabbed = None;
        }
        if self.within() == SLIDERS as i32 {
            self.snapped = Some(self.now);
            self.grabbed = None;
            ctx.cues.push(cue::SUCCESS);
            return self.t.snap_ms <= 0;
        }
        false
    }

    pub fn draw(&self, p: &mut Paint) {
        let all = p.look;
        let look = &all.enhanced_radar;
        let [w, h] = p.canvas;
        p.list.fill_rect(c(look.background), Rect::new(0, 0, w, h));
        let snapped = self.snapped.is_some();
        let frame_col = if snapped { look.snap } else { look.frame };
        p.list
            .stroke_rect(c(frame_col), if snapped { 10 } else { 5 }, FRAME);
        p.list.clip(FRAME.inset(6));
        self.draw_fish(p);
        // static across the picture, thicker the further off the sliders are
        let mean = (0..SLIDERS).map(|k| self.fault(k)).sum::<i32>() / SLIDERS as i32;
        let lines = mean / 40;
        for k in 0..lines {
            let r = mix(self.now as u64 * 977 + k as u64);
            let y = FRAME.y + (r % FRAME.h as u64) as i32;
            let x = FRAME.x + ((r >> 20) % (FRAME.w as u64 / 2)) as i32;
            let len = 60 + ((r >> 40) % 300) as i32;
            p.list
                .fill_rect(c(look.fish_line).alpha(30), Rect::new(x, y, len, 3));
        }
        p.list.unclip();
        if snapped {
            p.list.text(
                c(look.snap),
                p.font,
                44,
                Align::Centre,
                (FRAME.x + FRAME.w / 2, FRAME.bottom() - 40),
                &(p.t)("minigame.enhanced_radar.sharp").to_uppercase(),
            );
        }
        // the sliders
        for (k, &x) in TRACK_X.iter().enumerate() {
            p.list.line(c(look.track), 8, (x, TRACK_TOP), (x, TRACK_BOTTOM));
            for step in 0..=10 {
                let y = Self::track_y(step * 100);
                let half = if step % 5 == 0 { 26 } else { 14 };
                p.list.line(c(look.track), 3, (x - half, y), (x + half, y));
            }
            let y = Self::track_y(self.value[k]);
            let knob = Rect::new(x - KNOB[0] / 2, y - KNOB[1] / 2, KNOB[0], KNOB[1]);
            let held = self.grabbed == Some(k);
            let fill = if held { look.knob_line } else { look.knob };
            p.list.round_rect(c(fill), c(look.knob_line), 5, knob, 6);
            p.list.text(
                c(look.track),
                p.font,
                30,
                Align::Centre,
                (x, TRACK_BOTTOM + 44),
                &(k + 1).to_string(),
            );
        }
    }

    fn draw_fish(&self, p: &mut Paint) {
        let look = &p.look.enhanced_radar;
        let mid = (FRAME.x + FRAME.w / 2, FRAME.y + FRAME.h / 2);
        let len = FRAME.w * 62 / 100;
        let blur = self.t.blur_max * self.fault(0) / 1000;
        let pixel = self.t.pixel_max * self.fault(1) / 1000;
        let ghost = self.t.ghost_max * self.fault(2) / 1000;
        // the copies: (offset, alpha)
        let mut copies: Vec<(Pt, i32)> = Vec::new();
        if ghost > 1 {
            copies.push(((ghost, -ghost / 5), 40));
        }
        let main = (-ghost / 3, 0);
        if blur > 1 {
            for k in 0..BLUR_COPIES {
                let a = k * trig::TURN / BLUR_COPIES;
                let off = (blur * trig::cos(a) / trig::ONE, blur * trig::sin(a) / trig::ONE);
                copies.push(((main.0 + off.0, main.1 + off.1), 34));
            }
        } else {
            copies.push((main, 100));
        }
        let o = shape::fish(mid, len, 0, false);
        if pixel >= PIXEL_MIN {
            // the picture in coarse cells, sampled at each cell's middle
            let (x0, y0) = (mid.0 - len / 2 - pixel, mid.1 - len / 4 - pixel);
            let (cols, rows) = ((len + 2 * pixel) / pixel, (len / 2 + 2 * pixel) / pixel);
            let fill_of = |pt: Pt| -> Option<[i32; 3]> {
                if shape::inside(pt, &o.belly) {
                    Some(look.fish_line)
                } else if shape::inside(pt, &o.body) {
                    Some(look.fish)
                } else if shape::inside(pt, &o.tail) || shape::inside(pt, &o.fin) {
                    Some(look.fish_line)
                } else {
                    None
                }
            };
            let mut runs: Vec<(Rect, [i32; 3])> = Vec::new();
            for row in 0..rows {
                let mut col = 0;
                while col < cols {
                    let at = |col: i32| (x0 + col * pixel + pixel / 2, y0 + row * pixel + pixel / 2);
                    let f = fill_of(at(col));
                    let start = col;
                    while col < cols && fill_of(at(col)) == f {
                        col += 1;
                    }
                    if let Some(f) = f {
                        runs.push((
                            Rect::new(x0 + start * pixel, y0 + row * pixel, (col - start) * pixel, pixel),
                            f,
                        ));
                    }
                }
            }
            for (off, alpha) in &copies {
                for (r, f) in &runs {
                    p.list
                        .fill_rect(c(*f).alpha(*alpha), Rect::new(r.x + off.0, r.y + off.1, r.w, r.h));
                }
            }
        } else {
            for (off, alpha) in &copies {
                let o = shape::fish((mid.0 + off.0, mid.1 + off.1), len, 0, false);
                let edge = c(look.fish_line).alpha(*alpha);
                p.list.poly(c(look.fish_line).alpha(*alpha), edge, 4, &o.tail);
                p.list.poly(c(look.fish_line).alpha(*alpha), edge, 4, &o.fin);
                p.list.poly(c(look.fish).alpha(*alpha), edge, 5, &o.body);
                p.list
                    .poly(c(look.fish_line).alpha(*alpha * 6 / 10), Rgba::NONE, 0, &o.belly);
                p.list
                    .circle(c(look.eye).alpha(*alpha), Rgba::NONE, 0, o.eye, o.eye_r * 2);
            }
        }
    }
}
