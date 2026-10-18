//! The sound engine's rules. Expected values are worked out by hand from
//! the tuning below, not read back from the functions under test.

use serde_json::{json, Value};

use super::events::Table;
use super::motor::*;
use super::recipe::Params;
use super::*;

/// Tuning with round numbers: heard from 5 s at top speed, full within
/// 8 m, rising evenly (curve 1) to a gain of 1, true Doppler (scale 1).
pub(crate) const TUNING: &str = r#"{
  "boats": {"ignored": "by the sound engine"},
  "audio": {
    "rival_motor": {"hear_seconds": 5, "full_within_mm": 8000, "loudest": 1, "curve": 1,
      "sound_speed_mm_per_s": 343000, "doppler_scale": 1, "lowest_pitch": 0.5, "highest_pitch": 2},
    "own_motor": {"idle_gain": 0.2, "full_gain": 0.6, "idle_pitch": 1, "full_pitch": 3, "throttle_share": 0.5},
    "line_tension": {"starts_at": 0.5, "loudest": 0.4, "low_pitch": 1, "high_pitch": 2},
    "ratchet": {"slow_clicks_per_s": 10, "fast_clicks_per_s": 50, "slow_pitch": 1, "fast_pitch": 1.5},
    "pass_by": {"side_mm": 5000, "overrun": 0.25}
  }
}"#;

/// A small sounds file: a beep, a hum that stretches, an alias, a motor
/// loop, and events for each kind of row.
pub(crate) const SOUNDS: &str = r#"{
  "_note": "test",
  "noise": {"samples": 1024, "seed": 3},
  "smooth_ms": 50, "fade_ms": 200, "max_pending": 4, "click": "click",
  "cues": [
    {"id": "click", "label": "Click", "layers": [{"wave": "sine", "freq": 1000, "attack_ms": 1, "release_ms": 9, "gain": 0.2}]},
    {"id": "beep", "label": "Beep", "layers": [
      {"wave": "sine", "freq": 440, "attack_ms": 10, "hold_ms": 20, "release_ms": 30, "gain": 0.5}]},
    {"id": "hum", "label": "Hum", "layers": [
      {"wave": "triangle", "freq": 110, "attack_ms": 100, "hold_ms": 500, "release_ms": 200, "gain": 0.2, "stretch": true}]},
    {"id": "catch_common", "label": "Catch", "layers": [{"wave": "square", "freq": 500, "release_ms": 50, "gain": 0.1}]},
    {"id": "catch_legendary", "label": "Big catch", "layers": [{"wave": "square", "freq": 600, "release_ms": 50, "gain": 0.1}]},
    {"id": "boom", "same_as": "beep"}
  ],
  "loops": [
    {"id": "rival_motor", "label": "Motor", "layers": [
      {"wave": "sawtooth", "freq": 50, "gain": 0.5, "filter": {"type": "lowpass", "freq": 600}, "throb": {"rate": 10, "depth": 0.4}},
      {"wave": "noise", "gain": 0.1}]},
    {"id": "own_motor", "label": "Yours", "layers": [{"wave": "sine", "freq": 100, "gain": 0.5}]},
    {"id": "line_tension", "label": "Whine", "layers": [{"wave": "sine", "freq": 1000, "gain": 0.5}]}
  ],
  "events": {
    "Hooked": [{"cue": "beep", "who": "mine"}, {"cue": "click", "who": "others"}],
    "Caught": [{"cue": "catch_{tier}", "who": "mine", "each": {"tier": ["common", "legendary"]}}],
    "Pinged": [{"cue": "hum", "duration_field": "sweep_ms"}],
    "Wrecked": [{"cue": "boom"}, {"cue": "click", "delay_ms": 300, "gain": 0.5}]
  }
}"#;

fn bank() -> Bank {
    Bank::parse(SOUNDS, TUNING).unwrap()
}

fn tuning() -> AudioTuning {
    AudioTuning::parse(TUNING).unwrap()
}

const TOP: i64 = 16_000;

fn at(x: i64, vx: i64) -> Body {
    Body {
        pos: [x, 0],
        vel: [vx, 0],
    }
}

// ---- a rival's motor ----

#[test]
fn a_rival_motor_is_heard_from_5_s_away() {
    let t = tuning().rival_motor;
    let you = Body::default();
    // 5 s at 16 m/s is 80 m
    assert_eq!(rival_motor(&t, TOP, &you, &at(81_000, -TOP)).gain, 0);
    assert_eq!(rival_motor(&t, TOP, &you, &at(80_000, -TOP)).gain, 0);
    assert!(rival_motor(&t, TOP, &you, &at(79_000, -TOP)).gain > 0);
    // a slower top speed is heard from less far: 5 s at 10 m/s is 50 m
    assert_eq!(rival_motor(&t, 10_000, &you, &at(60_000, -TOP)).gain, 0);
}

#[test]
fn a_rival_motor_grows_louder_as_it_nears() {
    let t = tuning().rival_motor;
    let you = Body::default();
    // evenly from 80 m (silent) to 8 m (full): 44 m is halfway, 62 m a quarter
    assert_eq!(rival_motor(&t, TOP, &you, &at(44_000, -TOP)).gain, 500);
    assert_eq!(rival_motor(&t, TOP, &you, &at(62_000, -TOP)).gain, 250);
    assert_eq!(rival_motor(&t, TOP, &you, &at(8_000, -TOP)).gain, 1000);
    assert_eq!(rival_motor(&t, TOP, &you, &at(1_000, -TOP)).gain, 1000);
    let mut last = 0;
    for x in (2..20).rev().map(|k| k * 4_000) {
        let g = rival_motor(&t, TOP, &you, &at(x, -TOP)).gain;
        assert!(g > last, "{x} mm: {g} after {last}");
        last = g;
    }
    // curve 2: a quarter of the way in is a sixteenth as loud
    let mut t2 = t.clone();
    t2.curve = 2;
    assert_eq!(rival_motor(&t2, TOP, &you, &at(62_000, -TOP)).gain, 62);
}

#[test]
fn doppler_raises_pitch_when_closing_and_lowers_it_when_leaving() {
    let t = tuning().rival_motor;
    let you = Body::default();
    // the rival at 16 m/s toward you: 343 / (343 - 16) = 1.0489
    assert_eq!(rival_motor(&t, TOP, &you, &at(40_000, -TOP)).pitch, 1048);
    // away from you: 343 / (343 + 16) = 0.9554
    assert_eq!(rival_motor(&t, TOP, &you, &at(40_000, TOP)).pitch, 955);
    // you at 16 m/s toward a still rival: (343 + 16) / 343 = 1.0466
    assert_eq!(rival_motor(&t, TOP, &at(0, TOP), &at(40_000, 0)).pitch, 1046);
    // passing to the side: no closing speed, no shift
    let side = Body {
        pos: [0, 5_000],
        vel: [TOP, 0],
    };
    assert_eq!(rival_motor(&t, TOP, &you, &side).pitch, 1000);
    // scale 3 triples each speed: 343 / (343 - 48) = 1.1627
    let mut t3 = t.clone();
    t3.doppler_scale = 3000;
    assert_eq!(rival_motor(&t3, TOP, &you, &at(40_000, -TOP)).pitch, 1162);
    // the clamp: a rival near the speed of sound is held at the highest
    assert_eq!(rival_motor(&t, TOP, &you, &at(40_000, -340_000)).pitch, 2000);
}

// ---- your motor, the line, the reel ----

#[test]
fn your_motor_rises_from_idle_to_full() {
    let t = tuning().own_motor;
    assert_eq!(
        own_motor(&t, 0, 0),
        Heard {
            gain: 200,
            pitch: 1000
        }
    );
    assert_eq!(
        own_motor(&t, 1000, 1000),
        Heard {
            gain: 600,
            pitch: 3000
        }
    );
    // the throttle alone gives its share, half the way up
    assert_eq!(
        own_motor(&t, 1000, 0),
        Heard {
            gain: 400,
            pitch: 2000
        }
    );
    assert_eq!(
        own_motor(&t, 500, 0),
        Heard {
            gain: 300,
            pitch: 1500
        }
    );
}

#[test]
fn the_line_whines_only_above_its_starting_tension() {
    let t = tuning().line_tension;
    assert_eq!(line_tension(&t, 400).gain, 0);
    assert_eq!(line_tension(&t, 500).gain, 0);
    assert_eq!(
        line_tension(&t, 750),
        Heard {
            gain: 200,
            pitch: 1500
        }
    );
    assert_eq!(
        line_tension(&t, 1000),
        Heard {
            gain: 400,
            pitch: 2000
        }
    );
}

#[test]
fn the_ratchet_clicks_faster_as_you_reel_faster() {
    let t = tuning().ratchet;
    let count = |reel: i64| {
        let mut r = Ratchet::default();
        // one second of 20 ms ticks
        (0..50).filter_map(|_| r.step(&t, 20, reel)).count()
    };
    assert_eq!(count(0), 0);
    assert_eq!(count(1000), 50);
    assert_eq!(count(500), 30);
    let mut r = Ratchet::default();
    let pitch = (0..50).find_map(|_| r.step(&t, 20, 1000));
    assert_eq!(pitch, Some(1500));
}

// ---- recipes ----

/// A JSON value from text (the core's source holds no decimal literals).
fn j(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn layer(cue: &Value) -> &Value {
    &cue["layers"][0]
}

#[test]
fn a_cue_compiles_to_a_schedule_with_its_gain_pitch_and_delay() {
    let b = bank();
    let p = Params {
        gain: 500,
        pitch: 2000,
        delay_ms: 100,
        duration_ms: None,
    };
    let c = b.cue("beep").unwrap().compile(&p);
    let l = layer(&c);
    assert_eq!(l["wave"], "sine");
    assert_eq!(l["start"], j("0.1"));
    assert_eq!(l["stop"], j("0.16"));
    // 440 Hz an octave up
    assert_eq!(l["freq"], j(r#"[{"m": "setValueAtTime", "t": 0.1, "v": 880.0}]"#));
    // gain 0.5 at half: 0.25, up over 10 ms, held 20, down over 30
    assert_eq!(
        l["gain"],
        j(r#"[
            {"m": "setValueAtTime", "t": 0.1, "v": 0.0},
            {"m": "linearRampToValueAtTime", "t": 0.11, "v": 0.25},
            {"m": "linearRampToValueAtTime", "t": 0.13, "v": 0.25},
            {"m": "linearRampToValueAtTime", "t": 0.16, "v": 0.0}
        ]"#)
    );
}

#[test]
fn a_stretch_layer_lasts_as_long_as_the_game_asks() {
    let b = bank();
    let hum = b.cue("hum").unwrap();
    assert_eq!(layer(&hum.compile(&Params::default()))["stop"], j("0.8"));
    let p = Params {
        duration_ms: Some(3000),
        ..Params::default()
    };
    assert_eq!(layer(&hum.compile(&p))["stop"], j("3.0"));
}

#[test]
fn notes_and_repeats_lay_out_copies_of_a_layer() {
    let sounds = SOUNDS.replace(
        r#""attack_ms": 10, "hold_ms": 20"#,
        r#""to_freq": 220, "notes": [[0, 440], [100, 880]], "repeat": {"times": 2, "every_ms": 500}, "attack_ms": 10, "hold_ms": 20"#,
    );
    assert_ne!(sounds, SOUNDS);
    let b = Bank::parse(&sounds, TUNING).unwrap();
    let layers = &b.cue("beep").unwrap().layers;
    let got: Vec<(i64, i64, Option<i64>)> = layers.iter().map(|l| (l.at_ms, l.freq, l.to_freq)).collect();
    assert_eq!(
        got,
        [
            (0, 440_000, Some(220_000)),
            (100, 880_000, Some(440_000)),
            (500, 440_000, Some(220_000)),
            (600, 880_000, Some(440_000))
        ]
    );
}

#[test]
fn a_loop_splits_its_throb_from_its_steady_gain() {
    let b = bank();
    let layers = b.find_loop("rival_motor").unwrap().compile(500, 2000);
    // gain 0.5 at half is 0.25; a 0.4 throb beats 0.05 either side of 0.2
    assert_eq!(layers[0]["gain"], j("0.2"));
    assert_eq!(layers[0]["throb"], j(r#"{"rate": 20.0, "amount": 0.05}"#));
    assert_eq!(layers[0]["freq"], j("100.0"));
    assert_eq!(layers[0]["filter"]["freq"], j("1200.0"));
    assert_eq!(layers[1]["wave"], "noise");
    assert!(layers[1].get("freq").is_none());
}

#[test]
fn a_bad_recipe_is_refused_by_name() {
    let cases = [
        (
            r#""gain": 0.5}"#,
            r#""gian": 0.5}"#,
            "Cue beep layer 1 has a key \"gian\"",
        ),
        (
            r#""wave": "sine", "freq": 440"#,
            r#""wave": "saw", "freq": 440"#,
            "Cue beep layer 1 has wave \"saw\"",
        ),
        (
            r#""wave": "sine", "freq": 440"#,
            r#""wave": "noise", "freq": 440"#,
            "Cue beep layer 1 is noise",
        ),
        (
            r#""release_ms": 30, "gain": 0.5"#,
            r#""release_ms": 30, "gain": 1.5"#,
            "Cue beep layer 1 gain",
        ),
        (
            r#""same_as": "beep""#,
            r#""same_as": "bep""#,
            "Cue boom is the same as \"bep\"",
        ),
        (
            r#""cue": "hum""#,
            r#""cue": "humm""#,
            "Event Pinged plays \"humm\"",
        ),
        (
            r#""each": {"tier""#,
            r#""each": {"size""#,
            "Event Caught entry 1 cue \"catch_{tier}\" names {tier}",
        ),
        (
            r#""click": "click""#,
            r#""click": "clack""#,
            "The sounds file click \"clack\"",
        ),
        (
            r#""id": "own_motor""#,
            r#""id": "my_motor""#,
            "The sounds file has no loop \"own_motor\"",
        ),
    ];
    for (from, to, says) in cases {
        let bad = SOUNDS.replacen(from, to, 1);
        assert_ne!(bad, SOUNDS, "{from}");
        let e = Bank::parse(&bad, TUNING).unwrap_err();
        assert!(e.starts_with(says), "{e}");
    }
    let e = Bank::parse(SOUNDS, r#"{"boats": {}}"#).unwrap_err();
    assert_eq!(e, "The tuning file has no \"audio\" section.");
    let e = Bank::parse(SOUNDS, &TUNING.replace("\"curve\": 1", "\"curve\": 9")).unwrap_err();
    assert!(e.contains("curve"), "{e}");
}

// ---- events ----

fn cues(t: &Table, event: Value, listener: Option<i64>) -> Vec<String> {
    t.plays(&event, listener).into_iter().map(|(c, _)| c).collect()
}

#[test]
fn an_event_plays_its_cues_for_the_right_listener() {
    let t = bank().events;
    let hooked = json!({"Hooked": {"seat": 1, "fish": "Walleye"}});
    assert_eq!(cues(&t, hooked.clone(), Some(1)), ["beep"]);
    assert_eq!(cues(&t, hooked.clone(), Some(2)), ["click"]);
    // the same event, internally tagged
    assert_eq!(cues(&t, json!({"kind": "Hooked", "seat": 2}), Some(2)), ["beep"]);
    // a field fills the cue id, spelled as the id is
    let caught = json!({"Caught": {"seat": 0, "tier": "Legendary"}});
    assert_eq!(cues(&t, caught, Some(0)), ["catch_legendary"]);
    let odd = json!({"Caught": {"seat": 0, "tier": "Mythic"}});
    assert!(cues(&t, odd, Some(0)).is_empty());
    // a unit event, and one with no row
    assert!(cues(&t, json!("Wrecked"), None).len() == 2);
    assert!(cues(&t, json!({"Teleported": {"seat": 0}}), Some(0)).is_empty());
    assert!(cues(&t, json!("SomethingNew"), Some(0)).is_empty());
}

#[test]
fn an_event_sets_a_cue_s_delay_gain_and_length() {
    let t = bank().events;
    let wreck = t.plays(&json!({"Wrecked": {"seat": 3}}), Some(0));
    assert_eq!(wreck[1].1.delay_ms, 300);
    assert_eq!(wreck[1].1.gain, 500);
    let ping = t.plays(&json!({"Pinged": {"seat": 0, "sweep_ms": 3000}}), Some(0));
    assert_eq!(ping[0].1.duration_ms, Some(3000));
}

#[test]
fn every_placeholder_value_is_checked_against_the_cues() {
    let t = bank().events;
    let caught = &t.rows["Caught"][0];
    assert_eq!(caught.every_cue(), ["catch_common", "catch_legendary"]);
}

// ---- the frame ----

#[test]
fn a_frame_takes_the_waiting_cues_and_carries_the_volume() {
    let mut s = Sound::new(bank());
    assert!(s.play("beep", &Params::default()));
    assert!(s.play("boom", &Params::default()));
    assert!(!s.play("nothing", &Params::default()));
    let f = s.frame(80, Vec::new());
    assert_eq!(f["master"], j("0.8"));
    assert_eq!(f["smooth"], j("0.05"));
    assert_eq!(f["fade"], j("0.2"));
    let ids: Vec<&str> = f["cues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["beep", "beep"]);
    assert_eq!(s.frame(80, Vec::new())["cues"], json!([]));
}

#[test]
fn waiting_cues_beyond_the_limit_drop_the_oldest() {
    let mut s = Sound::new(bank());
    for id in ["click", "beep", "hum", "beep", "catch_common"] {
        s.play(id, &Params::default());
    }
    let ids: Vec<&str> = s.pending().iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["beep", "hum", "beep", "catch_common"]);
}

#[test]
fn a_voice_has_no_pan_and_a_silent_one_is_left_out() {
    let s = Sound::new(bank());
    let v = s
        .voice(
            "rival.1",
            "rival_motor",
            Heard {
                gain: 1000,
                pitch: 1000,
            },
        )
        .unwrap();
    assert_eq!(v["key"], "rival.1");
    assert!(!v.to_string().contains("pan"));
    assert!(s.voice("rival.1", "rival_motor", Heard::SILENT).is_none());
    assert!(s
        .voice(
            "rival.1",
            "nothing",
            Heard {
                gain: 1000,
                pitch: 1000
            }
        )
        .is_none());
}

#[test]
fn the_noise_is_the_same_every_time_and_within_one() {
    let b = bank();
    let n = b.noise();
    assert_eq!(n.len(), 1024);
    assert_eq!(n, b.noise());
    let one = crate::gfx::fx::ONE;
    assert!(n.iter().all(|v| (-one..=one).contains(v)));
    assert!(n.iter().any(|v| *v < 0) && n.iter().any(|v| *v > 0));
}
