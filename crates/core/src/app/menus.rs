//! The Title, Settings and Play's panels, and the Settings actions.

use super::*;
use crate::view::Choice;

impl App {
    pub(super) fn button(&self, id: &str) -> Item {
        Item::button(id, self.t(id))
    }

    /// The Settings screen's controls (`settings.*`).
    pub(super) fn settings_act(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            "settings.volume" => {
                let v: u32 = value
                    .parse()
                    .map_err(|_| format!("The volume \"{value}\" is not a whole number."))?;
                // the slider's own steps, whatever the page sends
                self.settings.volume = v.min(VOLUME_MAX) / VOLUME_STEP * VOLUME_STEP;
            }
            "settings.text_size" => {
                self.settings.text_size =
                    TextSize::from_key(value).ok_or_else(|| format!("There is no text size \"{value}\"."))?;
            }
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    pub(super) fn title_panels(&self) -> Vec<Panel> {
        vec![
            Panel::new("title", self.t("title"), Vec::new()),
            Panel::new(
                "menu",
                String::new(),
                vec![
                    self.button("menu.play"),
                    self.button("menu.settings"),
                    self.button("menu.notes"),
                    self.button("menu.gallery"),
                ],
            ),
            self.controls_panel(),
        ]
    }

    fn controls_panel(&self) -> Panel {
        let rows = ["pause"]
            .iter()
            .map(|k| {
                [
                    self.t(&format!("controls.{k}")),
                    self.t(&format!("controls.{k}_keys")),
                ]
            })
            .collect();
        Panel::new("controls", self.t("controls.heading"), vec![Item::List { rows }])
    }

    pub(super) fn settings_panels(&self) -> Vec<Panel> {
        let s = &self.settings;
        let sizes = TextSize::ALL
            .iter()
            .map(|t| Choice {
                value: t.key().into(),
                text: self.t(&format!("settings.text_size.{}", t.key())),
                disabled: false,
            })
            .collect();
        vec![Panel::new(
            "settings",
            self.t("settings.heading"),
            vec![
                Item::Slider {
                    id: "settings.volume".into(),
                    label: self.t("settings.volume"),
                    min: 0,
                    max: VOLUME_MAX,
                    step: VOLUME_STEP,
                    value: s.volume,
                    text: format!("{}%", s.volume),
                },
                Item::Select {
                    id: "settings.text_size".into(),
                    label: self.t("settings.text_size"),
                    value: s.text_size.key().into(),
                    options: sizes,
                },
                self.button("menu.back"),
            ],
        )]
    }

    /// Play's placeholder, and the pause menu over it while it is open.
    pub(super) fn play_panels(&self) -> Vec<Panel> {
        let mut play = Panel::new(
            "play",
            String::new(),
            vec![Item::quiet_button("menu.pause", self.t("menu.pause"))],
        );
        play.sub = self.t("play.placeholder");
        let mut panels = vec![play];
        if self.paused {
            panels.push(Panel::new(
                "pause",
                self.t("pause.heading"),
                vec![self.button("menu.resume"), self.button("menu.quit")],
            ));
        }
        panels
    }
}
