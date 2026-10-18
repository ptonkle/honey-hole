//! The full-screen minigames' frame in the magazine look (DECISIONS.md,
//! 2026-10-18): a paper border with an ink keyline round the canvas, the
//! triple stripe along its top, and a dot screen in the bottom corners.
//! The Fish-Boy's minigames are the device's, so they keep its phosphor
//! and get no frame. The look is `data/ui2d.json` `minigames.frame`.

use crate::ui2d::{Align, Font, List, Rect, Rgba};

use super::look::FrameLook;

pub fn draw(list: &mut List, f: &FrameLook, canvas: [i32; 2]) {
    let [w, h] = canvas;
    let b = f.width;
    let paper = Rgba::solid(f.paper);
    let ink = Rgba::solid(f.ink);
    // the paper border, then the keyline inside it
    list.fill_rect(paper, Rect::new(0, 0, w, b));
    list.fill_rect(paper, Rect::new(0, h - b, w, b));
    list.fill_rect(paper, Rect::new(0, 0, b, h));
    list.fill_rect(paper, Rect::new(w - b, 0, b, h));
    list.round_rect(
        Rgba::NONE,
        ink,
        f.keyline,
        Rect::new(b, b, w - 2 * b, h - 2 * b),
        f.radius,
    );
    // the triple stripe in the top border
    let band = f.stripe_h * f.stripes.len() as i32;
    crate::flow::magazine::stripes(list, &f.stripes, 0, (b - band) / 2, w, f.stripe_h);
    // a dot screen in the bottom border's corners
    let dots = Rgba::solid(f.dots);
    for x in [b, w - b - f.dots_w] {
        let mut k = 0;
        while k * f.dot_step < f.dots_w {
            let at = (x + k * f.dot_step + f.dot_step / 2, h - b / 2);
            // largest at the outer edge
            let from_edge = if x == b { k } else { f.dots_w / f.dot_step - k };
            let rad = (f.dot_step * 4 / 10 - from_edge).max(0);
            if rad > 0 {
                list.circle(dots, Rgba::NONE, 0, at, rad);
            }
            k += 1;
        }
    }
}

/// A call on a paper label centred at `at`: paper in the ink keyline,
/// the words in ink.
pub fn label(list: &mut List, f: &FrameLook, font: &Font, size: i32, at: (i32, i32), text: &str) {
    let w = font.width(text, size) + size * 3 / 2;
    let h = size * 17 / 10;
    let r = Rect::new(at.0 - w / 2, at.1 - h / 2, w, h);
    list.round_rect(Rgba::solid(f.paper), Rgba::solid(f.ink), f.keyline, r, f.radius);
    list.text(
        Rgba::solid(f.ink),
        font,
        size,
        Align::Centre,
        (at.0, at.1 + 2),
        text,
    );
}
