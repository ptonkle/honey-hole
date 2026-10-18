//! The patch notes (`data/patch-notes.json`, newest first): what each
//! version changed, for the What's New screen. The page fetches the file
//! and hands it to the core, which checks it before any screen shows it.
//!
//! A version is three numbers, major.minor.patch. One whose last number is
//! not 0 (0.1.2) is a minor version: fixes only. The JSON carries no flag
//! for it, so the flag cannot disagree with the number.

use serde::Deserialize;

/// One version's notes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub version: String,
    pub date: String,
    pub notes: Vec<String>,
}

impl Entry {
    /// A minor version: the last number is not 0.
    pub fn minor(&self) -> bool {
        parse_version(&self.version).is_some_and(|v| v.2 != 0)
    }
}

/// "0.10.2" as (0, 10, 2).
pub fn parse_version(version: &str) -> Option<(u32, u32, u32)> {
    let mut it = version.split('.');
    let a = it.next()?.parse().ok()?;
    let b = it.next()?.parse().ok()?;
    let c = it.next()?.parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    Some((a, b, c))
}

/// A date written YYYY-MM-DD (its shape, not the calendar).
fn is_date(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
}

/// Parses and checks the notes: at least one version, each with a version
/// number, a date and at least one line, listed newest first.
pub fn parse(json: &str) -> Result<Vec<Entry>, String> {
    let entries: Vec<Entry> =
        serde_json::from_str(json).map_err(|e| format!("The patch notes could not be read: {e}"))?;
    if entries.is_empty() {
        return Err("The patch notes list no version.".into());
    }
    for e in &entries {
        if parse_version(&e.version).is_none() {
            return Err(format!(
                "The patch notes version \"{}\" is not three numbers.",
                e.version
            ));
        }
        if !is_date(&e.date) {
            return Err(format!(
                "The patch notes for {} have the date \"{}\"; write it YYYY-MM-DD.",
                e.version, e.date
            ));
        }
        if e.notes.iter().all(|l| l.trim().is_empty()) {
            return Err(format!("The patch notes for {} have no line.", e.version));
        }
    }
    for w in entries.windows(2) {
        if parse_version(&w[0].version) <= parse_version(&w[1].version) {
            return Err(format!(
                "The patch notes list {} above {}; put the newest first.",
                w[0].version, w[1].version
            ));
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO: &str = r#"[
        {"version": "0.2.1", "date": "2026-10-18", "notes": ["A fix."]},
        {"version": "0.2.0", "date": "2026-10-17", "notes": ["A screen."]}
    ]"#;

    #[test]
    fn notes_are_read_newest_first_and_a_minor_version_is_known_by_its_last_number() {
        let all = parse(TWO).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].version, "0.2.1");
        assert!(all[0].minor());
        assert!(!all[1].minor());
    }

    #[test]
    fn notes_out_of_order_are_refused() {
        let swapped = r#"[
            {"version": "0.2.0", "date": "2026-10-17", "notes": ["A screen."]},
            {"version": "0.2.1", "date": "2026-10-18", "notes": ["A fix."]}
        ]"#;
        assert_eq!(
            parse(swapped).unwrap_err(),
            "The patch notes list 0.2.0 above 0.2.1; put the newest first."
        );
    }

    #[test]
    fn a_version_compares_by_number_not_by_text() {
        // "0.10.0" sorts before "0.9.0" as text, after it as numbers
        let ok = r#"[
            {"version": "0.10.0", "date": "2026-10-18", "notes": ["B."]},
            {"version": "0.9.0", "date": "2026-10-17", "notes": ["A."]}
        ]"#;
        assert!(parse(ok).is_ok());
    }

    #[test]
    fn a_bad_version_date_or_empty_entry_is_refused_by_name() {
        let one =
            |v: &str, d: &str, l: &str| format!(r#"[{{"version": "{v}", "date": "{d}", "notes": ["{l}"]}}]"#);
        assert_eq!(
            parse(&one("0.1", "2026-10-17", "A.")).unwrap_err(),
            "The patch notes version \"0.1\" is not three numbers."
        );
        assert_eq!(
            parse(&one("0.1.0", "17/10/2026", "A.")).unwrap_err(),
            "The patch notes for 0.1.0 have the date \"17/10/2026\"; write it YYYY-MM-DD."
        );
        assert_eq!(
            parse(&one("0.1.0", "2026-10-17", " ")).unwrap_err(),
            "The patch notes for 0.1.0 have no line."
        );
        assert_eq!(parse("[]").unwrap_err(), "The patch notes list no version.");
    }
}
