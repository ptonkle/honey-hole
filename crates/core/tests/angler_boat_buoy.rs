//! The shipped boat, angler and item buoy (data/models/): they load, stay
//! low-poly, and fit together as the game uses them.

use std::fs;
use std::path::{Path, PathBuf};

use tool_core::gfx::geom::V3;
use tool_core::model::{self, Anim, Kind, Model};

fn load(id: &str) -> Model {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/models")
        .join(format!("{id}.json"));
    let json = fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    model::parse(id, &json).unwrap_or_else(|e| panic!("{e}"))
}

/// The lowest point of the posed model, in millimetres.
fn lowest_mm(m: &Model, anim: Anim, ms: u32) -> i64 {
    let t = m.transforms(&m.poses(Some(anim), ms));
    let low = m.verts.iter().map(|(v, p)| t[*p].apply(*v).y).min().unwrap();
    low as i64 * 1000 / 65536
}

/// The angler takes up to 800 faces for its tapered limbs and round head
/// (SECOND-ORDER R1.140); the boat and the buoy keep 400.
#[test]
fn the_boat_angler_and_buoy_load_as_their_kinds_in_a_few_hundred_faces() {
    for (id, kind, budget) in [
        ("boat", Kind::Boat, 400),
        ("angler", Kind::Figure, 800),
        ("item-buoy", Kind::Item, 400),
    ] {
        let m = load(id);
        assert_eq!(m.kind, kind, "{id}");
        assert!(m.faces.len() <= budget, "{id} has {} faces", m.faces.len());
    }
}

/// Where a part's point is at the start of a pose, in millimetres.
fn posed_mm(m: &Model, anim: Option<Anim>, part: usize, at: V3) -> [i64; 3] {
    let t = m.transforms(&m.poses(anim, 0));
    let p = t[part].apply(at);
    [p.x, p.y, p.z].map(|c| c as i64 * 1000 / 65536)
}

/// The angler stands 1.62 m to the top of its hat; only the rod reaches
/// higher.
#[test]
fn the_angler_stands_1_62_m_to_the_top_of_its_hat() {
    let m = load("angler");
    let rod = m.parts.iter().position(|p| p.name == "rod").expect("a rod");
    let top = m
        .verts
        .iter()
        .filter(|(_, part)| *part != rod)
        .map(|(v, _)| v.y as i64 * 1000 / 65536)
        .max()
        .unwrap();
    assert!((1610..=1630).contains(&top), "the top of the hat is at {top} mm");
}

/// The Fish-Boy straps on at `wrist_l` (R1.128): a point on the left
/// forearm, so it follows the elbow when the arm bends.
#[test]
fn the_fish_boy_straps_onto_the_left_forearm_and_follows_the_elbow() {
    let m = load("angler");
    let a = m.anchor("wrist_l").expect("a wrist_l anchor");
    assert_eq!(m.parts[a.part].name, "forearm_l");
    let rest = posed_mm(&m, None, a.part, a.at);
    assert!(rest[0] > 150, "the anchor is on the angler's left (+x): {rest:?}");
    assert!((650..=900).contains(&rest[1]), "the anchor is at forearm height: {rest:?}");
    let fishing = posed_mm(&m, Some(Anim::Fish), a.part, a.at);
    // holding the rod's butt, the left hand comes forward and in
    assert!(fishing[2] > rest[2] + 50, "fishing brings the forearm forward: {rest:?} to {fishing:?}");
    assert!(fishing[0] < rest[0] - 100, "fishing brings the forearm in: {rest:?} to {fishing:?}");
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
            assert!(
                (-20..=20).contains(&low),
                "{anim:?} at {at} ms: lowest point {low} mm"
            );
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
