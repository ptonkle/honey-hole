//! The fish roster, from `data/fish.json`: each kind of fish's name on
//! screen, tier, model, typical length and base value. The Fishpedia, the
//! catch list and (later) the fishing rules read it; other systems extend
//! the file with fields of their own as they need them.

use serde::Deserialize;

use crate::map::fish::Tier;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FishKind {
    pub id: String,
    pub name: String,
    pub tier: Tier,
    pub model: String,
    pub base_length_mm: i32,
    pub value: i32,
    /// how many a match starts with
    pub count: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FishFile {
    #[serde(rename = "_note", default)]
    #[allow(dead_code)]
    note: String,
    fish: Vec<FileFish>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileFish {
    id: String,
    name: String,
    tier: String,
    model: String,
    base_length_mm: i32,
    value: i32,
    count: u32,
}

/// The tiers from best to least, as the Fishpedia lists them.
pub const TIERS: [Tier; 4] = [Tier::Legendary, Tier::Rare, Tier::Uncommon, Tier::Common];

pub fn tier_key(t: Tier) -> &'static str {
    match t {
        Tier::Legendary => "legendary",
        Tier::Rare => "rare",
        Tier::Uncommon => "uncommon",
        Tier::Common => "common",
    }
}

/// The roster, checked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Roster(pub Vec<FishKind>);

impl Roster {
    pub fn parse(json: &str) -> Result<Roster, String> {
        let file: FishFile =
            serde_json::from_str(json).map_err(|e| format!("The fish file could not be read: {e}"))?;
        if file.fish.is_empty() {
            return Err("The fish file lists no fish.".into());
        }
        let mut out: Vec<FishKind> = Vec::new();
        for f in file.fish {
            let tier = TIERS
                .into_iter()
                .find(|t| tier_key(*t) == f.tier)
                .ok_or_else(|| format!("The fish \"{}\" has no tier \"{}\".", f.id, f.tier))?;
            if out.iter().any(|k| k.id == f.id) {
                return Err(format!("The fish \"{}\" is listed twice.", f.id));
            }
            if f.name.trim().is_empty() {
                return Err(format!("The fish \"{}\" has no name.", f.id));
            }
            if f.base_length_mm <= 0 || f.value <= 0 {
                return Err(format!("The fish \"{}\" needs a length and a value.", f.id));
            }
            out.push(FishKind {
                id: f.id,
                name: f.name,
                tier,
                model: f.model,
                base_length_mm: f.base_length_mm,
                value: f.value,
                count: f.count,
            });
        }
        Ok(Roster(out))
    }

    pub fn get(&self, id: &str) -> Option<&FishKind> {
        self.0.iter().find(|k| k.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_roster_reads_its_tiers_and_refuses_a_bad_fish_by_name() {
        let ok = r#"{"fish": [{"id": "a", "name": "A", "tier": "rare", "model": "a", "base_length_mm": 300, "value": 4, "count": 2}]}"#;
        let r = Roster::parse(ok).unwrap();
        assert_eq!(r.get("a").unwrap().tier, Tier::Rare);
        let bad = ok.replace("rare", "mythic");
        assert_eq!(
            Roster::parse(&bad),
            Err("The fish \"a\" has no tier \"mythic\".".into())
        );
        let twice = r#"{"fish": [
            {"id": "a", "name": "A", "tier": "rare", "model": "a", "base_length_mm": 300, "value": 4, "count": 2},
            {"id": "a", "name": "B", "tier": "rare", "model": "a", "base_length_mm": 300, "value": 4, "count": 2}]}"#;
        assert_eq!(
            Roster::parse(twice),
            Err("The fish \"a\" is listed twice.".into())
        );
    }
}
