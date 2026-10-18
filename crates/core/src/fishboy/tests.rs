//! The Fish-Boy: raising and lowering, its tabs, the radar's ping and map
//! picture, the Fishpedia and the catch list. Expected values are worked
//! out by hand from the brief and the rules in these files.

use super::radar::*;
use super::*;
use crate::map::{Setting, Spawn, P};
use serde_json::{json, Value};

const NOTES: &str = r#"[{"version": "0.1.0", "date": "2026-10-17", "notes": ["A."]}]"#;
const THEME: &str = include_str!("../../../../data/ui2d.json");
const TUNING: &str = include_str!("../../../../data/tuning.json");
/// Three fish: one with a model, two without.
const FISH: &str = r#"{"fish": [
    {"id": "muskellunge", "name": "Muskellunge", "tier": "legendary", "model": "muskellunge", "base_length_mm": 1100, "value": 55},
    {"id": "pumpkinseed", "name": "Pumpkinseed", "tier": "common", "model": "pumpkinseed", "base_length_mm": 180, "value": 9},
    {"id": "bluegill", "name": "Bluegill", "tier": "common", "model": "bluegill", "base_length_mm": 200, "value": 10}]}"#;

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

fn models() -> Value {
    let fish = json!({
        "name": "Bluegill", "kind": "fish", "unit": "mm",
        "colours": { "c": [10, 20, 30] },
        "parts": [{ "name": "p", "vertices": [[0, 0, 0], [0, 50, 0], [0, 0, 200]] }],
        "faces": [{ "colour": "c", "v": [0, 1, 2] }],
        "animations": { "idle": { "ms": 1000, "keys": [{ "at": 0 }] }, "panic": { "ms": 1000, "keys": [{ "at": 0 }] } }
    });
    // the device: a back plate and a 192 x 108 mm screen at z = 0
    let device = json!({
        "name": "Fish-Boy", "kind": "device", "unit": "mm",
        "colours": { "c": [10, 20, 30], "s": [0, 40, 0] },
        "parts": [
            { "name": "body", "vertices": [[-120, -70, -10], [120, -70, -10], [0, 80, -10]] },
            { "name": "screen", "parent": "body", "vertices": [[-96, -54, 0], [96, -54, 0], [96, 54, 0], [-96, 54, 0]] }
        ],
        "faces": [{ "colour": "c", "v": [0, 1, 2] }, { "colour": "s", "v": [3, 4, 5, 6] }],
        "anchors": { "screen": { "part": "screen", "at": [0, 0, 0] } }
    });
    json!({ "bluegill": fish, "fishboy": device })
}

/// The test strings, with the placeholders the screens fill.
fn strings() -> String {
    let mut s: Value = serde_json::from_str(&crate::strings::tests::complete()).unwrap();
    for (k, v) in [
        ("fishpedia.left", "{count} left"),
        ("fishpedia.none_left", "None left"),
        ("fishpedia.total", "{count} fish left"),
        ("tier.legendary", "Legendary"),
        ("tier.common", "Common"),
        ("fishboy.tab.radar", "Radar"),
        ("fishboy.tab.fishpedia", "Fishpedia"),
        ("fishboy.tab.catch", "My catch"),
        ("catch.feet", "{feet} ft"),
        ("radar.ping.cooling", "Ready again in {seconds} s"),
        ("hud.fishboy", "Tab raises the Fish-Boy."),
    ] {
        s[k] = json!(v);
    }
    s.to_string()
}

/// One second of the page's clock, in µs.
const S: i64 = 1_000_000;

fn app() -> App {
    let mut a = App::new(&strings(), NOTES).unwrap();
    a.load_models(&models().to_string(), &look().to_string()).unwrap();
    a.act("data.ui2d", THEME).unwrap();
    a.act("data.fish", FISH).unwrap();
    a.act("data.tuning", TUNING).unwrap();
    a.resize(1600, 900, 100);
    a.advance(10 * S);
    a.act("menu.play", "").unwrap();
    a
}

/// Raised and settled.
fn raised() -> App {
    let mut a = app();
    a.act("key", "Tab").unwrap();
    a.advance(a.clock + S);
    a
}

fn near(a: i32, b: i32) -> bool {
    (a - b).abs() <= 2
}

// ---- raising, lowering, tabs ----

#[test]
fn tab_raises_esc_lowers_and_it_opens_again_on_the_last_tab() {
    let mut a = app();
    assert_eq!(a.ui().fishboy, "");
    a.act("key", "Tab").unwrap();
    assert_eq!(a.ui().fishboy, "radar", "it opens on the radar the first time");
    a.act("key", "Digit2").unwrap();
    assert_eq!(a.ui().fishboy, "fishpedia");
    a.act("key", "Escape").unwrap();
    assert_eq!(a.ui().fishboy, "", "Esc puts it down");
    assert!(!a.paused, "Esc with the Fish-Boy up does not pause");
    a.act("key", "Tab").unwrap();
    assert_eq!(a.ui().fishboy, "fishpedia", "it opens on the last tab viewed");
    a.act("key", "Tab").unwrap();
    assert_eq!(a.ui().fishboy, "");
    a.act("key", "Escape").unwrap();
    assert!(a.paused, "with it down, Esc pauses as before");
}

#[test]
fn raised_the_screen_spans_80_percent_of_a_wide_view() {
    // 1600 x 900 is 16:9 like the screen: 1280 x 720, 160 in and 90 down
    let a = raised();
    let r = a.fishboy_raised().unwrap();
    assert_eq!(r.amount, ONE);
    let p = r.px;
    assert!(
        near(p.x, 160) && near(p.y, 90) && near(p.w, 1280) && near(p.h, 720),
        "{p:?}"
    );
}

#[test]
fn on_a_tall_view_the_width_binds() {
    // 900 wide: the screen is 720 wide, 720 x 108 / 192 = 405 high, in
    // the middle of 1600
    let mut a = raised();
    a.resize(900, 1600, 100);
    let p = a.fishboy_raised().unwrap().px;
    assert!(
        near(p.w, 720) && near(p.h, 405) && near(p.x, 90) && near(p.y, 597),
        "{p:?}"
    );
}

#[test]
fn raising_slides_the_screen_up_from_below_the_view() {
    let mut a = app();
    a.act("key", "Tab").unwrap();
    assert!(a.fishboy_raised().is_none(), "nothing shows at the first instant");
    let raise = 380 * 1000;
    a.advance(a.clock + raise / 2);
    let mid = a.fishboy_raised().unwrap();
    assert!(
        mid.px.y > 90 + 100,
        "halfway it is still well below its place: {:?}",
        mid.px
    );
    assert!(near(mid.px.h, 720), "it does not change size on the way");
    a.advance(a.clock + raise / 2);
    assert!(near(a.fishboy_raised().unwrap().px.y, 90));
}

#[test]
fn lowering_halfway_up_goes_back_down_from_where_it_is() {
    let mut a = app();
    a.act("key", "Tab").unwrap();
    a.advance(a.clock + 190_000);
    let up = a.fishboy.amount(a.clock);
    a.act("key", "Tab").unwrap();
    assert!((a.fishboy.amount(a.clock) - up).abs() < ONE / 100, "no jump");
    a.advance(a.clock + 190_000);
    assert_eq!(a.fishboy.amount(a.clock), 0);
}

#[test]
fn clicking_a_tab_switches_to_it() {
    let mut a = raised();
    // the Radar tab is 40 to 40 + 5 x 34 x 60% + 52 = 194 units, the
    // Fishpedia tab starts 16 later, at 210; unit (230, 50) is pixel
    // (160 + 230 x 0.8, 90 + 50 x 0.8)
    a.act("pointer.down", "344 130").unwrap();
    assert_eq!(a.fishboy.tab, Tab::Fishpedia);
    let (x, y) = a.fishboy.pointer.unwrap();
    assert!(near(x, 230) && near(y, 50), "{x} {y}");
    a.act("pointer.up", "344.6 130.2").unwrap();
    assert!(!a.fishboy.pointer_down);
    assert!(a.act("pointer.move", "here").is_err());
}

#[test]
fn space_pings_on_the_radar_and_the_cooldown_holds_the_next() {
    let mut a = raised();
    a.act("key", "Space").unwrap();
    let first = a.fishboy.demo.as_ref().unwrap().ping.started;
    assert!(first.is_some());
    a.advance(a.clock + 2 * S);
    a.act("key", "Space").unwrap();
    assert_eq!(
        a.fishboy.demo.as_ref().unwrap().ping.started,
        first,
        "2 s in, the cooldown holds"
    );
    a.advance(a.clock + 3 * S);
    a.act("key", "Space").unwrap();
    assert_ne!(
        a.fishboy.demo.as_ref().unwrap().ping.started,
        first,
        "5 s on, it pings again"
    );
}

#[test]
fn space_on_another_tab_does_not_ping() {
    let mut a = raised();
    a.act("key", "Digit2").unwrap();
    a.act("key", "Space").unwrap();
    assert_eq!(a.fishboy.demo.as_ref().unwrap().ping.started, None);
}

// ---- the ping ----

fn tuning() -> RadarTuning {
    RadarTuning {
        ping_cooldown_ms: 5000,
        ping_sweep_ms: 3000,
    }
}

fn boat(x: i32, y: i32, cloaked: bool) -> BoatView {
    BoatView {
        at: P::new(x, y),
        heading: 0,
        slot: 0,
        name: "A".into(),
        cloaked,
    }
}

#[test]
fn a_ping_waits_out_its_five_second_cooldown() {
    let view = RadarView {
        me: 0,
        boats: vec![boat(0, 0, false)],
        buoy: None,
    };
    let mut p = Ping::default();
    assert!(p.start(0, &view, 100_000, &tuning()));
    // 5000 ms at 20 ms a tick is 250 ticks
    assert_eq!(p.state(100, &tuning()), PingState::Sweeping(666));
    assert_eq!(p.state(200, &tuning()), PingState::Cooling(50, 800));
    assert!(!p.start(249, &view, 100_000, &tuning()));
    assert!(p.start(250, &view, 100_000, &tuning()));
}

#[test]
fn the_sweep_crosses_the_map_in_its_time_and_sees_boats_as_it_passes() {
    // from a corner of a 100 m map, the far corner is 141 421 mm away; the
    // sweep takes 150 ticks, so a boat 70 710 mm out is reached at tick 75
    let view = RadarView {
        me: 0,
        boats: vec![boat(0, 0, false), boat(70_710, 0, false)],
        buoy: None,
    };
    let mut p = Ping::default();
    p.start(0, &view, 100_000, &tuning());
    assert_eq!(p.reach_mm, 141_421);
    for t in 0..=74 {
        p.step(t, &view, &tuning());
    }
    assert_eq!(p.sightings[1], None);
    p.step(75, &view, &tuning());
    assert_eq!(
        p.sightings[1].map(|s| (s.at, s.tick)),
        Some((P::new(70_710, 0), 75))
    );
    assert_eq!(p.sightings[0], None, "you are never a blip of your own");
    assert_eq!(p.radius(150, &tuning()), None, "the sweep is over after 3 s");
}

#[test]
fn a_cloaked_boat_is_not_seen_and_its_old_blip_is_wiped() {
    let mut view = RadarView {
        me: 0,
        boats: vec![boat(0, 0, false), boat(1000, 0, false)],
        buoy: None,
    };
    let mut p = Ping::default();
    p.start(0, &view, 100_000, &tuning());
    p.step(5, &view, &tuning());
    assert!(p.sightings[1].is_some());
    view.boats[1].cloaked = true;
    p.start(250, &view, 100_000, &tuning());
    p.step(255, &view, &tuning());
    assert_eq!(p.sightings[1], None);
}

#[test]
fn the_tuning_file_gives_the_ping_its_numbers_and_refuses_a_stray_key() {
    let t = RadarTuning::parse(TUNING).unwrap();
    assert_eq!((t.cooldown_ticks(), t.sweep_ticks()), (250, 150));
    let bad = r#"{"radar": {"ping_cooldown_ms": 5000, "ping_sweep_ms": 3000, "ping_range": 9}}"#;
    assert!(RadarTuning::parse(bad).unwrap_err().contains("ping_range"));
    let slow = r#"{"radar": {"ping_cooldown_ms": 2000, "ping_sweep_ms": 3000}}"#;
    assert!(
        RadarTuning::parse(slow).is_err(),
        "a sweep longer than the cooldown"
    );
}

// ---- the map picture ----

#[test]
fn the_map_picture_draws_the_shore_where_land_meets_water() {
    // 8 cells of 4 m: water in the west half, land in the east; blocks of
    // 2 m give 16 a row, and each row has one shore block each side of
    // the edge
    let n = 8;
    let water: Vec<bool> = (0..n * n).map(|i| i % n < 4).collect();
    let spawn = Spawn {
        at: P::new(2000, 2000),
        heading: 0,
        dock: P::new(0, 0),
    };
    let map = crate::map::Map::from_water(Setting::MichiganLake, 1, n, 4000, &water, spawn);
    let theme = Theme::parse(THEME).unwrap();
    let mut look = theme.radar.clone();
    look.block_mm = 2000;
    let b = map_bitmap(&map, &look);
    assert_eq!((b.w, b.h), (16, 16));
    for row in 0..16 {
        let px = &b.pixels[row * 16..row * 16 + 16];
        let scan = if row % 2 == 1 { 4 } else { 0 };
        assert!(
            px[0] == scan || px[0] == 1 + scan,
            "row {row} starts on water: {px:?}"
        );
        assert_eq!(px[15], 2 + scan, "row {row} ends on land");
        assert_eq!(
            px.iter().filter(|&&p| p == 3 + scan).count(),
            2,
            "row {row}: {px:?}"
        );
    }
    assert_eq!(b.id, "radar-michigan-lake-1");
}

// ---- what is drawn ----

fn texts(b: &crate::ui2d::Built) -> Vec<String> {
    b.list.strings.clone()
}

#[test]
fn down_the_hud_says_how_to_raise_it_and_no_device_is_drawn() {
    let a = app();
    let b = a.build_ui2d();
    assert!(texts(&b).contains(&"Tab raises the Fish-Boy.".to_string()));
    assert!(b.passes.is_empty());
}

#[test]
fn up_the_device_is_drawn_over_the_world_without_clearing_it() {
    let a = raised();
    let b = a.build_ui2d();
    let d = &b.passes[0];
    assert_eq!(d.clear, None);
    assert_eq!(d.gl_rect, [0, 0, 1600, 900]);
    assert!(!d.tris.is_empty());
    assert!(b.list.status.contains("raised=100"));
}

#[test]
fn the_fishpedia_shows_each_tier_its_fish_and_how_many_are_left() {
    let mut a = raised();
    a.act("key", "Digit2").unwrap();
    let b = a.build_ui2d();
    let t = texts(&b);
    // the demo starts the three fish at 3, 2 and 4
    // each row's total: 3 legendary, 2 + 4 common
    let wants = [
        "LEGENDARY",
        "COMMON",
        "MUSKELLUNGE",
        "BLUEGILL",
        "3 LEFT",
        "4 LEFT",
        "9 FISH LEFT",
        "3 left",
        "6 left",
    ];
    for want in wants {
        assert!(t.iter().any(|s| s == want), "{want} in {t:?}");
    }
    // only the bluegill has a model: one hole and its pass, after the
    // device's
    assert_eq!(b.list.shots.len(), 1);
    assert_eq!(b.list.shots[0].model, "bluegill");
    assert_eq!(b.passes.len(), 2);
    assert!(b.passes[1].clear.is_some(), "a box clears to its own colour");
}

#[test]
fn the_catch_list_gives_each_catch_its_length_in_feet() {
    let mut a = raised();
    a.act("key", "Digit3").unwrap();
    assert!(a.build_ui2d().list.status.contains("catches=0"));
    // the demo lands a fish every 12 s, every other one yours
    a.advance(a.clock + 40 * S);
    let rows = a.fishboy.demo.as_ref().unwrap().my_catches(&a.fishboy.roster);
    assert_eq!(rows.len(), 2);
    let b = a.build_ui2d();
    let want = format!("{} ft", crate::app::gallery::feet(rows[0].length_mm));
    assert!(texts(&b).contains(&want), "{want}");
}

#[test]
fn a_theme_without_a_radar_colour_is_refused_by_name() {
    let mut v: Value = serde_json::from_str(THEME).unwrap();
    v["radar"]["players"].as_object_mut().unwrap().remove("green");
    let mut a = app();
    assert_eq!(
        a.act("data.ui2d", &v.to_string()),
        Err("The ui2d file has no radar colour \"green\".".into())
    );
    let mut w: Value = serde_json::from_str(THEME).unwrap();
    w["tiers"]["rare"]["frame"] = json!("wavy");
    assert!(a.act("data.ui2d", &w.to_string()).is_err());
}
