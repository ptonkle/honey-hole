//! The fish in the lake: stocked at match start, roaming slowly in their
//! schools, swimming to a lure, pulled in on a line, or swimming free.
//!
//! - **Stocking** ([`stock`]). Each kind's count from `data/fish.json`
//!   moves by up to `count_variance_pct` either way, drawn from the match
//!   seed (a schooling kind never gets just one: one becomes two). `map::fish::place_kinds` puts them on the lake:
//!   commons and uncommons in schools of 2–4, rares and legendaries single
//!   and far apart, each tier at its depths. Each fish's length is drawn
//!   between `length_min_pct` and `length_max_pct` of its kind's base.
//! - **Roaming** ([`School`]). A school wanders from leg to leg within
//!   `roam_radius_mm` of where it was placed, resting between legs; each
//!   member keeps its own place in the school. A fish that got free swims
//!   `freed_swim_mm` away from the boat and then roams on its own from
//!   there, as a school of one. A caught fish leaves the lake for good.

use serde::Serialize;

use crate::map::fish::Tier;
use crate::map::rng::Rng;
use crate::map::{trig, Map, P};
use crate::roster::Roster;
use crate::sim::TICK_MS;

use super::tuning::FishTuning;

/// What a fish is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum FishState {
    /// with its school
    Roaming,
    /// swimming to this seat's lure, nibbling, or biting
    Lured { seat: usize },
    /// on this seat's line
    Hooked { seat: usize },
    /// got free: swimming off to this point
    Fleeing { to: P },
    /// landed by this seat; out of the lake
    Caught { seat: usize },
}

/// One fish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LakeFish {
    /// its place in [`super::Fishing::fish`]
    pub id: u32,
    /// its kind's place in the roster
    pub kind: usize,
    pub tier: Tier,
    pub length_mm: i32,
    pub at: P,
    /// below the surface
    pub depth_mm: i32,
    /// the depth it keeps while roaming
    pub home_depth_mm: i32,
    /// which way it swims (map angle)
    pub heading: i32,
    pub school: usize,
    /// its place in its school, from the school's middle
    pub offset: P,
    pub state: FishState,
    /// where its swim starts in its loop, so a school does not beat in step
    pub phase_ms: u32,
}

impl LakeFish {
    pub fn in_lake(&self) -> bool {
        !matches!(self.state, FishState::Caught { .. })
    }

    /// Free to notice a lure.
    pub fn roaming(&self) -> bool {
        self.state == FishState::Roaming
    }
}

/// A school (or a single fish) and its wander.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct School {
    /// its middle now
    pub at: P,
    /// where it was placed (or settled after getting free)
    pub home: P,
    pub target: P,
    /// resting before this tick
    pub rest_until: u32,
}

/// The lake bed's depth under a point, mm (0 on land or off the map).
pub fn bed_mm(map: &Map, p: P) -> i32 {
    map.index(p)
        .filter(|&i| map.cells[i].water())
        .map_or(0, |i| (-map.ground[i]).max(0))
}

/// `from` moved toward `to` by at most `step` mm.
pub fn toward(from: P, to: P, step: i32) -> P {
    let (dx, dy) = ((to.x - from.x) as i64, (to.y - from.y) as i64);
    let d = trig::hypot(dx, dy);
    if d <= step as i64 || d == 0 {
        return to;
    }
    P::new(
        from.x + (dx * step as i64 / d) as i32,
        from.y + (dy * step as i64 / d) as i32,
    )
}

/// A point `d` mm from `p` toward a map heading.
pub fn ahead(p: P, heading: i32, d: i32) -> P {
    P::new(
        p.x + (trig::cos(heading) as i64 * d as i64 / trig::ONE as i64) as i32,
        p.y + (trig::sin(heading) as i64 * d as i64 / trig::ONE as i64) as i32,
    )
}

/// mm a tick at a speed in mm/s, at least 1.
pub fn per_tick(mm_per_s: i32) -> i32 {
    (mm_per_s * TICK_MS / 1000).max(1)
}

/// How many fish of each kind a match stocks: each kind's count with its
/// variance, rounded to the nearest; a schooling kind gets none or two or
/// more.
pub fn counts(roster: &Roster, t: &FishTuning, rng: &mut Rng) -> Vec<(usize, u32)> {
    let mut out = Vec::new();
    for (k, kind) in roster.0.iter().enumerate() {
        let base = kind.count as i32 * 100;
        let swing = base * t.count_variance_pct / 100;
        let mut n = ((base + rng.range(-swing, swing) + 50) / 100).max(0) as u32;
        if kind.tier.schools() && n == 1 {
            n = 2;
        }
        if n > 0 {
            out.push((k, n));
        }
    }
    out
}

/// The fish and schools for a match on `map`.
pub fn stock(
    map: &Map,
    roster: &Roster,
    t: &FishTuning,
    seed: u64,
) -> Result<(Vec<LakeFish>, Vec<School>), String> {
    let mut rng = Rng::stream(seed, 31);
    let mut want = counts(roster, t, &mut rng);
    let mut placed = Err(String::new());
    // a lake too small for the counts takes a tenth fewer, a few times over
    for _ in 0..6 {
        let ids: Vec<(&str, Tier, u32)> = want
            .iter()
            .map(|&(k, c)| (roster.0[k].id.as_str(), roster.0[k].tier, c))
            .collect();
        placed = crate::map::fish::place_kinds(map, &ids, seed);
        if placed.is_ok() {
            break;
        }
        for w in want.iter_mut() {
            w.1 = if roster.0[w.0].tier.schools() {
                (w.1 * 9 / 10).max(2)
            } else {
                w.1 * 9 / 10
            };
        }
    }
    let placed = placed?;
    let mut fish = Vec::with_capacity(placed.len());
    let mut schools: Vec<School> = Vec::new();
    let mut school_of: Vec<(u32, usize)> = Vec::new();
    for f in &placed {
        let kind = roster
            .0
            .iter()
            .position(|k| k.id == f.kind)
            .ok_or_else(|| format!("There is no fish \"{}\".", f.kind))?;
        let s = match school_of.iter().find(|(n, _)| *n == f.school) {
            Some(&(_, s)) => s,
            None => {
                // the school's middle: the mean of its members
                let members: Vec<&crate::map::fish::Fish> =
                    placed.iter().filter(|g| g.school == f.school).collect();
                let n = members.len() as i64;
                let mid = P::new(
                    (members.iter().map(|g| g.at.x as i64).sum::<i64>() / n) as i32,
                    (members.iter().map(|g| g.at.y as i64).sum::<i64>() / n) as i32,
                );
                schools.push(School {
                    at: mid,
                    home: mid,
                    target: mid,
                    rest_until: rng.below(200),
                });
                school_of.push((f.school, schools.len() - 1));
                schools.len() - 1
            }
        };
        let base = roster.0[kind].base_length_mm;
        let pct = rng.range(t.length_min_pct * 10, t.length_max_pct * 10);
        let mid = schools[s].at;
        fish.push(LakeFish {
            id: fish.len() as u32,
            kind,
            tier: f.tier,
            length_mm: (base as i64 * pct as i64 / 1000) as i32,
            at: f.at,
            depth_mm: f.depth_mm,
            home_depth_mm: f.depth_mm,
            heading: rng.below(trig::TURN as u32) as i32,
            school: s,
            offset: P::new(f.at.x - mid.x, f.at.y - mid.y),
            state: FishState::Roaming,
            phase_ms: rng.below(60_000),
        });
    }
    Ok((fish, schools))
}

/// One tick of every school's wander, and of each roaming or fleeing fish.
pub fn roam(map: &Map, t: &FishTuning, rng: &mut Rng, tick: u32, fish: &mut [LakeFish], schools: &mut Vec<School>) {
    let leg = per_tick(t.roam_speed_mm_per_s);
    for s in schools.iter_mut() {
        if tick < s.rest_until {
            continue;
        }
        if s.at == s.target {
            // rest, then a new leg within the roam radius of home, over water
            let rest = rng.range(t.roam_rest_min_ms, t.roam_rest_max_ms);
            s.rest_until = tick + crate::sim::ticks(rest);
            let r = t.roam_radius_mm;
            for _ in 0..8 {
                let q = P::new(s.home.x + rng.range(-r, r), s.home.y + rng.range(-r, r));
                if q.dist2(s.home) <= r as i64 * r as i64 && bed_mm(map, q) > 0 && map.wet(q) {
                    s.target = q;
                    break;
                }
            }
            continue;
        }
        s.at = toward(s.at, s.target, leg);
    }
    let follow = per_tick(t.follow_speed_mm_per_s);
    let flee = per_tick(t.freed_speed_mm_per_s);
    for f in fish.iter_mut() {
        match f.state {
            FishState::Roaming => {
                let s = &schools[f.school];
                let want = P::new(s.at.x + f.offset.x, s.at.y + f.offset.y);
                if map.wet(want) {
                    swim(f, want, follow);
                }
                settle_depth(map, f, follow);
            }
            FishState::Fleeing { to } => {
                swim(f, to, flee);
                settle_depth(map, f, flee);
                if f.at == to {
                    schools.push(School {
                        at: to,
                        home: to,
                        target: to,
                        rest_until: tick,
                    });
                    f.school = schools.len() - 1;
                    f.offset = P::new(0, 0);
                    f.state = FishState::Roaming;
                }
            }
            _ => {}
        }
    }
}

/// Moves a fish toward a point, turning it to face the way it goes.
pub fn swim(f: &mut LakeFish, to: P, step: i32) {
    if f.at == to {
        return;
    }
    let next = toward(f.at, to, step);
    if next != f.at {
        f.heading = trig::atan2((next.y - f.at.y) as i64, (next.x - f.at.x) as i64);
    }
    f.at = next;
}

/// The fish's depth back toward its home depth, never into the bed.
fn settle_depth(map: &Map, f: &mut LakeFish, step: i32) {
    let bed = bed_mm(map, f.at);
    let want = f.home_depth_mm.min((bed - 200).max(100));
    f.depth_mm = crate::gfx::fx::approach(f.depth_mm, want, step);
}

/// Where a freed fish swims: `freed_swim_mm` straight away from `from`, or
/// turned as little as it must be to stay in the water.
pub fn flee_point(map: &Map, t: &FishTuning, at: P, from: P) -> P {
    let away = if at == from {
        0
    } else {
        trig::atan2((at.y - from.y) as i64, (at.x - from.x) as i64)
    };
    let eighth = trig::TURN / 8;
    for turn in [0, 1, -1, 2, -2, 3, -3, 4] {
        let q = ahead(at, away + turn * eighth, t.freed_swim_mm);
        if map.wet(q) && bed_mm(map, q) > 0 {
            return q;
        }
    }
    at
}
