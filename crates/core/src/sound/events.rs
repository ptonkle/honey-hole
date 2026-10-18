//! Game events to cues, by event name: the `events` table of
//! `data/sounds.json`. An event the table does not name plays nothing, so
//! a system can emit events before it has sounds.
//!
//! # Adding an event's sound
//!
//! 1. Find the event's name as the match emits it (`MatchEvent` in
//!    `crates/core/src/sim/`: the variant name, such as `Hooked`) and the
//!    fields it carries (`seat`, `tier`, ...).
//! 2. Add a row under `events` in `data/sounds.json`:
//!    `"Hooked": [{"cue": "hook_set", "who": "mine"}]`. A row is a list, so
//!    one event can play several cues (an explosion, then a chime).
//! 3. Each entry takes:
//!    - `cue`: the cue's id. `{field}` puts in an event field, lowercased,
//!      spaces as `_`: `"catch_{tier}"` plays `catch_legendary` for a
//!      legendary fish. Every field named this way is listed in `each` with
//!      the values it can take, so the test can check that every cue exists.
//!    - `who`: `all` (the default), `mine` (only when the event's seat is the
//!      listener's), or `others` (only when it is not). The seat is the
//!      field `seat`, or the one `seat_field` names.
//!    - `gain`, `pitch` (1 as written), `delay_ms`, and `duration_field`: an
//!      event field, in milliseconds, that sets how long the cue's
//!      `stretch` layers last.
//! 4. Run `cargo test sound`: `every_event_names_an_existing_cue` checks
//!    the new row.
//!
//! The match hands an event over as JSON (`serde_json::to_value(&event)`),
//! so any of serde's enum shapes works: `"MatchStarted"`,
//! `{"Hooked": {"seat": 1, ...}}`, or `{"kind": "Hooked", "seat": 1, ...}`
//! (also `type` or `event` for the tag).

use std::collections::BTreeMap;

use serde_json::Value;

use super::decimal::{parse_milli, MILLI};
use super::read::{self, Obj};
use super::recipe::Params;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Who {
    All,
    Mine,
    Others,
}

/// One cue an event plays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// a cue id, perhaps with `{field}`s
    pub cue: String,
    pub who: Who,
    pub seat_field: String,
    pub gain: i64,
    pub pitch: i64,
    pub delay_ms: i64,
    pub duration_field: Option<String>,
    /// each `{field}` in `cue` and the values it can take
    pub each: BTreeMap<String, Vec<String>>,
}

/// The tag fields serde's internally tagged enums use.
const TAGS: [&str; 3] = ["kind", "type", "event"];

/// An event field as the cue id spells it.
fn spell(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.trim().to_lowercase().replace([' ', '-'], "_")),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// The `{field}`s a cue id names.
pub fn placeholders(cue: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = cue;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else {
            break;
        };
        out.push(rest[open + 1..open + close].to_string());
        rest = &rest[open + close + 1..];
    }
    out
}

impl Entry {
    fn parse(v: &Value, at: &str) -> Result<Entry, String> {
        let o = read::obj(v, at)?;
        read::keys(
            o,
            &[
                "cue",
                "who",
                "seat_field",
                "gain",
                "pitch",
                "delay_ms",
                "duration_field",
                "each",
            ],
            at,
        )?;
        let cue = read::text(o, "cue", at)?.to_string();
        let who = match read::opt_text(o, "who", at)? {
            None | Some("all") => Who::All,
            Some("mine") => Who::Mine,
            Some("others") => Who::Others,
            Some(w) => return Err(format!("{at} has who \"{w}\"; who is all, mine or others.")),
        };
        let mut each = BTreeMap::new();
        if let Some(e) = o.get("each") {
            for (field, values) in read::obj(e, &format!("{at} each"))? {
                let list: Option<Vec<String>> = values
                    .as_array()
                    .filter(|l| !l.is_empty())
                    .and_then(|l| l.iter().map(spell).collect());
                let list = list.ok_or_else(|| format!("{at} each {field} must list the values it takes."))?;
                each.insert(field.clone(), list);
            }
        }
        for p in placeholders(&cue) {
            if !each.contains_key(&p) {
                return Err(format!(
                    "{at} cue \"{cue}\" names {{{p}}}, which \"each\" does not list."
                ));
            }
        }
        Ok(Entry {
            cue,
            who,
            seat_field: read::opt_text(o, "seat_field", at)?.unwrap_or("seat").to_string(),
            gain: read::opt_milli(o, "gain", at)?.unwrap_or(MILLI),
            pitch: read::opt_milli(o, "pitch", at)?.unwrap_or(MILLI),
            delay_ms: read::opt_whole(o, "delay_ms", at, 0)?,
            duration_field: read::opt_text(o, "duration_field", at)?.map(str::to_string),
            each,
        })
    }

    /// Every cue id this entry can play: one per mix of its fields' values.
    pub fn every_cue(&self) -> Vec<String> {
        let mut out = vec![self.cue.clone()];
        for (field, values) in &self.each {
            let key = format!("{{{field}}}");
            out = out
                .iter()
                .flat_map(|c| values.iter().map(|v| c.replace(&key, v)))
                .collect();
        }
        out
    }
}

/// The table, by event name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Table {
    pub rows: BTreeMap<String, Vec<Entry>>,
}

impl Table {
    pub fn parse(v: &Value) -> Result<Table, String> {
        let o = read::obj(v, "The events table")?;
        let mut rows = BTreeMap::new();
        for (name, list) in o.iter().filter(|(k, _)| !k.starts_with('_')) {
            let at = format!("Event {name}");
            let entries = list
                .as_array()
                .filter(|l| !l.is_empty())
                .ok_or_else(|| format!("{at} must be a list of cues to play."))?;
            let parsed: Result<Vec<Entry>, String> = entries
                .iter()
                .enumerate()
                .map(|(k, e)| Entry::parse(e, &format!("{at} entry {}", k + 1)))
                .collect();
            rows.insert(name.clone(), parsed?);
        }
        Ok(Table { rows })
    }

    /// The cues an event plays for `listener` (a seat), with their
    /// parameters. An unknown event, or a field value no cue is named for,
    /// gives nothing; the caller drops a cue id it does not have.
    pub fn plays(&self, event: &Value, listener: Option<i64>) -> Vec<(String, Params)> {
        let Some((name, fields)) = name_and_fields(event) else {
            return Vec::new();
        };
        let Some(entries) = self.rows.get(&name) else {
            return Vec::new();
        };
        let empty = Obj::new();
        let fields = fields.unwrap_or(&empty);
        let mut out = Vec::new();
        for e in entries {
            let seat = fields.get(&e.seat_field).and_then(Value::as_i64);
            let mine = seat.is_some() && seat == listener;
            let heard = match e.who {
                Who::All => true,
                Who::Mine => mine,
                Who::Others => seat.is_some() && !mine,
            };
            if !heard {
                continue;
            }
            let mut cue = e.cue.clone();
            let mut known = true;
            for p in placeholders(&e.cue) {
                match fields.get(&p).and_then(spell) {
                    Some(v) if e.each.get(&p).is_some_and(|l| l.contains(&v)) => {
                        cue = cue.replace(&format!("{{{p}}}"), &v);
                    }
                    _ => known = false,
                }
            }
            if !known {
                continue;
            }
            let duration_ms = e
                .duration_field
                .as_ref()
                .and_then(|f| fields.get(f))
                .and_then(|v| parse_milli(v, "duration").ok())
                .map(|m| m / MILLI);
            out.push((
                cue,
                Params {
                    gain: e.gain,
                    pitch: e.pitch,
                    delay_ms: e.delay_ms,
                    duration_ms,
                },
            ));
        }
        out
    }
}

/// An event's name and its fields, from any of serde's enum shapes.
fn name_and_fields(event: &Value) -> Option<(String, Option<&Obj>)> {
    match event {
        Value::String(name) => Some((name.clone(), None)),
        Value::Object(o) => {
            if let Some(tag) = TAGS.iter().find_map(|t| o.get(*t).and_then(Value::as_str)) {
                return Some((tag.to_string(), Some(o)));
            }
            if o.len() == 1 {
                let (name, inner) = o.iter().next()?;
                return Some((name.clone(), inner.as_object()));
            }
            None
        }
        _ => None,
    }
}
