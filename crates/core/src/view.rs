//! The view model's pieces: panels and the items inside them. The page has
//! one builder per item `type` and draws a panel without knowing what it
//! means; `kind` names the panel for the stylesheet only.
//!
//! Every control names an action (`id`) that the page sends back through
//! `act` with the control's new value: a button sends "", a toggle "on" or
//! "off", a slider or a select the value it shows.

use serde::Serialize;

fn is_false(b: &bool) -> bool {
    !*b
}

/// One block of the screen. Empty fields are left out of the JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Panel {
    /// the panel's name for the stylesheet ("menu", "pause")
    pub kind: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub heading: String,
    /// a line under the heading
    #[serde(skip_serializing_if = "String::is_empty")]
    pub sub: String,
    /// an action the heading sends when clicked
    #[serde(skip_serializing_if = "String::is_empty")]
    pub click: String,
    pub items: Vec<Item>,
}

impl Panel {
    pub fn new(kind: &str, heading: String, items: Vec<Item>) -> Panel {
        Panel {
            kind: kind.into(),
            heading,
            sub: String::new(),
            click: String::new(),
            items,
        }
    }
}

/// One choice of a select.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Choice {
    pub value: String,
    pub text: String,
    #[serde(skip_serializing_if = "is_false")]
    pub disabled: bool,
}

/// One row of a table. A marked row is drawn picked out (a player's own,
/// say), and says so in text as well as in colour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableRow {
    pub cells: Vec<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub marked: bool,
}

/// One version on the What's New screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NoteEntry {
    pub version: String,
    pub date: String,
    pub minor: bool,
    pub lines: Vec<String>,
}

/// One item in a panel, by `type`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Item {
    Button {
        id: String,
        text: String,
        /// drawn smaller, for a less used action
        #[serde(skip_serializing_if = "is_false")]
        quiet: bool,
        /// a small marker beside the text
        #[serde(skip_serializing_if = "Option::is_none")]
        badge: Option<String>,
        #[serde(skip_serializing_if = "is_false")]
        disabled: bool,
    },
    Slider {
        id: String,
        label: String,
        min: u32,
        max: u32,
        step: u32,
        value: u32,
        /// the value as the reader reads it ("80%")
        text: String,
    },
    Toggle {
        id: String,
        label: String,
        value: bool,
    },
    Select {
        id: String,
        label: String,
        value: String,
        options: Vec<Choice>,
    },
    /// a one-line text field; the page sends its text when it changes
    Text {
        id: String,
        label: String,
        value: String,
        /// the most characters it takes
        max: u32,
        /// shown greyed while the field is empty
        #[serde(skip_serializing_if = "String::is_empty")]
        placeholder: String,
    },
    /// label and value pairs
    List {
        rows: Vec<[String; 2]>,
    },
    Table {
        head: Vec<String>,
        rows: Vec<TableRow>,
    },
    Notes {
        id: String,
        entries: Vec<NoteEntry>,
    },
}

impl Item {
    pub fn button(id: &str, text: String) -> Item {
        Item::Button {
            id: id.into(),
            text,
            quiet: false,
            badge: None,
            disabled: false,
        }
    }

    pub fn quiet_button(id: &str, text: String) -> Item {
        Item::Button {
            id: id.into(),
            text,
            quiet: true,
            badge: None,
            disabled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_item_carries_its_type_and_leaves_unset_flags_out() {
        let plain = serde_json::to_value(Item::button("menu.back", "Back".into())).unwrap();
        assert_eq!(
            plain,
            json!({ "type": "button", "id": "menu.back", "text": "Back" })
        );
        let quiet = serde_json::to_value(Item::quiet_button("menu.pause", "Pause".into())).unwrap();
        assert_eq!(quiet["quiet"], true);
        let table = Item::Table {
            head: vec!["A".into()],
            rows: vec![TableRow {
                cells: vec!["1".into()],
                marked: true,
            }],
        };
        assert_eq!(
            serde_json::to_value(table).unwrap(),
            json!({ "type": "table", "head": ["A"], "rows": [{ "cells": ["1"], "marked": true }] })
        );
    }

    #[test]
    fn a_panel_leaves_out_an_empty_heading_sub_and_click() {
        let p = serde_json::to_value(Panel::new("menu", String::new(), Vec::new())).unwrap();
        assert_eq!(p, json!({ "kind": "menu", "items": [] }));
    }
}
