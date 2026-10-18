//! The match's flow round the play itself (build order step 4): the
//! countdown, the HUD, the popups, a wreck's burst and shake, the end, and
//! the postgame with its reveal, ranking, Rematch and Back to Title.
//!
//! - **The countdown** is the match's (`Match::countdown`, from
//!   `flow.countdown_ms` in `data/tuning.json`): 3-2-1-Go with the boats
//!   held still. Its beats are `Countdown` and `MatchStarted` events, which
//!   the sound engine plays.
//! - **The HUD** ([`hud`]) is drawn on the 2D layer (`ui2d`): the time
//!   left, a speed gauge, the driving/fishing badge, the item slot
//!   ([`App::hud_item`], the items system's hook), the radar ping's
//!   cooldown, key hints that fade after the first minute, and the map seed
//!   in the bottom-right when Settings' "Show map seed" is on.
//! - **Popups** ([`popups`]) come from the match's events through the
//!   table in `data/flow.json`: an event name, a string key and a style.
//!   Other systems add a popup by adding a row.
//! - **A wreck** ([`burst`]) goes off in the world where the boat was, the
//!   victim's camera shakes and their screen flashes; a respawned boat is
//!   drawn see-through while ghosted (`Frame::ghost`).
//! - **The end**: at time 0 the horn, "Lines up" for `flow.end_hold_ms`,
//!   then the postgame ([`postgame`], drawn by [`screens`]): each player's
//!   catches revealed one after another with the tally sound, then the
//!   ranking with the winner's fanfare. Ties share a place. Rematch plays
//!   again with the same setup and a new seed.
//!
//! The look is `data/flow.json` ([`look`]), sent by the page as
//! `act("data.flow", text)`. Without it the match still runs, with no HUD.

pub mod burst;
pub mod hud;
pub mod look;
pub mod popups;
pub mod postgame;
pub mod screens;
#[cfg(test)]
mod tests;

use serde_json::{json, Value};

use crate::app::{App, Screen};
use crate::map::list;
use crate::map::rng::mix;
use crate::roster::{tier_key, Roster};
use crate::sim::{Catch, Match, Occupant};

use burst::Burst;
use look::FlowLook;
use popups::{Names, Popups};
use postgame::Reveal;

/// How much time F8 leaves under `?debug=flow`.
pub const DEBUG_LEFT_MS: i32 = 3000;

/// What the flow keeps between frames.
#[derive(Debug, Clone, Default)]
pub struct Flow {
    pub look: Option<FlowLook>,
    pub popups: Popups,
    /// wrecks still going off
    pub bursts: Vec<Burst>,
    /// when the match's time ran out, page µs
    pub ended_us: Option<i64>,
    pub postgame: Option<Postgame>,
    /// the page's `?debug=flow`: F8 jumps the match to its last
    /// seconds, with made-up catches if no one has landed a fish
    /// ([`App::flow_debug_key`]), for the browser check and screenshots
    pub debug: bool,
}

/// The postgame screen's state.
#[derive(Debug, Clone)]
pub struct Postgame {
    pub reveal: Reveal,
    /// page µs it opened (moved back by a skip)
    pub started_us: i64,
    /// the ms into the reveal already heard (tally ticks, the fanfare)
    pub heard_ms: i64,
}

impl Postgame {
    pub fn ms(&self, clock: i64) -> i64 {
        (clock - self.started_us) / 1000
    }
}

impl Flow {
    /// A new match: nothing showing, nothing going off.
    pub fn reset(&mut self) {
        self.popups = Popups::default();
        self.bursts.clear();
        self.ended_us = None;
        self.postgame = None;
    }
}

/// **Test hook until the fishing system fills `scores`.** A few made-up
/// catches for every taken seat, picked from the roster by the match seed,
/// each worth its kind's value scaled by its length, so the postgame can be
/// seen and checked.
pub fn fake_catches(m: &mut Match, roster: &Roster) {
    if roster.0.is_empty() {
        return;
    }
    for seat in 0..m.scores.len() {
        if m.setup.seats[seat].occupant == Occupant::Empty {
            continue;
        }
        // a seat that has landed fish keeps its own
        if !m.scores[seat].catches.is_empty() {
            continue;
        }
        let n = 1 + (mix(m.setup.seed ^ seat as u64) % 4) as usize;
        for k in 0..n {
            let r = mix(m.setup.seed.wrapping_add((seat * 16 + k) as u64 + 1));
            let kind = &roster.0[(r % roster.0.len() as u64) as usize];
            // 1x to 1.5x the base length
            let pct = 100 + (r >> 20) % 51;
            let length_mm = (kind.base_length_mm as u64 * pct / 100) as i32;
            m.scores[seat].catches.push(Catch {
                kind: kind.id.clone(),
                length_mm,
                value: (kind.value as u64 * pct / 100) as i32,
            });
        }
    }
}

impl App {
    /// `data.flow` and the flow's own actions (`postgame.*`, `debug.*`).
    pub(crate) fn flow_act(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            "data.flow" => self.flow.look = Some(FlowLook::parse(value)?),
            "postgame.skip" => self.skip_reveal(),
            // online, the host's Rematch goes back to the Lobby (`app/online_play.rs`)
            "postgame.rematch" if self.online_rematch().is_some() => {}
            "postgame.rematch" => self.rematch()?,
            "postgame.title" => self.leave_match(),
            // the browser check and screenshots: jump the match's clock
            // to `value` ms left, and fill made-up catches
            "debug.time_left" => {
                let ms: i32 = value
                    .parse()
                    .map_err(|_| format!("\"{value}\" is not a time left in ms."))?;
                let p = self.play.as_mut().ok_or("There is no match to jump ahead.")?;
                let g = &mut p.game;
                g.countdown = 0;
                g.tick = g
                    .tick
                    .max(g.rules.length_ticks.saturating_sub(crate::sim::ticks(ms)));
            }
            "debug.catches" => {
                let roster = self.fishboy.roster.clone();
                let p = self.play.as_mut().ok_or("There is no match to fill.")?;
                fake_catches(&mut p.game, &roster);
            }
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    /// Under `?debug=flow`, F8 in a match: [`DEBUG_LEFT_MS`] left, and
    /// made-up catches for each seat that has landed none. Returns whether the key
    /// was used. F9 wrecks this seat's boat where it is (as if it had run
    /// aground), to see the burst, the shake and the ghosting.
    pub(crate) fn flow_debug_key(&mut self, code: &str) -> bool {
        if !self.flow.debug
            || code != "F8" && code != "F9"
            || self.screen != Screen::Play
            || self.play.is_none()
        {
            return false;
        }
        if code == "F9" {
            let Some(p) = self.play.as_mut() else {
                return false;
            };
            let (seat, tick) = (p.seat, p.game.tick);
            if p.game.boat(seat).is_none_or(|b| b.wrecked()) {
                return true;
            }
            p.game.wreck(seat, crate::sim::WreckCause::Grounded, tick);
            let heard: Vec<Value> = p
                .game
                .events
                .drain(..)
                .filter_map(|e| serde_json::to_value(e).ok())
                .collect();
            if let Some(s) = self.sound.as_mut() {
                for e in &heard {
                    s.event(e, Some(seat as i64));
                }
            }
            self.flow_hear(&heard);
            return true;
        }
        let _ = self.flow_act("debug.catches", "");
        let _ = self.flow_act("debug.time_left", &DEBUG_LEFT_MS.to_string());
        true
    }

    /// A tick's events heard: their popups, a wreck's burst, the end.
    pub(crate) fn flow_hear(&mut self, events: &[Value]) {
        let Some(p) = self.play.as_ref() else {
            return;
        };
        let players: Vec<String> = p.game.setup.seats.iter().map(|s| s.name.clone()).collect();
        let roster = &self.fishboy.roster;
        let fish = |id: &str| roster.get(id).map(|k| k.name.clone());
        let tier = |t| self.t(&format!("tier.{}", tier_key(t)));
        let text = |k: &str| self.t(k);
        let names = Names {
            players: &players,
            fish: &fish,
            tier: &tier,
            text: &text,
            someone: self.t("popup.someone"),
            a_fish: self.t("popup.fish"),
        };
        let made = match self.flow.look.as_ref() {
            Some(l) => popups::of_events(&l.popups.rows, events, p.seat, &names),
            None => Vec::new(),
        };
        let mut bursts = Vec::new();
        let mut ended = false;
        for e in events {
            match e.get("type").and_then(Value::as_str) {
                Some("Wrecked") => {
                    let seat = e.get("seat").and_then(Value::as_u64).unwrap_or(0) as usize;
                    if let (Some(b), Some(w)) = (p.game.boat(seat), self.world.as_ref()) {
                        bursts.push(Burst {
                            seat,
                            at: w.world(b.at(), 0),
                            born_us: self.clock,
                            seed: mix(p.game.setup.seed ^ ((seat as u64) << 32) ^ p.game.tick as u64),
                        });
                    }
                }
                Some("MatchEnded") => ended = true,
                _ => {}
            }
        }
        let now = self.clock;
        for pop in made {
            self.flow.popups.push(pop, now);
        }
        self.flow.bursts.extend(bursts);
        if ended && self.flow.ended_us.is_none() {
            self.flow.ended_us = Some(now);
        }
    }

    /// Each frame: popups show and expire, old bursts go, and the end
    /// opens the postgame, whose reveal plays its sounds as it goes.
    pub(crate) fn flow_advance(&mut self) {
        let now = self.clock;
        let (popup_ms, hold_ms) = self
            .tuning
            .as_ref()
            .map_or((2000, 0), |t| (t.flow.popup_ms, t.flow.end_hold_ms));
        // `?fast`: no "Lines up" hold (`app/fast.rs`)
        let hold_ms = if self.fast { 0 } else { hold_ms };
        if let Some(l) = self.flow.look.as_ref() {
            self.flow.popups.advance(now, popup_ms, l.popups.max_shown);
            let keep_us = l.burst.ms.max(l.burst.shake_ms).max(l.hud.wreck_flash_ms) as i64 * 1000;
            self.flow.bursts.retain(|b| now - b.born_us < keep_us);
        }
        if self.screen == Screen::Play && self.flow.postgame.is_none() {
            if let Some(end) = self.flow.ended_us {
                if now - end >= hold_ms as i64 * 1000 {
                    self.open_postgame();
                }
            }
        }
        self.postgame_sounds();
    }

    /// The postgame, from the match's scores.
    fn open_postgame(&mut self) {
        let (Some(p), Some(l)) = (self.play.as_ref(), self.flow.look.as_ref()) else {
            return;
        };
        let names: Vec<(usize, String)> = p
            .game
            .setup
            .seats
            .iter()
            .enumerate()
            .filter(|(_, s)| s.occupant != Occupant::Empty)
            .map(|(k, s)| (k, s.name.clone()))
            .collect();
        let reveal = Reveal::new(postgame::columns(&names, &p.game.scores), &l.postgame);
        self.flow.postgame = Some(Postgame {
            reveal,
            started_us: self.clock,
            heard_ms: -1,
        });
        self.flow.popups = Popups::default();
        self.go(Screen::Postgame);
        if self.fast {
            // `?fast`: every card at once, and the ranking (`app/fast.rs`)
            self.skip_reveal();
        }
    }

    /// The reveal's sounds since the last frame: a tally tick for each
    /// card, and the fanfare as the ranking shows.
    fn postgame_sounds(&mut self) {
        let clock = self.clock;
        let Some(pg) = self.flow.postgame.as_mut() else {
            return;
        };
        let ms = pg.ms(clock);
        let ticks = pg.reveal.cards_between(pg.heard_ms, ms);
        let fanfare = pg.heard_ms < pg.reveal.rank_ms && ms >= pg.reveal.rank_ms;
        pg.heard_ms = ms;
        if let Some(s) = self.sound.as_mut() {
            for _ in 0..ticks {
                s.event(&json!({"type": "TallyTick"}), None);
            }
            if fanfare {
                s.event(&json!({"type": "Winner"}), None);
            }
        }
    }

    /// Skip to the ranking.
    pub(crate) fn skip_reveal(&mut self) {
        let clock = self.clock;
        if let Some(pg) = self.flow.postgame.as_mut() {
            let ms = pg.ms(clock);
            if ms < pg.reveal.rank_ms {
                pg.started_us -= (pg.reveal.rank_ms - ms) * 1000;
                // the skipped cards' ticks are not played
                pg.heard_ms = pg.reveal.rank_ms - 1;
            }
        }
    }

    /// Whether the reveal has reached the ranking.
    pub fn postgame_ranked(&self) -> bool {
        self.flow
            .postgame
            .as_ref()
            .is_some_and(|pg| pg.ms(self.clock) >= pg.reveal.rank_ms)
    }

    /// Rematch: the same setup (seats, AI, length, buoy interval) with a
    /// new match seed and a new random layout of the lake.
    fn rematch(&mut self) -> Result<(), String> {
        let p = self.play.as_ref().ok_or("There is no match to play again.")?;
        let mut setup = p.game.setup.clone();
        setup.seed = mix(setup.seed ^ self.clock as u64 ^ 0x7265_6d61_7463_6800);
        let maps = self.maps.as_ref().ok_or("The map list file is not loaded yet.")?;
        let map_seed = list::pick(maps, setup.seed);
        self.begin(setup, map_seed)
    }
}
