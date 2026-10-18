//! The Fish-Boy's model file: what the game will rely on when it raises the
//! device to the player's face and draws the radar, the Fishpedia and the
//! minigames on its screen (BRIEF.md, "Fish-Boy").

use std::fs;
use std::path::Path;

use serde_json::Value;
use tool_core::model::{self, Kind};

fn file() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models/fishboy.json");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Every vertex in millimetres, with the name of its part.
fn vertices(v: &Value) -> Vec<(String, [i64; 3])> {
    let mut out = Vec::new();
    for part in v["parts"].as_array().unwrap() {
        let name = part["name"].as_str().unwrap().to_string();
        for p in part["vertices"].as_array().unwrap() {
            let c: Vec<i64> = p
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c.as_i64().unwrap())
                .collect();
            out.push((name.clone(), [c[0], c[1], c[2]]));
        }
    }
    out
}

#[test]
fn the_fish_boy_is_a_device_the_core_accepts() {
    let m = model::parse("fishboy", &file()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((m.name.as_str(), m.kind), ("Fish-Boy", Kind::Device));
}

/// The game draws the screen's contents on one flat 16:9 rectangle facing
/// forward (+z), found by the part and anchor named "screen".
#[test]
fn the_screen_is_a_flat_16_by_9_rectangle_facing_forward_with_its_anchor_in_the_middle() {
    let v: Value = serde_json::from_str(&file()).unwrap();
    let screen: Vec<[i64; 3]> = vertices(&v)
        .into_iter()
        .filter(|(part, _)| part == "screen")
        .map(|(_, p)| p)
        .collect();
    assert_eq!(screen.len(), 4, "the screen is four corners");
    assert!(
        screen.iter().all(|p| p[2] == 0),
        "the screen is flat at z = 0: {screen:?}"
    );
    let xs: Vec<i64> = screen.iter().map(|p| p[0]).collect();
    let ys: Vec<i64> = screen.iter().map(|p| p[1]).collect();
    for p in &screen {
        assert!(
            p[0].abs() == 96 && p[1].abs() == 54,
            "a corner off the 192 x 108 mm rectangle: {p:?}"
        );
    }
    assert_eq!(
        xs.iter().sum::<i64>() + ys.iter().sum::<i64>(),
        0,
        "the four corners differ"
    );

    // its one face, wound to face +z (seen from in front, anticlockwise)
    let m = model::parse("fishboy", &file()).unwrap();
    let screen_part = m.parts.iter().position(|p| p.name == "screen").unwrap();
    let faces: Vec<_> = m
        .faces
        .iter()
        .filter(|f| f.v.iter().all(|&i| m.verts[i].1 == screen_part))
        .collect();
    assert_eq!(faces.len(), 1);
    let corners: Vec<_> = faces[0].v.iter().map(|&i| m.verts[i].0).collect();
    let n = model::newell(&corners);
    assert!(
        n.z > 65_000 && n.x == 0 && n.y == 0,
        "the screen faces {n:?}, not +z"
    );

    let anchor = m.anchor("screen").expect("an anchor \"screen\"");
    assert_eq!(anchor.part, screen_part);
    assert_eq!(v["anchors"]["screen"]["at"], serde_json::json!([0, 0, 0]));
}

/// Nothing stands in front of the screen inside its rectangle, and the bezel's
/// front reaches 9/8 of the screen each way: a view fitting the screen to 80%
/// shows the bezel to 90% (BRIEF.md: screen 80%, bezel 10%).
#[test]
fn nothing_covers_the_screen_and_the_bezel_frames_it_at_nine_eighths() {
    let v: Value = serde_json::from_str(&file()).unwrap();
    let all = vertices(&v);
    for (part, p) in &all {
        if part != "screen" && p[2] > 0 {
            assert!(
                p[0].abs() >= 96 || p[1].abs() >= 54,
                "{part} has a vertex {p:?} in front of the screen"
            );
        }
    }
    let front: Vec<&[i64; 3]> = all.iter().filter(|(_, p)| p[2] >= 0).map(|(_, p)| p).collect();
    let wide = front.iter().map(|p| p[0].abs()).max().unwrap();
    let tall = front.iter().map(|p| p[1].abs()).max().unwrap();
    // 96 x 9/8 = 108; 54 x 9/8 = 60.75, so 61
    assert_eq!((wide, tall), (108, 61));
    // the bezel stands 3 mm proud (its rivets a little more), so perspective
    // barely changes it
    let proud = front
        .iter()
        .filter(|p| p[0].abs() > 100)
        .map(|p| p[2])
        .max()
        .unwrap();
    assert!(proud <= 5, "the bezel stands {proud} mm proud");
}
