//! The Sounds screen and what the page plays. Expected values are worked
//! out by hand from the test tuning (`sound::tests::TUNING`): heard from
//! 5 s at the map's 16 m/s top speed (80 m), full within 8 m, rising
//! evenly, true Doppler; the pass-by runs 25% beyond hearing (100 m each
//! way) 5 m to your side.

use serde_json::Value;

use super::*;
use crate::sound::tests::{SOUNDS, TUNING};

fn app() -> App {
    let notes = r#"[{"version": "0.1.0", "date": "2026-10-17", "notes": ["A."]}]"#;
    let mut a = App::new(&crate::strings::tests::complete(), notes).unwrap();
    a.load_sounds(SOUNDS, TUNING).unwrap();
    a
}

fn j(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn ids(a: &App) -> Vec<String> {
    let v = serde_json::to_value(a.ui()).unwrap();
    let mut out = Vec::new();
    for p in v["panels"].as_array().unwrap() {
        for item in p["items"].as_array().unwrap() {
            if let Some(id) = item["id"].as_str() {
                out.push(id.to_string());
            }
        }
    }
    out
}

/// The ids of the cues the next frame plays.
fn played(a: &mut App) -> Vec<String> {
    a.sound_frame()["cues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap().to_string())
        .collect()
}

fn voices(a: &mut App) -> Vec<Value> {
    a.sound_frame()["voices"].as_array().unwrap().clone()
}

#[test]
fn the_gallery_leads_to_sounds_which_lists_every_cue_then_back() {
    let mut a = app();
    a.act("menu.gallery", "").unwrap();
    assert!(ids(&a).contains(&"menu.sounds".to_string()));
    a.act("menu.sounds", "").unwrap();
    assert_eq!(a.screen, Screen::Sounds);
    let ids = ids(&a);
    // the six cues in file order; the alias is not a sound of its own
    assert_eq!(
        ids[..6],
        [
            "sounds.play.click",
            "sounds.play.beep",
            "sounds.play.hum",
            "sounds.play.line_drag",
            "sounds.play.catch_common",
            "sounds.play.catch_legendary"
        ]
    );
    assert_eq!(ids.last().map(String::as_str), Some("menu.back"));
    a.act("key", "Escape").unwrap();
    assert_eq!(a.screen, Screen::Title);
}

#[test]
fn a_sound_button_plays_its_cue_once_and_a_menu_button_clicks() {
    let mut a = app();
    a.act("menu.sounds", "").unwrap();
    assert_eq!(played(&mut a), ["click"]);
    a.act("sounds.play.beep", "").unwrap();
    assert_eq!(played(&mut a), ["beep"]);
    assert!(played(&mut a).is_empty());
    assert_eq!(
        a.act("sounds.play.nothing", ""),
        Err("There is no sound \"nothing\".".into())
    );
}

#[test]
fn the_volume_setting_is_the_master_volume() {
    let mut a = app();
    a.act("settings.volume", "40").unwrap();
    assert_eq!(a.sound_frame()["master"], j("0.4"));
}

#[test]
fn no_sound_plays_before_the_sounds_load() {
    let notes = r#"[{"version": "0.1.0", "date": "2026-10-17", "notes": ["A."]}]"#;
    let mut a = App::new(&crate::strings::tests::complete(), notes).unwrap();
    a.act("menu.gallery", "").unwrap();
    a.act("menu.sounds", "").unwrap();
    assert!(a.sound_frame().is_null());
    assert_eq!(ids(&a), ["menu.back"]);
}

#[test]
fn the_other_boat_drives_past_at_top_speed_louder_then_quieter_higher_then_lower() {
    let mut a = app();
    a.act("menu.sounds", "").unwrap();
    a.sound_frame();
    a.act("sounds.drive_past", "").unwrap();
    // at the start it is 100 m off: too far to hear
    assert!(voices(&mut a).is_empty());
    // after 3 s, 52 m before you (52.24 m away), closing at 15.93 m/s:
    // pitch 343 / (343 - 15.93) = 1.048, so the 50 Hz motor plays 52.4 Hz
    a.advance(3_000_000);
    let v = voices(&mut a);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0]["key"], "rival.debug");
    assert_eq!(v[0]["layers"][0]["freq"], j("52.4"));
    // gain (80 - 52.24) / 72 = 0.385 of the layer's 0.5, less a fifth for
    // the throb's middle: 0.192 -> 0.154 steady
    assert_eq!(v[0]["layers"][0]["gain"], j("0.154"));
    // after 6.25 s it is beside you: full gain (0.5, steady 0.4), no shift
    a.advance(6_250_000);
    let v = voices(&mut a);
    assert_eq!(v[0]["layers"][0]["freq"], j("50.0"));
    assert_eq!(v[0]["layers"][0]["gain"], j("0.4"));
    // after 9.5 s, 52 m past you and leaving: 343 / (343 + 15.93) = 0.955
    a.advance(9_500_000);
    assert_eq!(voices(&mut a)[0]["layers"][0]["freq"], j("47.75"));
    // after 12.5 s it has driven the whole 200 m line and stops
    a.advance(12_600_000);
    assert!(voices(&mut a).is_empty());
    assert_eq!(a.sounds.pass_from, None);
}

#[test]
fn the_slider_puts_the_other_boat_on_its_line() {
    let mut a = app();
    a.act("menu.sounds", "").unwrap();
    a.act("sounds.rival", "on").unwrap();
    a.act("sounds.rival_at", "0").unwrap();
    assert!(voices(&mut a).is_empty());
    // halfway along is beside you
    a.act("sounds.rival_at", "50").unwrap();
    assert_eq!(voices(&mut a)[0]["layers"][0]["gain"], j("0.4"));
    // the slider keeps to its 5% steps
    a.act("sounds.rival_at", "52").unwrap();
    assert_eq!(a.sounds.rival_at, 50);
    // leaving the screen stops it
    a.act("menu.back", "").unwrap();
    assert!(voices(&mut a).is_empty());
}

#[test]
fn your_motor_and_the_line_play_at_their_sliders() {
    let mut a = app();
    a.act("menu.sounds", "").unwrap();
    a.act("sounds.own", "on").unwrap();
    a.act("sounds.throttle", "100").unwrap();
    a.act("sounds.tension", "on").unwrap();
    a.act("sounds.tension_level", "75").unwrap();
    let v = voices(&mut a);
    // your motor at full: pitch 3, gain 0.6 of its 0.5 layer
    assert_eq!(v[0]["key"], "own");
    assert_eq!(v[0]["layers"][0]["freq"], j("300.0"));
    assert_eq!(v[0]["layers"][0]["gain"], j("0.3"));
    // the line is no loop: it ticks, at 0.75 halfway from its start, 12 a
    // second (4 to 20)
    assert_eq!(v.len(), 1);
    let mut drag = 0;
    for _ in 0..50 {
        a.advance(a.clock + 20_000);
        drag += played(&mut a).iter().filter(|id| *id == "line_drag").count();
    }
    assert_eq!(drag, 12);
}
