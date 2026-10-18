//! Popups: short announcements stacked at the top centre of the screen.
//!
//! **Data-driven.** Each row of `data/flow.json`'s `popups.rows` names a
//! `MatchEvent` (by its JSON `type`), the string key of its text, a style,
//! and how to fill each `{placeholder}` from the event's fields. Adding a
//! popup for a new event is adding a row (and its string):
//!
//! ```json
//! {"event": "Announced", "text": "popup.hooked", "style": "tier", "tier": "tier",
//!  "fill": {"player": "name:seat", "fish": "fish:kind"}}
//! ```
//!
//! A fill is `name:<path>` (a seat's player name), `fish:<path>` (a roster
//! id's fish name), `tier:<path>` (a tier's name) or `text:<path>` (the
//! value as it is). A path is dotted through the event's JSON:
//! `cause.rammed.by` reads `{"cause": {"rammed": {"by": 0}}}`. `when`
//! picks among rows for one event (`{"cause": "grounded"}`, or `"*"` for
//! any value at a path).
//!
//! **The queue.** A popup shows for `flow.popup_ms` (about 2 s), up to
//! `popups.max_shown` at once; more wait their turn, and each one's time
//! starts when it shows. Popups in one `group` replace each other where
//! they stand, so a countdown counts on in one place.

use serde_json::Value;

use super::look::PopupRow;
use crate::map::fish::Tier;
use crate::roster::{tier_key, TIERS};

/// One popup, filled in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Popup {
    /// the string key its text came from
    pub key: String,
    pub text: String,
    pub style: String,
    /// the event's tier, for a tier style
    pub tier: Option<Tier>,
    pub group: String,
    /// when it showed, page µs; `None` while it waits
    pub shown_us: Option<i64>,
}

/// The queue: the shown popups first, oldest at the top, then the waiting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Popups {
    pub queue: Vec<Popup>,
}

impl Popups {
    /// Adds a popup. One in the same group takes the old one's place (and
    /// its time starts again, if it was showing).
    pub fn push(&mut self, p: Popup, now_us: i64) {
        if !p.group.is_empty() {
            if let Some(old) = self.queue.iter_mut().find(|q| q.group == p.group) {
                let shown = old.shown_us.map(|_| now_us);
                *old = Popup { shown_us: shown, ..p };
                return;
            }
        }
        self.queue.push(p);
    }

    /// Drops the popups whose time is up and shows waiting ones while
    /// there is room.
    pub fn advance(&mut self, now_us: i64, popup_ms: i32, max_shown: usize) {
        let life = popup_ms as i64 * 1000;
        self.queue
            .retain(|p| p.shown_us.is_none_or(|s| now_us - s < life));
        let mut shown = self.queue.iter().filter(|p| p.shown_us.is_some()).count();
        for p in self.queue.iter_mut() {
            if shown >= max_shown {
                break;
            }
            if p.shown_us.is_none() {
                p.shown_us = Some(now_us);
                shown += 1;
            }
        }
    }

    /// The popups on screen, top first.
    pub fn shown(&self) -> impl Iterator<Item = &Popup> {
        self.queue.iter().filter(|p| p.shown_us.is_some())
    }
}

/// The value at a dotted path through an event's JSON.
pub fn at<'a>(event: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(event, |v, key| match v {
        Value::Object(o) => o.get(key),
        // an enum with no fields serialises as its name: "grounded" is
        // at path "cause", and also counts as having the key "grounded"
        Value::String(s) if s == key => Some(v),
        _ => None,
    })
}

/// A JSON value as text: strings as they are, numbers as digits.
fn plain(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// A tier from its JSON (`"Legendary"` or `"legendary"`).
pub fn tier_of(v: &Value) -> Option<Tier> {
    let s = v.as_str()?.to_lowercase();
    TIERS.into_iter().find(|t| tier_key(*t) == s)
}

/// The event's seat (`seat`), for `who`.
fn seat_of(event: &Value) -> Option<u64> {
    event.get("seat").and_then(Value::as_u64)
}

/// What the popups need to fill a row: the player names by seat, and the
/// fish and tier names.
pub struct Names<'a> {
    pub players: &'a [String],
    /// a roster id's fish name, if it is one
    pub fish: &'a dyn Fn(&str) -> Option<String>,
    pub tier: &'a dyn Fn(Tier) -> String,
    /// the string for a key
    pub text: &'a dyn Fn(&str) -> String,
    /// what an unknown player and an unknown fish are called
    pub someone: String,
    pub a_fish: String,
}

impl PopupRow {
    /// Whether this row is for `event`, heard at `me`'s seat.
    pub fn matches(&self, event: &Value, me: usize) -> bool {
        if event.get("type").and_then(Value::as_str) != Some(self.event.as_str()) {
            return false;
        }
        let mine = seat_of(event) == Some(me as u64);
        match self.who.as_str() {
            "mine" if !mine => return false,
            "others" if mine => return false,
            _ => {}
        }
        self.when.iter().all(|(path, want)| match at(event, path) {
            None => false,
            Some(_) if want == "*" => true,
            Some(v) => plain(v).as_deref() == Some(want.as_str()),
        })
    }

    /// The popup this row makes of `event`.
    pub fn popup(&self, event: &Value, names: &Names) -> Popup {
        let mut text = (names.text)(&self.text);
        for (key, how) in &self.fill {
            let (kind, path) = how.split_once(':').unwrap_or(("text", how));
            let v = at(event, path);
            let word = match kind {
                "name" => v
                    .and_then(Value::as_u64)
                    .and_then(|s| names.players.get(s as usize).cloned()),
                "fish" => v.and_then(Value::as_str).and_then(|id| (names.fish)(id)),
                "tier" => v.and_then(tier_of).map(|t| (names.tier)(t)),
                _ => v.and_then(plain),
            };
            let fallback = if kind == "fish" {
                &names.a_fish
            } else {
                &names.someone
            };
            text = text.replace(&format!("{{{key}}}"), &word.unwrap_or_else(|| fallback.clone()));
        }
        Popup {
            key: self.text.clone(),
            text,
            style: self.style.clone(),
            tier: at(event, &self.tier).and_then(tier_of),
            group: self.group.clone(),
            shown_us: None,
        }
    }
}

/// The popups a tick's events make, in order: each event's first row
/// that matches it.
pub fn of_events(rows: &[PopupRow], events: &[Value], me: usize, names: &Names) -> Vec<Popup> {
    events
        .iter()
        .filter_map(|e| rows.iter().find(|r| r.matches(e, me)).map(|r| r.popup(e, names)))
        .collect()
}
