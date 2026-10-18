//! Items on screen: the debug buoy, the HUD's slot and status, a minigame
//! taking the pointer, the popups, and your cloaked boat's tint. The
//! numbers are the shipped `data/ui2d.json`'s `items` section.

use super::*;
use crate::app::play::tests::{lobby, tick};

const THEME: &str = include_str!("../../../../../data/ui2d.json");

/// A match at 1600 × 900 with the shipped 2D look, its clock running.
fn started(flags: &str) -> App {
    let mut a = lobby();
    a.act("data.ui2d", THEME).unwrap();
    a.act("data.controls", include_str!("../../../../../data/controls.json"))
        .unwrap();
    a.act("debug.flags", flags).unwrap();
    a.resize(1600, 900, 100);
    a.act("lobby.start", "").unwrap();
    a.advance(0);
    tick(&mut a);
    a
}

fn status(a: &App) -> Vec<String> {
    a.build_ui2d().list.status.split(' ').map(String::from).collect()
}

fn has(a: &App, word: &str) -> bool {
    status(a).iter().any(|w| w == word)
}

#[test]
fn the_debug_flag_puts_a_buoy_ahead_and_driving_onto_it_fills_the_slot() {
    let mut a = started("items,item.nitrous");
    let g = &a.play.as_ref().unwrap().game;
    let b = g.items.buoy.expect("a buoy ahead");
    let me = g.boat(0).unwrap();
    let along = ahead(me.at(), me.heading, 40_000);
    assert!(b.dist(along) <= 100, "straight ahead: {b:?} {along:?}");
    let d = b.dist(me.at());
    assert!((39_900..=40_000).contains(&d), "40 m ahead, less rounding: {d}");
    assert!(has(&a, "buoy=1") && has(&a, "item=none"));
    a.play.as_mut().unwrap().game.boats[0].as_mut().unwrap().set_at(b);
    tick(&mut a);
    assert!(has(&a, "item=nitrous"), "{:?}", status(&a));
}

#[test]
fn a_full_screen_minigame_takes_the_pointer_and_esc_loses_the_item() {
    let mut a = started("items,item.nitrous");
    let b = a.play.as_ref().unwrap().game.items.buoy.unwrap();
    a.play.as_mut().unwrap().game.boats[0].as_mut().unwrap().set_at(b);
    tick(&mut a);
    assert!(!a.ui().pointer);
    a.act("key", "ShiftLeft").unwrap();
    tick(&mut a);
    a.act("key.up", "ShiftLeft").unwrap();
    assert!(a.ui().pointer, "the page lets the pointer go");
    assert!(has(&a, "mg=nitrous"), "{:?}", status(&a));
    // the minigame's 1600 × 900 canvas fills the 1600 × 900 view
    a.act("pointer.move", "400 300").unwrap();
    assert_eq!(a.fishboy.pointer, Some((400, 300)));
    a.act("key", "Escape").unwrap();
    tick(&mut a);
    assert!(!a.ui().pointer);
    assert!(has(&a, "item=none"));
    assert!(
        !a.play_panels().iter().any(|p| p.kind == "pause"),
        "Esc did not open the menu"
    );
}

#[test]
fn your_own_cloaked_boat_is_tinted_and_others_are_not() {
    let a = started("");
    let l = &a.fishboy.theme.as_ref().unwrap().items;
    let mut b = a.play_view().unwrap().world.boats[0].clone();
    let colour = [0, 0, ONE];
    assert_eq!(a.boat_colour(colour, &b, 0), colour);
    b.cloaked = true;
    let tint = render::rgb(l.cloak_tint);
    // 40% of the way from blue to the tint
    let want = [0, 1, 2].map(|k| colour[k] + (tint[k] - colour[k]) * 40 / 100);
    assert_eq!(a.boat_colour(colour, &b, 0), want);
    assert_eq!(
        a.boat_colour(colour, &b, 1),
        colour,
        "another player sees it as ever"
    );
}

#[test]
fn a_click_on_the_radar_picks_the_teleports_destination() {
    let mut a = started("items,item.teleporter,quick");
    let b = a.play.as_ref().unwrap().game.items.buoy.unwrap();
    a.play.as_mut().unwrap().game.boats[0].as_mut().unwrap().set_at(b);
    tick(&mut a);
    a.act("key", "ShiftLeft").unwrap();
    tick(&mut a);
    a.act("key.up", "ShiftLeft").unwrap();
    for _ in 0..30 {
        tick(&mut a);
    }
    assert!(has(&a, "choosing=1"), "{:?}", status(&a));
    let r = a.fishboy_raised().unwrap().px;
    // the middle of the radar's map square, in CSS pixels
    let (x, y) = (r.x + r.w * 436 / 1600, r.y + r.h * 490 / 900);
    a.act("pointer.move", &format!("{x} {y}")).unwrap();
    assert!(a.fishboy.pointer.is_some());
    a.act("pointer.down", &format!("{x} {y}")).unwrap();
    tick(&mut a);
    a.act("pointer.up", &format!("{x} {y}")).unwrap();
    let g = &a.play.as_ref().unwrap().game;
    assert!(g.items.seats[0].charge.is_some(), "the charge begins");
}

#[test]
fn the_flow_announces_a_pickup_with_the_items_name_and_a_wasted_item() {
    let mut a = started("items,item.nitrous");
    a.act("data.flow", include_str!("../../../../../data/flow.json"))
        .unwrap();
    let b = a.play.as_ref().unwrap().game.items.buoy.unwrap();
    a.play.as_mut().unwrap().game.boats[0].as_mut().unwrap().set_at(b);
    tick(&mut a);
    let texts = |a: &App| -> Vec<String> { a.flow.popups.queue.iter().map(|p| p.text.clone()).collect() };
    let picked = a.t("popup.item_picked").replace("{item}", &a.t("item.nitrous"));
    assert!(texts(&a).contains(&picked), "{:?}", texts(&a));
    // the flow's HUD draws the slot with the item's name
    assert!(has(&a, "item=nitrous"));
    a.act("key", "ShiftLeft").unwrap();
    tick(&mut a);
    a.act("key", "Escape").unwrap();
    tick(&mut a);
    let lost = a.t("popup.item_wasted").replace("{item}", &a.t("item.nitrous"));
    assert!(texts(&a).contains(&lost), "{:?}", texts(&a));
}
