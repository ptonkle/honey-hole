//! Nitrous: a box seen from above, with `nitrogen` N atoms and `oxygen` O
//! atoms bouncing round it. Each is pressed, dragged and dropped into the
//! "Nitrogen" or "Oxygen" bin along the bottom. The wrong bin throws the
//! atom back out into the box; a drop anywhere else lets it go where it
//! is. Done when every atom is binned.

use super::shape::{dist2, Pt};
use super::tuning::AtomsTuning;
use super::{cue, per_tick_milli, ticks_of, Ctx, Input, Paint};
use crate::map::rng::Rng;
use crate::map::trig;
use crate::ui2d::theme::c;
use crate::ui2d::{Align, Rect, Rgba};

/// The box's top, the gap above the bins, and the canvas's side margin.
const ARENA_TOP: i32 = 104;
const GAP: i32 = 24;
const SIDE: i32 = 60;
/// The bins' bottom, above the progress line.
const BINS_BOTTOM_GAP: i32 = 84;
/// A wrong bin flashes this long.
const FLASH_MS: i32 = 400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Where {
    Free,
    Held,
    /// in its bin, at this slot
    Binned(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Atom {
    pub nitrogen: bool,
    /// thousandths of a canvas unit
    pub x: i32,
    pub y: i32,
    /// thousandths a tick
    pub vx: i32,
    pub vy: i32,
    pub at: Where,
}

impl Atom {
    pub fn pos(&self) -> Pt {
        (self.x / 1000, self.y / 1000)
    }
}

pub struct Atoms {
    pub t: AtomsTuning,
    pub atoms: Vec<Atom>,
    pub arena: Rect,
    /// the Nitrogen bin, then the Oxygen bin
    pub bins: [Rect; 2],
    /// the atom held and where the pointer took it, relative to its middle
    held: Option<(usize, Pt)>,
    /// drops into the wrong bin
    pub bounces: i32,
    /// the wrong bin flashing, and ticks left
    flash: Option<(usize, u32)>,
}

impl Atoms {
    pub fn new(t: &AtomsTuning, canvas: [i32; 2], rng: &mut Rng) -> Atoms {
        let [w, h] = canvas;
        let bins_bottom = h - BINS_BOTTOM_GAP;
        let bins_top = bins_bottom - t.bin_h;
        let arena = Rect::new(SIDE, ARENA_TOP, w - 2 * SIDE, bins_top - GAP - ARENA_TOP);
        let half = (w - 2 * SIDE - GAP) / 2;
        let bins = [
            Rect::new(SIDE, bins_top, half, t.bin_h),
            Rect::new(SIDE + half + GAP, bins_top, half, t.bin_h),
        ];
        let r = t.atom_radius;
        let atoms = (0..t.nitrogen + t.oxygen)
            .map(|k| {
                let a = rng.below(trig::TURN as u32) as i32;
                let v = per_tick_milli(rng.range(t.speed_min_per_s, t.speed_max_per_s));
                Atom {
                    nitrogen: k < t.nitrogen,
                    x: rng.range(arena.x + r, arena.right() - r) * 1000,
                    y: rng.range(arena.y + r, arena.bottom() - r) * 1000,
                    vx: v * trig::cos(a) / trig::ONE,
                    vy: v * trig::sin(a) / trig::ONE,
                    at: Where::Free,
                }
            })
            .collect();
        Atoms {
            t: t.clone(),
            atoms,
            arena,
            bins,
            held: None,
            bounces: 0,
            flash: None,
        }
    }

    pub fn total(&self) -> i32 {
        self.atoms.len() as i32
    }

    pub fn binned(&self) -> i32 {
        self.atoms
            .iter()
            .filter(|a| matches!(a.at, Where::Binned(_)))
            .count() as i32
    }

    /// The atom the pointer would pick up at `p`: the one drawn on top.
    pub fn under(&self, p: Pt) -> Option<usize> {
        let reach = (self.t.atom_radius + self.t.grab_slop) as i64;
        (0..self.atoms.len())
            .rev()
            .find(|&k| self.atoms[k].at == Where::Free && dist2(self.atoms[k].pos(), p) <= reach * reach)
    }

    /// A free atom a press on its own middle picks up, and whether it is
    /// nitrogen.
    pub fn pickable(&self) -> Option<(bool, Pt)> {
        (0..self.atoms.len())
            .find(|&k| self.atoms[k].at == Where::Free && self.under(self.atoms[k].pos()) == Some(k))
            .map(|k| (self.atoms[k].nitrogen, self.atoms[k].pos()))
    }

    /// Keeps a free atom inside the box, bouncing off its walls.
    fn bounce(&self, a: &mut Atom) {
        let r = self.t.atom_radius * 1000;
        let (lo_x, hi_x) = (self.arena.x * 1000 + r, self.arena.right() * 1000 - r);
        let (lo_y, hi_y) = (self.arena.y * 1000 + r, self.arena.bottom() * 1000 - r);
        if a.x < lo_x {
            a.x = lo_x + (lo_x - a.x).min(hi_x - lo_x);
            a.vx = a.vx.abs();
        } else if a.x > hi_x {
            a.x = hi_x - (a.x - hi_x).min(hi_x - lo_x);
            a.vx = -a.vx.abs();
        }
        if a.y < lo_y {
            a.y = lo_y + (lo_y - a.y).min(hi_y - lo_y);
            a.vy = a.vy.abs();
        } else if a.y > hi_y {
            a.y = hi_y - (a.y - hi_y).min(hi_y - lo_y);
            a.vy = -a.vy.abs();
        }
    }

    fn drop(&mut self, ctx: &mut Ctx, k: usize) {
        let mut a = self.atoms[k];
        let p = a.pos();
        let bin = self.bins.iter().position(|b| b.contains(p.0, p.1));
        match bin {
            Some(b) if (b == 0) == a.nitrogen => {
                let slot = self
                    .atoms
                    .iter()
                    .filter(|o| o.nitrogen == a.nitrogen && matches!(o.at, Where::Binned(_)))
                    .count() as i32;
                a.at = Where::Binned(slot);
                ctx.cues.push(cue::TICK);
            }
            Some(b) => {
                // the wrong bin: back out into the box, upward
                a.at = Where::Free;
                a.y = (self.arena.bottom() - self.t.atom_radius - 1) * 1000;
                a.vy = -per_tick_milli(self.t.bounce_out_per_s);
                self.bounces += 1;
                self.flash = Some((b, ticks_of(FLASH_MS)));
                ctx.cues.push(cue::MISS);
            }
            None => a.at = Where::Free,
        }
        if a.at == Where::Free {
            self.bounce(&mut a);
        }
        self.atoms[k] = a;
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &Input) -> bool {
        if let Some((b, left)) = self.flash {
            self.flash = (left > 1).then_some((b, left - 1));
        }
        if input.pressed && self.held.is_none() {
            if let Some(p) = input.pointer {
                if let Some(k) = self.under(p) {
                    let at = self.atoms[k].pos();
                    self.atoms[k].at = Where::Held;
                    self.held = Some((k, (at.0 - p.0, at.1 - p.1)));
                }
            }
        }
        if let (Some((k, off)), Some(p)) = (self.held, input.pointer) {
            self.atoms[k].x = (p.0 + off.0) * 1000;
            self.atoms[k].y = (p.1 + off.1) * 1000;
        }
        if let Some((k, _)) = self.held {
            if input.released || !input.held {
                self.held = None;
                self.drop(ctx, k);
            }
        }
        for k in 0..self.atoms.len() {
            let mut a = self.atoms[k];
            if a.at == Where::Free {
                a.x += a.vx;
                a.y += a.vy;
                self.bounce(&mut a);
                self.atoms[k] = a;
            }
        }
        self.binned() == self.total()
    }

    /// Where a binned atom sits in its bin.
    fn slot_pos(&self, nitrogen: bool, slot: i32) -> Pt {
        let bin = self.bins[if nitrogen { 0 } else { 1 }];
        let r = self.t.atom_radius;
        let step = 2 * r + 10;
        let per_row = ((bin.w - 40) / step).max(1);
        let (col, row) = (slot % per_row, slot / per_row);
        (bin.x + 20 + r + col * step, bin.y + bin.h * 58 / 100 + row * step)
    }

    pub fn draw(&self, p: &mut Paint) {
        let look = &p.look.nitrous;
        let [w, h] = p.canvas;
        p.list.fill_rect(c(look.background), Rect::new(0, 0, w, h));
        p.list
            .round_rect(c(look.arena), c(look.arena_line), 6, self.arena, 18);
        let names = [
            (p.t)("minigame.nitrous.nitrogen"),
            (p.t)("minigame.nitrous.oxygen"),
        ];
        let symbols = [(p.t)("minigame.nitrous.n"), (p.t)("minigame.nitrous.o")];
        for (b, bin) in self.bins.iter().enumerate() {
            let tint = if b == 0 { look.nitrogen } else { look.oxygen };
            let wrong = self.flash.is_some_and(|(f, _)| f == b);
            let edge = if wrong { look.wrong } else { look.bin_line };
            p.list
                .round_rect(c(look.bin), c(edge), if wrong { 12 } else { 6 }, *bin, 22);
            p.list
                .round_rect(c(tint).alpha(22), Rgba::NONE, 0, bin.inset(6), 16);
            p.list.text(
                c(tint),
                p.font,
                44,
                Align::Left,
                (bin.x + 30, bin.y + 40),
                &names[b].to_uppercase(),
            );
            p.list.text(
                c(look.atom_ink).alpha(60),
                p.font,
                44,
                Align::Right,
                (bin.right() - 30, bin.y + 40),
                &symbols[b],
            );
        }
        let r = self.t.atom_radius;
        let draw_atom = |p: &mut Paint, a: &Atom, at: Pt, lifted: bool| {
            let fill = if a.nitrogen { look.nitrogen } else { look.oxygen };
            let rr = if lifted { r * 115 / 100 } else { r };
            if lifted {
                p.list
                    .circle(c([0, 0, 0]).alpha(30), Rgba::NONE, 0, (at.0 + 8, at.1 + 12), rr);
            }
            p.list.circle(c(fill), c(fill).shade(55), 4, at, rr);
            p.list.circle(
                c([255, 255, 255]).alpha(35),
                Rgba::NONE,
                0,
                (at.0 - rr / 3, at.1 - rr / 3),
                rr / 3,
            );
            let sym = &symbols[if a.nitrogen { 0 } else { 1 }];
            p.list
                .text(c(look.atom_ink), p.font, rr, Align::Centre, (at.0, at.1 + 2), sym);
        };
        for a in &self.atoms {
            match a.at {
                Where::Free => draw_atom(p, a, a.pos(), false),
                Where::Binned(slot) => draw_atom(p, a, self.slot_pos(a.nitrogen, slot), false),
                Where::Held => {}
            }
        }
        if let Some((k, _)) = self.held {
            let a = self.atoms[k];
            draw_atom(p, &a, a.pos(), true);
        }
    }
}
