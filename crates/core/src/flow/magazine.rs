//! The retro outdoors magazine's printed marks on the 2D layer
//! (DECISIONS.md, 2026-10-18): the 70s triple stripe and a dot screen
//! fading into a corner. The HUD's cards, the popups, the postgame and the
//! minigames' frames draw them; their colours and sizes are data
//! (`data/flow.json` `card`, `data/ui2d.json` `minigames.frame`).

use crate::ui2d::{List, Rect, Rgba};

/// The triple stripe: one band of `each` height per colour, top to
/// bottom, from `x` across `w`.
pub fn stripes(list: &mut List, colours: &[[i32; 3]], x: i32, y: i32, w: i32, each: i32) {
    for (k, c) in colours.iter().enumerate() {
        list.fill_rect(Rgba::solid(*c), Rect::new(x, y + k as i32 * each, w, each));
    }
}

/// A dot screen in the bottom-right corner of `r`: dots on a `step` grid
/// that shrink toward the top left, filling a triangle `w` wide and `h`
/// tall. Returns how many dots it drew.
pub fn dot_corner(list: &mut List, colour: Rgba, r: Rect, w: i32, h: i32, step: i32) -> usize {
    let step = step.max(4);
    let mut n = 0;
    let mut dy = step / 2;
    while dy < h {
        let mut dx = step / 2;
        while dx < w {
            // 0 at the triangle's long edge, 100 in the corner
            let near = 100 - (dx * 100 / w.max(1) + dy * 100 / h.max(1));
            if near > 0 {
                let rad = step * 45 * near / 10_000;
                if rad >= 1 {
                    list.circle(colour, Rgba::NONE, 0, (r.right() - dx, r.bottom() - dy), rad);
                    n += 1;
                }
            }
            dx += step;
        }
        dy += step;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dot_corner_fills_only_the_corner_triangle_with_dots_shrinking_inward() {
        // a 60 x 60 corner on a 20 grid: the grid points are at 10, 30 and
        // 50 each way; "near" is 100 - (dx + dy) * 100 / 60, so (10, 10)
        // is 68, (30, 10) and (10, 30) are 34, (30, 30), (50, 10) and
        // (10, 50) are 0 or less. Radii: 20 * 45 * 68 / 10000 = 6 and
        // 20 * 45 * 34 / 10000 = 3: three dots.
        let mut list = List::new();
        let n = dot_corner(
            &mut list,
            Rgba::solid([1, 2, 3]),
            Rect::new(0, 0, 200, 100),
            60,
            60,
            20,
        );
        assert_eq!(n, 3);
    }
}
