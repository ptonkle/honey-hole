//! The screen flow, Escape, Back, the pause menu, Settings and What's New.
//! Expected screens and values are worked out by hand from the rules in
//! `app.rs`, not read back from the app.

use super::*;
use serde_json::{json, Value};

const NOTES: &str = r#"[
    {"version": "0.1.1", "date": "2026-10-18", "notes": ["A fix."]},
    {"version": "0.1.0", "date": "2026-10-17", "notes": ["The menus."]}
]"#;

fn app() -> App {
    App::new(&crate::strings::tests::complete(), NOTES).unwrap()
}

fn ui(a: &App) -> Value {
    serde_json::to_value(a.ui()).unwrap()
}

/// Every action id the screen's controls send, in order.
fn ids(a: &App) -> Vec<String> {
    let v = ui(a);
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

fn kinds(a: &App) -> Vec<String> {
    ui(a)["panels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["kind"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn the_app_opens_on_the_title_with_its_menu() {
    let a = app();
    assert_eq!(a.screen, Screen::Title);
    assert_eq!(
        ids(&a),
        ["menu.play", "menu.settings", "menu.notes", "menu.gallery", "menu.lake"]
    );
    let v = ui(&a);
    assert_eq!(v["screen"], "title");
    assert_eq!(v["version"], "v0.1.0");
    assert_eq!(v["panels"][0]["heading"], "TITLE");
    assert_eq!(
        v["panels"][2]["items"][0]["rows"],
        json!([["CONTROLS.PAUSE", "CONTROLS.PAUSE_KEYS"]])
    );
}

#[test]
fn each_title_button_opens_its_screen() {
    for (id, screen) in [
        ("menu.play", Screen::Play),
        ("menu.settings", Screen::Settings),
        ("menu.notes", Screen::Notes),
    ] {
        let mut a = app();
        a.act(id, "").unwrap();
        assert_eq!(a.screen, screen, "{id}");
    }
}

#[test]
fn every_sub_screen_ends_with_back_and_back_goes_to_the_title() {
    for open in ["menu.settings", "menu.notes"] {
        let mut a = app();
        a.act(open, "").unwrap();
        assert_eq!(ids(&a).last().map(String::as_str), Some("menu.back"), "{open}");
        a.act("menu.back", "").unwrap();
        assert_eq!(a.screen, Screen::Title, "{open}");
    }
}

#[test]
fn escape_on_a_sub_screen_goes_back_to_the_title() {
    for open in ["menu.settings", "menu.notes"] {
        let mut a = app();
        a.act(open, "").unwrap();
        a.act("key", "Escape").unwrap();
        assert_eq!(a.screen, Screen::Title, "{open}");
    }
}

#[test]
fn escape_on_the_title_and_other_keys_change_nothing() {
    let mut a = app();
    a.act("key", "Escape").unwrap();
    assert_eq!(a.screen, Screen::Title);
    a.act("menu.play", "").unwrap();
    a.act("key", "KeyQ").unwrap();
    assert_eq!((a.screen, a.paused), (Screen::Play, false));
}

#[test]
fn escape_in_play_opens_and_closes_the_pause_menu() {
    let mut a = app();
    a.act("menu.play", "").unwrap();
    assert_eq!(kinds(&a), ["play"]);
    a.act("key", "Escape").unwrap();
    assert_eq!((a.screen, a.paused), (Screen::Play, true));
    assert_eq!(kinds(&a), ["play", "pause"]);
    assert_eq!(ids(&a), ["menu.pause", "menu.resume", "menu.quit"]);
    a.act("key", "Escape").unwrap();
    assert_eq!((a.screen, a.paused), (Screen::Play, false));
    assert_eq!(kinds(&a), ["play"]);
}

#[test]
fn the_pause_button_resume_and_quit() {
    let mut a = app();
    a.act("menu.play", "").unwrap();
    a.act("menu.pause", "").unwrap();
    assert!(a.paused);
    a.act("menu.resume", "").unwrap();
    assert!(!a.paused);
    a.act("menu.pause", "").unwrap();
    a.act("menu.quit", "").unwrap();
    assert_eq!(a.screen, Screen::Title);
    // Play opens again with the pause menu closed
    a.act("menu.play", "").unwrap();
    assert!(!a.paused);
}

#[test]
fn the_pause_button_does_nothing_off_play() {
    let mut a = app();
    a.act("menu.pause", "").unwrap();
    assert!(!a.paused);
}

#[test]
fn the_version_opens_whats_new_from_the_menus_only() {
    let mut a = app();
    assert!(a.ui().version_opens);
    a.act("version", "").unwrap();
    assert_eq!(a.screen, Screen::Notes);
    a.act("menu.back", "").unwrap();
    a.act("menu.play", "").unwrap();
    assert!(!a.ui().version_opens);
    a.act("version", "").unwrap();
    assert_eq!(a.screen, Screen::Play);
}

#[test]
fn the_volume_slider_keeps_to_its_range_and_steps() {
    let mut a = app();
    a.act("menu.settings", "").unwrap();
    assert_eq!(a.settings.volume, 80);
    a.act("settings.volume", "35").unwrap();
    assert_eq!(a.settings.volume, 35);
    a.act("settings.volume", "37").unwrap();
    assert_eq!(a.settings.volume, 35);
    a.act("settings.volume", "250").unwrap();
    assert_eq!(a.settings.volume, 100);
    assert_eq!(ui(&a)["panels"][0]["items"][0]["text"], "100%");
    assert_eq!(ui(&a)["volume"], 100);
    assert_eq!(
        a.act("settings.volume", "loud"),
        Err("The volume \"loud\" is not a whole number.".into())
    );
    assert_eq!(a.settings.volume, 100);
}

#[test]
fn the_text_size_select_takes_a_listed_size_only() {
    let mut a = app();
    a.act("menu.settings", "").unwrap();
    let v = ui(&a);
    let select = &v["panels"][0]["items"][1];
    assert_eq!(select["value"], "medium");
    assert_eq!(
        select["options"][2],
        json!({ "value": "large", "text": "SETTINGS.TEXT_SIZE.LARGE" })
    );
    a.act("settings.text_size", "large").unwrap();
    assert_eq!(ui(&a)["text_size"], "large");
    assert_eq!(
        a.act("settings.text_size", "huge"),
        Err("There is no text size \"huge\".".into())
    );
    assert_eq!(a.settings.text_size, TextSize::Large);
}

#[test]
fn settings_outlast_a_trip_to_the_title() {
    let mut a = app();
    a.act("settings.volume", "10").unwrap();
    a.act("menu.back", "").unwrap();
    a.act("menu.settings", "").unwrap();
    assert_eq!(ui(&a)["panels"][0]["items"][0]["value"], 10);
}

#[test]
fn whats_new_lists_major_versions_until_minor_ones_are_ticked_in() {
    let mut a = app();
    a.act("menu.notes", "").unwrap();
    let versions = |a: &App| -> Vec<String> {
        ui(a)["panels"][0]["items"][1]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["version"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(versions(&a), ["v0.1.0"]);
    a.act("notes.minor", "on").unwrap();
    assert_eq!(versions(&a), ["v0.1.1", "v0.1.0"]);
    assert_eq!(ui(&a)["panels"][0]["items"][0]["value"], true);
    a.act("notes.minor", "off").unwrap();
    assert_eq!(versions(&a), ["v0.1.0"]);
}

#[test]
fn an_unknown_action_is_refused_by_name() {
    let mut a = app();
    assert_eq!(
        a.act("menu.jump", ""),
        Err("Unknown action \"menu.jump\".".into())
    );
    assert_eq!(
        a.act("settings.speed", "1"),
        Err("Unknown action \"settings.speed\".".into())
    );
}
