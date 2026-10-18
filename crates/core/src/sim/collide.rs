//! Collisions: a boat against the land, and boats against each other.
//!
//! **The hull** is a rounded box: a line from `half_length` behind the
//! middle to `half_length` ahead of it, with `half_beam` all round. Land is
//! any cell of the map's grid that is not water (`map::Cell`: the shore
//! land, islands, and marsh, which the generator makes land); off the map
//! counts as land. Reeds stand in water and do nothing.
//!
//! **Land.** Eight points on the hull's outline (bow, stern, the four
//! corners of the straight sides, the middle of each side) are tested
//! against the grid where the boat would move to. On a hit the boat stays
//! where it was, and the land's direction is the sum of the touching points'
//! directions from the middle. If the speed toward the land is
//! `land_wreck` or more, the boat is wrecked; slower, the boat bounces
//! back at `bounce_pct` of that speed and keeps `scrape_keep_pct` of its
//! speed along the shore, so it slides along it, losing speed.
//!
//! **Boats.** Two hulls touch when their middle lines come closer than two
//! half-beams. A boat whose speed toward the other (along the line between
//! the nearest points) is `ram_wreck` or more wrecks it, and its own motor
//! stalls. If both are that fast, both are wrecked. Slower, they bump: each
//! is pushed apart and their closing speed is reversed at `bump_pct`. A
//! ghosted boat passes through boats.

use super::boat::{dir, Boat, ONE, UM};
use super::{Match, MatchEvent, Rules, WreckCause};
use crate::map::trig::isqrt;
use crate::map::{Map, P};

/// How far a boat caught inside land is pushed out each tick, mm.
const PUSH_OUT_MM: i64 = 200;

pub enum LandHit {
    Clear,
    /// bounced; the speed toward the land, mm/s
    Bounced(i32),
    Wrecked,
}

/// The hull's outline points for a middle (mm) and heading.
pub fn outline(at: [i64; 2], heading: i32, r: &Rules) -> [[i64; 2]; 8] {
    let (c, s) = dir(heading);
    let (c, s) = (c as i64, s as i64);
    let one = ONE as i64;
    let (hl, hb) = (r.half_length as i64, r.half_beam as i64);
    // forward (c, s), left (-s, c)
    let p = |f: i64, l: i64| [at[0] + (f * c - l * s) / one, at[1] + (f * s + l * c) / one];
    [
        p(hl + hb, 0),
        p(-(hl + hb), 0),
        p(hl, hb),
        p(hl, -hb),
        p(-hl, hb),
        p(-hl, -hb),
        p(0, hb),
        p(0, -hb),
    ]
}

fn land_at(map: &Map, p: [i64; 2]) -> bool {
    !map.wet(P::new(p[0] as i32, p[1] as i32))
}

/// A vector made one long (Q16), or zero.
pub fn unit(v: [i64; 2]) -> [i64; 2] {
    let len = isqrt(v[0] * v[0] + v[1] * v[1]);
    if len == 0 {
        return [0, 0];
    }
    [v[0] * ONE as i64 / len, v[1] * ONE as i64 / len]
}

fn dot(v: [i32; 2], n: [i64; 2]) -> i64 {
    (v[0] as i64 * n[0] + v[1] as i64 * n[1]) / ONE as i64
}

/// The land's push on the hull at `at` (mm): the unit direction away from
/// the touching outline points, and whether any touch.
fn land_normal(map: &Map, at: [i64; 2], heading: i32, r: &Rules) -> Option<[i64; 2]> {
    let mut sum = [0i64, 0];
    let mut touching = false;
    for q in outline(at, heading, r) {
        if land_at(map, q) {
            touching = true;
            sum[0] += at[0] - q[0];
            sum[1] += at[1] - q[1];
        }
    }
    touching.then(|| unit(sum))
}

/// Moves a boat one tick and settles its contact with land.
pub fn land(map: &Map, r: &Rules, b: &mut Boat) -> LandHit {
    let old = b.pos_um;
    let step = super::TICK_MS as i64;
    let new = [
        old[0] + b.vel_um[0] as i64 * step / 1000,
        old[1] + b.vel_um[1] as i64 * step / 1000,
    ];
    let vel = b.vel();
    let mm = |p: [i64; 2]| [p[0] / UM, p[1] / UM];
    let Some(mut n) = land_normal(map, mm(new), b.heading, r) else {
        b.pos_um = new;
        return LandHit::Clear;
    };
    if n == [0, 0] {
        // touching evenly all round: push back the way it came
        n = unit([-(vel[0] as i64), -(vel[1] as i64)]);
    }
    let vn = dot(vel, n);
    let into = (-vn).max(0) as i32;
    if into >= r.land_wreck {
        return LandHit::Wrecked;
    }
    // bounce off: the speed into the land comes back at bounce_pct, and the
    // speed along the shore is scraped down to scrape_keep_pct
    let (vx, vy) = (vel[0] as i64, vel[1] as i64);
    let normal = vn.min(0);
    let (tx, ty) = (vx - normal * n[0] / ONE as i64, vy - normal * n[1] / ONE as i64);
    let back = -normal * r.bounce_pct as i64 / 100;
    let keep = r.scrape_keep_pct as i64;
    let (nvx, nvy) = (
        if vn < 0 {
            tx * keep / 100 + back * n[0] / ONE as i64
        } else {
            vx
        },
        if vn < 0 {
            ty * keep / 100 + back * n[1] / ONE as i64
        } else {
            vy
        },
    );
    b.set_vel([nvx as i32, nvy as i32]);
    if land_normal(map, mm(old), b.heading, r).is_some() {
        // caught inside (a turn swung the hull into land): push out
        b.pos_um = [
            old[0] + n[0] * PUSH_OUT_MM * UM / ONE as i64,
            old[1] + n[1] * PUSH_OUT_MM * UM / ONE as i64,
        ];
    } else {
        b.pos_um = old;
    }
    LandHit::Bounced(into)
}

/// The hull's middle line, in mm.
fn spine(b: &Boat, r: &Rules) -> ([i64; 2], [i64; 2]) {
    let (c, s) = dir(b.heading);
    let at = [b.pos_um[0] / UM, b.pos_um[1] / UM];
    let hl = r.half_length as i64;
    let f = [hl * c as i64 / ONE as i64, hl * s as i64 / ONE as i64];
    ([at[0] - f[0], at[1] - f[1]], [at[0] + f[0], at[1] + f[1]])
}

/// The point of segment `a..b` nearest `p`.
fn nearest_on(p: [i64; 2], a: [i64; 2], b: [i64; 2]) -> [i64; 2] {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let len2 = ab[0] * ab[0] + ab[1] * ab[1];
    if len2 == 0 {
        return a;
    }
    let t = ((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]).clamp(0, len2);
    [a[0] + ab[0] * t / len2, a[1] + ab[1] * t / len2]
}

fn cross(o: [i64; 2], a: [i64; 2], b: [i64; 2]) -> i64 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

fn d2(a: [i64; 2], b: [i64; 2]) -> i64 {
    (a[0] - b[0]).pow(2) + (a[1] - b[1]).pow(2)
}

/// The nearest points of two segments (on the first, on the second); the
/// same point twice when they cross.
pub fn nearest_points(a: ([i64; 2], [i64; 2]), b: ([i64; 2], [i64; 2])) -> ([i64; 2], [i64; 2]) {
    let (d1, d2_, d3, d4) = (
        cross(b.0, b.1, a.0),
        cross(b.0, b.1, a.1),
        cross(a.0, a.1, b.0),
        cross(a.0, a.1, b.1),
    );
    if ((d1 > 0 && d2_ < 0) || (d1 < 0 && d2_ > 0)) && ((d3 > 0 && d4 < 0) || (d3 < 0 && d4 > 0)) {
        let m = nearest_on(a.0, b.0, b.1);
        return (m, m);
    }
    let options = [
        (a.0, nearest_on(a.0, b.0, b.1)),
        (a.1, nearest_on(a.1, b.0, b.1)),
        (nearest_on(b.0, a.0, a.1), b.0),
        (nearest_on(b.1, a.0, a.1), b.1),
    ];
    let mut best = options[0];
    for o in options {
        if d2(o.0, o.1) < d2(best.0, best.1) {
            best = o;
        }
    }
    best
}

/// Every touching pair of boats, in seat order.
pub fn boats(m: &mut Match) {
    let n = m.boats.len();
    let tick = m.tick;
    for i in 0..n {
        for j in i + 1..n {
            let (Some(a), Some(b)) = (m.boats[i].as_ref(), m.boats[j].as_ref()) else {
                continue;
            };
            if a.wrecked() || b.wrecked() || a.ghosted(tick) || b.ghosted(tick) {
                continue;
            }
            let r = &m.rules;
            let (pa, pb) = nearest_points(spine(a, r), spine(b, r));
            let dist2 = d2(pa, pb);
            let reach = 2 * r.half_beam as i64;
            if dist2 >= reach * reach {
                continue;
            }
            let mut d = unit([pb[0] - pa[0], pb[1] - pa[1]]);
            if d == [0, 0] {
                let (ca, cb) = (a.pos_um, b.pos_um);
                d = unit([cb[0] - ca[0], cb[1] - ca[1]]);
            }
            if d == [0, 0] {
                d = [ONE as i64, 0];
            }
            let a_toward = dot(a.vel(), d) as i32;
            let b_toward = -dot(b.vel(), d) as i32;
            let (a_rams, b_rams) = (a_toward >= r.ram_wreck, b_toward >= r.ram_wreck);
            if a_rams || b_rams {
                if a_rams {
                    m.wreck(j, WreckCause::Rammed { by: i }, tick);
                }
                if b_rams {
                    m.wreck(i, WreckCause::Rammed { by: j }, tick);
                }
                for (seat, rams, other_rams, toward, sign) in [
                    (i, a_rams, b_rams, a_toward, 1i64),
                    (j, b_rams, a_rams, b_toward, -1),
                ] {
                    if rams && !other_rams {
                        let stall = tick + m.rules.stall_ticks;
                        let boat = m.boats[seat].as_mut().unwrap();
                        boat.stall_until = stall;
                        // it hit something: its speed toward the wreck is gone
                        let mut v = boat.vel();
                        v[0] -= (toward as i64 * sign * d[0] / ONE as i64) as i32;
                        v[1] -= (toward as i64 * sign * d[1] / ONE as i64) as i32;
                        boat.set_vel(v);
                        m.events.push(MatchEvent::Stalled { seat });
                    }
                }
                continue;
            }
            bump(m, i, j, d, dist2, reach);
        }
    }
}

/// Two boats bump apart.
fn bump(m: &mut Match, i: usize, j: usize, d: [i64; 2], dist2: i64, reach: i64) {
    let one = ONE as i64;
    let (va, vb) = (
        m.boats[i].as_ref().unwrap().vel(),
        m.boats[j].as_ref().unwrap().vel(),
    );
    let rel = dot([vb[0] - va[0], vb[1] - va[1]], d);
    let push = (reach - isqrt(dist2)) / 2 + 1;
    let k = if rel < 0 {
        -rel * (100 + m.rules.bump_pct as i64) / 200
    } else {
        0
    };
    for (seat, sign) in [(i, -1i64), (j, 1)] {
        let b = m.boats[seat].as_mut().unwrap();
        let mut v = b.vel();
        v[0] += (sign * k * d[0] / one) as i32;
        v[1] += (sign * k * d[1] / one) as i32;
        b.set_vel(v);
        b.pos_um[0] += sign * push * UM * d[0] / one;
        b.pos_um[1] += sign * push * UM * d[1] / one;
    }
    if rel < 0 {
        m.events.push(MatchEvent::Bumped {
            a: i,
            b: j,
            speed: (-rel) as i32,
        });
    }
}
