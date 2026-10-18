//! The real data files: the strings and patch notes the page fetches are the
//! ones the core accepts, and the newest patch notes are this version's.

use std::fs;
use std::path::{Path, PathBuf};

use tool_core::app::{App, VERSION};
use tool_core::patch_notes;

fn data(name: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

#[test]
fn the_shipped_strings_and_patch_notes_start_the_app() {
    if let Err(e) = App::new(&data("strings.json"), &data("patch-notes.json")) {
        panic!("{e}");
    }
}

/// A version cannot go out without its notes.
#[test]
fn the_newest_patch_notes_are_this_version() {
    let notes = patch_notes::parse(&data("patch-notes.json")).unwrap();
    assert_eq!(
        notes[0].version, VERSION,
        "add this version's entry to the top of data/patch-notes.json"
    );
}

/// Every model file the build bundles is one the core accepts, under the
/// name the gallery and the game will know it by.
#[test]
fn every_model_file_is_accepted() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("data/models")
        .map(|e| e.expect("entry").path())
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "data/models holds no model");
    for path in paths {
        let id = path.file_stem().unwrap().to_str().unwrap().to_string();
        assert_eq!(
            path.extension().and_then(|x| x.to_str()),
            Some("json"),
            "{}",
            path.display()
        );
        assert!(
            id.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "{id}: a model's file name is lowercase words joined by hyphens"
        );
        let text = fs::read_to_string(&path).unwrap();
        if let Err(e) = tool_core::model::parse(&id, &text) {
            panic!("{e}");
        }
    }
}

/// `data/tuning.json` holds every key the core reads, each in its range
/// (the core refuses a missing, unknown or out-of-range key by name), and
/// the numbers the brief and the person's answers name.
#[test]
fn tuning_file_is_complete_and_valid() {
    let t = tool_core::tuning::parse(&data("tuning.json")).unwrap_or_else(|e| panic!("{e}"));
    // BRIEF: 8 minutes, up to 4 players, top speed in about 5 s, rams at
    // 80%, grounding at 50%, respawn after 2 s; round 1: a 3 s stall, a few
    // seconds of ghosting; the map work: 16 m/s; an item buoy a minute
    assert_eq!(t.game.length_ms, 8 * 60 * 1000);
    assert_eq!(t.game.seats, 4);
    assert_eq!(t.game.buoy_interval_ms, 60 * 1000);
    assert_eq!(t.boat.top_speed_mm_per_s, 16_000);
    assert_eq!(t.boat.accel_ms, 5000);
    assert_eq!((t.wreck.ram_wreck_pct, t.wreck.land_wreck_pct), (80, 50));
    assert_eq!(
        (t.wreck.respawn_ms, t.wreck.stall_ms, t.wreck.ghost_ms),
        (2000, 3000, 3000)
    );
    assert_eq!(t.boat.top_speed_mm_per_s, tool_core::map::BOAT_TOP_SPEED_MM_PER_S);
}

/// The controls, the AI names and the map list the page hands the core are
/// accepted, and a match starts from them.
#[test]
fn the_match_files_start_a_match() {
    let mut app = App::new(&data("strings.json"), &data("patch-notes.json")).unwrap();
    for (name, file) in [
        ("data.tuning", "tuning.json"),
        ("data.controls", "controls.json"),
        ("data.names", "names.json"),
        ("data.maps", "maps/michigan-lake.json"),
    ] {
        app.act(name, &data(file))
            .unwrap_or_else(|e| panic!("{file}: {e}"));
    }
    app.act("menu.play", "").unwrap();
    app.act("lobby.ai", "3").unwrap();
    app.act("lobby.start", "").unwrap();
    let play = app.play.as_ref().expect("a match");
    assert_eq!(play.game.boats.iter().flatten().count(), 4);
}
