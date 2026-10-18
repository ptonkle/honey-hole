//! Small integer geometry the minigames share: turning points, ellipses,
//! a fish's outline, and whether a point is inside a polygon.

use crate::map::trig;

pub type Pt = (i32, i32);

/// (x, y) turned by `angle` (angle units, clockwise on screen).
pub fn turn(p: Pt, angle: i32) -> Pt {
    let (s, c) = (trig::sin(angle) as i64, trig::cos(angle) as i64);
    let one = trig::ONE as i64;
    let (x, y) = (p.0 as i64, p.1 as i64);
    (((x * c - y * s) / one) as i32, ((x * s + y * c) / one) as i32)
}

/// `n` points round an ellipse of radii (rx, ry), turned by `angle`, about
/// `at`.
pub fn ellipse(at: Pt, rx: i32, ry: i32, n: i32, angle: i32) -> Vec<Pt> {
    (0..n)
        .map(|k| {
            let a = k * trig::TURN / n.max(1);
            let p = (rx * trig::cos(a) / trig::ONE, ry * trig::sin(a) / trig::ONE);
            let q = turn(p, angle);
            (at.0 + q.0, at.1 + q.1)
        })
        .collect()
}

/// A side-on fish `len` long, nose along `angle`, middle at `at`; `flip`
/// turns it belly up (a fish flying leftward stays dorsal side up).
pub struct FishOutline {
    pub body: Vec<Pt>,
    pub tail: Vec<Pt>,
    pub fin: Vec<Pt>,
    pub belly: Vec<Pt>,
    pub eye: Pt,
    pub eye_r: i32,
}

pub fn fish(at: Pt, len: i32, angle: i32, flip: bool) -> FishOutline {
    let s = if flip { -1 } else { 1 };
    // in the fish's own frame: nose at +x, back toward -y (up on screen)
    let place = |p: Pt| {
        let q = turn((p.0, p.1 * s), angle);
        (at.0 + q.0, at.1 + q.1)
    };
    let half = len / 2;
    let body: Vec<Pt> = ellipse((0, 0), half * 7 / 10, half * 30 / 100, 18, 0)
        .into_iter()
        .map(|p| place((p.0 + half / 10, p.1)))
        .collect();
    let tail = vec![
        place((-half * 5 / 10, 0)),
        place((-half, -half * 3 / 10)),
        place((-half * 85 / 100, 0)),
        place((-half, half * 3 / 10)),
    ];
    let fin = vec![
        place((-half * 2 / 10, -half * 26 / 100)),
        place((half / 10, -half * 45 / 100)),
        place((half * 3 / 10, -half * 26 / 100)),
    ];
    let belly: Vec<Pt> = ellipse((0, 0), half * 55 / 100, half * 12 / 100, 12, 0)
        .into_iter()
        .map(|p| place((p.0 + half / 10, p.1 + half * 14 / 100)))
        .collect();
    FishOutline {
        body,
        tail,
        fin,
        belly,
        eye: place((half * 55 / 100, -half * 6 / 100)),
        eye_r: (len / 28).max(2),
    }
}

/// Whether (x, y) is inside the polygon (even-odd rule).
pub fn inside(p: Pt, poly: &[Pt]) -> bool {
    let (x, y) = (p.0 as i64, p.1 as i64);
    let mut odd = false;
    let n = poly.len();
    for i in 0..n {
        let (ax, ay) = (poly[i].0 as i64, poly[i].1 as i64);
        let (bx, by) = (poly[(i + n - 1) % n].0 as i64, poly[(i + n - 1) % n].1 as i64);
        if (ay > y) != (by > y) {
            // x of the edge at height y, compared without dividing
            let lhs = (x - ax) * (by - ay);
            let rhs = (bx - ax) * (y - ay);
            if (by > ay && lhs < rhs) || (by < ay && lhs > rhs) {
                odd = !odd;
            }
        }
    }
    odd
}

/// The squared distance between two points.
pub fn dist2(a: Pt, b: Pt) -> i64 {
    let (dx, dy) = ((a.0 - b.0) as i64, (a.1 - b.1) as i64);
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_is_inside_a_square_and_outside_beyond_it() {
        let sq = [(0, 0), (100, 0), (100, 100), (0, 100)];
        assert!(inside((50, 50), &sq));
        assert!(inside((1, 99), &sq));
        assert!(!inside((150, 50), &sq));
        assert!(!inside((50, -1), &sq));
        // a triangle, with the point either side of its slanted edge
        let tri = [(0, 0), (100, 0), (0, 100)];
        assert!(inside((40, 40), &tri));
        assert!(!inside((60, 60), &tri));
    }

    #[test]
    fn a_quarter_turn_takes_right_to_down_on_screen() {
        // clockwise on screen: +x turns to +y (down)
        assert_eq!(turn((100, 0), trig::QUARTER), (0, 100));
        assert_eq!(turn((0, 100), trig::QUARTER), (-100, 0));
    }
}
