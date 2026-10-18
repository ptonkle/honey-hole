//! The sounds that follow the game frame by frame: a rival's motor heard
//! from your boat (louder as it nears, with a Doppler shift), your own
//! motor's idle and throttle, the line's drag, which ticks faster as the
//! line tightens, and the reel's ratchet, which clicks faster as you reel. Each is a pure function of the
//! game's numbers and the audio section of `data/tuning.json`.
//!
//! Gains and pitches are thousandths (1000 is the recipe as written).
//! Positions are millimetres on the map, velocities mm/s (PLAN.md, 1).

use serde_json::Value;

use super::decimal::{mul, MILLI};
use super::read::{self, Obj};
use crate::map::trig::isqrt;

/// How a rival's motor is heard. The brief: heard from about 5 s away at
/// top speed, a Doppler shift, no direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RivalTuning {
    /// silent beyond the distance a rival covers in this long at top speed
    /// (thousandths of a second)
    pub hear_seconds: i64,
    /// at full volume this close
    pub full_within_mm: i64,
    /// the voice's gain at full volume
    pub loudest: i64,
    /// how the volume rises from silence to full: 1 evenly, 2 slowly at
    /// first and quickly near (a whole number, 1 to 4)
    pub curve: u32,
    pub sound_speed_mm_per_s: i64,
    /// 1 is true to life; boats are slow next to sound, so the true shift
    /// is under a semitone and the game exaggerates it
    pub doppler_scale: i64,
    pub lowest_pitch: i64,
    pub highest_pitch: i64,
}

/// Your own motor: the pitch and volume rise with speed and with the
/// throttle held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnTuning {
    pub idle_gain: i64,
    pub full_gain: i64,
    pub idle_pitch: i64,
    pub full_pitch: i64,
    /// how much of the rise comes from the throttle rather than the speed
    pub throttle_share: i64,
}

/// The line's drag: silent below `starts_at` tension, then a quiet tick
/// (the `line_drag` cue) that comes faster and a little higher up to full
/// tension, like a reel's drag paying out. It replaced a whine the person
/// said sounded "like I'm being abducted by an alien" (2026-10-18).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TensionTuning {
    pub starts_at: i64,
    /// ticks a second (thousandths) just past `starts_at` and at full
    pub slow_ticks_per_s: i64,
    pub fast_ticks_per_s: i64,
    /// each tick's gain, thousandths of the cue's own
    pub gain: i64,
    pub low_pitch: i64,
    pub high_pitch: i64,
}

/// The reel's ratchet: clicks a second at a slow reel and a full one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RatchetTuning {
    pub slow_clicks_per_s: i64,
    pub fast_clicks_per_s: i64,
    pub slow_pitch: i64,
    pub fast_pitch: i64,
}

/// The Sounds screen's pass-by: a rival at top speed on a straight line
/// this far to the side of you.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassByTuning {
    pub side_mm: i64,
    /// the line starts and ends this much beyond hearing (thousandths of
    /// the hearing distance)
    pub overrun: i64,
}

/// The audio section of `data/tuning.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioTuning {
    pub rival_motor: RivalTuning,
    pub own_motor: OwnTuning,
    pub line_tension: TensionTuning,
    pub ratchet: RatchetTuning,
    pub pass_by: PassByTuning,
}

fn section<'a>(o: &'a Obj, key: &str, allowed: &[&str]) -> Result<(&'a Obj, String), String> {
    let at = format!("tuning audio.{key}");
    let v = o
        .get(key)
        .ok_or_else(|| format!("tuning audio has no \"{key}\"."))?;
    let s = read::obj(v, &at)?;
    read::keys(s, allowed, &at)?;
    Ok((s, at))
}

fn share(o: &Obj, key: &str, at: &str) -> Result<i64, String> {
    read::within(
        read::milli(o, key, at)?,
        0,
        MILLI,
        &format!("{at} {key}"),
        "0 to 1",
    )
}

fn pitch(o: &Obj, key: &str, at: &str) -> Result<i64, String> {
    read::within(
        read::milli(o, key, at)?,
        1,
        16 * MILLI,
        &format!("{at} {key}"),
        "more than 0 and at most 16",
    )
}

impl AudioTuning {
    /// The `audio` object of the tuning file. Other sections of the file
    /// belong to other systems and are not read here.
    pub fn parse(tuning_json: &str) -> Result<AudioTuning, String> {
        let all: Value = serde_json::from_str(tuning_json)
            .map_err(|e| format!("The tuning file could not be read: {e}"))?;
        let audio = all
            .get("audio")
            .ok_or_else(|| "The tuning file has no \"audio\" section.".to_string())?;
        let o = read::obj(audio, "tuning audio")?;
        read::keys(
            o,
            &["rival_motor", "own_motor", "line_tension", "ratchet", "pass_by"],
            "tuning audio",
        )?;

        let (r, at) = section(
            o,
            "rival_motor",
            &[
                "hear_seconds",
                "full_within_mm",
                "loudest",
                "curve",
                "sound_speed_mm_per_s",
                "doppler_scale",
                "lowest_pitch",
                "highest_pitch",
            ],
        )?;
        let curve = read::whole(r, "curve", &at)?;
        if !(1..=4).contains(&curve) {
            return Err(format!("{at} curve must be a whole number from 1 to 4."));
        }
        let rival_motor = RivalTuning {
            hear_seconds: read::within(
                read::milli(r, "hear_seconds", &at)?,
                1,
                60 * MILLI,
                &format!("{at} hear_seconds"),
                "more than 0 and at most 60",
            )?,
            full_within_mm: read::within(
                read::whole(r, "full_within_mm", &at)?,
                0,
                i64::MAX,
                &format!("{at} full_within_mm"),
                "0 or more",
            )?,
            loudest: share(r, "loudest", &at)?,
            curve: curve as u32,
            sound_speed_mm_per_s: read::within(
                read::whole(r, "sound_speed_mm_per_s", &at)?,
                1,
                i64::MAX,
                &format!("{at} sound_speed_mm_per_s"),
                "more than 0",
            )?,
            doppler_scale: read::within(
                read::milli(r, "doppler_scale", &at)?,
                0,
                20 * MILLI,
                &format!("{at} doppler_scale"),
                "0 to 20",
            )?,
            lowest_pitch: pitch(r, "lowest_pitch", &at)?,
            highest_pitch: pitch(r, "highest_pitch", &at)?,
        };
        if rival_motor.lowest_pitch > MILLI || rival_motor.highest_pitch < MILLI {
            return Err(format!(
                "{at} lowest_pitch must be at most 1 and highest_pitch at least 1."
            ));
        }

        let (m, at) = section(
            o,
            "own_motor",
            &[
                "idle_gain",
                "full_gain",
                "idle_pitch",
                "full_pitch",
                "throttle_share",
            ],
        )?;
        let own_motor = OwnTuning {
            idle_gain: share(m, "idle_gain", &at)?,
            full_gain: share(m, "full_gain", &at)?,
            idle_pitch: pitch(m, "idle_pitch", &at)?,
            full_pitch: pitch(m, "full_pitch", &at)?,
            throttle_share: share(m, "throttle_share", &at)?,
        };

        let (t, at) = section(
            o,
            "line_tension",
            &[
                "starts_at",
                "slow_ticks_per_s",
                "fast_ticks_per_s",
                "gain",
                "low_pitch",
                "high_pitch",
            ],
        )?;
        let rate = |key: &str| {
            read::within(
                read::milli(t, key, &at)?,
                1,
                200 * MILLI,
                &format!("{at} {key}"),
                "more than 0 and at most 200",
            )
        };
        let line_tension = TensionTuning {
            starts_at: read::within(
                read::milli(t, "starts_at", &at)?,
                0,
                MILLI - 1,
                &format!("{at} starts_at"),
                "0 to under 1",
            )?,
            slow_ticks_per_s: rate("slow_ticks_per_s")?,
            fast_ticks_per_s: rate("fast_ticks_per_s")?,
            gain: share(t, "gain", &at)?,
            low_pitch: pitch(t, "low_pitch", &at)?,
            high_pitch: pitch(t, "high_pitch", &at)?,
        };

        let (c, at) = section(
            o,
            "ratchet",
            &[
                "slow_clicks_per_s",
                "fast_clicks_per_s",
                "slow_pitch",
                "fast_pitch",
            ],
        )?;
        let ratchet = RatchetTuning {
            slow_clicks_per_s: read::within(
                read::milli(c, "slow_clicks_per_s", &at)?,
                1,
                200 * MILLI,
                &format!("{at} slow_clicks_per_s"),
                "more than 0 and at most 200",
            )?,
            fast_clicks_per_s: read::within(
                read::milli(c, "fast_clicks_per_s", &at)?,
                1,
                200 * MILLI,
                &format!("{at} fast_clicks_per_s"),
                "more than 0 and at most 200",
            )?,
            slow_pitch: pitch(c, "slow_pitch", &at)?,
            fast_pitch: pitch(c, "fast_pitch", &at)?,
        };

        let (p, at) = section(o, "pass_by", &["side_mm", "overrun"])?;
        let pass_by = PassByTuning {
            side_mm: read::within(
                read::whole(p, "side_mm", &at)?,
                0,
                i64::MAX,
                &format!("{at} side_mm"),
                "0 or more",
            )?,
            overrun: share(p, "overrun", &at)?,
        };

        Ok(AudioTuning {
            rival_motor,
            own_motor,
            line_tension,
            ratchet,
            pass_by,
        })
    }
}

/// A boat as the ear needs it: where it is and how it moves (no heading:
/// the brief rules out directional sound).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Body {
    /// mm, map space (x east, y north)
    pub pos: [i64; 2],
    /// mm/s
    pub vel: [i64; 2],
}

/// A voice's gain and pitch this frame, in thousandths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Heard {
    pub gain: i64,
    pub pitch: i64,
}

impl Heard {
    pub const SILENT: Heard = Heard {
        gain: 0,
        pitch: MILLI,
    };
}

/// `a + (b - a) * x`, `x` in thousandths.
fn lerp(a: i64, b: i64, x: i64) -> i64 {
    a + mul(b - a, x)
}

/// The distance a rival is first heard from: `hear_seconds` at
/// `top_speed_mm_per_s` (the boat's top speed without nitrous, from the
/// boat's own tuning).
pub fn hearing_mm(t: &RivalTuning, top_speed_mm_per_s: i64) -> i64 {
    top_speed_mm_per_s * t.hear_seconds / MILLI
}

/// A rival's motor heard from `you`: silent at or beyond the hearing
/// distance, rising as it nears to `loudest` within `full_within_mm`, and
/// pitched up while the gap closes and down while it opens (Doppler: the
/// rival's speed toward you raises the pitch as `c / (c - v)`, yours toward
/// it as `(c + v) / c`, each speed times `doppler_scale`).
pub fn rival_motor(t: &RivalTuning, top_speed_mm_per_s: i64, you: &Body, rival: &Body) -> Heard {
    let dx = rival.pos[0] - you.pos[0];
    let dy = rival.pos[1] - you.pos[1];
    let d = isqrt(dx * dx + dy * dy);
    let hear = hearing_mm(t, top_speed_mm_per_s);
    if d >= hear {
        return Heard::SILENT;
    }
    // the nearness, 0 at the hearing distance to 1000 at full
    let near = if d <= t.full_within_mm {
        MILLI
    } else {
        (hear - d) * MILLI / (hear - t.full_within_mm)
    };
    let mut rise = MILLI;
    for _ in 0..t.curve {
        rise = mul(rise, near);
    }
    let gain = mul(t.loudest, rise);

    let pitch = if d == 0 {
        MILLI
    } else {
        // speeds along the line between the boats, mm/s
        let rival_toward = -(dx * rival.vel[0] + dy * rival.vel[1]) / d;
        let you_toward = (dx * you.vel[0] + dy * you.vel[1]) / d;
        let c = t.sound_speed_mm_per_s;
        let src = mul(rival_toward, t.doppler_scale);
        let lis = mul(you_toward, t.doppler_scale);
        // a source at the speed of sound would divide by zero; the clamp
        // below makes any such pitch the highest anyway
        let below = (c - src).max(1);
        (c + lis).max(0) * MILLI / below
    };
    Heard {
        gain,
        pitch: pitch.clamp(t.lowest_pitch, t.highest_pitch),
    }
}

/// Your own motor. `throttle` is how hard it is pushed (0 to 1000: W or S
/// held), `speed` the boat's speed as thousandths of top speed (nitrous
/// may take it past 1000, and the pitch with it, up to twice).
pub fn own_motor(t: &OwnTuning, throttle: i64, speed: i64) -> Heard {
    let throttle = throttle.clamp(0, MILLI);
    let speed = speed.clamp(0, 2 * MILLI);
    let push = mul(speed, MILLI - t.throttle_share) + mul(throttle, t.throttle_share);
    Heard {
        gain: lerp(t.idle_gain, t.full_gain, push.min(MILLI)),
        pitch: lerp(t.idle_pitch, t.full_pitch, push),
    }
}

/// The line's drag over `dt_ms` at `tension` (0 to 1000, where 1000
/// snaps it): a tick to play, at its gain and pitch, or none. `drag` keeps
/// the time between ticks.
pub fn line_drag(t: &TensionTuning, drag: &mut Ratchet, dt_ms: i64, tension: i64) -> Option<Heard> {
    if tension <= t.starts_at {
        return None;
    }
    let x = ((tension - t.starts_at) * MILLI / (MILLI - t.starts_at)).min(MILLI);
    let as_ratchet = RatchetTuning {
        slow_clicks_per_s: t.slow_ticks_per_s,
        fast_clicks_per_s: t.fast_ticks_per_s,
        slow_pitch: t.low_pitch,
        fast_pitch: t.high_pitch,
    };
    // a tension just past the start still ticks (the ratchet is silent at 0)
    let pitch = drag.step(&as_ratchet, dt_ms, x.max(1))?;
    Some(Heard { gain: t.gain, pitch })
}

/// The reel's ratchet: call `step` every tick with how fast the player is
/// reeling (0 to 1000); it says when a click plays, and at what pitch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ratchet {
    /// thousandths of a click since the last one
    phase: i64,
}

impl Ratchet {
    pub fn step(&mut self, t: &RatchetTuning, dt_ms: i64, reel: i64) -> Option<i64> {
        if reel <= 0 {
            return None;
        }
        let reel = reel.min(MILLI);
        let per_s = lerp(t.slow_clicks_per_s, t.fast_clicks_per_s, reel);
        // clicks a second (thousandths) times seconds gives thousandths of
        // a click
        self.phase += per_s * dt_ms / MILLI;
        if self.phase < MILLI {
            return None;
        }
        self.phase %= MILLI;
        Some(lerp(t.slow_pitch, t.fast_pitch, reel))
    }
}
