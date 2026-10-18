//! The Lobby and Play: the data files, the setup feeding the match, keys
//! and the mouse reaching it, and ticks following the page's clock.

use super::*;
use crate::sim::Mode;

const NOTES: &str = r#"[{"version": "0.1.0", "date": "2026-10-17", "notes": ["A."]}]"#;

const CONTROLS: &str = r#"{
    "bindings": {"forward": ["KeyW"], "back": ["KeyS"], "left": ["KeyA"], "right": ["KeyD"],
                 "mode": ["KeyQ"], "cancel": ["Escape"], "click": ["Mouse0"]},
    "mouse": {"yaw_deg_per_100px": 25, "pitch_deg_per_100px": 18,
              "pitch_min_deg": -10, "pitch_max_deg": 60, "pitch_start_deg": 15}
}"#;

const MAPS: &str = r#"{"setting": "michigan-lake", "from": 0, "scanned": 4,
    "maps": [{"seed": 0, "stamp": "-"}, {"seed": 3, "stamp": "-"}]}"#;

const NAMES: &str = r#"{"ai": ["Gil Waters", "Rod Fisher", "Bob Pike", "Wade Rivers"]}"#;

/// The app with every match file loaded, on the Lobby.
pub(crate) fn lobby() -> App {
    let mut a = App::new(&crate::strings::tests::complete(), NOTES).unwrap();
    a.act("data.tuning", &crate::tuning::tests::sample_json().to_string())
        .unwrap();
    a.act("data.controls", CONTROLS).unwrap();
    a.act("data.maps", MAPS).unwrap();
    a.act("data.names", NAMES).unwrap();
    a.act("menu.play", "").unwrap();
    a.act("menu.offline", "").unwrap();
    a
}

/// A match started from the Lobby with its defaults, its clock at 0.
pub(crate) fn started() -> App {
    let mut a = lobby();
    a.act("lobby.start", "").unwrap();
    a.advance(0);
    a
}

/// One tick of page time (20 ms).
pub(crate) fn tick(a: &mut App) {
    a.advance(a.clock + 20_000);
}

/// A key pressed and let go, a tick each.
pub(crate) fn key(a: &mut App, code: &str) {
    a.act("key", code).unwrap();
    tick(a);
    a.act("key.up", code).unwrap();
    tick(a);
}

fn game(a: &App) -> &Match {
    &a.play.as_ref().unwrap().game
}

#[test]
fn the_lobby_setup_feeds_the_match() {
    let mut a = lobby();
    a.act("settings.name", "  Rosie\n ").unwrap();
    a.act("lobby.ai", "2").unwrap();
    a.act("lobby.ai.1", "hard").unwrap();
    a.act("lobby.ai.2", "easy").unwrap();
    a.act("lobby.length", "240000").unwrap();
    a.act("lobby.start", "").unwrap();
    assert_eq!(a.screen, Screen::Play);
    let m = game(&a);
    let who: Vec<Occupant> = m.setup.seats.iter().map(|s| s.occupant).collect();
    assert_eq!(
        who,
        [
            Occupant::Human,
            Occupant::Ai(Difficulty::Hard),
            Occupant::Ai(Difficulty::Easy),
            Occupant::Empty
        ]
    );
    assert_eq!(m.setup.seats[0].name, "Rosie");
    let (n1, n2) = (&m.setup.seats[1].name, &m.setup.seats[2].name);
    assert_ne!(n1, n2, "AI names differ");
    assert!(["Gil Waters", "Rod Fisher", "Bob Pike", "Wade Rivers"].contains(&n1.as_str()));
    // 4 minutes, at 50 ticks a second
    assert_eq!(m.rules.length_ticks, 12_000);
    assert_eq!(m.setup.buoy_interval_ms, 60_000);
    assert_eq!(m.boats.iter().flatten().count(), 3);
    assert!([0, 3].contains(&m.map.seed), "a listed layout");
}

#[test]
fn a_typed_map_seed_must_be_a_checked_layout() {
    let mut a = lobby();
    a.act("lobby.seed", "5").unwrap();
    assert_eq!(a.act("lobby.start", ""), Err("LOBBY.SEED_UNKNOWN".into()));
    assert_eq!(a.screen, Screen::Lobby);
    a.act("lobby.seed", "3x").unwrap();
    assert_eq!(a.lobby.as_ref().unwrap().seed, "3");
    a.act("lobby.start", "").unwrap();
    assert_eq!(game(&a).map.seed, 3);
}

#[test]
fn a_choice_the_tuning_does_not_offer_is_refused() {
    let mut a = lobby();
    assert!(a.act("lobby.ai", "4").is_err());
    assert!(a.act("lobby.length", "1000").is_err());
    assert!(a.act("lobby.ai.1", "brutal").is_err());
    assert!(a.act("lobby.ai.4", "easy").is_err());
}

#[test]
fn a_match_cannot_start_before_its_files_load() {
    let mut a = App::new(&crate::strings::tests::complete(), NOTES).unwrap();
    a.act("menu.play", "").unwrap();
    assert_eq!(
        a.act("lobby.start", ""),
        Err("The map list file is not loaded yet.".into())
    );
}

#[test]
fn held_w_drives_the_boat_through_the_controls_file() {
    let mut a = started();
    a.act("key", "KeyW").unwrap();
    for _ in 0..50 {
        tick(&mut a);
    }
    // 50 ticks of 64 mm/s
    assert_eq!(game(&a).boat(0).unwrap().forward_speed(), 3200);
    a.act("key.up", "KeyW").unwrap();
    tick(&mut a);
    assert_eq!(game(&a).boat(0).unwrap().forward_speed(), 3200 - 53);
}

#[test]
fn ticks_follow_the_clock_and_never_skip_inputs() {
    let mut a = started();
    // uneven frames adding up to one second: 50 ticks
    for ms in [7, 16, 33, 9, 120, 15, 200, 100, 240, 240, 20] {
        a.advance(a.clock + ms * 1000);
    }
    assert_eq!(game(&a).tick, 50);
    // a tap of Q between two ticks still switches the mode
    a.act("key", "KeyQ").unwrap();
    a.act("key.up", "KeyQ").unwrap();
    a.advance(a.clock + 20_000);
    assert_eq!(game(&a).boat(0).unwrap().mode, Mode::Fishing);
    // ten seconds away from the page run only a quarter of a second
    let t = game(&a).tick;
    a.advance(a.clock + 10_000_000);
    assert_eq!(game(&a).tick, t + MAX_CATCH_UP);
}

#[test]
fn the_mouse_turns_and_tilts_the_camera_within_its_limits() {
    let mut a = started();
    let p0 = a.play.as_ref().unwrap().cam_pitch;
    assert_eq!(p0, deg(15));
    a.act("mouse.move", "100,0").unwrap();
    // 100 px right: 25° clockwise from above
    assert_eq!(a.play.as_ref().unwrap().cam_yaw, fx::wrap(-deg(25)));
    a.act("mouse.move", "0,10000").unwrap();
    assert_eq!(a.play.as_ref().unwrap().cam_pitch, deg(60));
    let v = a.play_view().unwrap();
    let heading = game(&a).map.spawn.heading;
    assert_eq!(v.cam_yaw, crate::map::trig::wrap(heading - deg(25)));
    assert!(a.act("mouse.move", "left").is_err());
    // fractions of a pixel count by their whole part
    a.act("mouse.move", "-100.75,0.5").unwrap();
    assert_eq!(a.play.as_ref().unwrap().cam_yaw, 0);
    assert_eq!(whole("-.5"), Some(0));
    assert_eq!(whole("2.x"), None);
}

#[test]
fn settings_from_the_match_menu_go_back_to_the_match_which_waits_offline() {
    let mut a = started();
    key(&mut a, "Escape");
    let t = game(&a).tick;
    a.act("menu.settings", "").unwrap();
    assert_eq!(a.screen, Screen::Settings);
    for _ in 0..10 {
        tick(&mut a);
    }
    assert_eq!(game(&a).tick, t, "an offline match waits under the menu");
    a.act("key", "Escape").unwrap();
    assert_eq!(a.screen, Screen::Play, "Esc in Settings goes back to the match");
    assert!(a.play.as_ref().unwrap().menu_open());
    assert!(!a.ui().capture, "the menu has the mouse");
}

#[test]
fn the_hud_shows_the_time_left_and_the_page_keeps_the_keys() {
    let a = started();
    let ui = a.ui();
    assert!(ui.scene && ui.capture && ui.live);
    assert_eq!(ui.panels[0].heading, "HUD.TIME");
    assert_eq!(clock_text(480_000), "8:00");
    assert_eq!(clock_text(59_001), "1:00");
    assert_eq!(clock_text(59_000), "0:59");
    assert_eq!(clock_text(0), "0:00");
}
