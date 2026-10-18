//! Teleporter, osu!-style: circles appear round the canvas, one every
//! `spawn_every_ms`, each with a ring closing on it over `approach_ms`. A
//! click inside a circle within `window_ms` of the ring meeting its edge is
//! a hit; a click inside it at any other moment, or no click by the end of
//! its window, is a miss and the circle goes. Circles keep coming until
//! `hits` are right.

use super::shape::{dist2, Pt};
use super::tuning::BeatsTuning;
use super::{cue, ticks_of, Ctx, Input, Paint, TOP_LINE};
use crate::map::rng::Rng;
use crate::ui2d::theme::c;
use crate::ui2d::{Align, Rect, Rgba};

/// Tries for a place clear of the other circles before taking the last.
const PLACE_TRIES: i32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Circle {
    pub at: Pt,
    /// the tick it appeared
    pub born: u32,
    /// its number, counted from one
    pub number: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Mark {
    at: Pt,
    hit: bool,
    tick: u32,
}

pub struct Beats {
    pub t: BeatsTuning,
    rng: Rng,
    canvas: [i32; 2],
    pub circles: Vec<Circle>,
    marks: Vec<Mark>,
    pub hits: i32,
    pub misses: i32,
    /// ticks since play began
    pub now: u32,
    next_spawn: u32,
    spawned: i32,
    approach: u32,
    window: u32,
    every: u32,
}

impl Beats {
    pub fn new(t: &BeatsTuning, canvas: [i32; 2], rng: Rng) -> Beats {
        Beats {
            t: t.clone(),
            rng,
            canvas,
            circles: Vec::new(),
            marks: Vec::new(),
            hits: 0,
            misses: 0,
            now: 0,
            next_spawn: 0,
            spawned: 0,
            approach: ticks_of(t.approach_ms),
            window: crate::sim::ticks(t.window_ms),
            every: ticks_of(t.spawn_every_ms),
        }
    }

    /// The tick a circle's ring meets its edge.
    pub fn meets(&self, c: &Circle) -> u32 {
        c.born + self.approach
    }

    fn place(&mut self) -> Pt {
        let [w, h] = self.canvas;
        let m = self.t.margin;
        let gap = self.t.min_gap as i64;
        let mut at = (w / 2, h / 2);
        for _ in 0..PLACE_TRIES {
            at = (
                self.rng.range(m, w - m),
                self.rng.range((TOP_LINE + 40 + self.t.radius).max(m), h - m),
            );
            let clear = self.circles.iter().all(|c| dist2(c.at, at) >= gap * gap);
            if clear {
                break;
            }
        }
        at
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &Input) -> bool {
        if self.now >= self.next_spawn {
            let at = self.place();
            self.spawned += 1;
            self.circles.push(Circle {
                at,
                born: self.now,
                number: self.spawned,
            });
            self.next_spawn = self.now + self.every;
        }
        if input.pressed {
            if let Some(p) = input.pointer {
                let r = self.t.radius as i64;
                // the oldest circle under the pointer takes the click
                if let Some(k) = self.circles.iter().position(|c| dist2(c.at, p) <= r * r) {
                    let circle = self.circles.remove(k);
                    let off = self.now.abs_diff(self.meets(&circle));
                    let hit = off <= self.window;
                    self.mark(circle.at, hit, ctx);
                }
            }
        }
        // a circle whose window has passed is missed
        let late = self.window;
        let approach = self.approach;
        let now = self.now;
        let (gone, kept): (Vec<Circle>, Vec<Circle>) =
            self.circles.iter().partition(|c| now > c.born + approach + late);
        self.circles = kept;
        for g in gone {
            self.mark(g.at, false, ctx);
        }
        let fade = ticks_of(self.t.feedback_ms);
        self.marks.retain(|m| now < m.tick + fade);
        self.now += 1;
        self.hits >= self.t.hits
    }

    fn mark(&mut self, at: Pt, hit: bool, ctx: &mut Ctx) {
        if hit {
            self.hits += 1;
            ctx.cues.push(cue::TICK);
        } else {
            self.misses += 1;
            ctx.cues.push(cue::MISS);
        }
        self.marks.push(Mark {
            at,
            hit,
            tick: self.now,
        });
    }

    pub fn draw(&self, p: &mut Paint) {
        let all = p.look;
        let look = &all.teleporter;
        let [w, h] = p.canvas;
        p.list.fill_rect(c(look.background), Rect::new(0, 0, w, h));
        p.list.radial(
            c(look.glow).alpha(50),
            c(look.background),
            (w / 2, h / 2),
            0,
            w * 6 / 10,
        );
        let r = self.t.radius;
        let fade = ticks_of(self.t.feedback_ms) as i32;
        for m in &self.marks {
            let age = (self.now.saturating_sub(m.tick)) as i32;
            let alpha = 100 - 100 * age / fade.max(1);
            if m.hit {
                p.list
                    .circle(Rgba::NONE, c(look.hit).alpha(alpha), 8, m.at, r + age * 2);
            } else {
                let d = r / 2;
                let col = c(look.miss).alpha(alpha);
                p.list
                    .line(col, 12, (m.at.0 - d, m.at.1 - d), (m.at.0 + d, m.at.1 + d));
                p.list
                    .line(col, 12, (m.at.0 - d, m.at.1 + d), (m.at.0 + d, m.at.1 - d));
            }
        }
        // newest underneath, so the next to click is on top
        for circle in self.circles.iter().rev() {
            let age = self.now.saturating_sub(circle.born) as i32;
            let appear = 100.min(age * 100 / 8 + 20);
            p.list.alpha(appear);
            p.list.circle(c(look.circle), c(look.edge), 8, circle.at, r);
            p.list.text(
                c(look.number),
                p.font,
                r,
                Align::Centre,
                circle.at,
                &circle.number.to_string(),
            );
            // the closing ring
            let left = self.meets(circle) as i32 - self.now as i32;
            let span = self.approach as i32;
            let extra = r * (self.t.ring_start_pct - 100) / 100;
            let ring = r + extra * left.max(0) / span.max(1);
            let ready = left.unsigned_abs() <= self.window;
            let colour = if ready { look.ring_ready } else { look.ring };
            p.list
                .circle(Rgba::NONE, c(colour), if ready { 10 } else { 6 }, circle.at, ring);
            p.list.alpha(100);
        }
    }
}
