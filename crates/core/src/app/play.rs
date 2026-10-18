//! The match screens: the offline match setup (the Lobby screen, "New
//! match") and Play, where a match runs.
//!
//! - **Data.** The page hands over `data/tuning.json`, `data/controls.json`,
//!   the map list and `data/names.json` through `act("data.<name>", text)`;
//!   each is checked here, and a refusal names the file.
//! - **Setup.** The reader chooses how many AI players (0 to 3) and each
//!   one's difficulty, the match length, the item buoy interval and the map
//!   seed (blank for a random checked layout). The person sits in seat 1.
//! - **Input.** The page sends raw key codes (`key`, `key.up`), mouse
//!   buttons (`mouse.down`, `mouse.up`) and mouse movement (`mouse.move`,
//!   "dx,dy" in CSS pixels). `controls.rs` turns them into one `SeatInput` a
//!   tick. The mouse also turns the camera at once, for smooth drawing.
//! - **Ticks.** `advance(now)` runs whole ticks of the match for the time
//!   that passed, at most [`MAX_CATCH_UP`] a frame (a page left in the
//!   background does not race to catch up), and keeps the remainder to draw
//!   between ticks.
//! - **Drawing.** [`App::play_view`] is the plain data a world renderer
//!   reads: the interpolated boats, the camera's yaw and pitch, the time
//!   left. `app/lake.rs` (`App::match_frame`) draws the match on the
//!   lake's world.

use serde::{Deserialize, Serialize};

use super::*;
use crate::controls::{self, Gather};
use crate::gfx::fx::{self, ONE, TURN};
use crate::map::list::{self, MapList};
use crate::map::rng::mix;
use crate::map::{generate, Setting};
use crate::sim::view::WorldView;
use crate::sim::{self, Difficulty, Match, MatchEvent, Occupant, Setup, TICK_MS};
use crate::tuning::{self, Tuning};
use crate::view::Choice;

#[cfg(test)]
pub(crate) mod tests;

/// The most ticks one `advance` runs (a quarter of a second).
pub const MAX_CATCH_UP: u32 = 12;
/// The most events [`Play::log`] keeps.
const LOG_KEEP: usize = 64;
/// The longest name a person may type.
pub const NAME_MAX: u32 = 20;
/// The longest map seed (a `u32` has ten digits).
const SEED_MAX: u32 = 10;

/// `data/names.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct NamesFile {
    #[serde(rename = "_note", default)]
    _note: String,
    ai: Vec<String>,
}

/// The match setup the reader is choosing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lobby {
    /// AI players, 0 to the seats left
    pub ai: usize,
    /// by AI seat (seats 2, 3, 4)
    pub difficulty: Vec<Difficulty>,
    pub length_ms: i32,
    pub buoy_interval_ms: i32,
    /// typed digits, or empty for a random map
    pub seed: String,
}

impl Lobby {
    fn new(t: &Tuning) -> Lobby {
        let seats = t.game.seats as usize;
        Lobby {
            ai: 0,
            difficulty: vec![Difficulty::Normal; seats.saturating_sub(1)],
            length_ms: t.game.length_ms,
            buoy_interval_ms: t.game.buoy_interval_ms,
            seed: String::new(),
        }
    }
}

/// A match in progress, and the local player's view of it.
pub struct Play {
    pub game: Match,
    /// the seat this browser plays
    pub seat: usize,
    gather: Gather,
    /// µs of page time not yet run as ticks
    pending_us: i64,
    last_now: Option<i64>,
    /// the camera's turn from behind the boat, and its tilt down
    pub cam_yaw: i32,
    pub cam_pitch: i32,
    /// the latest events, oldest first, with their ticks
    pub log: Vec<(u32, MatchEvent)>,
}

impl Play {
    /// A match to play from seat `seat`, its clock started at `start_us`
    /// (page time), the camera tilted `pitch_deg` down.
    pub fn begin(game: Match, seat: usize, pitch_deg: i32, start_us: i64) -> Play {
        Play {
            game,
            seat,
            gather: Gather::default(),
            pending_us: 0,
            last_now: Some(start_us),
            cam_yaw: 0,
            cam_pitch: deg(pitch_deg),
            log: Vec::new(),
        }
    }

    /// How far between the last tick and the next the drawing is (Q16).
    pub fn alpha(&self) -> i32 {
        (self.pending_us * ONE as i64 / (TICK_MS as i64 * 1000)) as i32
    }

    /// W or S is held (the motor's sound rises with it).
    pub fn throttle_held(&self) -> bool {
        self.gather.held().forward || self.gather.held().back
    }

    pub fn menu_open(&self) -> bool {
        self.game.seats[self.seat].menu_open
    }
}

/// What a world renderer draws for Play.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlayView {
    pub world: WorldView,
    /// the seat this browser plays
    pub seat: usize,
    /// the camera's heading in map angles (0 east, a quarter north): the
    /// boat's heading turned by the mouse
    pub cam_yaw: i32,
    /// how far the camera looks down, in angle units
    pub cam_pitch: i32,
}

fn deg(d: i32) -> i32 {
    (d as i64 * TURN as i64 / 360) as i32
}

/// A number the page sent, to its whole part ("1.5" is 1, "-0.5" is 0):
/// some browsers report mouse movement in fractions of a pixel.
fn whole(text: &str) -> Option<i32> {
    let t = text.trim();
    let int = t.split('.').next()?;
    let frac = &t[int.len()..];
    if !frac.is_empty() && !frac[1..].chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    match int {
        "" | "-" if !frac.is_empty() => Some(0),
        _ => int.parse().ok(),
    }
}

/// m:ss for a time in ms, rounded up to the second.
pub fn clock_text(ms: i32) -> String {
    let s = (ms.max(0) + 999) / 1000;
    format!("{}:{:02}", s / 60, s % 60)
}

impl App {
    /// The data files the match needs, from the page.
    pub(super) fn data_act(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            "data.tuning" => {
                let t = tuning::parse(value)?;
                self.lobby = Some(Lobby::new(&t));
                self.tuning = Some(t);
            }
            "data.controls" => self.controls = Some(controls::parse(value)?),
            "data.maps" => {
                let l = list::parse(value)?;
                self.maps = Some(l);
            }
            "data.names" => {
                let f: NamesFile =
                    serde_json::from_str(value).map_err(|e| format!("The names file does not read: {e}."))?;
                if f.ai.len() < sim::SEAT_COLOURS.len() {
                    return Err("The names file needs a name for each AI seat.".into());
                }
                self.names = f.ai;
            }
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    /// The person's name: Settings', or the stand-in.
    pub fn player_name(&self) -> String {
        if self.settings.name.is_empty() {
            self.t("name.default")
        } else {
            self.settings.name.clone()
        }
    }

    pub(super) fn set_name(&mut self, value: &str) {
        let clean: String = value
            .chars()
            .filter(|c| !c.is_control())
            .take(NAME_MAX as usize)
            .collect();
        self.settings.name = clean.trim().to_string();
    }

    fn need<'a, T>(&self, v: &'a Option<T>, file: &str) -> Result<&'a T, String> {
        v.as_ref()
            .ok_or_else(|| format!("The {file} file is not loaded yet."))
    }

    /// The Lobby's controls (`lobby.*`).
    pub(super) fn lobby_act(&mut self, name: &str, value: &str) -> Result<(), String> {
        if name == "lobby.start" {
            return self.start_match();
        }
        let t = self.need(&self.tuning, "tuning")?.clone();
        let lobby = self.lobby.as_mut().ok_or("The tuning file is not loaded yet.")?;
        let seats = t.game.seats as usize;
        match name {
            "lobby.ai" => {
                let n: usize = value
                    .parse()
                    .map_err(|_| format!("\"{value}\" is not a number of AI players."))?;
                if n >= seats {
                    return Err(format!("A match has room for {} AI players.", seats - 1));
                }
                lobby.ai = n;
            }
            "lobby.length" => {
                let ms: i32 = value.parse().unwrap_or(-1);
                if !t.game.length_choices_ms.contains(&ms) {
                    return Err(format!("There is no match length \"{value}\"."));
                }
                lobby.length_ms = ms;
            }
            "lobby.buoy" => {
                let ms: i32 = value.parse().unwrap_or(-1);
                if !t.game.buoy_interval_choices_ms.contains(&ms) {
                    return Err(format!("There is no buoy interval \"{value}\"."));
                }
                lobby.buoy_interval_ms = ms;
            }
            "lobby.seed" => {
                lobby.seed = value
                    .chars()
                    .filter(char::is_ascii_digit)
                    .take(SEED_MAX as usize)
                    .collect();
            }
            _ => {
                let k: usize = name
                    .strip_prefix("lobby.ai.")
                    .and_then(|k| k.parse().ok())
                    .filter(|k| (1..seats).contains(k))
                    .ok_or_else(|| format!("Unknown action \"{name}\"."))?;
                lobby.difficulty[k - 1] = Difficulty::from_key(value)
                    .ok_or_else(|| format!("There is no difficulty \"{value}\"."))?;
            }
        }
        self.online_setup_changed();
        Ok(())
    }

    /// The map seed for this match: the typed one, if it is a checked
    /// layout, or one picked by the match seed.
    pub(super) fn map_seed(&self, list: &MapList, match_seed: u64) -> Result<u32, String> {
        let lobby = self.lobby.as_ref().ok_or("The tuning file is not loaded yet.")?;
        if lobby.seed.is_empty() {
            return Ok(list::pick(list, match_seed));
        }
        let seed: u32 = lobby
            .seed
            .parse()
            .map_err(|_| format!("\"{}\" is not a map seed.", lobby.seed))?;
        if list.maps.iter().any(|e| e.seed == seed) {
            Ok(seed)
        } else {
            Err(self
                .t("lobby.seed_unknown")
                .replace("{seed}", &seed.to_string())
                .replace("{map}", &self.t(Setting::MichiganLake.name_key())))
        }
    }

    /// The setup the Lobby describes, for a match seed.
    pub fn setup(&self, match_seed: u64) -> Result<Setup, String> {
        let t = self.need(&self.tuning, "tuning")?;
        let lobby = self.lobby.as_ref().ok_or("The tuning file is not loaded yet.")?;
        let mut s = Setup::new(t);
        s.seed = match_seed;
        s.length_ms = lobby.length_ms;
        s.buoy_interval_ms = lobby.buoy_interval_ms;
        s.offline = true;
        // the roster's starting counts (`data/fish.json`)
        s.fish = self
            .fishboy
            .roster
            .0
            .iter()
            .map(|k| (k.id.clone(), k.count))
            .collect();
        s = s.with(0, Occupant::Human, &self.player_name());
        // distinct AI names, picked by the seed
        let mut pool = self.names.clone();
        for k in 1..=lobby.ai {
            let name = if pool.is_empty() {
                format!("{} {}", self.t("lobby.ai_seat"), k + 1)
            } else {
                let at = (mix(match_seed.wrapping_add(k as u64)) % pool.len() as u64) as usize;
                pool.remove(at)
            };
            s = s.with(k, Occupant::Ai(lobby.difficulty[k - 1]), &name);
        }
        Ok(s)
    }

    fn start_match(&mut self) -> Result<(), String> {
        let list = self.need(&self.maps, "map list")?.clone();
        let t = self.need(&self.tuning, "tuning")?.clone();
        let controls = self.need(&self.controls, "controls")?.clone();
        // the match seed comes from the page's clock: no two matches alike,
        // and the same clock gives the same match
        let match_seed = mix(self.clock as u64 ^ 0x6d61_7463_6800_0000);
        let map_seed = self.map_seed(&list, match_seed)?;
        let setup = self.setup(match_seed)?;
        let map = generate(Setting::MichiganLake, map_seed);
        // the lake's world, uploaded once (`app/lake.rs`); without the
        // lake's look the match draws its stand-in
        let _ = self.show_world(&map);
        let mut game = Match::new(map, &t, setup)?;
        // the lake's fish (`sim::fishing`); a page that sent no roster
        // plays a lake without
        if !self.fishboy.roster.0.is_empty() {
            game.stock(&self.fishboy.roster)?;
        }
        self.play = Some(Play {
            game,
            seat: 0,
            gather: Gather::default(),
            pending_us: 0,
            last_now: None,
            cam_yaw: 0,
            cam_pitch: deg(controls.mouse.pitch_start_deg),
            log: Vec::new(),
        });
        self.go(Screen::Play);
        Ok(())
    }

    /// Leaves the match for the Title.
    pub(super) fn leave_match(&mut self) {
        self.play = None;
        self.settings_return = false;
        self.go(Screen::Title);
    }

    /// Raw input from the page while a match runs (`key`, `key.up`,
    /// `mouse.*`, `keys.release`). Returns false when it is not match input.
    pub(super) fn match_input(&mut self, name: &str, value: &str) -> Result<bool, String> {
        let Some(controls) = self.controls.as_ref() else {
            return Ok(false);
        };
        let Some(p) = self.play.as_mut().filter(|_| self.screen == Screen::Play) else {
            return Ok(false);
        };
        match name {
            "key" => {
                p.gather.down(controls, value);
            }
            "key.up" => p.gather.up(controls, value),
            "mouse.down" => {
                p.gather.down(controls, &format!("Mouse{value}"));
            }
            "mouse.up" => p.gather.up(controls, &format!("Mouse{value}")),
            "keys.release" => p.gather.release_all(),
            "mouse.move" => {
                let (dx, dy) = value
                    .split_once(',')
                    .and_then(|(x, y)| Some((whole(x)?, whole(y)?)))
                    .ok_or_else(|| format!("The mouse move \"{value}\" is not two whole numbers."))?;
                p.gather.mouse(dx, dy);
                // with the Fish-Boy up the mouse points at its screen
                if !p.menu_open() && !p.game.seats[p.seat].fishboy {
                    let m = &controls.mouse;
                    // right turns the camera clockwise from above: a smaller map angle
                    p.cam_yaw =
                        fx::wrap(p.cam_yaw - (dx as i64 * deg(m.yaw_deg_per_100px) as i64 / 100) as i32);
                    let pitch = p.cam_pitch + (dy as i64 * deg(m.pitch_deg_per_100px) as i64 / 100) as i32;
                    p.cam_pitch = pitch.clamp(deg(m.pitch_min_deg), deg(m.pitch_max_deg));
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// An on-screen menu button that acts as a press of `action`.
    pub(crate) fn tap(&mut self, action: &str, when_open: bool) {
        if let Some(p) = self.play.as_mut() {
            if p.menu_open() == when_open {
                p.gather.tap(action);
            }
        }
    }

    /// Runs the match's ticks for the page time since the last call.
    pub(super) fn run_ticks(&mut self, now: i64) {
        let Some(p) = self.play.as_mut() else {
            return;
        };
        let step_us = TICK_MS as i64 * 1000;
        let dt = p.last_now.map_or(0, |last| (now - last).max(0));
        p.last_now = Some(now);
        p.pending_us += dt;
        let mut ran = 0;
        let mut heard = Vec::new();
        let mut online = self.online.session.as_mut();
        while p.pending_us >= step_us && ran < MAX_CATCH_UP {
            let mut inputs = vec![sim::SeatInput::default(); p.game.seats.len()];
            inputs[p.seat] = p.gather.take();
            let tick = p.game.tick;
            p.game.step(&inputs);
            if let Some(s) = online.as_mut() {
                s.after_step(&mut p.game, now / 1000);
            }
            for e in &p.game.events {
                p.log.push((tick + 1, e.clone()));
                heard.push(serde_json::to_value(e).unwrap_or_default());
            }
            p.pending_us -= step_us;
            ran += 1;
        }
        let landed = heard
            .iter()
            .any(|e| e["type"] == "BobberLanded" && e["seat"] == p.seat as u64);
        // the rest is dropped, not run later
        p.pending_us = p.pending_us.min(step_us - 1);
        let over = p.log.len().saturating_sub(LOG_KEEP);
        p.log.drain(..over);
        // the events' sounds, heard from this browser's seat
        let listener = Some(p.seat as i64);
        if let Some(s) = self.sound.as_mut() {
            for e in &heard {
                s.event(e, listener);
            }
        }
        self.debug_fish(landed);
    }

    /// The match as a world renderer reads it.
    pub fn play_view(&self) -> Option<PlayView> {
        let p = self.play.as_ref()?;
        let world = p.game.view(p.alpha());
        let heading = world
            .boats
            .iter()
            .find(|b| b.seat == p.seat)
            .map_or(0, |b| b.heading);
        Some(PlayView {
            world,
            seat: p.seat,
            cam_yaw: crate::map::trig::wrap(heading + p.cam_yaw),
            cam_pitch: p.cam_pitch,
        })
    }

    // ---- panels ----

    pub(super) fn time_choices(&self, id: &str, label: &str, value: i32, choices: &[i32]) -> Item {
        let options = choices
            .iter()
            .map(|&ms| Choice {
                value: ms.to_string(),
                text: if ms % 60_000 == 0 {
                    self.t("units.min").replace("{n}", &(ms / 60_000).to_string())
                } else {
                    self.t("units.s").replace("{n}", &(ms / 1000).to_string())
                },
                disabled: false,
            })
            .collect();
        Item::Select {
            id: id.into(),
            label: self.t(label),
            value: value.to_string(),
            options,
        }
    }

    pub(super) fn lobby_panels(&self) -> Vec<Panel> {
        let (Some(t), Some(lobby)) = (self.tuning.as_ref(), self.lobby.as_ref()) else {
            return vec![Panel::new(
                "lobby",
                self.t("lobby.heading"),
                vec![self.button("menu.back")],
            )];
        };
        let seats = t.game.seats as usize;
        let rows = (0..seats)
            .map(|k| {
                let who = if k == 0 {
                    self.t("lobby.you").replace("{name}", &self.player_name())
                } else if k <= lobby.ai {
                    self.t("lobby.ai_player").replace(
                        "{difficulty}",
                        &self.t(&format!("difficulty.{}", lobby.difficulty[k - 1].key())),
                    )
                } else {
                    self.t("lobby.empty")
                };
                [self.t(&format!("colour.{}", sim::SEAT_COLOURS[k])), who]
            })
            .collect();
        let mut items = vec![Item::List { rows }];
        items.push(Item::Select {
            id: "lobby.ai".into(),
            label: self.t("lobby.ai"),
            value: lobby.ai.to_string(),
            options: (0..seats)
                .map(|n| Choice {
                    value: n.to_string(),
                    text: n.to_string(),
                    disabled: false,
                })
                .collect(),
        });
        for k in 1..=lobby.ai {
            items.push(Item::Select {
                id: format!("lobby.ai.{k}"),
                label: self
                    .t("lobby.difficulty")
                    .replace("{colour}", &self.t(&format!("colour.{}", sim::SEAT_COLOURS[k]))),
                value: lobby.difficulty[k - 1].key().into(),
                options: Difficulty::ALL
                    .iter()
                    .map(|d| Choice {
                        value: d.key().into(),
                        text: self.t(&format!("difficulty.{}", d.key())),
                        disabled: false,
                    })
                    .collect(),
            });
        }
        items.push(self.time_choices(
            "lobby.length",
            "lobby.length",
            lobby.length_ms,
            &t.game.length_choices_ms,
        ));
        items.push(self.time_choices(
            "lobby.buoy",
            "lobby.buoy",
            lobby.buoy_interval_ms,
            &t.game.buoy_interval_choices_ms,
        ));
        items.push(Item::Text {
            id: "lobby.seed".into(),
            label: self.t("lobby.seed"),
            value: lobby.seed.clone(),
            max: SEED_MAX,
            placeholder: self.t("lobby.seed_random"),
        });
        items.push(self.button("lobby.start"));
        items.push(self.button("menu.back"));
        let mut p = Panel::new("lobby", self.t("lobby.heading"), items);
        p.sub = self.t(Setting::MichiganLake.name_key());
        vec![p]
    }

    /// Play's HUD, and the in-match menu over it while it is open.
    pub(super) fn play_panels(&self) -> Vec<Panel> {
        let Some(view) = self.play_view() else {
            return vec![Panel::new("hud", String::new(), vec![self.button("menu.quit")])];
        };
        let p = self.play.as_ref().unwrap();
        let w = &view.world;
        let mut lines = Vec::new();
        if let Some(b) = w.boats.iter().find(|b| b.seat == p.seat) {
            lines.push(self.t(if b.mode == sim::Mode::Driving {
                "hud.driving"
            } else {
                "hud.fishing"
            }));
            // mm/s to km/h: × 3600 / 1 000 000
            let kmh = (b.speed_mm_per_s as i64 * 36 / 10_000) as i32;
            lines.push(self.t("hud.speed").replace("{speed}", &kmh.to_string()));
            if b.wrecked {
                lines.push(self.t("hud.wrecked"));
            }
        }
        lines.extend(self.online_notice());
        let mut hud = Panel::new(
            "hud",
            self.t("hud.time").replace("{time}", &clock_text(w.remaining_ms)),
            vec![Item::quiet_button("menu.pause", self.t("menu.pause"))],
        );
        hud.sub = lines.join(" · ");
        // the Fish-Boy up covers the HUD's panel
        let mut panels = if self.fishboy.open { Vec::new() } else { vec![hud] };
        if !self.fishboy.open {
            panels.extend(self.fishing_panels());
        }
        if w.ended {
            panels.push(Panel::new(
                "pause",
                self.t("hud.ended"),
                vec![self.button("menu.quit")],
            ));
        } else if p.menu_open() {
            let heading = if w.paused { "pause.heading" } else { "menu.heading" };
            panels.push(Panel::new(
                "pause",
                self.t(heading),
                vec![
                    self.button("menu.resume"),
                    self.button("menu.settings"),
                    self.button("menu.quit"),
                ],
            ));
        }
        panels
    }
}
