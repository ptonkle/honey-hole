//! The lake's world props (data/models/): each file is a prop the core
//! accepts, stands on its own origin, is cheap enough for map generation to
//! place many times, and the plants move (SECOND-ORDER R1.100 to R1.104).

use std::fs;
use std::path::{Path, PathBuf};

use tool_core::model::{self, Anim, Kind, Model};

/// Every prop of the first map setting, by file name.
const PROPS: [&str; 11] = [
    "pine",
    "pine-tall",
    "pine-small",
    "birch",
    "cattails",
    "boulder",
    "shore-rocks",
    "fallen-log",
    "dock",
    "lily-pads",
    "cabin",
];

/// The most triangles one prop may draw: the props are placed by the
/// hundred, so each stays near a hundred triangles (R1.101).
const MAX_TRIANGLES: usize = 160;

fn load(id: &str) -> Model {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/models")
        .join(format!("{id}.json"));
    let json = fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    model::parse(id, &json).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn every_lake_prop_is_a_prop_the_core_accepts() {
    for id in PROPS {
        assert_eq!(load(id).kind, Kind::Prop, "{id}");
    }
}

/// The origin is where map generation puts the prop: on the ground (or the
/// water's surface), under the prop's footprint. Nothing hangs more than a
/// metre below it (the dock's posts reach 1 m down to the lake bed), the
/// lowest point is at most 50 mm above it (lily pads float 25 mm up, clear
/// of the water's face), and the prop rises above it.
#[test]
fn every_prop_stands_on_its_origin() {
    for id in PROPS {
        let m = load(id);
        let (lo, hi) = (m.min_mm, m.max_mm);
        assert!(lo[0] <= 0 && 0 <= hi[0], "{id}: the origin is beside the prop in x");
        assert!(lo[2] <= 0 && 0 <= hi[2], "{id}: the origin is beside the prop in z");
        assert!((-1000..=50).contains(&lo[1]), "{id}: the base is at {} mm", lo[1]);
        assert!(hi[1] > 0, "{id}: nothing stands above the ground");
    }
}

#[test]
fn every_prop_is_cheap_enough_to_place_many_times() {
    for id in PROPS {
        let m = load(id);
        let triangles: usize = m.faces.iter().map(|f| f.v.len() - 2).sum();
        assert!(triangles <= MAX_TRIANGLES, "{id}: {triangles} triangles");
    }
}

#[test]
fn the_plants_sway_and_the_lily_pads_bob() {
    for id in ["pine", "pine-tall", "pine-small", "birch", "cattails"] {
        assert!(load(id).animation(Anim::Sway).is_some(), "{id} does not sway");
    }
    assert!(load("lily-pads").animation(Anim::Bob).is_some(), "the lily pads do not bob");
    for id in ["boulder", "shore-rocks", "fallen-log", "dock", "cabin"] {
        assert!(load(id).animations.is_empty(), "{id} moves");
    }
}
