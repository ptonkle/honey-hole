//! The items' decisions, each a pure function of plain values, so the
//! online host can make the same call on a client's claim ("I picked up
//! the buoy", "I used my item") and confirm or refuse it, and a test can
//! check it alone. The match makes every one of these calls through here.

use super::{Item, SeatItems};
use crate::map::fish::Tier;
use crate::map::{Map, P};
use crate::sim::fishing::lake::LakeFish;

/// Who takes the buoy this tick: the boat nearest it within `radius_mm`
/// of its middle; on a tie the lower seat. `boats` holds each boat that
/// may take it (not wrecked) as (seat, its middle).
pub fn pickup(buoy: Option<P>, boats: &[(usize, P)], radius_mm: i32) -> Option<usize> {
    let at = buoy?;
    let r2 = radius_mm as i64 * radius_mm as i64;
    boats
        .iter()
        .map(|&(seat, p)| (p.dist2(at), seat))
        .filter(|&(d2, _)| d2 <= r2)
        .min()
        .map(|(_, seat)| seat)
}

/// The item a roll draws: `roll` below the weights' total picks the item
/// whose share it falls in, in [`Item::ALL`]'s order.
pub fn draw(weight: impl Fn(Item) -> i32, roll: u32) -> Item {
    let total: i64 = Item::ALL.iter().map(|&i| weight(i).max(0) as i64).sum();
    let mut r = roll as i64 % total.max(1);
    for item in Item::ALL {
        let w = weight(item).max(0) as i64;
        if r < w {
            return item;
        }
        r -= w;
    }
    Item::ALL[0]
}

/// Why Shift did nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    /// no item held
    Empty,
    /// a minigame, a teleport's choice or charge, or the camera's offer is
    /// already under way
    Busy,
    /// the boat is wrecked
    Wrecked,
    /// the High-Quality Camera is used after a catch, not with Shift
    AfterCatch,
}

/// Shift: the item to use now, or why not.
pub fn use_item(s: &SeatItems, wrecked: bool) -> Result<Item, Refusal> {
    let item = s.held.ok_or(Refusal::Empty)?;
    if s.busy() {
        return Err(Refusal::Busy);
    }
    if wrecked {
        return Err(Refusal::Wrecked);
    }
    if item == Item::Camera {
        return Err(Refusal::AfterCatch);
    }
    Ok(item)
}

/// Where a teleport to `want` lands: `want` itself when it is water at
/// least `clear_mm` from land, else the middle of the nearest such cell
/// (a click on land snaps to the nearest open water). `None` when the map
/// has no such water.
pub fn teleport_target(map: &Map, want: P, clear_mm: i32) -> Option<P> {
    let open = |i: usize| map.cells[i].water() && map.shore[i] >= clear_mm;
    if map.index(want).is_some_and(&open) {
        return Some(want);
    }
    (0..map.cells.len())
        .filter(|&i| open(i))
        .min_by_key(|&i| (map.centre(i).dist2(want), i))
        .map(|i| map.centre(i))
}

/// The tiers in the order an Enhanced Radar marks them.
pub const REVEAL_TIERS: [Tier; 4] = [Tier::Legendary, Tier::Rare, Tier::Uncommon, Tier::Common];

/// The fish an Enhanced Radar reveals from `from`: the nearest fish of
/// each tier still in the lake (not landed), by its index, legendary
/// first; a tier with none left is skipped.
pub fn reveal(fish: &[LakeFish], from: P) -> Vec<usize> {
    REVEAL_TIERS
        .iter()
        .filter_map(|&tier| {
            fish.iter()
                .enumerate()
                .filter(|(_, f)| f.tier == tier && f.in_lake())
                .min_by_key(|(k, f)| (f.at.dist2(from), *k))
                .map(|(k, _)| k)
        })
        .collect()
}
