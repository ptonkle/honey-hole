//! High-Quality Camera: fish leap in from the canvas's left, right and
//! bottom edges at random intervals and arc across under gravity, fruit
//! ninja style; each click within reach of a fish photographs it. Done at
//! `photos` photographs. The pointer is a viewfinder (corner brackets and a
//! cross in the middle) the minigame draws itself.
//!
//! The fish are drawn as flat 2D outlines rather than the 3D models: a
//! model viewport is a rectangle with its own background, which would cut
//! square holes round each fish crossing the sky and each other
//! (SECOND-ORDER R3.84).

use super::shape::{self, dist2, Pt};
use super::tuning::CameraTuning;
use super::{cue, ticks_of, Ctx, Input, Paint};
use crate::map::rng::Rng;
use crate::map::trig;
use crate::sim::TICK_MS;
use crate::ui2d::theme::c;
use crate::ui2d::{Rect, Rgba};

/// Where the sky meets the water, thousandths down the canvas.
const HORIZON_PERMILLE: i32 = 640;
/// A photo's frame lingers this many flashes long.
const PHOTO_FLASHES: u32 = 3;
/// The sky's bands, top to horizon.
const SKY_BANDS: i32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flyer {
    /// thousandths of a unit
    pub x: i32,
    pub y: i32,
    /// thousandths a tick
    pub vx: i32,
    pub vy: i32,
    pub colour: usize,
    pub born: u32,
}

impl Flyer {
    pub fn pos(&self) -> Pt {
        (self.x / 1000, self.y / 1000)
    }

    fn angle(&self) -> i32 {
        trig::atan2(self.vy as i64, self.vx as i64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Photo {
    fish: Flyer,
    tick: u32,
}

pub struct Shoot {
    pub t: CameraTuning,
    rng: Rng,
    canvas: [i32; 2],
    pub fish: Vec<Flyer>,
    photos_taken: Vec<Photo>,
    pub photos: i32,
    /// clicks that caught no fish
    pub missed: i32,
    pub now: u32,
    next_spawn: u32,
    /// gravity, thousandths a tick a tick
    gravity: i32,
    spawned: usize,
}

impl Shoot {
    pub fn new(t: &CameraTuning, canvas: [i32; 2], rng: Rng) -> Shoot {
        let tick = TICK_MS as i64;
        Shoot {
            t: t.clone(),
            rng,
            canvas,
            fish: Vec::new(),
            photos_taken: Vec::new(),
            photos: 0,
            missed: 0,
            now: 0,
            next_spawn: 0,
            gravity: (t.gravity_per_s2 as i64 * tick * tick / 1000) as i32,
            spawned: 0,
        }
    }

    /// A tick's worth of `per_s` units a second, in thousandths.
    fn per_tick(per_s: i64) -> i32 {
        (per_s * TICK_MS as i64) as i32
    }

    fn spawn(&mut self) {
        let [w, h] = self.canvas;
        let half = self.t.fish_len / 2;
        let speed = self.rng.range(self.t.speed_min_per_s, self.t.speed_max_per_s) as i64;
        let g = self.t.gravity_per_s2 as i64;
        // up fast enough to rise `rise` units before falling back
        let launch = |rise: i64| -> i64 {
            if g == 0 {
                speed
            } else {
                trig::isqrt(2 * g * rise)
            }
        };
        let edge = self.rng.below(3);
        let (x, y, vx, vy) = match edge {
            // the left or the right edge, thrown across and a little up
            0 | 1 => {
                let y = self.rng.range(h * 35 / 100, h * 75 / 100);
                let rise = self.rng.range(h * 5 / 100, h * 30 / 100) as i64;
                let dir = if edge == 0 { 1 } else { -1 };
                let x = if edge == 0 { -half } else { w + half };
                (x, y, dir * speed, -launch(rise))
            }
            // the bottom edge, thrown up and drifting toward the middle
            _ => {
                let x = self.rng.range(w * 15 / 100, w * 85 / 100);
                let rise = self.rng.range(h * 45 / 100, h * 80 / 100) as i64;
                let drift = self.rng.range(0, (speed / 2) as i32) as i64;
                let toward = if x < w / 2 { drift } else { -drift };
                (x, h + half, toward, -launch(rise))
            }
        };
        // the look's colours, taken in turn (read modulo its list)
        let colour = self.spawned;
        self.spawned += 1;
        self.fish.push(Flyer {
            x: x * 1000,
            y: y * 1000,
            vx: Self::per_tick(vx),
            vy: Self::per_tick(vy),
            colour,
            born: self.now,
        });
    }

    /// The fish a click at `p` photographs: the nearest within reach.
    pub fn under(&self, p: Pt) -> Option<usize> {
        let reach = (self.t.fish_len as i64 / 2) * self.t.hit_pct as i64 / 100;
        (0..self.fish.len())
            .filter(|&k| dist2(self.fish[k].pos(), p) <= reach * reach)
            .min_by_key(|&k| dist2(self.fish[k].pos(), p))
    }

    /// A fish whose middle is on the canvas.
    pub fn visible(&self) -> Option<Pt> {
        let [w, h] = self.canvas;
        self.fish
            .iter()
            .map(|f| f.pos())
            .find(|p| (0..w).contains(&p.0) && (0..h).contains(&p.1))
    }

    fn gone(&self, f: &Flyer) -> bool {
        let [w, h] = self.canvas;
        let m = self.t.fish_len;
        let (x, y) = f.pos();
        (x < -m && f.vx <= 0) || (x > w + m && f.vx >= 0) || (y > h + m && f.vy > 0)
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &Input) -> bool {
        if self.now >= self.next_spawn {
            if (self.fish.len() as i32) < self.t.max_fish {
                self.spawn();
                let lo = ticks_of(self.t.spawn_min_ms) as i32;
                let hi = ticks_of(self.t.spawn_max_ms) as i32;
                self.next_spawn = self.now + self.rng.range(lo, hi) as u32;
            } else {
                self.next_spawn = self.now + 1;
            }
        }
        if input.pressed {
            if let Some(p) = input.pointer {
                ctx.cues.push(cue::SHUTTER);
                match self.under(p) {
                    Some(k) => {
                        let fish = self.fish.remove(k);
                        self.photos += 1;
                        self.photos_taken.push(Photo { fish, tick: self.now });
                    }
                    None => self.missed += 1,
                }
            }
        }
        for f in &mut self.fish {
            f.x += f.vx;
            f.y += f.vy;
            f.vy += self.gravity;
        }
        let kept: Vec<Flyer> = self.fish.iter().copied().filter(|f| !self.gone(f)).collect();
        self.fish = kept;
        let linger = ticks_of(self.t.flash_ms) * PHOTO_FLASHES;
        let now = self.now;
        self.photos_taken.retain(|ph| now < ph.tick + linger);
        self.now += 1;
        self.photos >= self.t.photos
    }

    fn draw_fish(&self, p: &mut Paint, f: &Flyer, alpha: i32) {
        let look = &p.look.camera;
        let a = f.angle();
        let flip = f.vx < 0;
        let o = shape::fish(f.pos(), self.t.fish_len, a, flip);
        let body = look.fish[f.colour % look.fish.len()];
        let edge = c(body).shade(55).alpha(alpha);
        p.list.alpha(alpha);
        p.list.poly(c(body).shade(80), edge, 3, &o.tail);
        p.list.poly(c(body).shade(80), edge, 3, &o.fin);
        p.list.poly(c(body), edge, 3, &o.body);
        p.list.poly(c(look.belly).alpha(70), Rgba::NONE, 0, &o.belly);
        p.list.circle(c(look.eye), Rgba::NONE, 0, o.eye, o.eye_r);
        p.list.alpha(100);
    }

    pub fn draw(&self, p: &mut Paint) {
        let all = p.look;
        let look = &all.camera;
        let [w, h] = p.canvas;
        let horizon = h * HORIZON_PERMILLE / 1000;
        // the sky in bands, from its top colour down to the haze at the
        // horizon
        for k in 0..SKY_BANDS {
            let mix = |a: i32, b: i32| a + (b - a) * k / (SKY_BANDS - 1);
            let col = [0, 1, 2].map(|i| mix(look.sky[i], look.sky_low[i]));
            let (y0, y1) = (horizon * k / SKY_BANDS, horizon * (k + 1) / SKY_BANDS);
            p.list.fill_rect(c(col), Rect::new(0, y0, w, y1 - y0));
        }
        p.list
            .fill_rect(c(look.water), Rect::new(0, horizon, w, h - horizon));
        p.list.line(c(look.water_line), 4, (0, horizon), (w, horizon));
        for k in 1..4 {
            let y = horizon + (h - horizon) * k / 4;
            p.list.line(
                c(look.water_line).alpha(40),
                3,
                (w / 10 * k, y),
                (w / 10 * k + w / 6, y),
            );
        }
        for f in &self.fish {
            self.draw_fish(p, f, 100);
        }
        // each photo: the fish caught in a white frame, fading
        let linger = ticks_of(self.t.flash_ms) * PHOTO_FLASHES;
        let flash = ticks_of(self.t.flash_ms);
        for ph in &self.photos_taken {
            let age = self.now.saturating_sub(ph.tick);
            let alpha = 100 - (100 * age / linger.max(1)) as i32;
            self.draw_fish(p, &ph.fish, alpha);
            let (x, y) = ph.fish.pos();
            let [vw, vh] = look.viewfinder_size;
            let r = Rect::new(x - vw / 2, y - vh / 2, vw, vh);
            p.list.stroke_rect(c(look.photo).alpha(alpha), 8, r);
            if age < flash {
                let a = 70 - (70 * age / flash) as i32;
                p.list.fill_rect(c(look.flash).alpha(a), Rect::new(0, 0, w, h));
            }
        }
    }

    /// The viewfinder at the pointer, over everything.
    pub fn draw_viewfinder(&self, p: &mut Paint) {
        let Some((x, y)) = p.pointer else {
            return;
        };
        let look = &p.look.camera;
        let col = c(look.viewfinder);
        let [vw, vh] = look.viewfinder_size;
        let b = look.bracket;
        let (l, r, t, bt) = (x - vw / 2, x + vw / 2, y - vh / 2, y + vh / 2);
        for (cx, cy, dx, dy) in [(l, t, 1, 1), (r, t, -1, 1), (l, bt, 1, -1), (r, bt, -1, -1)] {
            p.list.line(col, 6, (cx, cy), (cx + dx * b, cy));
            p.list.line(col, 6, (cx, cy), (cx, cy + dy * b));
        }
        let d = b / 3;
        p.list.line(col, 4, (x - d, y - d), (x + d, y + d));
        p.list.line(col, 4, (x - d, y + d), (x + d, y - d));
    }
}
