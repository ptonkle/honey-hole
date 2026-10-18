//! The Burt Lake roster's fish models (data/models/): each is a fish the
//! core accepts, at a real adult's length, low-poly, with an idle swim and a
//! panic swim that is about three times faster and bends harder
//! (DECISIONS.md, "questions before the night"; SECOND-ORDER R1.43, R2.110).

use std::fs;
use std::path::{Path, PathBuf};

use tool_core::model::{self, Anim, Kind, Model};

/// Every fish on the Burt Lake roster with its base length in millimetres:
/// a catch can be up to 1.5 times the base, so 1.5 times is a trophy and not
/// past a record (R2.111).
const ROSTER: [(&str, i32); 13] = [
    ("bluegill", 170),
    ("yellow-perch", 200),
    ("pumpkinseed", 150),
    ("rock-bass", 180),
    ("smallmouth-bass", 350),
    ("largemouth-bass", 380),
    ("walleye", 430),
    ("black-crappie", 220),
    ("northern-pike", 650),
    ("channel-catfish", 550),
    ("lake-trout", 600),
    ("muskellunge", 950),
    ("lake-sturgeon", 1500),
];

/// The most faces one fish may have: a lake holds dozens of fish at once.
const MAX_FACES: usize = 600;

fn load(id: &str) -> Model {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/models")
        .join(format!("{id}.json"));
    let json = fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    model::parse(id, &json).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn every_roster_fish_is_a_low_poly_fish_at_its_base_length() {
    for (id, base) in ROSTER {
        let m = load(id);
        assert_eq!(m.kind, Kind::Fish, "{id}");
        assert!(m.faces.len() <= MAX_FACES, "{id} has {} faces", m.faces.len());
        let len = m.length_mm();
        // within a centimetre: whiskers or a fin may reach a little past
        assert!((len - base).abs() <= 10, "{id} is {len} mm long, not {base}");
    }
}

/// The largest turn any part makes in any key of a swim, in angle units.
fn bend(m: &Model, anim: Anim) -> i32 {
    let a = m.animation(anim).unwrap();
    a.keys
        .iter()
        .flat_map(|k| k.poses.iter())
        .flat_map(|p| p.turn.iter())
        .map(|t| t.abs())
        .max()
        .unwrap()
}

/// Each swim is a wave sampled at 12 keys and played straight; the panic
/// swim runs 2.5 to 4 times as fast as the idle swim and bends further.
#[test]
fn every_roster_fish_swims_at_ease_and_in_a_panic() {
    for (id, _) in ROSTER {
        let m = load(id);
        let idle = m
            .animation(Anim::Idle)
            .unwrap_or_else(|| panic!("{id} has no idle swim"));
        let panic = m
            .animation(Anim::Panic)
            .unwrap_or_else(|| panic!("{id} has no panic swim"));
        for a in [idle, panic] {
            assert_eq!(a.keys.len(), 12, "{id}");
            assert!(!a.smooth, "{id}: a sampled wave plays linear");
        }
        // 2.5 <= idle / panic <= 4, in whole numbers
        assert!(
            idle.ms * 2 >= panic.ms * 5 && idle.ms <= panic.ms * 4,
            "{id}: idle {} ms, panic {} ms",
            idle.ms,
            panic.ms
        );
        assert!(
            bend(&m, Anim::Panic) > bend(&m, Anim::Idle),
            "{id}: the panic swim bends no further"
        );
    }
}
