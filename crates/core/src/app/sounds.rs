//! The Sounds screen (from the Gallery): a button for every cue, and the
//! loops at settings you choose, for a listen to the sound design. Another
//! boat's motor can sit anywhere on a line past you (the slider) or drive
//! the whole line at top speed (Drive past), to hear the volume rise and
//! the Doppler shift.
//!
//! Also here: what the page plays each frame (`sound_frame`), and the menu
//! click.

use serde_json::Value;

use super::*;
use crate::sound::decimal::MILLI;
use crate::sound::motor::{self, Body, Heard};
use crate::sound::recipe::Params;
use crate::sound::{Bank, Sound, LINE_TENSION, OWN_MOTOR, RIVAL_MOTOR};

/// The sliders' range and step, in percent.
const SLIDER_MAX: u32 = 100;
const SLIDER_STEP: u32 = 5;

/// The Sounds screen's choices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundsScreen {
    pub own: bool,
    /// percent of full throttle and of top speed
    pub throttle: u32,
    pub tension: bool,
    pub tension_level: u32,
    pub rival: bool,
    /// where on the pass-by line the other boat sits, percent from start
    pub rival_at: u32,
    /// when a drive past started (the app's clock, µs)
    pub pass_from: Option<i64>,
}

impl Default for SoundsScreen {
    fn default() -> SoundsScreen {
        SoundsScreen {
            own: false,
            throttle: 50,
            tension: false,
            tension_level: 70,
            rival: false,
            rival_at: 30,
            pass_from: None,
        }
    }
}

/// The top speed a rival is heard at before the tuning file loads: the
/// figure the map was sized for.
pub fn top_speed_mm_per_s() -> i64 {
    crate::map::BOAT_TOP_SPEED_MM_PER_S as i64
}

impl App {
    /// The boat's top speed without nitrous, from the tuning file.
    pub fn top_speed(&self) -> i64 {
        self.tuning
            .as_ref()
            .map_or(top_speed_mm_per_s(), |t| t.boat.top_speed_mm_per_s as i64)
    }

    /// The match's motors as heard from this browser's boat: its own (by
    /// throttle and speed) and each rival afloat (by place and velocity).
    fn match_voices(&self, s: &Sound) -> Vec<Value> {
        let Some(p) = self.play.as_ref().filter(|_| self.screen == Screen::Play) else {
            return Vec::new();
        };
        let t = &s.bank.tuning;
        let top = self.top_speed();
        let mut voices = Vec::new();
        let body = |b: &crate::sim::Boat| Body {
            pos: [b.pos_um[0] / 1000, b.pos_um[1] / 1000],
            vel: b.vel().map(|v| v as i64),
        };
        let Some(me) = p.game.boat(p.seat).filter(|b| !b.wrecked()) else {
            return voices;
        };
        let speed = me.speed_mm_per_s() as i64 * MILLI / top.max(1);
        let push = if p.throttle_held() { MILLI } else { 0 };
        voices.extend(s.voice("own", OWN_MOTOR, motor::own_motor(&t.own_motor, push, speed)));
        for b in p.game.boats.iter().flatten() {
            if b.seat != p.seat && !b.wrecked() {
                let heard = motor::rival_motor(&t.rival_motor, top, &body(me), &body(b));
                voices.extend(s.voice(&format!("rival.{}", b.seat), RIVAL_MOTOR, heard));
            }
        }
        // the line's whine while reeling (0 to 1000, as MILLI)
        if let Some(level) = self.line_tension() {
            voices.extend(s.voice("tension", LINE_TENSION, motor::line_tension(&t.line_tension, level)));
        }
        voices
    }
    /// Takes the sound recipes and the tuning file, checked, or says why not.
    pub fn load_sounds(&mut self, sounds_json: &str, tuning_json: &str) -> Result<(), String> {
        self.sound = Some(Sound::new(Bank::parse(sounds_json, tuning_json)?));
        Ok(())
    }

    /// Plays cue `id` as written, if the sounds are loaded.
    pub fn play(&mut self, id: &str) {
        if let Some(s) = &mut self.sound {
            s.play(id, &Params::default());
        }
    }

    /// Plays the menu click, for a button press.
    pub(super) fn click(&mut self) {
        if let Some(s) = &mut self.sound {
            let id = s.bank.click.clone();
            s.play(&id, &Params::default());
        }
    }

    /// The pass-by line's half length: hearing distance plus the overrun.
    fn pass_half_mm(bank: &Bank) -> i64 {
        let t = &bank.tuning;
        let hear = motor::hearing_mm(&t.rival_motor, top_speed_mm_per_s());
        hear + hear * t.pass_by.overrun / MILLI
    }

    /// The other boat on the pass-by line: driving past, or where the
    /// slider puts it, always at top speed toward +x. You sit still at the
    /// origin.
    fn rival_body(&self, bank: &Bank) -> Body {
        let half = Self::pass_half_mm(bank);
        let v = self.top_speed();
        let x = match self.sounds.pass_from {
            Some(from) => -half + v * (self.clock - from).max(0) / 1_000_000,
            None => -half + 2 * half * self.sounds.rival_at as i64 / SLIDER_MAX as i64,
        };
        Body {
            pos: [x, bank.tuning.pass_by.side_mm],
            vel: [v, 0],
        }
    }

    fn rival_heard(&self, bank: &Bank) -> Heard {
        motor::rival_motor(
            &bank.tuning.rival_motor,
            self.top_speed(),
            &Body::default(),
            &self.rival_body(bank),
        )
    }

    /// The Sounds screen's controls (`sounds.*`).
    pub(super) fn sounds_act(&mut self, name: &str, value: &str) -> Result<(), String> {
        if let Some(id) = name.strip_prefix("sounds.play.") {
            let known = self.sound.as_ref().is_some_and(|s| s.bank.cue(id).is_some());
            if !known {
                return Err(format!("There is no sound \"{id}\"."));
            }
            self.play(id);
            return Ok(());
        }
        let percent = || -> Result<u32, String> {
            let v: u32 = value
                .parse()
                .map_err(|_| format!("The value \"{value}\" is not a whole number."))?;
            Ok(v.min(SLIDER_MAX) / SLIDER_STEP * SLIDER_STEP)
        };
        let s = &mut self.sounds;
        match name {
            "sounds.own" => s.own = value == "on",
            "sounds.throttle" => s.throttle = percent()?,
            "sounds.tension" => s.tension = value == "on",
            "sounds.tension_level" => s.tension_level = percent()?,
            "sounds.rival" => {
                s.rival = value == "on";
                s.pass_from = None;
            }
            "sounds.rival_at" => {
                s.rival_at = percent()?;
                s.pass_from = None;
            }
            "sounds.drive_past" => s.pass_from = Some(self.clock),
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    /// What the page plays now (`sound::Sound::frame`): the cues queued
    /// since the last call, and the loops running; null before the sounds
    /// load. Off the Sounds screen no
    /// loop runs yet; the match will name its motors and line here.
    pub fn sound_frame(&mut self) -> Value {
        let volume = self.settings.volume;
        let on_screen = self.screen == Screen::Sounds;
        if let (Some(from), Some(s)) = (self.sounds.pass_from, &self.sound) {
            let half = Self::pass_half_mm(&s.bank);
            let gone = self.top_speed() * (self.clock - from) / 1_000_000 > 2 * half;
            if gone || !on_screen {
                self.sounds.pass_from = None;
            }
        }
        let Some(s) = &self.sound else {
            return Value::Null;
        };
        let mut voices = self.match_voices(s);
        if on_screen {
            let t = &s.bank.tuning;
            let sc = &self.sounds;
            if sc.own {
                let push = sc.throttle as i64 * MILLI / SLIDER_MAX as i64;
                voices.extend(s.voice("own", OWN_MOTOR, motor::own_motor(&t.own_motor, push, push)));
            }
            if sc.tension {
                let level = sc.tension_level as i64 * MILLI / SLIDER_MAX as i64;
                voices.extend(s.voice(
                    "tension",
                    LINE_TENSION,
                    motor::line_tension(&t.line_tension, level),
                ));
            }
            if sc.rival || sc.pass_from.is_some() {
                voices.extend(s.voice("rival.debug", RIVAL_MOTOR, self.rival_heard(&s.bank)));
            }
        }
        let s = self.sound.as_mut().expect("checked above");
        s.frame(volume, voices)
    }

    /// The noise buffer every noise layer plays (Q16.16), empty before the
    /// sounds load.
    pub fn sound_noise(&self) -> Vec<i32> {
        self.sound.as_ref().map_or(Vec::new(), |s| s.bank.noise())
    }

    fn percent_slider(&self, id: &str, value: u32) -> Item {
        Item::Slider {
            id: id.into(),
            label: self.t(id),
            min: 0,
            max: SLIDER_MAX,
            step: SLIDER_STEP,
            value,
            text: format!("{value}%"),
        }
    }

    fn toggle(&self, id: &str, value: bool) -> Item {
        Item::Toggle {
            id: id.into(),
            label: self.t(id),
            value,
        }
    }

    pub(super) fn sounds_panels(&self) -> Vec<Panel> {
        let Some(s) = &self.sound else {
            let mut p = Panel::new("sounds", self.t("sounds.heading"), vec![self.button("menu.back")]);
            p.sub = self.t("sounds.none");
            return vec![p];
        };
        let cues = s
            .bank
            .cues
            .iter()
            .map(|c| Item::button(&format!("sounds.play.{}", c.id), c.label.clone()))
            .collect();
        let mut list = Panel::new("sounds", self.t("sounds.heading"), cues);
        list.sub = self.t("sounds.sub");

        let sc = &self.sounds;
        let rival = self.rival_body(&s.bank);
        let metres = (rival.pos[0] * rival.pos[0] + rival.pos[1] * rival.pos[1]).isqrt() / 1000;
        let key = if rival.pos[0] < 0 {
            "sounds.closing"
        } else {
            "sounds.leaving"
        };
        let heard = self.rival_heard(&s.bank);
        let heard_text = if heard.gain > 0 {
            self.t("sounds.heard_as")
                .replace("{gain}", &(heard.gain / 10).to_string())
                .replace("{pitch}", &(heard.pitch / 10).to_string())
        } else {
            self.t("sounds.silent")
        };
        let loops = Panel::new(
            "sound-loops",
            self.t("sounds.loops"),
            vec![
                self.toggle("sounds.own", sc.own),
                self.percent_slider("sounds.throttle", sc.throttle),
                self.toggle("sounds.tension", sc.tension),
                self.percent_slider("sounds.tension_level", sc.tension_level),
                self.toggle("sounds.rival", sc.rival),
                self.percent_slider("sounds.rival_at", sc.rival_at),
                Item::List {
                    rows: vec![
                        [
                            self.t("sounds.distance"),
                            self.t(key).replace("{metres}", &metres.to_string()),
                        ],
                        [self.t("sounds.heard"), heard_text],
                    ],
                },
                self.button("sounds.drive_past"),
                self.button("menu.back"),
            ],
        );
        vec![list, loops]
    }
}

#[cfg(test)]
mod tests;
