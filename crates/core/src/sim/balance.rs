//! The `balance` section of `data/tuning.json`: knobs for the ramming
//! question (PLAN.md V-balance, `notes/balance-options.md`). Each one's
//! shipped value is the game as it was before the knob, so a knob does
//! nothing until its value is changed:
//!
//! - `announce_delay_ms` (by tier, 0): a hook's popup (`MatchEvent::Announced`)
//!   comes this long after the hook ([`super::fishing::announce`]). The AI
//!   hears only the popup.
//! - `rammed_swim_mm` (by tier, `fish.freed_swim_mm`'s 12 m): how far a fish
//!   knocked off a line by a ram swims off; any other lost fish swims
//!   `fish.freed_swim_mm`.
//! - `reeling_ram_wreck_pct` (80, `wreck.ram_wreck_pct`): the speed toward a
//!   boat that is reeling a fish in that wrecks it.
//! - `reeling_anchored` (false): a boat reeling a fish in is not pushed by a
//!   bump; the other boat takes the whole bump.
//! - `rammer_wreck_pct` (0, off): a rammer this fast toward its victim, in
//!   percent of top speed, is wrecked too.
//!
//! The last three change rules the person set (BRIEF: a ram at 80% wrecks;
//! round 1, B1: the rammer stalls), so they are for measuring only.

use serde::Deserialize;

use super::fishing::tuning::PerTier;
use crate::tuning::Checker;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BalanceTuning {
    #[serde(rename = "_note", default)]
    pub note: String,
    pub announce_delay_ms: PerTier,
    pub rammed_swim_mm: PerTier,
    pub reeling_ram_wreck_pct: i32,
    pub reeling_anchored: bool,
    pub rammer_wreck_pct: i32,
}

/// The section's ranges. `ram_wreck_pct` is `wreck.ram_wreck_pct`: the
/// reeling boat's threshold is no lower, and a rammer's own wreck (when on)
/// no lower either.
pub fn check(c: &mut Checker, b: &BalanceTuning, ram_wreck_pct: i32) {
    for (name, v) in tiers(&b.announce_delay_ms) {
        c.range(&format!("balance.announce_delay_ms.{name}"), v, 0, 600_000);
    }
    for (name, v) in tiers(&b.rammed_swim_mm) {
        c.range(&format!("balance.rammed_swim_mm.{name}"), v, 0, 400_000);
    }
    c.range(
        "balance.reeling_ram_wreck_pct",
        b.reeling_ram_wreck_pct,
        ram_wreck_pct,
        400,
    );
    if b.rammer_wreck_pct != 0 {
        c.range("balance.rammer_wreck_pct", b.rammer_wreck_pct, ram_wreck_pct, 400);
    }
}

fn tiers(p: &PerTier) -> [(&'static str, i32); 4] {
    [
        ("common", p.common),
        ("uncommon", p.uncommon),
        ("rare", p.rare),
        ("legendary", p.legendary),
    ]
}

#[cfg(test)]
pub mod tests {
    /// The section with every knob off: the game as it was before them.
    pub fn sample_json() -> serde_json::Value {
        serde_json::json!({
            "announce_delay_ms": {"common": 0, "uncommon": 0, "rare": 0, "legendary": 0},
            "rammed_swim_mm": {"common": 12000, "uncommon": 12000, "rare": 12000, "legendary": 12000},
            "reeling_ram_wreck_pct": 80,
            "reeling_anchored": false,
            "rammer_wreck_pct": 0
        })
    }

    #[test]
    fn a_reeling_threshold_under_the_ram_speed_is_refused() {
        let mut v = crate::tuning::tests::sample_json();
        v["balance"]["reeling_ram_wreck_pct"] = 70.into();
        let e = crate::tuning::parse(&v.to_string()).unwrap_err();
        assert!(e.contains("balance.reeling_ram_wreck_pct is 70"), "{e}");
        let mut v = crate::tuning::tests::sample_json();
        v["balance"]["rammer_wreck_pct"] = 50.into();
        let e = crate::tuning::parse(&v.to_string()).unwrap_err();
        assert!(e.contains("balance.rammer_wreck_pct is 50"), "{e}");
    }
}
