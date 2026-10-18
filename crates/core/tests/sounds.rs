//! The real sound files: every recipe in data/sounds.json is one the core
//! accepts and the page can play, every event names a cue that exists, and
//! the cues the brief asks for are there.

use std::fs;
use std::path::{Path, PathBuf};

use tool_core::sound::recipe::Params;
use tool_core::sound::{Bank, Sound};

fn data(name: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn bank() -> Bank {
    Bank::parse(&data("sounds.json"), &data("tuning.json")).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn every_cue_in_the_sounds_file_is_valid() {
    let b = bank();
    assert!(!b.cues.is_empty());
    let mut s = Sound::new(b.clone());
    for c in &b.cues {
        assert!(!c.layers.is_empty(), "{}", c.id);
        assert!(c.length_ms() > 0, "{}", c.id);
        // the layers sounding at once add up; much over 1 and the page clips
        let peak = c
            .layers
            .iter()
            .map(|at| {
                let t = at.at_ms + at.attack_ms;
                c.layers
                    .iter()
                    .filter(|l| l.at_ms <= t && t < l.end_ms())
                    .map(|l| l.gain)
                    .sum::<i64>()
            })
            .max()
            .unwrap_or(0);
        assert!(
            peak <= 1200,
            "{}: its layers add to a gain of {peak} thousandths",
            c.id
        );
        assert!(s.play(&c.id, &Params::default()), "{}", c.id);
        let f = s.frame(100, Vec::new());
        let layers = f["cues"][0]["layers"].as_array().unwrap();
        assert_eq!(layers.len(), c.layers.len(), "{}", c.id);
        for l in layers {
            // every point of every schedule is a number the page can use
            for p in l["gain"].as_array().unwrap() {
                assert!(p["t"].is_number() && p["v"].is_number(), "{}", c.id);
            }
        }
    }
    for (alias, target) in &b.aliases {
        assert!(b.cue(alias).is_some_and(|c| &c.id == target), "{alias}");
    }
    for l in &b.loops {
        assert!(!l.layers.is_empty(), "{}", l.id);
    }
}

#[test]
fn every_event_names_an_existing_cue() {
    let b = bank();
    assert!(!b.events.rows.is_empty());
    for (name, entries) in &b.events.rows {
        for e in entries {
            for id in e.every_cue() {
                assert!(
                    b.cue(&id).is_some(),
                    "event {name} names \"{id}\", which is not a cue"
                );
            }
        }
    }
}

#[test]
fn the_sounds_the_brief_names_are_all_there() {
    let b = bank();
    let wanted = [
        "menu_click",
        "match_start",
        "count_3",
        "count_2",
        "count_1",
        "cast_whoosh",
        "bobber_plop",
        "nibble_tick",
        "bite",
        "hook_set",
        "reel_click",
        "line_snap",
        "catch_common",
        "catch_uncommon",
        "catch_rare",
        "catch_legendary",
        "radar_ping",
        "sweep_hum",
        "radar_blip",
        "buoy_warning",
        "buoy_beat_5",
        "buoy_beat_4",
        "buoy_beat_3",
        "buoy_beat_2",
        "buoy_beat_1",
        "item_pickup",
        "minigame_start",
        "minigame_tick",
        "minigame_success",
        "explosion",
        "respawn_whoosh",
        "bump",
        "cloak_on",
        "cloak_off",
        "nitrous_roar",
        "teleport_charge",
        "teleport_zap",
        "camera_shutter",
        "fish_boy_open",
        "fish_boy_close",
        "hook_announce",
        "wreck_announce",
        "match_end_horn",
        "tally_tick",
        "winner_fanfare",
    ];
    for id in wanted {
        assert!(b.cue(id).is_some(), "no cue \"{id}\"");
    }
    for id in ["own_motor", "rival_motor", "line_tension"] {
        assert!(b.find_loop(id).is_some(), "no loop \"{id}\"");
    }
    // the item buoy's countdown: five different beats, the last the highest
    let pitch = |id: &str| b.cue(id).unwrap().layers[0].freq;
    let beats: Vec<i64> = (1..=5).rev().map(|k| pitch(&format!("buoy_beat_{k}"))).collect();
    for w in beats.windows(2) {
        assert!(w[1] > w[0], "the beats climb: {beats:?}");
    }
    // a bigger jingle for each tier: longer, the legendary the longest
    let len = |id: &str| b.cue(id).unwrap().length_ms();
    let tiers = ["catch_common", "catch_uncommon", "catch_rare", "catch_legendary"];
    for w in tiers.windows(2) {
        assert!(len(w[1]) > len(w[0]), "{} is no bigger than {}", w[1], w[0]);
    }
}
