//! Recipes: how each sound is made from oscillators, noise, envelopes and
//! filters (`data/sounds.json`), checked when the file is read, and
//! compiled into the plain schedule the page's synth replays.
//!
//! A **cue** plays once: a list of layers, each a waveform or noise with a
//! pitch (and an optional glide to a second pitch), an envelope (attack,
//! hold, release) and an optional filter (with its own sweep). A layer can
//! repeat itself (`repeat`) or play a little tune (`notes`), so a fanfare is
//! one layer with notes. A layer marked `stretch` holds for as long as the
//! game asks (`Params::duration_ms`), so the radar's sweep hum lasts the
//! sweep.
//!
//! A **loop** plays until the game stops naming it: the motors and the
//! line's whine. Its layers have no envelope; the game sets its gain and
//! pitch every frame. `throb` beats its volume (a motor's putter) at a rate
//! that rises with the pitch; `wobble` bends its pitch.
//!
//! Units: frequencies in Hz, times in milliseconds, gains 0 to 1, Q as
//! WebAudio's, wobble depth in cents. Held as thousandths (`decimal.rs`).

use serde_json::{json, Value};

use super::decimal::{dec, mul, MILLI};
use super::read::{self, Obj};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wave {
    Sine,
    Square,
    Sawtooth,
    Triangle,
    Noise,
}

impl Wave {
    const ALL: [(&'static str, Wave); 5] = [
        ("sine", Wave::Sine),
        ("square", Wave::Square),
        ("sawtooth", Wave::Sawtooth),
        ("triangle", Wave::Triangle),
        ("noise", Wave::Noise),
    ];

    fn parse(o: &Obj, at: &str) -> Result<Wave, String> {
        let name = read::text(o, "wave", at)?;
        Wave::ALL
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, w)| *w)
            .ok_or_else(|| {
                format!("{at} has wave \"{name}\"; a wave is sine, square, sawtooth, triangle or noise.")
            })
    }

    pub fn key(self) -> &'static str {
        Wave::ALL
            .iter()
            .find(|(_, w)| *w == self)
            .map_or("sine", |(n, _)| n)
    }
}

/// The filter types the page's BiquadFilterNode is given.
const FILTER_TYPES: [&str; 3] = ["lowpass", "highpass", "bandpass"];

/// Q when a filter names none (WebAudio's own default).
const DEFAULT_Q: i64 = 707;

/// How a pitch or a filter sweep moves: exponential sounds even to the
/// ear; linear is for a sweep that should rush at the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glide {
    Exp,
    Linear,
}

impl Glide {
    fn method(self) -> &'static str {
        match self {
            Glide::Exp => "exponentialRampToValueAtTime",
            Glide::Linear => "linearRampToValueAtTime",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub kind: &'static str,
    /// Hz, thousandths
    pub freq: i64,
    pub to_freq: Option<i64>,
    pub q: i64,
}

impl Filter {
    fn parse(v: &Value, at: &str, sweeps: bool) -> Result<Filter, String> {
        let at = format!("{at} filter");
        let o = read::obj(v, &at)?;
        let allowed: &[&str] = if sweeps {
            &["type", "freq", "to_freq", "q"]
        } else {
            &["type", "freq", "q"]
        };
        read::keys(o, allowed, &at)?;
        let name = read::text(o, "type", &at)?;
        let kind = FILTER_TYPES
            .iter()
            .find(|t| **t == name)
            .copied()
            .ok_or_else(|| format!("{at} has type \"{name}\"; a filter is lowpass, highpass or bandpass."))?;
        let freq = positive(read::milli(o, "freq", &at)?, &format!("{at} freq"))?;
        let to_freq = read::opt_milli(o, "to_freq", &at)?
            .map(|f| positive(f, &format!("{at} to_freq")))
            .transpose()?;
        let q = positive(
            read::opt_milli(o, "q", &at)?.unwrap_or(DEFAULT_Q),
            &format!("{at} q"),
        )?;
        Ok(Filter {
            kind,
            freq,
            to_freq,
            q,
        })
    }
}

/// A slow bend of the pitch, `cents` either way, `rate` times a second.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wobble {
    pub rate: i64,
    pub cents: i64,
}

impl Wobble {
    fn parse(v: &Value, at: &str) -> Result<Wobble, String> {
        let at = format!("{at} wobble");
        let o = read::obj(v, &at)?;
        read::keys(o, &["rate", "cents"], &at)?;
        Ok(Wobble {
            rate: positive(read::milli(o, "rate", &at)?, &format!("{at} rate"))?,
            cents: positive(read::milli(o, "cents", &at)?, &format!("{at} cents"))?,
        })
    }

    fn json(&self) -> Value {
        json!({ "rate": dec(self.rate), "cents": dec(self.cents) })
    }
}

/// A beat in the volume: `depth` of the gain (0 to 1) comes and goes
/// `rate` times a second at the loop's own pitch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Throb {
    pub rate: i64,
    pub depth: i64,
}

fn positive(v: i64, what: &str) -> Result<i64, String> {
    if v > 0 {
        Ok(v)
    } else {
        Err(format!("{what} must be more than zero."))
    }
}

fn gain_of(o: &Obj, at: &str) -> Result<i64, String> {
    let g = read::milli(o, "gain", at)?;
    read::within(g, 1, MILLI, &format!("{at} gain"), "more than 0 and at most 1")
}

/// One sounding part of a cue, its notes and repeats already laid out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    pub wave: Wave,
    /// when it starts after the cue does
    pub at_ms: i64,
    /// Hz in thousandths; zero for noise
    pub freq: i64,
    pub to_freq: Option<i64>,
    pub glide: Glide,
    pub attack_ms: i64,
    pub hold_ms: i64,
    pub release_ms: i64,
    pub gain: i64,
    pub filter: Option<Filter>,
    pub stretch: bool,
    pub wobble: Option<Wobble>,
}

impl Layer {
    pub fn end_ms(&self) -> i64 {
        self.at_ms + self.attack_ms + self.hold_ms + self.release_ms
    }

    /// The layer as written, then one copy per note and per repeat.
    fn parse(v: &Value, at: &str) -> Result<Vec<Layer>, String> {
        let o = read::obj(v, at)?;
        read::keys(
            o,
            &[
                "wave",
                "at_ms",
                "freq",
                "to_freq",
                "glide",
                "attack_ms",
                "hold_ms",
                "release_ms",
                "gain",
                "filter",
                "stretch",
                "wobble",
                "notes",
                "repeat",
            ],
            at,
        )?;
        let wave = Wave::parse(o, at)?;
        let (freq, to_freq) = if wave == Wave::Noise {
            if o.contains_key("freq") || o.contains_key("to_freq") {
                return Err(format!(
                    "{at} is noise, which has no freq; colour it with a filter."
                ));
            }
            (0, None)
        } else {
            let f = positive(read::milli(o, "freq", at)?, &format!("{at} freq"))?;
            let to = read::opt_milli(o, "to_freq", at)?
                .map(|t| positive(t, &format!("{at} to_freq")))
                .transpose()?;
            (f, to)
        };
        let glide = match read::opt_text(o, "glide", at)? {
            None | Some("exp") => Glide::Exp,
            Some("linear") => Glide::Linear,
            Some(other) => return Err(format!("{at} has glide \"{other}\"; a glide is exp or linear.")),
        };
        let ms = |key: &str, least: i64| -> Result<i64, String> {
            let v = read::opt_whole(o, key, at, least)?;
            if v < least {
                Err(format!("{at} {key} must be at least {least}."))
            } else {
                Ok(v)
            }
        };
        let base = Layer {
            wave,
            at_ms: ms("at_ms", 0)?,
            freq,
            to_freq,
            glide,
            // at least a millisecond each way, or the sound clicks
            attack_ms: ms("attack_ms", 1)?,
            hold_ms: ms("hold_ms", 0)?,
            release_ms: ms("release_ms", 1)?,
            gain: gain_of(o, at)?,
            filter: o.get("filter").map(|f| Filter::parse(f, at, true)).transpose()?,
            stretch: read::flag(o, "stretch", at)?,
            wobble: o.get("wobble").map(|w| Wobble::parse(w, at)).transpose()?,
        };

        let mut out = vec![base.clone()];
        if let Some(notes) = o.get("notes") {
            if wave == Wave::Noise {
                return Err(format!("{at} is noise, which plays no notes."));
            }
            let list = notes
                .as_array()
                .filter(|l| !l.is_empty())
                .ok_or_else(|| format!("{at} notes must be a list of [at_ms, freq] pairs."))?;
            out = Vec::new();
            for (k, n) in list.iter().enumerate() {
                let what = format!("{at} note {}", k + 1);
                let pair = n
                    .as_array()
                    .filter(|p| p.len() == 2)
                    .ok_or_else(|| format!("{what} must be [at_ms, freq]."))?;
                let when = pair[0]
                    .as_i64()
                    .filter(|w| *w >= 0)
                    .ok_or_else(|| format!("{what}: at_ms must be a whole number from 0."))?;
                let f = positive(super::decimal::parse_milli(&pair[1], &what)?, &what)?;
                let mut l = base.clone();
                l.at_ms += when;
                l.freq = f;
                // a glide keeps its shape on every note
                l.to_freq = base.to_freq.map(|t| t * f / base.freq);
                out.push(l);
            }
        }
        if let Some(r) = o.get("repeat") {
            let what = format!("{at} repeat");
            let ro = read::obj(r, &what)?;
            read::keys(ro, &["times", "every_ms"], &what)?;
            let times = read::whole(ro, "times", &what)?;
            let every = read::whole(ro, "every_ms", &what)?;
            if !(1..=64).contains(&times) || every < 1 {
                return Err(format!("{what} must play 1 to 64 times, at least 1 ms apart."));
            }
            let once = out;
            out = Vec::new();
            for k in 0..times {
                for l in &once {
                    let mut c = l.clone();
                    c.at_ms += k * every;
                    out.push(c);
                }
            }
        }
        Ok(out)
    }
}

/// How the game plays a cue this time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Params {
    /// thousandths of the recipe's gain
    pub gain: i64,
    /// thousandths of the recipe's pitch (1000 as written; 2000 an octave up)
    pub pitch: i64,
    /// how long after now it starts
    pub delay_ms: i64,
    /// how long the cue's `stretch` layers last, start to end
    pub duration_ms: Option<i64>,
}

impl Default for Params {
    fn default() -> Params {
        Params {
            gain: MILLI,
            pitch: MILLI,
            delay_ms: 0,
            duration_ms: None,
        }
    }
}

/// One point of a WebAudio parameter's schedule: the AudioParam method,
/// seconds after the cue is handed over, and the value.
fn point(method: &str, ms: i64, v: i64) -> Value {
    json!({ "m": method, "t": dec(ms), "v": dec(v) })
}

/// A recipe that plays once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cue {
    pub id: String,
    pub label: String,
    pub layers: Vec<Layer>,
}

impl Cue {
    pub fn parse(o: &Obj, id: &str, at: &str) -> Result<Cue, String> {
        read::keys(o, &["id", "label", "layers"], at)?;
        let label = read::text(o, "label", at)?.to_string();
        let list = o
            .get("layers")
            .and_then(Value::as_array)
            .filter(|l| !l.is_empty())
            .ok_or_else(|| format!("{at} needs a list of layers."))?;
        let mut layers = Vec::new();
        for (k, l) in list.iter().enumerate() {
            layers.extend(Layer::parse(l, &format!("{at} layer {}", k + 1))?);
        }
        Ok(Cue {
            id: id.to_string(),
            label,
            layers,
        })
    }

    /// How long the cue lasts as written.
    pub fn length_ms(&self) -> i64 {
        self.layers.iter().map(Layer::end_ms).max().unwrap_or(0)
    }

    /// The schedule the page replays, times in seconds from when it gets
    /// it. Every number is final: the page multiplies nothing.
    pub fn compile(&self, p: &Params) -> Value {
        let layers: Vec<Value> = self
            .layers
            .iter()
            .map(|l| {
                let mut hold = l.hold_ms;
                if let (true, Some(d)) = (l.stretch, p.duration_ms) {
                    hold = (d - l.at_ms - l.attack_ms - l.release_ms).max(0);
                }
                let start = p.delay_ms + l.at_ms;
                let peak = start + l.attack_ms;
                let fall = peak + hold;
                let end = fall + l.release_ms;
                let lin = "linearRampToValueAtTime";
                let g = mul(l.gain, p.gain);
                let gain = vec![
                    point("setValueAtTime", start, 0),
                    point(lin, peak, g),
                    point(lin, fall, g),
                    point(lin, end, 0),
                ];
                let sweep = |from: i64, to: Option<i64>, glide: Glide| {
                    let mut pts = vec![point("setValueAtTime", start, from)];
                    if let Some(to) = to {
                        pts.push(point(glide.method(), end, to));
                    }
                    Value::Array(pts)
                };
                let mut out = json!({
                    "wave": l.wave.key(),
                    "start": dec(start),
                    "stop": dec(end),
                    "gain": gain,
                });
                if l.wave != Wave::Noise {
                    out["freq"] = sweep(mul(l.freq, p.pitch), l.to_freq.map(|t| mul(t, p.pitch)), l.glide);
                }
                if let Some(f) = &l.filter {
                    out["filter"] = json!({
                        "type": f.kind,
                        "q": dec(f.q),
                        "freq": sweep(mul(f.freq, p.pitch), f.to_freq.map(|t| mul(t, p.pitch)), Glide::Exp),
                    });
                }
                if let Some(w) = &l.wobble {
                    out["wobble"] = w.json();
                }
                out
            })
            .collect();
        json!({ "id": self.id, "layers": layers })
    }
}

/// One sounding part of a loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopLayer {
    pub wave: Wave,
    pub freq: i64,
    pub gain: i64,
    pub filter: Option<Filter>,
    pub wobble: Option<Wobble>,
    pub throb: Option<Throb>,
}

/// A recipe that plays until the game stops naming it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loop {
    pub id: String,
    pub label: String,
    pub layers: Vec<LoopLayer>,
}

impl Loop {
    pub fn parse(o: &Obj, id: &str, at: &str) -> Result<Loop, String> {
        read::keys(o, &["id", "label", "layers"], at)?;
        let label = read::text(o, "label", at)?.to_string();
        let list = o
            .get("layers")
            .and_then(Value::as_array)
            .filter(|l| !l.is_empty())
            .ok_or_else(|| format!("{at} needs a list of layers."))?;
        let mut layers = Vec::new();
        for (k, v) in list.iter().enumerate() {
            let at = format!("{at} layer {}", k + 1);
            let o = read::obj(v, &at)?;
            read::keys(o, &["wave", "freq", "gain", "filter", "wobble", "throb"], &at)?;
            let wave = Wave::parse(o, &at)?;
            let freq = if wave == Wave::Noise {
                if o.contains_key("freq") {
                    return Err(format!(
                        "{at} is noise, which has no freq; colour it with a filter."
                    ));
                }
                0
            } else {
                positive(read::milli(o, "freq", &at)?, &format!("{at} freq"))?
            };
            let throb = match o.get("throb") {
                None => None,
                Some(t) => {
                    let what = format!("{at} throb");
                    let to = read::obj(t, &what)?;
                    read::keys(to, &["rate", "depth"], &what)?;
                    Some(Throb {
                        rate: positive(read::milli(to, "rate", &what)?, &format!("{what} rate"))?,
                        depth: read::within(
                            read::milli(to, "depth", &what)?,
                            1,
                            MILLI,
                            &format!("{what} depth"),
                            "more than 0 and at most 1",
                        )?,
                    })
                }
            };
            layers.push(LoopLayer {
                wave,
                freq,
                gain: gain_of(o, &at)?,
                filter: o
                    .get("filter")
                    .map(|f| Filter::parse(f, &at, false))
                    .transpose()?,
                wobble: o.get("wobble").map(|w| Wobble::parse(w, &at)).transpose()?,
                throb,
            });
        }
        Ok(Loop {
            id: id.to_string(),
            label,
            layers,
        })
    }

    /// This frame's settings for each layer at `gain` and `pitch`
    /// (thousandths). A throb's beat is split out of the gain, so the page
    /// adds a beating `amount` to a steady `gain` and computes nothing.
    pub fn compile(&self, gain: i64, pitch: i64) -> Vec<Value> {
        self.layers
            .iter()
            .map(|l| {
                let g = mul(l.gain, gain);
                let mut out = json!({ "wave": l.wave.key(), "gain": dec(g) });
                if let Some(t) = &l.throb {
                    let amount = mul(g, t.depth) / 2;
                    out["gain"] = dec(g - amount);
                    out["throb"] = json!({ "rate": dec(mul(t.rate, pitch)), "amount": dec(amount) });
                }
                if l.wave != Wave::Noise {
                    out["freq"] = dec(mul(l.freq, pitch));
                }
                if let Some(f) = &l.filter {
                    out["filter"] = json!({ "type": f.kind, "q": dec(f.q), "freq": dec(mul(f.freq, pitch)) });
                }
                if let Some(w) = &l.wobble {
                    out["wobble"] = w.json();
                }
                out
            })
            .collect()
    }
}
