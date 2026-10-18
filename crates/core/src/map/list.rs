//! The pregenerated maps: the seeds of a setting that passed the
//! validator, kept in `data/maps/<setting>.json`, and the pick of one for a
//! match.
//!
//! Each entry keeps the map's fingerprint too, so a change to the
//! generator that would quietly turn a seed someone reported into another
//! lake fails a test (`crates/core/tests/maps.rs`) until the list is
//! scanned again (`cargo run --release --example maps -- scan`).

use serde::{Deserialize, Serialize};

use super::rng::mix;
use super::validate::{self, Problem};
use super::{generate, Map, Setting};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub seed: u32,
    /// [`fingerprint`] of the map, as 16 hex digits (its stamp).
    pub stamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapList {
    pub setting: String,
    /// The seeds scanned were `from..from + scanned`.
    pub from: u32,
    pub scanned: u32,
    pub maps: Vec<Entry>,
}

/// Reads a map list and checks it names a known setting and holds a map.
pub fn parse(json: &str) -> Result<MapList, String> {
    let list: MapList =
        serde_json::from_str(json).map_err(|e| format!("The map list does not read: {e}."))?;
    if Setting::from_id(&list.setting).is_none() {
        return Err(format!("There is no map setting \"{}\".", list.setting));
    }
    if list.maps.is_empty() {
        return Err("The map list holds no maps.".into());
    }
    Ok(list)
}

/// The map seed for a match seed: any match seed picks one of the list's.
pub fn pick(list: &MapList, match_seed: u64) -> u32 {
    let k = mix(match_seed ^ 0x6d61_7073) % list.maps.len() as u64;
    list.maps[k as usize].seed
}

/// A 64-bit FNV-1a hash of everything in a map.
pub fn fingerprint(map: &Map) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |v: i64| {
        for b in v.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    for i in 0..map.n * map.n {
        eat(map.cells[i].letter() as i64);
        eat(map.ground[i] as i64);
        eat(map.island[i] as i64);
    }
    for r in &map.reeds {
        eat(r.at.x as i64);
        eat(r.at.y as i64);
        eat(r.clumps.len() as i64);
    }
    for t in &map.trees {
        eat(t.at.x as i64);
        eat(t.at.y as i64);
        eat(t.kind as i64);
        eat(t.scale as i64);
    }
    eat(map.spawn.at.x as i64);
    eat(map.spawn.at.y as i64);
    eat(map.spawn.heading as i64);
    format!("{h:016x}")
}

/// Every seed in `from..from + count`: the accepted ones as a list, and
/// each rejected one with its problems.
pub fn scan(setting: Setting, from: u32, count: u32) -> (MapList, Vec<(u32, Vec<Problem>)>) {
    let mut maps = Vec::new();
    let mut rejected = Vec::new();
    for seed in from..from + count {
        let map = generate(setting, seed);
        let verdict = validate::check(&map);
        if verdict.ok() {
            maps.push(Entry {
                seed,
                stamp: fingerprint(&map),
            });
        } else {
            rejected.push((seed, verdict.problems));
        }
    }
    let list = MapList {
        setting: setting.id().to_string(),
        from,
        scanned: count,
        maps,
    };
    (list, rejected)
}
