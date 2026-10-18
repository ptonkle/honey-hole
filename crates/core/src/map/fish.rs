//! Placing the fish at match start, from a seed and a count per kind.
//!
//! Common and uncommon fish swim in schools of 2 to 4; rare and legendary
//! fish are single and kept far apart. Every fish has a depth: commons swim
//! shallow and legendaries deep ([`Tier::depths`]), so a fish goes only
//! where the lake is deep enough for its tier. Fish go only on water a boat
//! can reach ([`Map::boatable`]) and not right by the spawn.
//!
//! The counts are an input of each match, so a playtest can vary them.
//! [`place_kinds`] takes any roster's kinds (a match passes the kinds of
//! `data/fish.json`, `crate::roster`); [`place`] takes ids from the
//! stand-in [`ROSTER`], which the map list's checks use.

use serde::{Deserialize, Serialize};

use super::rng::Rng;
use super::{Map, P};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tier {
    Common,
    Uncommon,
    Rare,
    Legendary,
}

impl Tier {
    /// Whether the tier swims in schools.
    pub fn schools(self) -> bool {
        matches!(self, Tier::Common | Tier::Uncommon)
    }

    /// How deep the lake must be (mm) where the tier is placed: commons
    /// keep to the water near shore, rare and legendary fish to the deeps.
    pub fn beds(self) -> (i32, i32) {
        let (lo, _) = self.depths();
        match self {
            Tier::Common => (lo + RULES.under_mm, 6000),
            Tier::Uncommon => (lo + RULES.under_mm, 11_000),
            Tier::Rare | Tier::Legendary => (lo + RULES.under_mm, i32::MAX),
        }
    }

    /// The depths (mm below the surface) the tier swims at.
    pub fn depths(self) -> (i32, i32) {
        match self {
            Tier::Common => (500, 3000),
            Tier::Uncommon => (2000, 6000),
            Tier::Rare => (3000, 9000),
            Tier::Legendary => (6000, 14_000),
        }
    }
}

/// A kind of fish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Kind {
    pub id: &'static str,
    pub tier: Tier,
}

/// The stand-in roster: Midwest lake fish.
pub const ROSTER: [Kind; 7] = [
    Kind {
        id: "bluegill",
        tier: Tier::Common,
    },
    Kind {
        id: "yellow-perch",
        tier: Tier::Common,
    },
    Kind {
        id: "smallmouth-bass",
        tier: Tier::Uncommon,
    },
    Kind {
        id: "walleye",
        tier: Tier::Uncommon,
    },
    Kind {
        id: "northern-pike",
        tier: Tier::Rare,
    },
    Kind {
        id: "muskellunge",
        tier: Tier::Legendary,
    },
    Kind {
        id: "lake-sturgeon",
        tier: Tier::Legendary,
    },
];

pub fn kind(id: &str) -> Option<Kind> {
    ROSTER.into_iter().find(|k| k.id == id)
}

/// The brief's example spread (30 common, 20 uncommon, 10 rare, 5
/// legendary) over the stand-in roster.
pub const EXAMPLE_COUNTS: [(&str, u32); 7] = [
    ("bluegill", 16),
    ("yellow-perch", 14),
    ("smallmouth-bass", 10),
    ("walleye", 10),
    ("northern-pike", 10),
    ("muskellunge", 3),
    ("lake-sturgeon", 2),
];

/// One fish at match start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fish {
    pub kind: String,
    pub tier: Tier,
    pub at: P,
    /// Below the surface, in mm.
    pub depth_mm: i32,
    /// Fish of one school share a number; a single fish has its own.
    pub school: u32,
}

/// The placement rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rules {
    pub school_size: (u32, u32),
    /// Every fish of a school is this near the school's middle.
    pub school_radius_mm: i32,
    /// Between the middles of two schools.
    pub school_gap_mm: i32,
    /// Between any two rare or legendary fish.
    pub single_gap_mm: i32,
    /// From the spawn, for every fish, and for rare and legendary fish.
    pub spawn_gap_mm: (i32, i32),
    /// Water kept under a fish.
    pub under_mm: i32,
}

pub const RULES: Rules = Rules {
    school_size: (2, 4),
    school_radius_mm: 6000,
    school_gap_mm: 24_000,
    single_gap_mm: 60_000,
    spawn_gap_mm: (40_000, 100_000),
    under_mm: 300,
};

/// The fish for a match: `counts` names each kind and how many.
pub fn place(map: &Map, counts: &[(&str, u32)], seed: u64) -> Result<Vec<Fish>, String> {
    let mut kinds = Vec::new();
    for &(id, count) in counts {
        let k = kind(id).ok_or_else(|| format!("There is no fish \"{id}\"."))?;
        kinds.push((k.id, k.tier, count));
    }
    place_kinds(map, &kinds, seed)
}

/// The fish for a match: each kind's id, tier and count.
pub fn place_kinds(map: &Map, counts: &[(&str, Tier, u32)], seed: u64) -> Result<Vec<Fish>, String> {
    let mut kinds = Vec::new();
    for &(id, tier, count) in counts {
        if tier.schools() && count == 1 {
            return Err(format!(
                "One {id} cannot make a school of {}.",
                RULES.school_size.0
            ));
        }
        kinds.push((Kind2 { id, tier }, count));
    }
    // a few fresh starts when the single fish cannot be spread far enough
    let mut last = String::new();
    for attempt in 0..6u64 {
        match try_place(map, &kinds, seed ^ attempt.wrapping_mul(0x9E37_79B9)) {
            Ok(f) => return Ok(f),
            Err(e) => last = e,
        }
    }
    Err(last)
}

fn try_place(map: &Map, kinds: &[(Kind2, u32)], seed: u64) -> Result<Vec<Fish>, String> {
    let rules = &RULES;
    let boatable = map.boatable();
    let n = map.n;
    let room = |tier: Tier, gap: i32| -> Vec<usize> {
        let (lo, hi) = tier.beds();
        (0..n * n)
            .filter(|&i| {
                boatable[i]
                    && (lo..=hi).contains(&-map.ground[i])
                    && map.centre(i).dist(map.spawn.at) >= gap as i64
            })
            .collect()
    };
    let depth = |r: &mut Rng, tier: Tier, i: usize| {
        let (lo, hi) = tier.depths();
        r.range(lo, hi.min(-map.ground[i] - rules.under_mm).max(lo))
    };
    let mut r = Rng::stream(seed, 11);
    let mut fish = Vec::new();
    let mut school = 0;

    // single fish, legendary first, each the farthest of a few tries from the rest
    let mut singles: Vec<P> = Vec::new();
    let mut order: Vec<&(Kind2, u32)> = kinds.iter().filter(|(k, _)| !k.tier.schools()).collect();
    order.sort_by_key(|(k, _)| std::cmp::Reverse(k.tier));
    for &&(k, count) in &order {
        let cells = room(k.tier, rules.spawn_gap_mm.1);
        if cells.is_empty() && count > 0 {
            return Err(format!("The lake has no water deep enough for {}.", k.id));
        }
        for _ in 0..count {
            let i = farthest(&mut r, map, &cells, &singles, 64);
            let at = jitter(&mut r, map, i);
            if singles.iter().any(|s| s.dist(at) < rules.single_gap_mm as i64) {
                return Err(format!(
                    "The rare and legendary fish do not fit {} m apart.",
                    rules.single_gap_mm / 1000
                ));
            }
            singles.push(at);
            school += 1;
            fish.push(Fish {
                kind: k.id.to_string(),
                tier: k.tier,
                at,
                depth_mm: depth(&mut r, k.tier, i),
                school,
            });
        }
    }

    // schools
    let mut middles: Vec<P> = Vec::new();
    for &(k, count) in kinds.iter().filter(|(k, _)| k.tier.schools()) {
        let cells = room(k.tier, rules.spawn_gap_mm.0);
        if cells.is_empty() && count > 0 {
            return Err(format!("The lake has no water deep enough for {}.", k.id));
        }
        let (lo, hi) = k.tier.beds();
        let mut left = count;
        while left > 0 {
            let (smin, smax) = (rules.school_size.0, rules.school_size.1);
            let size = if left <= smax {
                left
            } else {
                r.range(smin as i32, smax.min(left - smin) as i32) as u32
            };
            left -= size;
            let i = farthest(&mut r, map, &cells, &middles, 24);
            let mid = map.centre(i);
            if middles.iter().any(|m| m.dist(mid) < rules.school_gap_mm as i64) {
                return Err("The schools do not fit apart.".into());
            }
            middles.push(mid);
            school += 1;
            for _ in 0..size {
                let rad = rules.school_radius_mm;
                let mut at = mid;
                let mut cell = i;
                for _ in 0..30 {
                    let q = P::new(mid.x + r.range(-rad, rad), mid.y + r.range(-rad, rad));
                    if q.dist2(mid) > rad as i64 * rad as i64 {
                        continue;
                    }
                    if let Some(j) = map
                        .index(q)
                        .filter(|&j| boatable[j] && (lo..=hi).contains(&-map.ground[j]))
                    {
                        at = q;
                        cell = j;
                        break;
                    }
                }
                fish.push(Fish {
                    kind: k.id.to_string(),
                    tier: k.tier,
                    at,
                    depth_mm: depth(&mut r, k.tier, cell),
                    school,
                });
            }
        }
    }
    Ok(fish)
}

/// A kind as placement reads it, from any roster.
#[derive(Debug, Clone, Copy)]
struct Kind2<'a> {
    id: &'a str,
    tier: Tier,
}

/// Of `tries` random cells, the one farthest from every point in `from`.
fn farthest(r: &mut Rng, map: &Map, cells: &[usize], from: &[P], tries: u32) -> usize {
    let mut best = (-1i64, cells[0]);
    for _ in 0..tries {
        let i = cells[r.below(cells.len() as u32) as usize];
        let c = map.centre(i);
        let d = from.iter().map(|f| f.dist2(c)).min().unwrap_or(i64::MAX);
        if d > best.0 {
            best = (d, i);
        }
    }
    best.1
}

/// A point inside a cell, off its exact middle.
fn jitter(r: &mut Rng, map: &Map, i: usize) -> P {
    let c = map.centre(i);
    let h = map.cell_mm / 2 - 1;
    P::new(c.x + r.range(-h, h), c.y + r.range(-h, h))
}
