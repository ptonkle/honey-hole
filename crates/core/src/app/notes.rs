//! What's New: each version's notes, newest first.

use super::*;
use crate::view::NoteEntry;

impl App {
    /// The major versions, and the minor ones too when the reader ticks
    /// them in.
    pub(super) fn notes_panels(&self) -> Vec<Panel> {
        let entries = self
            .notes
            .iter()
            .filter(|e| self.settings.notes_minor || !e.minor())
            .map(|e| NoteEntry {
                version: format!("v{}", e.version),
                date: e.date.clone(),
                minor: e.minor(),
                lines: e.notes.clone(),
            })
            .collect();
        vec![Panel::new(
            "notes",
            self.t("notes.heading"),
            vec![
                Item::Toggle {
                    id: "notes.minor".into(),
                    label: self.t("notes.minor"),
                    value: self.settings.notes_minor,
                },
                Item::Notes {
                    id: "notes.list".into(),
                    entries,
                },
                self.button("menu.back"),
            ],
        )]
    }
}
