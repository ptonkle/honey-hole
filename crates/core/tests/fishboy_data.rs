//! The Fish-Boy's shipped data files: the 2D theme, the fish roster and the
//! radar's tuning are the ones the core accepts, and the roster is the
//! person's (DECISIONS.md, "questions before the night").

use std::fs;
use std::path::{Path, PathBuf};

use tool_core::roster::{tier_key, Roster};
use tool_core::ui2d::theme::Theme;

fn data(name: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

#[test]
fn the_shipped_theme_roster_and_tuning_are_accepted() {
    Theme::parse(&data("ui2d.json")).unwrap();
    Roster::parse(&data("fish.json")).unwrap();
    let t = tool_core::tuning::parse(&data("tuning.json")).unwrap();
    // the brief: a 5 s cooldown and a sweep of about 3 s
    assert_eq!((t.radar.ping_cooldown_ms, t.radar.ping_sweep_ms), (5000, 3000));
}

#[test]
fn the_roster_is_the_persons_thirteen_fish_in_their_tiers() {
    let r = Roster::parse(&data("fish.json")).unwrap();
    let got: Vec<(&str, &str)> = r.0.iter().map(|k| (k.id.as_str(), tier_key(k.tier))).collect();
    let want = [
        ("muskellunge", "legendary"),
        ("lake-sturgeon", "legendary"),
        ("northern-pike", "rare"),
        ("channel-catfish", "rare"),
        ("lake-trout", "rare"),
        ("smallmouth-bass", "uncommon"),
        ("largemouth-bass", "uncommon"),
        ("walleye", "uncommon"),
        ("black-crappie", "uncommon"),
        ("bluegill", "common"),
        ("yellow-perch", "common"),
        ("pumpkinseed", "common"),
        ("rock-bass", "common"),
    ];
    assert_eq!(got, want);
}
