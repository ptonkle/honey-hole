//! The Minigames screen: a debug screen, opened from the Title, that plays
//! each item's minigame on its own, so the person can try them outside a
//! match. A minigame meant for the Fish-Boy is drawn where the device's
//! screen sits when raised (`fishboy.screen_pct` of the view) inside a
//! drawn bezel; the rest fill the view, as they will in a match.
//!
//! The page sends the pointer over the 2D layer (`pointer.*`, CSS pixels)
//! and the clock (`advance`); the screen runs the minigame a tick
//! ([`TICK_MS`]) at a time from them, as the match does its own.

use std::collections::BTreeMap;

use super::*;
use crate::minigame::{Input, Kind, Layer, Minigame, State};
use crate::sim::TICK_MS;
use crate::ui2d::theme::{c, Theme};
use crate::ui2d::{Built, Rect, Rgba, Transform};

/// Ticks run at most per frame; the rest is dropped (a hidden tab).
const MAX_CATCH_UP: u32 = 10;

#[derive(Default)]
pub struct MinigamesScreen {
    pub game: Option<Minigame>,
    /// how each kind last ended
    pub results: BTreeMap<Kind, State>,
    /// minigames started, mixed into the next one's seed
    pub started: u64,
    last_now: Option<i64>,
    pending_us: i64,
    /// the pointer in the minigame's canvas units, and the button
    pointer: Option<(i32, i32)>,
    held: bool,
    pressed: bool,
    released: bool,
    quit: bool,
}

impl MinigamesScreen {
    fn take_input(&mut self) -> Input {
        let input = Input {
            pointer: self.pointer,
            held: self.held,
            pressed: self.pressed,
            released: self.released,
            quit: self.quit,
        };
        self.pressed = false;
        self.released = false;
        self.quit = false;
        input
    }
}

/// Milliseconds as seconds to two places ("3.42").
fn seconds(ms: u32) -> String {
    format!("{}.{:02}", ms / 1000, ms % 1000 / 10)
}

impl App {
    /// Starts a minigame: refused until the tuning file is in.
    fn start_minigame(&mut self, key: &str) -> Result<(), String> {
        let kind = Kind::from_key(key).ok_or_else(|| format!("There is no minigame \"{key}\"."))?;
        let tuning = self
            .tuning
            .as_ref()
            .ok_or("The tuning file has not loaded, so no minigame can start.")?;
        let m = &mut self.minigames;
        m.started += 1;
        let seed = crate::map::rng::mix(self.clock as u64 ^ m.started.wrapping_mul(0x9E37_79B9));
        let mut t = tuning.minigames.clone();
        if self.fast {
            // `?fast`: no instruction banner on this screen (`app/fast.rs`)
            t.banner_ms = 0;
        }
        let m = &mut self.minigames;
        m.game = Some(Minigame::new(kind, seed, &t));
        m.last_now = None;
        m.pending_us = 0;
        m.held = false;
        m.pressed = false;
        m.released = false;
        m.quit = false;
        Ok(())
    }

    /// Escape on the Minigames screen: quits the minigame being played,
    /// or goes back to the Title.
    pub(super) fn minigames_escape(&mut self) {
        if self.minigames.game.is_some() {
            self.minigames.quit = true;
        } else {
            self.back();
        }
    }

    /// Where the minigame is drawn, in buffer pixels: the whole view, or
    /// the Fish-Boy's screen.
    fn minigame_target(&self, g: &Minigame) -> Rect {
        let view = Rect::new(0, 0, self.size[0], self.size[1]);
        match g.layer() {
            Layer::Hud => view,
            Layer::FishBoy => {
                let pct = self.fishboy.theme.as_ref().map_or(80, |t| t.fishboy.screen_pct);
                let [cw, ch] = g.canvas();
                let fit = Transform::letterbox(g.canvas(), view).rect(Rect::new(0, 0, cw, ch));
                let (w, h) = (fit.w * pct / 100, fit.h * pct / 100);
                Rect::new(fit.x + (fit.w - w) / 2, fit.y + (fit.h - h) / 2, w, h)
            }
        }
    }

    /// The screen's actions (`minigames.*`) and the pointer over it.
    pub(super) fn minigames_act(&mut self, name: &str, value: &str) -> Result<(), String> {
        if let Some(key) = name.strip_prefix("minigames.play.") {
            return self.start_minigame(key);
        }
        match name {
            "pointer.move" | "pointer.down" | "pointer.up" => {
                let at = self.minigame_pointer(value)?;
                let m = &mut self.minigames;
                m.pointer = at;
                match name {
                    "pointer.down" => {
                        m.held = true;
                        m.pressed = true;
                    }
                    "pointer.up" => {
                        m.held = false;
                        m.released = true;
                    }
                    _ => {}
                }
                let held = m.held;
                if let (Some(g), Some(p)) = (m.game.as_mut(), at) {
                    g.point(p, held);
                }
            }
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    /// The pointer, in CSS pixels ("x y"), in the minigame's units.
    fn minigame_pointer(&self, value: &str) -> Result<Option<(i32, i32)>, String> {
        let mut it = value
            .split_whitespace()
            .map(|v| v.split('.').next().unwrap_or("").parse::<i64>());
        let (Some(Ok(x)), Some(Ok(y))) = (it.next(), it.next()) else {
            return Err(format!("The pointer \"{value}\" is not two whole numbers."));
        };
        let k = self.fishboy.dpr_pct as i64;
        let (px, py) = ((x * k / 100) as i32, (y * k / 100) as i32);
        Ok(self
            .minigames
            .game
            .as_ref()
            .map(|g| g.transform(self.minigame_target(g)).invert(px, py)))
    }

    /// Each frame: the minigame's ticks for the page time since the last,
    /// its sounds, and its end.
    pub(super) fn minigames_advance(&mut self) {
        if self.screen != Screen::Minigames {
            self.minigames.game = None;
            return;
        }
        let now = self.clock;
        let m = &mut self.minigames;
        let Some(mut g) = m.game.take() else {
            return;
        };
        let step_us = TICK_MS as i64 * 1000;
        let dt = m.last_now.map_or(0, |last| (now - last).max(0));
        m.last_now = Some(now);
        m.pending_us += dt;
        let mut ran = 0;
        let mut cues = Vec::new();
        // Esc counts at once, even before a tick is due
        if m.quit {
            m.pending_us = m.pending_us.max(step_us);
        }
        while m.pending_us >= step_us && ran < MAX_CATCH_UP {
            let input = m.take_input();
            g.step(&input);
            cues.extend(g.take_cues());
            m.pending_us -= step_us;
            ran += 1;
            if g.state() != State::Playing {
                break;
            }
        }
        // the cues asked for at the start, before any tick
        cues.extend(g.take_cues());
        m.pending_us = m.pending_us.min(step_us - 1);
        let state = g.state();
        let kind = g.kind;
        if state == State::Playing {
            m.game = Some(g);
        } else {
            m.results.insert(kind, state);
        }
        for cue in cues {
            self.play(cue);
        }
    }

    pub(super) fn minigames_panels(&self) -> Vec<Panel> {
        if self.minigames.game.is_some() {
            return Vec::new();
        }
        let mut items: Vec<Item> = Kind::ALL
            .iter()
            .map(|k| self.button(&format!("minigames.play.{}", k.key())))
            .collect();
        let rows = Kind::ALL
            .iter()
            .map(|k| {
                let result = match self.minigames.results.get(k) {
                    Some(State::Done { ms_taken }) => {
                        self.t("minigames.done").replace("{seconds}", &seconds(*ms_taken))
                    }
                    Some(State::Quit) => self.t("minigames.quit"),
                    _ => self.t("minigames.unplayed"),
                };
                [self.t(&format!("minigames.play.{}", k.key())), result]
            })
            .collect();
        items.push(Item::List { rows });
        items.push(self.button("menu.back"));
        vec![Panel::new("minigames", self.t("minigames.heading"), items)]
    }

    /// The page hides its cursor: the minigame draws its own.
    pub fn minigames_hide_cursor(&self) -> bool {
        self.screen == Screen::Minigames && self.minigames.game.as_ref().is_some_and(|g| g.hides_cursor())
    }

    /// The minigame's 2D list.
    pub(crate) fn minigames_ui2d(&self, theme: &Theme) -> Built {
        let mut built = Built::default();
        let Some(g) = self.minigames.game.as_ref() else {
            built.list.status = "mg=none".into();
            return built;
        };
        let list = &mut built.list;
        let target = self.minigame_target(g);
        if g.layer() == Layer::FishBoy {
            // the device's case round its screen: the brief's 10% more
            list.layer(Transform::IDENTITY);
            let pct = theme.fishboy.screen_pct.max(1);
            let (bw, bh) = (target.w * (pct + 10) / pct, target.h * (pct + 10) / pct);
            let bezel = Rect::new(
                target.x - (bw - target.w) / 2,
                target.y - (bh - target.h) / 2,
                bw,
                bh,
            );
            let look = &theme.minigames;
            list.round_rect(c(look.bezel), c(look.bezel).shade(60), 6, bezel, bh / 14);
        }
        let t = g.transform(target);
        list.layer(t);
        g.draw(list, theme, &|k| self.t(k));
        if g.layer() == Layer::FishBoy {
            // the Fish-Boy's scanlines over its screen
            let s = &theme.screen;
            let [w, h] = g.canvas();
            let dark = Rgba::solid([0, 0, 0]).alpha(s.scanline_pct);
            let mut y = 0;
            while y < h {
                list.fill_rect(dark, Rect::new(0, y, w, s.scanline_every / 2));
                y += s.scanline_every.max(1);
            }
        }
        let [cw, ch] = g.canvas();
        let px = t.rect(Rect::new(0, 0, cw, ch));
        built.list.status = format!("{} rect={},{},{},{}", g.status(), px.x, px.y, px.w, px.h);
        built
    }
}

#[cfg(test)]
mod tests;
