//! Fishing on screen: the underwater camera and its picture-in-picture, the
//! HUD's meters and the popups. Expected values are worked by hand from
//! `data/look.json`'s fishing section and the sample tuning.

use super::*;
use crate::app::lake::tests::{models, LOOK};
use crate::app::play::tests::{lobby, tick};
use crate::sim::fishing::reel::Reel;
use crate::view::Item;
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

fn panels_of(a: &App, kind: &str) -> Vec<Panel> {
    a.play_panels().into_iter().filter(|p| p.kind == kind).collect()
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
    // the murk ends at 16 m
    let f = a.frame();
    assert_eq!(f.view[at + 1], mm(16_000));
    // 26% of 1600 wide is 416, 60% of that tall is 249; 1% (16) from the
    // top-left corner, so 900 - 16 - 249 = 635 up from the bottom; the
    // border is 3 thousandths of 1600, 4 pixels
    let (rect, border, inset) = a.inset_frame();
    assert_eq!(rect, vec![16, 635, 416, 249]);
    assert_eq!(border, vec![12, 631, 424, 257]);
    assert_eq!(inset.view[at + 1], mm(120_000), "the inset is the boat camera");
    // the line and the bobber are drawn in both
    assert!(!inset.tris.is_empty());
    let panel = &panels_of(&a, "fishing")[0];
    assert_eq!(panel.sub, a.t("fishing.depth"));
}

#[test]
fn holding_space_charges_a_meter() {
    let mut a = fishing();
    a.act("key", "Space").unwrap();
    for _ in 0..15 {
        tick(&mut a);
    }
    let p = &panels_of(&a, "fishing")[0];
    assert_eq!(p.heading, a.t("fishing.charging"));
    // 15 ticks of the sample's 50: 30%
    match &p.items[0] {
        Item::Meter { fill, text, .. } => {
            assert!((28..=32).contains(fill), "{fill}");
            assert_eq!(text, &format!("{fill}%"));
        }
        other => panic!("{other:?}"),
    }
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
    let p = &panels_of(&a, "fishing")[0];
    assert_eq!(p.heading, a.t("fishing.reeling"));
    // the zone is 280 thousandths, centred: 36% to 64%; the fish at 50%;
    // the meter starts at 30%
    assert!(
        matches!(
            &p.items[0],
            Item::Meter {
                zone: Some([36, 64]),
                marker: Some(50),
                vertical: true,
                ..
            }
        ),
        "{:?}",
        p.items[0]
    );
    assert!(
        matches!(
            &p.items[1],
            Item::Meter {
                fill: 30,
                vertical: true,
                ..
            }
        ),
        "{:?}",
        p.items[1]
    );
}

#[test]
fn a_hook_pops_up_for_every_player_in_its_tier_for_two_seconds() {
    let mut a = fishing();
    let p = a.play.as_mut().unwrap();
    let now = p.game.tick;
    p.log.push((
        now,
        MatchEvent::Hooked {
            seat: 1,
            fish: 0,
            kind: "walleye".into(),
            tier: crate::map::fish::Tier::Uncommon,
        },
    ));
    let pops = panels_of(&a, "popup tier-uncommon");
    assert_eq!(pops.len(), 1);
    let who = a.play.as_ref().unwrap().game.setup.seats[1].name.clone();
    let want = a
        .t("popup.hooked")
        .replace("{player}", &who)
        .replace("{fish}", "Walleye");
    assert_eq!(pops[0].heading, want);
    assert_eq!(pops[0].sub, a.t("tier.uncommon"));
    // 2 s is 100 ticks: gone then
    for _ in 0..99 {
        tick(&mut a);
    }
    assert_eq!(panels_of(&a, "popup tier-uncommon").len(), 1);
    tick(&mut a);
    assert!(panels_of(&a, "popup tier-uncommon").is_empty());
}
