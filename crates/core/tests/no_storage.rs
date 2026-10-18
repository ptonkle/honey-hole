//! Reference default (D17): the page keeps nothing between visits. This reads
//! every file under web/ (vendor/ aside) and fails if one names a browser
//! storage API. From Feedback Drafts and Status Trends.
//!
//! This project's change (BUILDER-SPEC, "Changes to the defaults"): the
//! person chose to keep settings between visits (`brief/questions.md` P3,
//! "Yes, in browser storage"). One file, `web/settings-store.js`, may name
//! `localStorage` and nothing else on the list, and it keeps one key. The
//! core writes what is stored and checks what comes back
//! (`crates/core/src/app/settings.rs`, where an unreadable save is treated
//! as no save).

use std::fs;
use std::path::{Path, PathBuf};

const STORAGE_NAMES: &[&str] = &[
    "localStorage",
    "sessionStorage",
    "indexedDB",
    "IDBFactory",
    "document.cookie",
    "caches",
    "CacheStorage",
    "serviceWorker",
    "navigator.storage",
    "openDatabase",
];

/// The one file allowed browser storage, and the one name it may use.
const SETTINGS_STORE: &str = "settings-store.js";
const SETTINGS_STORE_MAY_NAME: &str = "localStorage";

fn storage_names_in(text: &str) -> Vec<&'static str> {
    STORAGE_NAMES
        .iter()
        .copied()
        .filter(|name| text.contains(name))
        .collect()
}

/// The storage names a file under web/ may not use: all of them, but for
/// `localStorage` in the settings store.
fn refused_in(file: &str, text: &str) -> Vec<&'static str> {
    storage_names_in(text)
        .into_iter()
        .filter(|name| !(file == SETTINGS_STORE && *name == SETTINGS_STORE_MAY_NAME))
        .collect()
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(read) = fs::read_dir(&d) else {
            continue;
        };
        for entry in read {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                if path.file_name().map(|n| n != "vendor").unwrap_or(true) {
                    stack.push(path);
                }
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn web() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web")
}

#[test]
fn the_page_names_no_browser_storage_but_the_settings_store() {
    let web = web();
    let mut found = Vec::new();
    for path in files_under(&web) {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        // the store is allowed only at the top of web/
        let file = if path.parent() == Some(web.as_path()) {
            path.file_name().and_then(|n| n.to_str()).unwrap_or("")
        } else {
            ""
        };
        for name in refused_in(file, &text) {
            found.push(format!("{}: {name}", path.display()));
        }
    }
    assert!(
        found.is_empty(),
        "browser storage in the page (D17; only web/{SETTINGS_STORE} may name {SETTINGS_STORE_MAY_NAME}):\n{}",
        found.join("\n")
    );
}

#[test]
fn the_settings_store_keeps_one_key_and_wraps_every_access() {
    let text = fs::read_to_string(web().join(SETTINGS_STORE)).expect("web/settings-store.js");
    let keys: Vec<&str> = text
        .lines()
        .filter(|l| l.trim_start().starts_with("const KEY"))
        .collect();
    assert_eq!(keys.len(), 1, "one storage key: {keys:?}");
    let uses = text.matches("localStorage.").count();
    let wrapped = text.matches("try {").count();
    assert_eq!(uses, 2, "one read and one write");
    assert!(wrapped >= uses, "each access is inside a try");
    assert!(
        !text.contains("localStorage.getItem(\"") && !text.contains("localStorage.setItem(\""),
        "every access goes through the one KEY"
    );
}

#[test]
fn the_scanner_sees_storage() {
    assert_eq!(
        storage_names_in("window.localStorage.setItem('n', name)"),
        vec!["localStorage"]
    );
    assert_eq!(storage_names_in("indexedDB.open('x')"), vec!["indexedDB"]);
    assert!(storage_names_in("el(\"note\").value").is_empty());
    // the store's allowance is localStorage alone, and in no other file
    assert!(refused_in(SETTINGS_STORE, "localStorage.getItem(KEY)").is_empty());
    assert_eq!(
        refused_in(SETTINGS_STORE, "sessionStorage.getItem(KEY)"),
        vec!["sessionStorage"]
    );
    assert_eq!(
        refused_in("app.js", "localStorage.getItem(KEY)"),
        vec!["localStorage"]
    );
}
