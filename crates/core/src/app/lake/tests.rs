//! The Lake preview: its screen, its controls and its frame, on the real
//! look file and the models it names. Expected values are worked out by
//! hand from the rules in `app/lake.rs`.

use super::*;
use serde_json::{json, Value};

const NOTES: &str = r#"[{"version": "0.1.0", "date": "2026-10-17", "notes": ["A."]}]"#;

/// The models the lake's look names, and the boat and angler.
fn models() -> String {
    let files = [
        ("pine", include_str!("../../../../../data/models/pine.json")),
        (
            "pine-tall",
            include_str!("../../../../../data/models/pine-tall.json"),
        ),
        (
            "pine-small",
            include_str!("../../../../../data/models/pine-small.json"),
        ),
        ("birch", include_str!("../../../../../data/models/birch.json")),
        (
            "cattails",
            include_str!("../../../../../data/models/cattails.json"),
        ),
        (
            "lily-pads",
            include_str!("../../../../../data/models/lily-pads.json"),
        ),
        (
            "shore-rocks",
            include_str!("../../../../../data/models/shore-rocks.json"),
        ),
        ("boulder", include_str!("../../../../../data/models/boulder.json")),
        (
            "fallen-log",
            include_str!("../../../../../data/models/fallen-log.json"),
        ),
        ("dock", include_str!("../../../../../data/models/dock.json")),
        ("cabin", include_str!("../../../../../data/models/cabin.json")),
        ("boat", include_str!("../../../../../data/models/boat.json")),
        ("angler", include_str!("../../../../../data/models/angler.json")),
    ];
    let mut all = serde_json::Map::new();
    for (id, text) in files {
        all.insert(id.into(), serde_json::from_str::<Value>(text).unwrap());
    }
    Value::Object(all).to_string()
}

const LOOK: &str = include_str!("../../../../../data/look.json");

fn maps() -> String {
    json!({
        "setting": "michigan-lake",
        "from": 0,
        "scanned": 4,
        "maps": [{ "seed": 0, "stamp": "a" }, { "seed": 3, "stamp": "b" }]
    })
    .to_string()
}

fn app() -> App {
    let mut a = App::new(&crate::strings::tests::complete(), NOTES).unwrap();
    a.load_models(&models(), LOOK).unwrap();
    a.act("data.maps", &maps()).unwrap();
    a.resize(1600, 900, 100);
    a
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

#[test]
fn the_lake_preview_opens_from_the_title_on_the_first_checked_map() {
    let mut a = app();
    assert_eq!(a.world_stamp(), 0, "no world before the preview opens");
    a.act("menu.lake", "").unwrap();
    assert_eq!(a.screen, Screen::Lake);
    assert!(a.ui().scene, "the preview draws the 3D scene");
    assert_eq!(a.world.as_ref().map(|w| w.map.seed), Some(0));
    assert_ne!(a.world_stamp(), 0);
    assert_eq!(
        ids(&a),
        ["lake.seed", "lake.view", "lake.fog_on", "lake.fog", "menu.back"]
    );
    a.act("key", "Escape").unwrap();
    assert_eq!(a.screen, Screen::Title);
}

#[test]
fn a_seed_from_the_list_builds_its_map_and_another_is_refused() {
    let mut a = app();
    a.act("menu.lake", "").unwrap();
    let first = a.world_stamp();
    a.act("lake.seed", "3").unwrap();
    assert_eq!(a.world.as_ref().map(|w| w.map.seed), Some(3));
    assert_ne!(a.world_stamp(), first, "a new map, a new stamp for the page");
    assert_eq!(
        a.act("lake.seed", "4"),
        Err("There is no checked map seed \"4\".".into())
    );
    assert_eq!(a.world.as_ref().map(|w| w.map.seed), Some(3));
}

#[test]
fn the_fog_slider_keeps_to_its_range_and_steps_and_the_toggle_turns_it_off() {
    let mut a = app();
    a.act("menu.lake", "").unwrap();
    // the map's own 120 m to start
    assert_eq!(a.sight_mm(), Some(120_000));
    // 137 m steps down to 130; below 30 is 30; above 400 is 400
    a.act("lake.fog", "137").unwrap();
    assert_eq!(a.sight_mm(), Some(130_000));
    a.act("lake.fog", "5").unwrap();
    assert_eq!(a.sight_mm(), Some(30_000));
    a.act("lake.fog", "9999").unwrap();
    assert_eq!(a.sight_mm(), Some(400_000));
    assert_eq!(
        a.act("lake.fog", "far"),
        Err("The fog distance \"far\" is not a whole number.".into())
    );
    a.act("lake.fog_on", "off").unwrap();
    assert_eq!(a.sight_mm(), None);
    a.act("lake.fog_on", "on").unwrap();
    assert_eq!(a.sight_mm(), Some(400_000));
}

#[test]
fn every_view_draws_the_world_with_its_boat_and_an_unknown_one_is_refused() {
    let mut a = app();
    a.act("menu.lake", "").unwrap();
    let lake = a.world.as_ref().unwrap();
    let statics = [lake.terrain[0], lake.terrain[1], lake.water[0], lake.water[1]];
    for v in View::ALL {
        a.act("lake.view", v.key()).unwrap();
        a.advance(a.clock + 1_000_000);
        let f = a.frame();
        assert_eq!(f.view.len(), render::VIEW_VALUES, "{v:?}");
        assert_eq!(f.statics, statics, "{v:?}");
        assert!(!f.tris.is_empty(), "{v:?}: the boat is posed");
    }
    assert_eq!(
        a.act("lake.view", "sideways"),
        Err("There is no lake view \"sideways\".".into())
    );
}

#[test]
fn without_the_lakes_look_the_preview_says_why_and_draws_nothing() {
    let mut a = App::new(&crate::strings::tests::complete(), NOTES).unwrap();
    let mut look: Value = serde_json::from_str(LOOK).unwrap();
    look.as_object_mut().unwrap().remove("lake");
    a.load_models(&models(), &look.to_string()).unwrap();
    a.act("data.maps", &maps()).unwrap();
    a.act("menu.lake", "").unwrap();
    assert!(a.world.is_none());
    assert_eq!(a.ui().panels[0].sub, "LAKE.NONE");
    assert_eq!(a.frame(), Frame::default());
}

#[test]
fn a_match_draws_its_boats_on_the_lakes_world_and_without_the_look_nothing() {
    let mut a = crate::app::play::tests::lobby();
    a.load_models(&models(), LOOK).unwrap();
    a.act("lobby.start", "").unwrap();
    a.advance(0);
    let map = &a.play.as_ref().unwrap().game.map;
    assert_eq!(
        a.world.as_ref().map(|w| &w.map),
        Some(map),
        "the match's own map is built"
    );
    let f = a.frame();
    let lake = a.world.as_ref().unwrap();
    assert_eq!(
        f.statics,
        [lake.terrain[0], lake.terrain[1], lake.water[0], lake.water[1]]
    );
    assert!(!f.groups.is_empty(), "the props near the spawn are drawn");
    assert!(!f.tris.is_empty(), "the boats are posed");
    // the fog is the map's own 120 m: whole at 120 m
    let (at, _) = render::uniform_at("uFog");
    assert_eq!(f.view[at + 1], crate::scene::mm(120_000));

    // a look file with no lake: no world, and no stand-in (SECOND-ORDER R2.66)
    let mut b = crate::app::play::tests::lobby();
    let mut look: Value = serde_json::from_str(LOOK).unwrap();
    look.as_object_mut().unwrap().remove("lake");
    b.load_models(&models(), &look.to_string()).unwrap();
    b.act("lobby.start", "").unwrap();
    assert!(b.world.is_none());
    assert_eq!(b.frame(), Frame::default());
}
