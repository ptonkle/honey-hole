//! Copy tests: they read GLOSSARY.md, TONE.md, README.md and every string in
//! data/, and fail when the words drift.
//!
//! - Reference default (D15): every glossary entry has a definition, a number
//!   a rule decides is a {placeholder} the core fills, and no reader-visible
//!   string uses a word a glossary entry's "Not:" line rules out.
//! - Reference default (D16): no reader-visible string, glossary definition or
//!   README line uses a phrase listed between the lint markers in TONE.md.
//!
//! These read text, not meaning. A list of three made for rhythm, a slogan or
//! a claim the tool cannot back passes them; the person's reading covers
//! those. When a word is right in one place, add it to ALLOWED below with the
//! reason; do not take it out of the glossary or the tone file.

use std::fs;
use std::path::{Path, PathBuf};

use tool_core::glossary;

/// (file name, word, reason): a ruled-out word that is right in one file.
const ALLOWED: &[(&str, &str, &str)] = &[];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn read(name: &str) -> String {
    fs::read_to_string(root().join(name)).unwrap_or_else(|e| panic!("cannot read {name}: {e}"))
}

/// Every JSON file under data/ at any depth, skipping data/shaders.
fn data_json_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root().join("data")];
    while let Some(d) = stack.pop() {
        let Ok(read) = fs::read_dir(&d) else {
            continue;
        };
        for entry in read {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                if path.file_name().map(|n| n != "shaders").unwrap_or(true) {
                    stack.push(path);
                }
            } else if path.extension().map(|x| x == "json").unwrap_or(false) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Every string value in a JSON document, with its key path. Keys starting
/// with `_` hold notes for people and are skipped.
fn strings_in(value: &serde_json::Value, at: &str, out: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::String(s) => out.push((at.to_string(), s.clone())),
        serde_json::Value::Array(items) => {
            for (k, v) in items.iter().enumerate() {
                strings_in(v, &format!("{at}[{k}]"), out);
            }
        }
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                if !k.starts_with('_') {
                    strings_in(v, &format!("{at}.{k}"), out);
                }
            }
        }
        _ => {}
    }
}

/// (where, text) for every reader-visible string in data/.
fn reader_strings() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for path in data_json_files() {
        let text = fs::read_to_string(&path).expect("read data file");
        let value: serde_json::Value =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()));
        let name = path.strip_prefix(root()).unwrap_or(&path).display().to_string();
        strings_in(&value, &name, &mut out);
    }
    out
}

/// The phrases listed between `<!-- lint:start -->` and `<!-- lint:end -->`
/// in TONE.md, one per `- ` bullet.
fn tone_phrases() -> Vec<String> {
    let tone = read("TONE.md");
    let start = tone
        .find("<!-- lint:start -->")
        .expect("TONE.md has no <!-- lint:start --> marker");
    let end = tone
        .find("<!-- lint:end -->")
        .expect("TONE.md has no <!-- lint:end --> marker");
    tone[start..end]
        .lines()
        .filter_map(|l| l.trim().strip_prefix("- "))
        .map(|p| p.trim().trim_matches('`').to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\''
}

/// Whether `text` contains `phrase`, ignoring case. A phrase that starts and
/// ends with a letter must stand as whole words; anything else (`!`, `--`)
/// matches anywhere.
fn contains_phrase(text: &str, phrase: &str) -> bool {
    let hay = text.to_lowercase();
    let needle = phrase.to_lowercase();
    if needle.is_empty() {
        return false;
    }
    let wordy = needle.chars().next().map(is_word_char).unwrap_or(false)
        && needle.chars().last().map(is_word_char).unwrap_or(false);
    let mut from = 0;
    while let Some(at) = hay[from..].find(&needle) {
        let start = from + at;
        let end = start + needle.len();
        let before = hay[..start].chars().last();
        let after = hay[end..].chars().next();
        if !wordy || (!before.map(is_word_char).unwrap_or(false) && !after.map(is_word_char).unwrap_or(false))
        {
            return true;
        }
        from = start + needle.chars().next().map(char::len_utf8).unwrap_or(1);
    }
    false
}

fn allowed(place: &str, word: &str) -> bool {
    ALLOWED
        .iter()
        .any(|(file, w, _)| place.starts_with(file) && w.eq_ignore_ascii_case(word))
}

/// A digit outside a `{placeholder}`.
fn has_bare_digit(text: &str) -> bool {
    let mut depth = 0;
    for c in text.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            d if d.is_ascii_digit() && depth == 0 => return true,
            _ => {}
        }
    }
    false
}

fn glossary_entries() -> Vec<glossary::Entry> {
    glossary::parse(&read("GLOSSARY.md")).unwrap_or_else(|e| panic!("GLOSSARY.md: {}", e.message()))
}

#[test]
fn every_glossary_entry_is_complete_and_states_no_bare_number() {
    let mut problems = Vec::new();
    for e in glossary_entries() {
        if has_bare_digit(&e.definition) {
            problems.push(format!(
                "{}: write the number as a {{placeholder}} the core fills, so the glossary cannot disagree with the rule",
                e.term
            ));
        }
    }
    assert!(problems.is_empty(), "GLOSSARY.md:\n{}", problems.join("\n"));
}

#[test]
fn no_reader_string_uses_a_word_the_glossary_rules_out() {
    let entries = glossary_entries();
    let mut problems = Vec::new();
    for (place, text) in reader_strings() {
        for e in &entries {
            for word in &e.not {
                if contains_phrase(&text, word) && !allowed(&place, word) {
                    problems.push(format!(
                        "{place}: \"{word}\" is ruled out for \"{}\" in GLOSSARY.md: {text}",
                        e.term
                    ));
                }
            }
        }
    }
    assert!(problems.is_empty(), "glossary words:\n{}", problems.join("\n"));
}

#[test]
fn no_text_uses_a_phrase_the_tone_file_rules_out() {
    let phrases = tone_phrases();
    let mut texts = reader_strings();
    for e in glossary_entries() {
        texts.push((format!("GLOSSARY.md ({})", e.term), e.definition));
    }
    for (n, line) in read("README.md").lines().enumerate() {
        texts.push((format!("README.md:{}", n + 1), line.to_string()));
    }
    let mut problems = Vec::new();
    for (place, text) in &texts {
        for p in &phrases {
            if contains_phrase(text, p) && !allowed(place, p) {
                problems.push(format!("{place}: \"{p}\" (TONE.md): {text}"));
            }
        }
    }
    assert!(problems.is_empty(), "tone:\n{}", problems.join("\n"));
}

// ---- The lint is a test too: each listed phrase must be caught. ----

#[test]
fn the_tone_lint_catches_each_listed_phrase() {
    let phrases = tone_phrases();
    assert!(!phrases.is_empty(), "TONE.md lists no phrases to lint");
    for p in &phrases {
        let sample = format!("Here we {p} the plan.");
        assert!(contains_phrase(&sample, p), "the lint misses {p:?}");
    }
}

#[test]
fn phrase_matching_respects_word_edges() {
    assert!(contains_phrase("We delve into it.", "delve"));
    assert!(contains_phrase("Done!", "!"));
    assert!(!contains_phrase("The tapestries hang.", "tapestry"));
    assert!(!contains_phrase("Just-in-time is a term.", "just in time"));
    assert!(!contains_phrase("The landscaper came.", "landscape"));
    assert!(contains_phrase("Not only fast", "not only"));
}

#[test]
fn bare_digits_are_found_outside_placeholders() {
    assert!(has_bare_digit("Adds 8 Housing."));
    assert!(!has_bare_digit("Adds {brood_cell.housing} Housing."));
}
