//! The sound engine's core (PLAN.md, decision 9). There are no audio
//! files: every sound is a recipe in `data/sounds.json` that the page's
//! small synth (`web/audio.js`) plays with WebAudio. The core decides what
//! sounds and how: which cues play this frame (each compiled to a plain
//! schedule, gain and pitch applied), which loops are running and at what
//! gain and pitch, and the master volume from Settings. The page replays it
//! and computes nothing.
//!
//! - `recipe.rs`: the recipes, checked, and compiled for the page.
//! - `events.rs`: game events to cues, by name (how to add one is there).
//! - `motor.rs`: rival motors (distance and Doppler), your motor, the
//!   line's whine and the reel's ratchet; their numbers are the `audio`
//!   section of `data/tuning.json`.
//!
//! The game calls, each frame: `play` or `event` as things happen, then
//! `frame` with this frame's loops (`voice`). There is no stereo and no
//! direction (the brief), so nothing is panned.

use serde_json::{json, Value};

pub mod decimal;
pub mod events;
pub mod motor;
mod read;
pub mod recipe;
#[cfg(test)]
pub(crate) mod tests;

use decimal::{dec, MILLI};
use motor::{AudioTuning, Heard};
use recipe::{Cue, Loop, Params};

/// The noise every noise layer plays: `samples` values from a seed, so it
/// is the same on every machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoiseSpec {
    pub samples: usize,
    pub seed: u64,
}

/// `data/sounds.json` and the audio tuning, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bank {
    /// in file order (the Sounds screen lists them so)
    pub cues: Vec<Cue>,
    /// (id, the cue it plays)
    pub aliases: Vec<(String, String)>,
    pub loops: Vec<Loop>,
    pub events: events::Table,
    pub tuning: AudioTuning,
    pub noise: NoiseSpec,
    /// how quickly a loop follows its new gain and pitch
    pub smooth_ms: i64,
    /// how long a loop takes to fade out when the game stops naming it
    pub fade_ms: i64,
    /// cues waiting for the page beyond this many are dropped, oldest first
    pub max_pending: usize,
    /// the cue every menu button plays
    pub click: String,
}

/// The loops the game names by id: your motor, a rival's, the line's whine.
pub const OWN_MOTOR: &str = "own_motor";
pub const RIVAL_MOTOR: &str = "rival_motor";
pub const LINE_TENSION: &str = "line_tension";

/// The longest a cue may last as written.
const MAX_CUE_MS: i64 = 20_000;

/// A cue or loop id: lowercase words joined by `_`.
fn check_id(id: &str, at: &str) -> Result<(), String> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(format!("{at} id \"{id}\" must be lowercase words joined by _."));
    }
    Ok(())
}

impl Bank {
    pub fn parse(sounds_json: &str, tuning_json: &str) -> Result<Bank, String> {
        let tuning = AudioTuning::parse(tuning_json)?;
        let all: Value = serde_json::from_str(sounds_json)
            .map_err(|e| format!("The sounds file could not be read: {e}"))?;
        let o = read::obj(&all, "The sounds file")?;
        read::keys(
            o,
            &[
                "noise",
                "smooth_ms",
                "fade_ms",
                "max_pending",
                "click",
                "cues",
                "loops",
                "events",
            ],
            "The sounds file",
        )?;
        let top = "The sounds file";

        let n = read::obj(o.get("noise").unwrap_or(&Value::Null), "The sounds file noise")?;
        read::keys(n, &["samples", "seed"], "The sounds file noise")?;
        let samples = read::whole(n, "samples", "The sounds file noise")?;
        if !(256..=1 << 20).contains(&samples) {
            return Err("The sounds file noise samples must be 256 to 1048576.".into());
        }
        let noise = NoiseSpec {
            samples: samples as usize,
            seed: read::whole(n, "seed", "The sounds file noise")? as u64,
        };

        let mut cues: Vec<Cue> = Vec::new();
        let mut aliases = Vec::new();
        let list = o
            .get("cues")
            .and_then(Value::as_array)
            .ok_or_else(|| "The sounds file needs a list of cues.".to_string())?;
        for (k, v) in list.iter().enumerate() {
            let c = read::obj(v, &format!("Cue {}", k + 1))?;
            let id = read::text(c, "id", &format!("Cue {}", k + 1))?;
            let at = format!("Cue {id}");
            check_id(id, &at)?;
            if cues.iter().any(|q| q.id == id) || aliases.iter().any(|(a, _)| a == id) {
                return Err(format!("{at} is listed twice."));
            }
            if let Some(target) = read::opt_text(c, "same_as", &at)? {
                read::keys(c, &["id", "same_as"], &at)?;
                aliases.push((id.to_string(), target.to_string()));
                continue;
            }
            let cue = Cue::parse(c, id, &at)?;
            if cue.length_ms() > MAX_CUE_MS {
                return Err(format!(
                    "{at} lasts {} ms; a cue lasts at most {MAX_CUE_MS} ms.",
                    cue.length_ms()
                ));
            }
            cues.push(cue);
        }
        for (id, target) in &aliases {
            if !cues.iter().any(|c| &c.id == target) {
                return Err(format!(
                    "Cue {id} is the same as \"{target}\", which is not a cue."
                ));
            }
        }

        let mut loops: Vec<Loop> = Vec::new();
        let list = o
            .get("loops")
            .and_then(Value::as_array)
            .ok_or_else(|| "The sounds file needs a list of loops.".to_string())?;
        for (k, v) in list.iter().enumerate() {
            let l = read::obj(v, &format!("Loop {}", k + 1))?;
            let id = read::text(l, "id", &format!("Loop {}", k + 1))?;
            let at = format!("Loop {id}");
            check_id(id, &at)?;
            if loops.iter().any(|q| q.id == id) {
                return Err(format!("{at} is listed twice."));
            }
            loops.push(Loop::parse(l, id, &at)?);
        }

        let events = events::Table::parse(o.get("events").unwrap_or(&Value::Null))?;
        let bank = Bank {
            cues,
            aliases,
            loops,
            events,
            tuning,
            noise,
            smooth_ms: read::whole(o, "smooth_ms", top)?.max(1),
            fade_ms: read::whole(o, "fade_ms", top)?.max(1),
            max_pending: read::whole(o, "max_pending", top)?.max(1) as usize,
            click: read::text(o, "click", top)?.to_string(),
        };
        for id in [OWN_MOTOR, RIVAL_MOTOR, LINE_TENSION] {
            if bank.find_loop(id).is_none() {
                return Err(format!(
                    "The sounds file has no loop \"{id}\", which the game plays."
                ));
            }
        }
        if bank.cue(&bank.click).is_none() {
            return Err(format!("The sounds file click \"{}\" is not a cue.", bank.click));
        }
        for (name, entries) in &bank.events.rows {
            for e in entries {
                for id in e.every_cue() {
                    if bank.cue(&id).is_none() {
                        return Err(format!("Event {name} plays \"{id}\", which is not a cue."));
                    }
                }
            }
        }
        Ok(bank)
    }

    /// A cue by id, through its alias if it is one.
    pub fn cue(&self, id: &str) -> Option<&Cue> {
        let id = self
            .aliases
            .iter()
            .find(|(a, _)| a == id)
            .map_or(id, |(_, t)| t.as_str());
        self.cues.iter().find(|c| c.id == id)
    }

    pub fn find_loop(&self, id: &str) -> Option<&Loop> {
        self.loops.iter().find(|l| l.id == id)
    }

    /// The noise buffer, as Q16.16 values from -1 to 1 (the shim turns
    /// them into floats as it does the frame's).
    pub fn noise(&self) -> Vec<i32> {
        let mut rng = crate::map::rng::Rng::new(self.noise.seed);
        let one = crate::gfx::fx::ONE as u32;
        (0..self.noise.samples)
            .map(|_| rng.below(2 * one + 1) as i32 - one as i32)
            .collect()
    }
}

/// The sounds waiting for the page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sound {
    pub bank: Bank,
    pending: Vec<Value>,
}

impl Sound {
    pub fn new(bank: Bank) -> Sound {
        Sound {
            bank,
            pending: Vec::new(),
        }
    }

    /// Queues cue `id` for the page. An unknown id plays nothing and says
    /// so (false).
    pub fn play(&mut self, id: &str, p: &Params) -> bool {
        let Some(cue) = self.bank.cue(id) else {
            return false;
        };
        let compiled = cue.compile(p);
        if self.pending.len() >= self.bank.max_pending {
            self.pending.remove(0);
        }
        self.pending.push(compiled);
        true
    }

    /// Plays what a game event sounds like to `listener` (a seat; `None`
    /// hears only the events everyone hears). See `events.rs`.
    pub fn event(&mut self, event: &Value, listener: Option<i64>) {
        for (id, p) in self.bank.events.plays(event, listener) {
            self.play(&id, &p);
        }
    }

    /// Cues waiting for the page (for tests and the Sounds screen).
    pub fn pending(&self) -> &[Value] {
        &self.pending
    }

    /// One loop this frame: `key` names the voice across frames (`own`,
    /// `rival.2`), `loop_id` its recipe. Silent or unknown, it is left out,
    /// and the page fades it.
    pub fn voice(&self, key: &str, loop_id: &str, heard: Heard) -> Option<Value> {
        let l = self.bank.find_loop(loop_id)?;
        if heard.gain <= 0 {
            return None;
        }
        Some(json!({ "key": key, "loop": loop_id, "layers": l.compile(heard.gain, heard.pitch) }))
    }

    /// What the page plays now: the waiting cues (taken), this frame's
    /// loops, and the master volume from Settings (percent).
    pub fn frame(&mut self, volume_pct: u32, voices: Vec<Value>) -> Value {
        let cues = std::mem::take(&mut self.pending);
        json!({
            "master": dec(volume_pct as i64 * MILLI / 100),
            "smooth": dec(self.bank.smooth_ms),
            "fade": dec(self.bank.fade_ms),
            "cues": cues,
            "voices": voices,
        })
    }
}
