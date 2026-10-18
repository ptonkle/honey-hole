//! Reading the hand-written sound files: each object's keys are checked
//! against the keys it may have, so a misspelt key is refused by name
//! instead of being ignored. Keys starting with `_` are notes for people.

use serde_json::{Map, Value};

use super::decimal::parse_milli;

pub type Obj = Map<String, Value>;

/// `v` as an object, or a refusal naming `at`.
pub fn obj<'a>(v: &'a Value, at: &str) -> Result<&'a Obj, String> {
    v.as_object().ok_or_else(|| format!("{at} is not an object."))
}

/// Refuses a key that is not in `allowed` (notes, `_…`, aside).
pub fn keys(o: &Obj, allowed: &[&str], at: &str) -> Result<(), String> {
    match o
        .keys()
        .find(|k| !k.starts_with('_') && !allowed.contains(&k.as_str()))
    {
        Some(k) => Err(format!(
            "{at} has a key \"{k}\" the sound engine does not know; it takes {}.",
            allowed.join(", ")
        )),
        None => Ok(()),
    }
}

pub fn milli(o: &Obj, key: &str, at: &str) -> Result<i64, String> {
    match o.get(key) {
        Some(v) => parse_milli(v, &format!("{at} {key}")),
        None => Err(format!("{at} has no \"{key}\".")),
    }
}

pub fn opt_milli(o: &Obj, key: &str, at: &str) -> Result<Option<i64>, String> {
    o.get(key)
        .map(|v| parse_milli(v, &format!("{at} {key}")))
        .transpose()
}

/// A whole number (milliseconds, counts).
pub fn whole(o: &Obj, key: &str, at: &str) -> Result<i64, String> {
    match o.get(key) {
        Some(v) => v
            .as_i64()
            .ok_or_else(|| format!("{at} {key} must be a whole number.")),
        None => Err(format!("{at} has no \"{key}\".")),
    }
}

pub fn opt_whole(o: &Obj, key: &str, at: &str, default: i64) -> Result<i64, String> {
    match o.get(key) {
        Some(_) => whole(o, key, at),
        None => Ok(default),
    }
}

pub fn text<'a>(o: &'a Obj, key: &str, at: &str) -> Result<&'a str, String> {
    match o.get(key) {
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(s),
        Some(_) => Err(format!("{at} {key} must be some text.")),
        None => Err(format!("{at} has no \"{key}\".")),
    }
}

pub fn opt_text<'a>(o: &'a Obj, key: &str, at: &str) -> Result<Option<&'a str>, String> {
    match o.get(key) {
        None => Ok(None),
        Some(_) => text(o, key, at).map(Some),
    }
}

pub fn flag(o: &Obj, key: &str, at: &str) -> Result<bool, String> {
    match o.get(key) {
        None => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(format!("{at} {key} must be true or false.")),
    }
}

/// A number that must lie in `lo..=hi` (thousandths).
pub fn within(v: i64, lo: i64, hi: i64, what: &str, range: &str) -> Result<i64, String> {
    if v < lo || v > hi {
        Err(format!("{what} must be {range}."))
    } else {
        Ok(v)
    }
}
