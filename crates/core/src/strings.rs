//! The reader-visible strings, by key, from `data/strings.json`. The core
//! lists every key it uses in `KEYS`, and the file must hold exactly those:
//! a missing key would draw as nothing, and a key no screen uses is a string
//! nobody reads. Keys starting with `_` are notes for people and are skipped.

use std::collections::BTreeMap;

/// Every key a screen reads, in the order the screens use them.
pub const KEYS: &[&str] = &[
    "title",
    "menu.play",
    "menu.settings",
    "menu.notes",
    "menu.back",
    "menu.pause",
    "menu.resume",
    "menu.quit",
    "controls.heading",
    "controls.pause",
    "controls.pause_keys",
    "settings.heading",
    "settings.volume",
    "settings.text_size",
    "settings.text_size.small",
    "settings.text_size.medium",
    "settings.text_size.large",
    "notes.heading",
    "notes.minor",
    "pause.heading",
    "menu.gallery",
    "map.burt_lake",
    "gallery.heading",
    "gallery.model",
    "gallery.show",
    "gallery.still",
    "gallery.rider",
    "gallery.colour",
    "gallery.colour.all",
    "gallery.view",
    "gallery.view.model",
    "gallery.view.above",
    "gallery.size",
    "gallery.size.fit",
    "gallery.size.lineup",
    "gallery.turn",
    "gallery.long",
    "gallery.tall",
    "gallery.none",
    "gallery.no_scene",
    "anim.idle",
    "anim.panic",
    "anim.drive",
    "anim.fish",
    "anim.bob",
    "anim.sway",
    "colour.blue",
    "colour.red",
    "colour.green",
    "colour.yellow",
    "fishboy.tab.radar",
    "fishboy.tab.fishpedia",
    "fishboy.tab.catch",
    "fishboy.keys",
    "hud.fishboy",
    "radar.buoy",
    "radar.ping",
    "radar.ping.ready",
    "radar.ping.sweeping",
    "radar.ping.cooling",
    "radar.you",
    "radar.seen",
    "radar.unseen",
    "tier.legendary",
    "tier.rare",
    "tier.uncommon",
    "tier.common",
    "fishpedia.left",
    "fishpedia.none_left",
    "fishpedia.total",
    "catch.heading",
    "catch.none",
    "catch.feet",
    "catch.count",
    "controls.drive",
    "controls.drive_keys",
    "controls.steer",
    "controls.steer_keys",
    "controls.mode",
    "controls.mode_keys",
    "controls.look",
    "controls.look_keys",
    "menu.heading",
    "settings.name",
    "name.default",
    "lobby.heading",
    "lobby.you",
    "lobby.ai_player",
    "lobby.empty",
    "lobby.ai",
    "lobby.difficulty",
    "lobby.length",
    "lobby.buoy",
    "lobby.seed",
    "lobby.seed_random",
    "lobby.seed_unknown",
    "lobby.start",
    "lobby.ai_seat",
    "difficulty.easy",
    "difficulty.normal",
    "difficulty.hard",
    "units.min",
    "units.s",
    "hud.time",
    "hud.driving",
    "hud.fishing",
    "hud.speed",
    "hud.wrecked",
    "hud.ended",
    "menu.sounds",
    "sounds.heading",
    "sounds.sub",
    "sounds.none",
    "sounds.loops",
    "sounds.own",
    "sounds.throttle",
    "sounds.tension",
    "sounds.tension_level",
    "sounds.rival",
    "sounds.rival_at",
    "sounds.distance",
    "sounds.closing",
    "sounds.leaving",
    "sounds.heard",
    "sounds.heard_as",
    "sounds.silent",
    "sounds.drive_past",
];

/// The strings file, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strings(BTreeMap<String, String>);

/// Why a strings file cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringsError {
    NotJson(String),
    Missing(String),
    Empty(String),
    Unused(String),
}

impl StringsError {
    /// The message the page shows.
    pub fn message(&self) -> String {
        match self {
            StringsError::NotJson(detail) => format!("The strings file could not be read: {detail}"),
            StringsError::Missing(key) => format!("The strings file has no \"{key}\"."),
            StringsError::Empty(key) => format!("The strings file has an empty \"{key}\"."),
            StringsError::Unused(key) => {
                format!("The strings file has \"{key}\", which no screen uses.")
            }
        }
    }
}

/// Parses and checks the strings file. A string that is only whitespace
/// counts as empty, because the page would draw nothing.
pub fn parse(json: &str) -> Result<Strings, StringsError> {
    let all: BTreeMap<String, String> =
        serde_json::from_str(json).map_err(|e| StringsError::NotJson(e.to_string()))?;
    let map: BTreeMap<String, String> = all.into_iter().filter(|(k, _)| !k.starts_with('_')).collect();
    for key in KEYS {
        match map.get(*key) {
            None => return Err(StringsError::Missing(key.to_string())),
            Some(text) if text.trim().is_empty() => return Err(StringsError::Empty(key.to_string())),
            Some(_) => {}
        }
    }
    if let Some(key) = map.keys().find(|k| !KEYS.contains(&k.as_str())) {
        return Err(StringsError::Unused(key.clone()));
    }
    Ok(Strings(map))
}

impl Strings {
    /// The string for a key. A key outside `KEYS` comes back as itself, so
    /// a slip shows on screen instead of drawing nothing; `parse` has made
    /// sure every listed key is there.
    pub fn t(&self, key: &str) -> String {
        self.0.get(key).cloned().unwrap_or_else(|| key.to_string())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A complete strings file: each key's text is the key in capitals.
    pub(crate) fn complete() -> String {
        let map: BTreeMap<&str, String> = KEYS.iter().map(|k| (*k, k.to_uppercase())).collect();
        serde_json::to_string(&map).unwrap()
    }

    fn without(key: &str) -> String {
        let mut v: serde_json::Value = serde_json::from_str(&complete()).unwrap();
        v.as_object_mut().unwrap().remove(key);
        v.to_string()
    }

    fn with(key: &str, text: &str) -> String {
        let mut v: serde_json::Value = serde_json::from_str(&complete()).unwrap();
        v[key] = serde_json::json!(text);
        v.to_string()
    }

    #[test]
    fn a_complete_file_is_accepted_and_read_by_key() {
        let s = parse(&complete()).unwrap();
        assert_eq!(s.t("menu.back"), "MENU.BACK");
    }

    #[test]
    fn a_missing_key_is_refused_by_name() {
        let e = parse(&without("menu.back")).unwrap_err();
        assert_eq!(e.message(), "The strings file has no \"menu.back\".");
    }

    #[test]
    fn an_empty_string_is_refused_by_name() {
        let e = parse(&with("title", "  ")).unwrap_err();
        assert_eq!(e, StringsError::Empty("title".into()));
        assert_eq!(e.message(), "The strings file has an empty \"title\".");
    }

    #[test]
    fn a_key_no_screen_uses_is_refused_but_a_note_is_not() {
        let e = parse(&with("menu.jump", "Jump")).unwrap_err();
        assert_eq!(e, StringsError::Unused("menu.jump".into()));
        assert!(parse(&with("_note", "for people")).is_ok());
    }

    #[test]
    fn text_that_is_not_json_is_refused() {
        assert!(matches!(parse("title: T"), Err(StringsError::NotJson(_))));
    }
}
