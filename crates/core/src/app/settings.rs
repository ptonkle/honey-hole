//! The settings kept between visits, and the Settings screen's controls
//! beyond the volume and the text size (`app/menus.rs`).
//!
//! The person chose to keep settings in browser storage (`brief/questions.md`
//! P3; BUILDER-SPEC, the D17 change). The core decides what is kept and
//! checks what comes back: [`App::stored_settings`] writes the settings as
//! one JSON object (`Ui::settings`), which `web/settings-store.js` stores as
//! it is under one key; on the next visit the page hands the text back
//! through `act("settings.load", text)`. Every value goes through the same
//! setter its control uses, so a stored value is held to the same rules as
//! a typed one. Anything unreadable or out of range is refused whole and
//! the settings stay as they were: a bad save is treated as no save.

use serde::{Deserialize, Serialize};

use super::*;
use crate::controls::{self, MOUSE_PCT_MAX, MOUSE_PCT_MIN, MOUSE_PCT_STEP};

/// What is kept between visits. A field missing from an older save keeps
/// its default; a field this version does not know is ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Kept {
    #[serde(default = "default_volume")]
    volume: u32,
    #[serde(default = "default_text_size")]
    text_size: String,
    #[serde(default)]
    notes_minor: bool,
    #[serde(default)]
    name: String,
    #[serde(default)]
    show_seed: bool,
    #[serde(default = "default_mouse")]
    mouse_pct: u32,
    #[serde(default)]
    invert_y: bool,
}

fn default_volume() -> u32 {
    Settings::default().volume
}

fn default_text_size() -> String {
    Settings::default().text_size.key().into()
}

fn default_mouse() -> u32 {
    Settings::default().mouse_pct
}

impl App {
    /// The settings as the page keeps them between visits.
    pub fn stored_settings(&self) -> String {
        let s = &self.settings;
        let kept = Kept {
            volume: s.volume,
            text_size: s.text_size.key().into(),
            notes_minor: s.notes_minor,
            name: s.name.clone(),
            show_seed: s.show_seed,
            mouse_pct: s.mouse_pct,
            invert_y: s.invert_y,
        };
        serde_json::to_string(&kept).unwrap_or_default()
    }

    /// Settings kept from an earlier visit (`settings.load`): every value
    /// through its control's own setter, all or nothing.
    fn load_settings(&mut self, text: &str) -> Result<(), String> {
        let kept: Kept =
            serde_json::from_str(text).map_err(|e| format!("The kept settings could not be read: {e}"))?;
        let before = self.settings.clone();
        let applied = (|| {
            if kept.volume > VOLUME_MAX || !kept.volume.is_multiple_of(VOLUME_STEP) {
                return Err(format!("The kept volume {} is not on the slider.", kept.volume));
            }
            if !(MOUSE_PCT_MIN..=MOUSE_PCT_MAX).contains(&kept.mouse_pct)
                || !kept.mouse_pct.is_multiple_of(MOUSE_PCT_STEP)
            {
                return Err(format!(
                    "The kept mouse sensitivity {} is not on the slider.",
                    kept.mouse_pct
                ));
            }
            self.settings_act("settings.volume", &kept.volume.to_string())?;
            self.settings_act("settings.text_size", &kept.text_size)?;
            self.set_name(&kept.name);
            self.settings.notes_minor = kept.notes_minor;
            self.settings.show_seed = kept.show_seed;
            self.settings.mouse_pct = kept.mouse_pct;
            self.settings.invert_y = kept.invert_y;
            Ok(())
        })();
        if applied.is_err() {
            self.settings = before;
        }
        applied
    }

    /// The Settings controls `app/menus.rs` does not handle.
    pub(super) fn more_settings_act(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            "settings.load" => self.load_settings(value)?,
            "settings.mouse" => {
                let pct: u32 = value
                    .parse()
                    .map_err(|_| format!("The mouse sensitivity \"{value}\" is not a whole number."))?;
                self.settings.mouse_pct = controls::mouse_pct(pct);
            }
            "settings.invert_y" => self.settings.invert_y = value == "on",
            // under `?debug=fog`: the fog distance, as the Lake preview's
            // slider sets it
            "settings.fog" if self.debug_fog() => self.lake_act("lake.fog", value)?,
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    /// The page's `?debug=fog`: Settings shows the fog distance.
    pub(super) fn debug_fog(&self) -> bool {
        self.debug_flags.iter().any(|f| f == "fog")
    }

    /// The Settings screen's controls after the text size: the mouse, and
    /// the fog distance under `?debug=fog`.
    pub(super) fn more_settings_items(&self) -> Vec<Item> {
        let s = &self.settings;
        let mut items = vec![
            Item::Slider {
                id: "settings.mouse".into(),
                label: self.t("settings.mouse"),
                min: MOUSE_PCT_MIN,
                max: MOUSE_PCT_MAX,
                step: MOUSE_PCT_STEP,
                value: s.mouse_pct,
                text: format!("{}%", s.mouse_pct),
            },
            Item::Toggle {
                id: "settings.invert_y".into(),
                label: self.t("settings.invert_y"),
                value: s.invert_y,
            },
        ];
        if self.debug_fog() {
            let metres = (self.sight_mm().unwrap_or(0) / 1000) as u32;
            items.push(Item::Slider {
                id: "settings.fog".into(),
                label: self.t("lake.fog"),
                min: lake::FOG_MIN_M,
                max: lake::FOG_MAX_M,
                step: lake::FOG_STEP_M,
                value: metres,
                text: self.t("lake.metres").replace("{m}", &metres.to_string()),
            });
        }
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        crate::app::tests::app()
    }

    #[test]
    fn settings_kept_from_a_visit_come_back_whole() {
        let mut a = app();
        a.act("settings.volume", "35").unwrap();
        a.act("settings.text_size", "large").unwrap();
        a.act("settings.name", "Marta").unwrap();
        a.act("settings.mouse", "150").unwrap();
        a.act("settings.invert_y", "on").unwrap();
        a.act("settings.show_seed", "on").unwrap();
        let kept = a.ui().settings;
        let mut next = app();
        next.act("settings.load", &kept).unwrap();
        let s = &next.settings;
        assert_eq!(
            (s.volume, s.text_size, s.name.as_str()),
            (35, TextSize::Large, "Marta")
        );
        assert_eq!((s.mouse_pct, s.invert_y, s.show_seed), (150, true, true));
    }

    #[test]
    fn an_unreadable_or_out_of_range_save_is_treated_as_none() {
        let fresh = Settings::default();
        let mut a = app();
        assert!(a.act("settings.load", "{not json").is_err());
        assert_eq!(a.settings, fresh);
        // one bad value refuses the whole save: the volume before it is not
        // kept either
        let bad = r#"{"volume": 40, "text_size": "huge"}"#;
        assert!(a.act("settings.load", bad).is_err());
        assert_eq!(a.settings, fresh);
        // a volume off the slider's steps, a sensitivity off its range
        assert!(a.act("settings.load", r#"{"volume": 42}"#).is_err());
        assert!(a.act("settings.load", r#"{"mouse_pct": 1000}"#).is_err());
        assert_eq!(a.settings, fresh);
        // an older save without the newer fields keeps their defaults; a
        // name is cleaned as the field cleans it
        a.act(
            "settings.load",
            r#"{"volume": 60, "name": "  Ann  ", "future": 1}"#,
        )
        .unwrap();
        assert_eq!((a.settings.volume, a.settings.name.as_str()), (60, "Ann"));
        assert_eq!((a.settings.mouse_pct, a.settings.invert_y), (100, false));
    }

    #[test]
    fn sensitivity_and_invert_reach_the_match_camera() {
        use crate::gfx::fx::{self, deg};
        let mut a = crate::app::play::tests::started();
        a.act("settings.mouse", "200").unwrap();
        a.act("settings.invert_y", "on").unwrap();
        let p0 = a.play.as_ref().unwrap().cam_pitch;
        // 100 px right at 200%: 2 x 25° clockwise from above
        a.act("mouse.move", "100,0").unwrap();
        assert_eq!(a.play.as_ref().unwrap().cam_yaw, fx::wrap(-deg(50)));
        // 10 px down, inverted, at 200%: 20 px up, 18° per 100 px
        a.act("mouse.move", "0,10").unwrap();
        let p1 = a.play.as_ref().unwrap().cam_pitch;
        assert_eq!(p1, p0 - (20 * deg(18) as i64 / 100) as i32);
    }

    #[test]
    fn each_seat_row_of_the_new_match_carries_its_player_colour() {
        let a = crate::app::play::tests::lobby();
        let v = serde_json::to_value(a.ui()).unwrap();
        let rows = v["panels"][0]["items"][0]["rows"].as_array().unwrap();
        let tones: Vec<&str> = rows.iter().map(|r| r["tone"].as_str().unwrap()).collect();
        assert_eq!(tones, ["blue", "red", "green", "yellow"]);
        // and names it in text: colour is never the only cue
        assert_eq!(rows[1]["cells"][0], "COLOUR.RED");
        assert_eq!(rows[0]["marked"], true, "your own seat is picked out");
    }

    #[test]
    fn the_fog_distance_shows_in_settings_only_under_the_debug_flag() {
        let mut a = app();
        a.act("menu.settings", "").unwrap();
        let ids = |a: &App| {
            serde_json::to_value(a.ui()).unwrap()["panels"][0]["items"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|i| i["id"].as_str().map(String::from))
                .collect::<Vec<_>>()
        };
        assert!(ids(&a).contains(&"settings.mouse".to_string()));
        assert!(!ids(&a).contains(&"settings.fog".to_string()));
        assert!(a.act("settings.fog", "200").is_err());
        a.act("debug.flags", "fog").unwrap();
        assert!(ids(&a).contains(&"settings.fog".to_string()));
        a.act("settings.fog", "200").unwrap();
        assert_eq!(a.sight_mm(), Some(200_000));
    }
}
