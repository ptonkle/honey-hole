//! Fishing on screen: the underwater camera and its picture-in-picture, the
//! HUD's meters and the popups. Expected values are worked by hand from
//! `data/look.json`'s fishing section and the sample tuning.

use super::*;
use crate::app::lake::tests::{models, LOOK};
use crate::app::play::tests::{lobby, tick};
use crate::sim::fishing::reel::Reel;
use serde_json::Value;

/// The lake's models, the fish's, the bobber and the lure.
fn all_models() -> String {
    let mut all: Value = serde_json::from_str(&models()).unwrap();
    for (id, text) in [
        (
            "bluegill",
            include_str!("../../../../../data/models/bluegill.json"),
        ),
        ("bobber", include_str!("../../../../../data/models/bobber.json")),
        ("lure", include_str!("../../../../../data/models/lure.json")),
    ] {
        all[id] = serde_json::from_str(text).unwrap();
    }
    all.to_string()
}

/// A match on the real look at 1600 × 900, with the roster, its first boat
/// in fishing mode.
fn fishing() -> App {
    let mut a = lobby();
    a.load_models(&all_models(), LOOK).unwrap();
    a.act("data.fish", include_str!("../../../../../data/fish.json"))
        .unwrap();
    a.act("data.controls", include_str!("../../../../../data/controls.json"))
        .unwrap();
    a.resize(1600, 900, 100);
    a.act("lobby.start", "").unwrap();
    a.advance(0);
    let g = &mut a.play.as_mut().unwrap().game;
    g.boats[0].as_mut().unwrap().mode = Mode::Fishing;
    a
}

fn hud(a: &App) -> FishingHud {
    a.fishing_hud().expect("the fishing HUD shows")
}

#[test]
fn the_lake_is_stocked_from_the_roster() {
    let a = fishing();
    let g = &a.play.as_ref().unwrap().game;
    assert!(g.fishing.fish.len() > 40, "{} fish", g.fishing.fish.len());
    let left: u32 = g.fish_left().iter().map(|(_, n)| n).sum();
    assert_eq!(left as usize, g.fishing.fish.len());
}

#[test]
fn a_line_in_the_water_turns_on_the_underwater_camera_and_the_picture_in_picture() {
    let mut a = fishing();
    assert!(!a.underwater());
    assert!(a.inset_frame().0.is_empty());
    // the boat camera's fog is the map's 120 m
    let (at, _) = render::uniform_at("uFog");
    assert_eq!(a.frame().view[at + 1], mm(120_000));
    a.act("key", "Space").unwrap();
    tick(&mut a);
    a.act("key.up", "Space").unwrap();
    for _ in 0..60 {
        tick(&mut a);
    }
    assert!(a.underwater(), "{:?}", a.my_fishing().map(|v| v.stage));
    // the murk ends at 32 m
    let f = a.frame();
    assert_eq!(f.view[at + 1], mm(32_000));
    // under the water the shafts are on at 55%, looking down turns to the
    // look's dark green-blue (18, 48, 52), and the surface's sheen is 16%
    let value = |name: &str| {
        let (i, n) = render::uniform_at(name);
        f.view[i..i + n].to_vec()
    };
    assert_eq!(value("uShaftLook")[0], 55 * ONE / 100);
    assert_eq!(
        value("uDeep"),
        vec![18 * ONE / 255, 48 * ONE / 255, 52 * ONE / 255]
    );
    assert_eq!(value("uCaustic")[3], 16 * ONE / 100);
    // and the boat camera has none of it
    let (_, _, boat) = a.inset_frame();
    let (i, _) = render::uniform_at("uShaftLook");
    assert_eq!(boat.view[i], 0);
    // 26% of 1600 wide is 416, 60% of that tall is 249; 1% (16) from the
    // top-left corner, so 900 - 16 - 249 = 635 up from the bottom; the
    // border is 3 thousandths of 1600, 4 pixels
    let (rect, border, inset) = a.inset_frame();
    assert_eq!(rect, vec![16, 635, 416, 249]);
    assert_eq!(border, vec![12, 631, 424, 257]);
    assert_eq!(inset.view[at + 1], mm(120_000), "the inset is the boat camera");
    // the line and the bobber are drawn in both
    assert!(!inset.tris.is_empty());
    assert_eq!(hud(&a).depth, Some(a.t("fishing.depth")));
}

#[test]
fn holding_space_charges_a_meter() {
    let mut a = fishing();
    a.act("key", "Space").unwrap();
    for _ in 0..15 {
        tick(&mut a);
    }
    let h = hud(&a);
    // no words while charging: the bar says it (KISS)
    assert_eq!((h.key, h.prompt.clone()), ("charging", None));
    // 15 ticks of the sample's 50: 30%
    let fill = h.charge.unwrap();
    assert!((28..=32).contains(&fill), "{fill}");
}

#[test]
fn the_reel_bar_shows_the_zone_the_fish_and_the_catch_meter() {
    let mut a = fishing();
    let g = &mut a.play.as_mut().unwrap().game;
    let t = g.fishing.tuning.reel.clone();
    let line = &mut g.fishing.lines[0];
    line.stage = crate::sim::fishing::Stage::Reeling;
    line.fish = Some(0);
    line.reel = Some(Reel::new(crate::map::fish::Tier::Common, &t, false));
    let h = hud(&a);
    assert_eq!(h.prompt, None, "the reel bar needs no words");
    // the zone is 280 thousandths, centred: 36% to 64%; the fish at 50%;
    // the meter starts at 30%
    let r = h.reel.unwrap();
    assert_eq!((r.zone, r.fish, r.meter), ([36, 64], 50, 30));
}

#[test]
fn the_murk_darkens_with_the_cameras_depth() {
    let a = fishing();
    let lake = a.world.as_ref().unwrap();
    let u = &a.fishing_look().unwrap().under;
    // just under: the look's colour; half of deep_mm (6 m) down: halfway
    // to colour_deep; past it: all colour_deep
    let (top, _) = under_air(lake, u, v3(0, -mm(1), 0), 0);
    assert_eq!(top.fog, [56, 122, 118]);
    let (mid, _) = under_air(lake, u, v3(0, -mm(6000), 0), 0);
    assert_eq!(mid.fog, [42, 98, 101]);
    let (low, _) = under_air(lake, u, v3(0, -mm(20_000), 0), 0);
    assert_eq!(low.fog, [28, 74, 84]);
}

#[test]
fn specks_drift_round_the_eye_under_the_surface_and_over_the_bed() {
    let a = fishing();
    let lake = a.world.as_ref().unwrap();
    let m = &a.fishing_look().unwrap().under.motes;
    // over the spawn, 2 m down, looking north
    let eye = lake.world(lake.map.spawn.at, -2000);
    let cam = Camera {
        eye,
        fwd: v3(0, 0, -ONE),
        fov: deg(60),
        near: mm(50),
        far: mm(24_000),
    };
    let mut first = Vec::new();
    motes(lake, m, &cam, 0, &mut first);
    // two triangles of ten values a speck; some are drawn, none more than
    // there are
    let specks = first.len() / 60;
    assert!(specks > 20 && specks <= m.count as usize, "{specks}");
    let half = mm(m.box_mm) / 2 + mm(m.bubble_mm);
    for v in first.chunks(10) {
        assert!(v[1] < 0, "a speck over the surface");
        assert!((v[0] - eye.x).abs() <= half && (v[2] - eye.z).abs() <= half);
    }
    // a second later they have moved
    let mut later = Vec::new();
    motes(lake, m, &cam, 1000, &mut later);
    assert_ne!(first, later);
}
