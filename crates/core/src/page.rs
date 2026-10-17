//! The placeholder rule: the page's text comes from `data/strings.json`, and
//! the core decides whether that file is usable before the page draws it.
//! Replace this module with your tool's rules.

use serde::{Deserialize, Serialize};

/// The reader-visible strings the page needs. They live in data, not source.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Strings {
    pub title: String,
    pub intro: String,
}

/// Why a strings file cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringsError {
    NotJson(String),
    Empty(&'static str),
}

impl StringsError {
    /// The message the page shows. Wording that depends on the rule stays
    /// beside the rule.
    pub fn message(&self) -> String {
        match self {
            StringsError::NotJson(detail) => {
                format!("The strings file could not be read: {detail}")
            }
            StringsError::Empty(field) => format!("The strings file has an empty \"{field}\"."),
        }
    }
}

/// Parses and checks the strings file. A field that is only whitespace counts
/// as empty, because the page would draw nothing.
pub fn parse_strings(json: &str) -> Result<Strings, StringsError> {
    let strings: Strings = serde_json::from_str(json).map_err(|e| StringsError::NotJson(e.to_string()))?;
    if strings.title.trim().is_empty() {
        return Err(StringsError::Empty("title"));
    }
    if strings.intro.trim().is_empty() {
        return Err(StringsError::Empty("intro"));
    }
    Ok(strings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_complete_file_is_accepted() {
        let s = parse_strings(r#"{"title": "T", "intro": "I"}"#).unwrap();
        assert_eq!(s.title, "T");
        assert_eq!(s.intro, "I");
    }

    #[test]
    fn an_empty_title_is_rejected_by_name() {
        let e = parse_strings(r#"{"title": "  ", "intro": "I"}"#).unwrap_err();
        assert_eq!(e, StringsError::Empty("title"));
        assert_eq!(e.message(), "The strings file has an empty \"title\".");
    }

    #[test]
    fn an_empty_intro_is_rejected_by_name() {
        let e = parse_strings(r#"{"title": "T", "intro": ""}"#).unwrap_err();
        assert_eq!(e, StringsError::Empty("intro"));
    }

    #[test]
    fn an_unknown_field_is_rejected() {
        assert!(matches!(
            parse_strings(r#"{"title": "T", "intro": "I", "extra": "x"}"#),
            Err(StringsError::NotJson(_))
        ));
    }

    #[test]
    fn text_that_is_not_json_is_rejected() {
        assert!(matches!(parse_strings("title: T"), Err(StringsError::NotJson(_))));
    }
}
