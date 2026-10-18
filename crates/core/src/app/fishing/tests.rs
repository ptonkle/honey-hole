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

// ---- the bite's cue (T4, cue.rs) ----

/// The look's bite: the bobber 260 mm under in 80 ms; the line's middle
/// shaking 150 mm, a 120 ms period, gone by 450 ms; three rings 150 ms
/// apart, each 450 ms long, so the last is gone by 750 ms, inside the
/// 0.75 s hook window.
#[test]
fn the_bite_cue_runs_its_course_inside_the_hook_window() {
    let a = fishing();
    let b = &a.fishing_look().unwrap().bite;
    assert_eq!([0, 40, 80, 500].map(|ms| b.plunge_mm(ms)), [0, 130, 260, 260]);
    // a quarter period in (30 ms) the shake is at its widest, less the
    // 30/450 it has died away: 150 * 420 / 450 = 140 (to the sine's
    // rounding); none at the start, nor from 450 ms
    assert!((138..=141).contains(&b.twitch_mm(30)), "{}", b.twitch_mm(30));
    assert_eq!((b.twitch_mm(0), b.twitch_mm(450)), (0, 0));
    // a ring is 32 segments of two triangles, 30 values each
    let rings = |ms| {
        let mut out = Vec::new();
        b.rings(&mut out, v3(0, 0, 0), ms);
        out.len() / (32 * 2 * 30)
    };
    assert_eq!([0, 200, 400, 700, 750].map(rings), [1, 2, 3, 1, 0]);
    // the first ring 90 ms in (a fifth of its 450) is a fifth of the way
    // out, 100 + 1900 / 5 = 480 mm round, four fifths of its 280 mm width:
    // from 368 to 592 mm, 20 mm up
    let mut out = Vec::new();
    b.rings(&mut out, v3(0, 0, 0), 90);
    for v in out.chunks(10) {
        let r = fx::isqrt((v[0] as i64 * v[0] as i64 + v[2] as i64 * v[2] as i64) as u128) as i32;
        assert!(r >= mm(366) && r <= mm(594), "{}", scene::to_mm(r));
        assert_eq!(v[1], mm(20));
    }
}

/// Seat 0's lure 10 m ahead of its bow a metre down, with a fish at it;
/// `biting` starts its bite five ticks (100 ms) ago.
fn lure_with_fish(a: &mut App, biting: bool) -> P {
    use crate::sim::fishing::lake::FishState;
    use crate::sim::fishing::Interest;
    let g = &mut a.play.as_mut().unwrap().game;
    let rod = g.rod(0);
    let at = P::new(rod.x + 10_000, rod.y);
    let window = crate::sim::ticks(g.fishing.tuning.hook_window_ms);
    // a minute in, so five ticks ago is in the match
    g.tick = 3000;
    let tick = g.tick;
    let line = &mut g.fishing.lines[0];
    line.stage = Stage::Set;
    line.bobber = at;
    line.lure_depth_mm = 1000;
    line.length_mm = 40_000;
    line.interest = Some(Interest {
        fish: 0,
        arrived: true,
        next: u32::MAX,
        nibbles_left: 0,
        biting_until: biting.then(|| tick + window - 5),
    });
    let f = &mut g.fishing.fish[0];
    f.at = at;
    f.depth_mm = 1000;
    f.state = FishState::Lured { seat: 0 };
    at
}

/// The vertices of `tris` in colour `c` (look values).
fn in_colour(tris: &[i32], c: [i32; 3]) -> Vec<&[i32]> {
    let c = render::rgb(c);
    tris.chunks(render::VERTEX_VALUES)
        .filter(|v| v[6..9] == c[..])
        .collect()
}

/// The top of the bobber's red half drawn within 300 mm of `w`, world y.
fn bobber_top(f: &Frame, w: V3) -> i32 {
    in_colour(&f.tris, [222, 40, 36])
        .into_iter()
        .filter(|v| (v[0] - w.x).abs() <= mm(300) && (v[2] - w.z).abs() <= mm(300))
        .map(|v| v[1])
        .max()
        .expect("the bobber is drawn")
}

#[test]
fn on_the_bite_the_bobber_plunges_in_rings_and_bubbles_burst_from_the_lure() {
    let mut tops = Vec::new();
    for biting in [false, true] {
        let mut a = fishing();
        let at = lure_with_fish(&mut a, biting);
        let lake = a.world.as_ref().unwrap();
        let w = lake.world(at, 0);
        let look = a.fishing_look().unwrap().bite.clone();
        // over the water (the picture-in-picture's boat camera): 100 ms in,
        // the bobber is all the way under and one ring is out
        let (_, _, boat) = a.inset_frame();
        let rings = in_colour(&boat.tris, look.ring).len();
        tops.push(bobber_top(&boat, w));
        // under the water: the bubbles round the lure
        let under = a.frame();
        let bubbles = in_colour(&under.tris, look.bubble).len();
        if biting {
            // one ring and the 14 drops of the splash, in the same colour
            assert_eq!(rings, 32 * 2 * 3 + 14 * 2 * 3);
            // 18 bubbles of two triangles, all a metre down
            assert_eq!(bubbles, 18 * 2 * 3);
        } else {
            assert_eq!((rings, bubbles), (0, 0));
        }
    }
    // the bobber is 260 mm lower on the bite (to a unit's rounding)
    assert!((tops[0] - tops[1] - mm(260)).abs() <= 2, "{tops:?}");
}
