//! The Gallery: its screen, its choices and its frame. Expected values are
//! worked out by hand from the rules in `gallery.rs`.

use super::*;
use crate::gfx::render::VERTEX_VALUES;
use serde_json::{json, Value};

const NOTES: &str = r#"[{"version": "0.1.0", "date": "2026-10-17", "notes": ["A."]}]"#;

fn look() -> Value {
    json!({
        "players": { "blue": [0, 0, 255], "red": [255, 0, 0], "green": [0, 255, 0], "yellow": [255, 255, 0] },
        "gallery": {
            "light": { "fog": [1, 2, 3], "sun_dir": [0, 100, 0], "sun": [9, 9, 9], "sky": [9, 9, 9], "ground": [9, 9, 9] },
            "floor": [100, 100, 100],
            "turn_seconds": 24
        }
    })
}

/// One triangle a metre across, in a colour or the player's slot.
fn tri(kind: &str, name: &str, slot: bool) -> Value {
    let colour = if slot {
        json!({ "slot": "player" })
    } else {
        json!([10, 20, 30])
    };
    json!({
        "name": name, "kind": kind, "unit": "mm", "view": { "yaw": 90, "pitch": 10 },
        "colours": { "c": colour },
        "parts": [{ "name": "p", "vertices": [[0, 0, 0], [1000, 0, 0], [0, 1000, 0]] }],
        "faces": [{ "colour": "c", "v": [0, 1, 2] }]
    })
}

fn models() -> Value {
    let anims = |names: &[&str]| {
        let mut m = serde_json::Map::new();
        for n in names {
            m.insert(n.to_string(), json!({ "ms": 1000, "keys": [{ "at": 0 }] }));
        }
        Value::Object(m)
    };
    let mut fish = tri("fish", "Perch", false);
    fish["animations"] = anims(&["idle", "panic"]);
    let mut angler = tri("figure", "Angler", true);
    angler["animations"] = anims(&["drive", "fish"]);
    // an anchor of its own, which a figure never uses to carry a rider
    angler["anchors"] = json!({ "drive": { "part": "p", "at": [0, 0, 0] } });
    let mut boat = tri("boat", "Boat", true);
    boat["anchors"] =
        json!({ "drive": { "part": "p", "at": [0, 0, 0] }, "fish": { "part": "p", "at": [0, 0, 500] } });
    json!({ "perch": fish, "angler": angler, "boat": boat, "rock": tri("prop", "Rock", false) })
}

/// The test strings, with the gallery's placeholders in.
fn strings() -> String {
    let mut s: Value = serde_json::from_str(&crate::strings::tests::complete()).unwrap();
    s["gallery.rider"] = json!("{figure}: {pose}");
    s["gallery.tall"] = json!("{feet} ft tall");
    s.to_string()
}

fn app() -> App {
    let mut a = App::new(&strings(), NOTES).unwrap();
    a.load_models(&models().to_string(), &look().to_string()).unwrap();
    a.resize(1600, 900, 100);
    a.act("menu.gallery", "").unwrap();
    a
}

fn panel(a: &App) -> Value {
    serde_json::to_value(a.ui()).unwrap()["panels"][0].clone()
}

/// The items' ids, and a select's values by id.
fn ids(a: &App) -> Vec<String> {
    panel(a)["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|i| i["id"].as_str().map(String::from))
        .collect()
}

fn options(a: &App, id: &str) -> Vec<String> {
    let p = panel(a);
    let item = p["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == id)
        .unwrap()
        .clone();
    item["options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["value"].as_str().unwrap().to_string())
        .collect()
}

/// Triangles in this frame.
fn triangles(a: &App) -> usize {
    a.frame().1.len() / VERTEX_VALUES / 3
}

#[test]
fn the_gallery_opens_from_the_title_with_its_scene_and_escape_closes_it() {
    let mut a = App::new(&crate::strings::tests::complete(), NOTES).unwrap();
    assert!(!a.ui().scene);
    a.act("menu.gallery", "").unwrap();
    assert_eq!(a.screen, Screen::Gallery);
    assert!(a.ui().scene);
    a.act("key", "Escape").unwrap();
    assert_eq!(a.screen, Screen::Title);
    assert!(!a.ui().scene);
}

#[test]
fn with_no_models_the_gallery_says_so_and_draws_nothing() {
    let mut a = App::new(&crate::strings::tests::complete(), NOTES).unwrap();
    a.act("menu.gallery", "").unwrap();
    assert_eq!(panel(&a)["sub"], "GALLERY.NONE");
    assert_eq!(ids(&a), ["menu.back"]);
    assert_eq!(a.frame(), (Vec::new(), Vec::new()));
}

#[test]
fn models_come_in_gallery_order_and_open_still_at_their_own_angle() {
    let mut a = app();
    assert_eq!(options(&a, "gallery.model"), ["perch", "boat", "angler", "rock"]);
    assert_eq!(panel(&a)["heading"], "Perch");
    a.act("gallery.turn", "off").unwrap();
    a.act("gallery.show", "anim.idle").unwrap();
    a.act("gallery.model", "rock").unwrap();
    assert_eq!(panel(&a)["heading"], "Rock");
    assert_eq!(a.gallery.show, Show::Still);
    assert_eq!(a.gallery_yaw(), deg(90));
    assert_eq!(
        a.act("gallery.model", "shark"),
        Err("There is no model \"shark\".".into())
    );
}

#[test]
fn each_model_shows_its_own_animations_and_a_boat_its_rider() {
    let mut a = app();
    assert_eq!(options(&a, "gallery.show"), ["still", "anim.idle", "anim.panic"]);
    a.act("gallery.model", "boat").unwrap();
    assert_eq!(
        options(&a, "gallery.show"),
        ["still", "rider.drive", "rider.fish"]
    );
    a.act("gallery.model", "angler").unwrap();
    assert_eq!(options(&a, "gallery.show"), ["still", "anim.drive", "anim.fish"]);
    assert_eq!(
        a.act("gallery.show", "anim.idle"),
        Err("This model cannot show \"anim.idle\".".into())
    );
}

#[test]
fn the_rider_is_labelled_by_name_and_pose_and_drawn_on_the_boat() {
    let mut a = app();
    a.act("gallery.model", "boat").unwrap();
    let p = panel(&a);
    let show = p["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == "gallery.show")
        .unwrap()
        .clone();
    assert_eq!(show["options"][1]["text"], "Angler: ANIM.DRIVE");
    // the boat's triangle and the floor's 32
    assert_eq!(triangles(&a), 1 + 32);
    a.act("gallery.show", "rider.fish").unwrap();
    assert_eq!(triangles(&a), 2 + 32);
}

#[test]
fn the_colour_choice_shows_for_a_slot_and_all_four_draws_four() {
    let mut a = app();
    assert!(!ids(&a).contains(&"gallery.colour".to_string()));
    a.act("gallery.model", "boat").unwrap();
    assert_eq!(
        ids(&a),
        [
            "gallery.model",
            "gallery.show",
            "gallery.colour",
            "gallery.view",
            "gallery.turn",
            "menu.back"
        ]
    );
    assert_eq!(
        options(&a, "gallery.colour"),
        ["blue", "red", "green", "yellow", "all"]
    );
    // the player's colour fills the slot: red is 255, 0, 0
    a.act("gallery.colour", "red").unwrap();
    let (_, tris) = a.frame();
    assert_eq!(&tris[6..9], &[ONE, 0, 0]);
    a.act("gallery.colour", "all").unwrap();
    assert_eq!(triangles(&a), 4 + 32);
    let (_, tris) = a.frame();
    let colours: Vec<&[i32]> = (0..4).map(|k| &tris[k * 27 + 6..k * 27 + 9]).collect();
    assert_eq!(
        colours,
        [&[0, 0, ONE][..], &[ONE, 0, 0], &[0, ONE, 0], &[ONE, ONE, 0]]
    );
    assert_eq!(
        a.act("gallery.colour", "pink"),
        Err("There is no player colour \"pink\".".into())
    );
}

#[test]
fn the_turntable_turns_once_a_cycle_and_stops_where_it_is() {
    let mut a = app();
    let start = a.gallery_yaw();
    assert_eq!(start, deg(90));
    // 24 s a turn: 6 s is a quarter
    a.advance(6_000_000);
    assert_eq!(a.gallery_yaw(), fx::wrap(deg(90) + TURN / 4));
    a.act("gallery.turn", "off").unwrap();
    a.advance(9_000_000);
    assert_eq!(a.gallery_yaw(), fx::wrap(deg(90) + TURN / 4));
    a.act("gallery.turn", "on").unwrap();
    a.advance(15_000_000);
    assert_eq!(a.gallery_yaw(), fx::wrap(deg(90) + TURN / 2));
    // the clock does not go back
    a.advance(1);
    assert_eq!(a.clock, 15_000_000);
}

#[test]
fn the_view_from_above_looks_nearly_straight_down() {
    let mut a = app();
    let centre = a.models[a.gallery.model].ball().0;
    // the eye's height over the model's middle, and how far off to the side
    let eye = |a: &App| {
        let v = a.frame().0;
        let e = v3(v[16], v[17], v[18]).sub(centre);
        (e.y, v3(e.x, 0, e.z).len())
    };
    a.act("gallery.view", "above").unwrap();
    // 88° down: up 28.6 for every 1 across (tan 88°)
    let (up, across) = eye(&a);
    assert!(up > across * 28 && up < across * 29, "{up} {across}");
    a.act("gallery.view", "model").unwrap();
    // the perch's own 10°: up 0.176 for every 1 across (tan 10°)
    let (up, across) = eye(&a);
    assert!(
        up * 1000 > across * 170 && up * 1000 < across * 183,
        "{up} {across}"
    );
    assert_eq!(
        a.act("gallery.view", "below"),
        Err("There is no view \"below\".".into())
    );
}

#[test]
fn lengths_show_in_feet_to_two_places() {
    assert_eq!(feet(3048), "10.00");
    assert_eq!(feet(200), "0.66");
    assert_eq!(feet(1000), "3.28");
    let mut a = app();
    a.act("gallery.model", "rock").unwrap();
    // a prop by its height: 1000 mm
    assert_eq!(panel(&a)["sub"], "3.28 ft tall");
}

#[test]
fn a_bad_look_file_is_refused_by_name() {
    let mut a = App::new(&crate::strings::tests::complete(), NOTES).unwrap();
    let m = models().to_string();
    let mut l = look();
    l["players"].as_object_mut().unwrap().remove("yellow");
    assert_eq!(
        a.load_models(&m, &l.to_string()),
        Err("The look file has no player colour \"yellow\".".into())
    );
    let mut l = look();
    l["players"]["pink"] = json!([1, 1, 1]);
    assert_eq!(
        a.load_models(&m, &l.to_string()),
        Err("The look file has a player colour \"pink\" no slot uses.".into())
    );
    let mut l = look();
    l["gallery"]["turn_seconds"] = json!(0);
    assert_eq!(
        a.load_models(&m, &l.to_string()),
        Err("The gallery's turn must take some seconds.".into())
    );
    assert!(a.models.is_empty(), "a refused load keeps nothing");
}

#[test]
fn every_animation_and_player_colour_has_a_string() {
    for a in Anim::ALL {
        assert!(
            crate::strings::KEYS.contains(&format!("anim.{}", a.key()).as_str()),
            "{a:?}"
        );
    }
    for c in PLAYER_COLOURS {
        assert!(
            crate::strings::KEYS.contains(&format!("colour.{c}").as_str()),
            "{c}"
        );
    }
}
