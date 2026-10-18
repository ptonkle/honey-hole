//! The shipped boat, angler and item buoy (data/models/): they load, stay
//! low-poly, and fit together as the game uses them.

use std::fs;
use std::path::{Path, PathBuf};

use tool_core::model::{self, Anim, Kind, Model};

fn load(id: &str) -> Model {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/models")
        .join(format!("{id}.json"));
    let json =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    model::parse(id, &json).unwrap_or_else(|e| panic!("{e}"))
}

/// The lowest point of the posed model, in millimetres.
fn lowest_mm(m: &Model, anim: Anim, ms: u32) -> i64 {
    let t = m.transforms(&m.poses(Some(anim), ms));
    let low = m.verts.iter().map(|(v, p)| t[*p].apply(*v).y).min().unwrap();
    low as i64 * 1000 / 65536
}

#[test]
fn the_boat_angler_and_buoy_load_as_their_kinds_in_a_few_hundred_faces() {
    for (id, kind) in [("boat", Kind::Boat), ("angler", Kind::Figure), ("item-buoy", Kind::Item)] {
        let m = load(id);
        assert_eq!(m.kind, kind, "{id}");
        assert!(m.faces.len() <= 400, "{id} has {} faces", m.faces.len());
    }
}

/// The figure's origin is at its feet, so in either pose its feet rest on
/// the anchor's floor: not sunk through it, not floating above it.
#[test]
fn the_angler_keeps_its_feet_on_the_anchor_seated_and_standing() {
    let m = load("angler");
    for anim in [Anim::Drive, Anim::Fish] {
        let ms = m.animation(anim).unwrap().ms;
        for at in [0, ms / 2] {
            let low = lowest_mm(&m, anim, at);
            assert!((-20..=20).contains(&low), "{anim:?} at {at} ms: lowest point {low} mm");
        }
    }
}

/// The driver sits at the stern on the floor; the angler fishes from the
/// raised casting deck at the bow. The hull wears the player's colour.
#[test]
fn the_boat_seats_the_driver_aft_and_the_angler_on_the_raised_bow_deck() {
    let m = load("boat");
    let drive = m.anchor("drive").expect("a drive anchor");
    let fish = m.anchor("fish").expect("a fish anchor");
    assert!(drive.at.z < 0, "the driver sits behind the middle");
    assert!(fish.at.z > 0, "the angler stands ahead of the middle");
    assert!(fish.at.y > drive.at.y, "the casting deck is above the floor");
    assert!(m.has_slot());
}

#[test]
fn the_item_buoy_bobs() {
    let m = load("item-buoy");
    let bob = m.animation(Anim::Bob).expect("a bob animation");
    assert_ne!(m.poses(Some(Anim::Bob), 0), m.poses(Some(Anim::Bob), bob.ms / 4));
}
