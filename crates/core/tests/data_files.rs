//! The real data files: the strings and patch notes the page fetches are the
//! ones the core accepts, and the newest patch notes are this version's.

use std::fs;
use std::path::{Path, PathBuf};

use tool_core::app::{App, VERSION};
use tool_core::patch_notes;

fn data(name: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

#[test]
fn the_shipped_strings_and_patch_notes_start_the_app() {
    if let Err(e) = App::new(&data("strings.json"), &data("patch-notes.json")) {
        panic!("{e}");
    }
}

/// A version cannot go out without its notes.
#[test]
fn the_newest_patch_notes_are_this_version() {
    let notes = patch_notes::parse(&data("patch-notes.json")).unwrap();
    assert_eq!(
        notes[0].version, VERSION,
        "add this version's entry to the top of data/patch-notes.json"
    );
}

/// Every model file the build bundles is one the core accepts, under the
/// name the gallery and the game will know it by.
#[test]
fn every_model_file_is_accepted() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("data/models")
        .map(|e| e.expect("entry").path())
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "data/models holds no model");
    for path in paths {
        let id = path.file_stem().unwrap().to_str().unwrap().to_string();
        assert_eq!(
            path.extension().and_then(|x| x.to_str()),
            Some("json"),
            "{}",
            path.display()
        );
        assert!(
            id.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "{id}: a model's file name is lowercase words joined by hyphens"
        );
        let text = fs::read_to_string(&path).unwrap();
        if let Err(e) = tool_core::model::parse(&id, &text) {
            panic!("{e}");
        }
    }
}
