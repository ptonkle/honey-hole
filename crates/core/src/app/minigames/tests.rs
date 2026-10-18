//! The Minigames screen, driven through `act` and `advance` as the page
//! drives it. Expected values are worked by hand from the sample tuning
//! (`tuning::tests::sample_json`: a 1 s banner, five turns of the reel)
//! and the shipped look (`data/ui2d.json`: the Fish-Boy's screen is 80% of
//! the view).

use serde_json::Value;

use super::*;
use crate::map::trig;

const THEME: &str = include_str!("../../../../../data/ui2d.json");
const SOUNDS: &str = include_str!("../../../../../data/sounds.json");
const TUNING: &str = include_str!("../../../../../data/tuning.json");

fn app() -> App {
    let notes = r#"[{"version": "0.1.0", "date": "2026-10-17", "notes": ["A."]}]"#;
    let mut a = App::new(&crate::strings::tests::complete(), notes).unwrap();
    a.act("data.ui2d", THEME).unwrap();
    a.act("data.tuning", &crate::tuning::tests::sample_json().to_string())
        .unwrap();
    a.load_sounds(SOUNDS, TUNING).unwrap();
    // a 1600 x 900 view at one pixel a CSS pixel
    a.resize(1600, 900, 100);
    a
}

fn ui(a: &App) -> Value {
    serde_json::to_value(a.ui()).unwrap()
}

fn ids(a: &App) -> Vec<String> {
    let mut out = Vec::new();
    for p in ui(a)["panels"].as_array().unwrap() {
        for item in p["items"].as_array().unwrap() {
            if let Some(id) = item["id"].as_str() {
                out.push(id.to_string());
            }
        }
    }
    out
}

/// Advances the clock by `ms`, a tick at a time.
fn wait(a: &mut App, ms: i64) {
    for _ in 0..ms / 20 {
        a.advance(a.clock + 20_000);
    }
}

fn played(a: &mut App) -> Vec<String> {
    a.sound_frame()["cues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap_or("").to_string())
        .collect()
}

#[test]
fn the_title_opens_the_minigames_screen_with_a_button_for_each() {
    let mut a = app();
    a.act("menu.minigames", "").unwrap();
    assert_eq!(a.screen, Screen::Minigames);
    assert_eq!(
        ids(&a),
        [
            "minigames.play.cloak",
            "minigames.play.power_reel",
            "minigames.play.nitrous",
            "minigames.play.teleporter",
            "minigames.play.camera",
            "minigames.play.enhanced_radar",
            "menu.back",
        ]
    );
    let v = ui(&a);
    assert_eq!(v["scene"], true, "the frame loop runs, to draw the 2D layer");
    assert_eq!(v["pointer"], false, "no minigame: the pointer is the page's");
    a.act("key", "Escape").unwrap();
    assert_eq!(a.screen, Screen::Title);
}

#[test]
fn a_minigame_needs_the_tuning_file() {
    let notes = r#"[{"version": "0.1.0", "date": "2026-10-17", "notes": ["A."]}]"#;
    let mut a = App::new(&crate::strings::tests::complete(), notes).unwrap();
    a.act("menu.minigames", "").unwrap();
    assert_eq!(
        a.act("minigames.play.power_reel", ""),
        Err("The tuning file has not loaded, so no minigame can start.".into())
    );
    assert_eq!(
        app().act("minigames.play.jump", ""),
        Err("There is no minigame \"jump\".".into())
    );
}

#[test]
fn a_started_minigame_takes_the_pointer_and_esc_quits_it() {
    let mut a = app();
    a.act("menu.minigames", "").unwrap();
    played(&mut a);
    a.act("minigames.play.nitrous", "").unwrap();
    let v = ui(&a);
    assert_eq!(v["pointer"], true);
    assert_eq!(
        v["panels"].as_array().unwrap().len(),
        0,
        "the minigame has the screen"
    );
    wait(&mut a, 200);
    assert_eq!(played(&mut a), ["minigame_start"]);
    a.act("key", "Escape").unwrap();
    a.advance(a.clock + 1000);
    assert_eq!(
        a.screen,
        Screen::Minigames,
        "Esc quit the minigame, not the screen"
    );
    assert!(a.minigames.game.is_none());
    assert_eq!(a.minigames.results.get(&Kind::Nitrous), Some(&State::Quit));
    let rows = &ui(&a)["panels"][0]["items"][6]["rows"];
    assert_eq!(rows[2][1], "MINIGAMES.QUIT");
    a.act("key", "Escape").unwrap();
    assert_eq!(a.screen, Screen::Title);
}

#[test]
fn the_pointer_lands_in_the_minigames_own_units() {
    let mut a = app();
    a.act("menu.minigames", "").unwrap();
    // the reel fills the 1600 x 900 view: one unit a pixel
    a.act("minigames.play.power_reel", "").unwrap();
    a.act("pointer.move", "800 450").unwrap();
    assert_eq!(a.minigames.pointer, Some((800, 450)));
    // the cloak sits on the Fish-Boy's screen, 80% of the view in the
    // middle: (160, 90) to (1440, 810)
    a.act("minigames.play.cloak", "").unwrap();
    a.act("pointer.move", "160 90").unwrap();
    assert_eq!(a.minigames.pointer, Some((0, 0)));
    a.act("pointer.move", "1440 810").unwrap();
    assert_eq!(a.minigames.pointer, Some((1600, 900)));
    // at twice the pixels to a CSS pixel, the same place is half the
    // CSS pixels
    a.resize(800, 450, 200);
    a.act("pointer.move", "400 225").unwrap();
    assert_eq!(a.minigames.pointer, Some((800, 450)));
}

#[test]
fn power_reel_is_played_through_by_pointer_moves() {
    let mut a = app();
    a.act("menu.minigames", "").unwrap();
    a.act("minigames.play.power_reel", "").unwrap();
    while a.minigames.game.as_ref().unwrap().in_banner() {
        wait(&mut a, 20);
    }
    // five turns at 24 moves a turn, one move a tick, round (800, 470)
    for k in 0..=5 * 24 {
        let ang = k * trig::TURN / 24;
        let (x, y) = (
            800 + 150 * trig::cos(ang) / trig::ONE,
            470 + 150 * trig::sin(ang) / trig::ONE,
        );
        a.act("pointer.move", &format!("{x} {y}")).unwrap();
        wait(&mut a, 20);
    }
    wait(&mut a, 40);
    assert!(a.minigames.game.is_none(), "the minigame ended");
    assert!(
        matches!(
            a.minigames.results.get(&Kind::PowerReel),
            Some(State::Done { .. })
        ),
        "{:?}",
        a.minigames.results
    );
    let cues = played(&mut a);
    assert_eq!(
        cues.iter().filter(|c| *c == "minigame_tick").count(),
        4,
        "a tick a turn but the last: {cues:?}"
    );
    assert_eq!(cues.last().map(String::as_str), Some("minigame_success"));
}

#[test]
fn the_camera_hides_the_pages_cursor_while_it_plays() {
    let mut a = app();
    a.act("menu.minigames", "").unwrap();
    a.act("minigames.play.camera", "").unwrap();
    assert_eq!(ui(&a)["hide_cursor"], true);
    a.act("key", "Escape").unwrap();
    a.advance(a.clock + 1000);
    assert_eq!(ui(&a)["hide_cursor"], false);
}

#[test]
fn every_cue_a_minigame_asks_for_is_a_shipped_sound() {
    let a = app();
    let bank = &a.sound.as_ref().unwrap().bank;
    for id in [
        crate::minigame::cue::START,
        crate::minigame::cue::TICK,
        crate::minigame::cue::MISS,
        crate::minigame::cue::SUCCESS,
        crate::minigame::cue::SHUTTER,
    ] {
        assert!(bank.cue(id).is_some(), "{id}");
    }
}

#[test]
fn the_2d_layer_draws_the_minigame_and_says_where() {
    let mut a = app();
    a.act("menu.minigames", "").unwrap();
    assert_eq!(a.build_ui2d().list.status, "mg=none");
    a.act("minigames.play.enhanced_radar", "").unwrap();
    let built = a.build_ui2d();
    assert_eq!(
        built.list.status,
        "mg=enhanced_radar phase=banner done=0 total=3 rect=160,90,1280,720"
    );
    assert!(built.list.strings.iter().any(|s| s == "MINIGAME.ENHANCED_RADAR"));
}
