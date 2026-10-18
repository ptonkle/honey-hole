//! The pregenerated map list (`data/maps/michigan-lake.json`): every seed
//! in it still makes the map it was accepted as, passes the validator, and
//! takes the example fish counts. Split in four so the parts run at once.

use std::fs;
use std::path::Path;

use tool_core::map::{fish, generate, list, validate, Setting};

fn listed() -> list::MapList {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/maps/michigan-lake.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    list::parse(&text).unwrap()
}

fn check_part(part: usize) {
    let list = listed();
    assert_eq!(list.setting, "michigan-lake");
    for e in list.maps.iter().skip(part).step_by(4) {
        let map = generate(Setting::MichiganLake, e.seed);
        assert_eq!(
            list::fingerprint(&map),
            e.stamp,
            "seed {} makes another map now: scan again (cargo run --release --example maps -- scan 0 256)",
            e.seed
        );
        let v = validate::check(&map);
        assert!(v.ok(), "seed {}: {:?}", e.seed, v.problems);
        fish::place(&map, &fish::EXAMPLE_COUNTS, e.seed as u64)
            .unwrap_or_else(|err| panic!("seed {}: {err}", e.seed));
    }
}

#[test]
fn listed_maps_part_1_are_unchanged_and_pass() {
    check_part(0);
}

#[test]
fn listed_maps_part_2_are_unchanged_and_pass() {
    check_part(1);
}

#[test]
fn listed_maps_part_3_are_unchanged_and_pass() {
    check_part(2);
}

#[test]
fn listed_maps_part_4_are_unchanged_and_pass() {
    check_part(3);
}

#[test]
fn the_list_holds_enough_maps() {
    let list = listed();
    assert!(list.maps.len() >= 50, "{} maps", list.maps.len());
    let mut seeds: Vec<u32> = list.maps.iter().map(|m| m.seed).collect();
    seeds.dedup();
    assert_eq!(seeds.len(), list.maps.len(), "a seed listed twice");
}
