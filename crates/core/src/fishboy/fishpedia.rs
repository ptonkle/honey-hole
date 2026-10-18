//! The Fishpedia: each kind of fish on this map, in tier rows (legendary
//! to common, as `brief/fishpedia.png` lays them out), each in a box with
//! its name, its live idle swim and how many are left; and the catch list
//! ("My catch"): your own catches this match, with no values.
//!
//! Both read plain view structs the match fills ([`FishpediaRow`],
//! [`CatchRow`]). A tier shows by its colour, its name and its frame's
//! shape, never by colour alone.

use crate::map::fish::Tier;
use crate::model::Anim;
use crate::roster::{Roster, TIERS};
use crate::ui2d::theme::{c, Theme};
use crate::ui2d::{Align, List, Rect, Rgba};

/// One kind of fish on this map and how many are left (public knowledge).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FishpediaRow {
    /// the fish's id in `data/fish.json`
    pub kind: String,
    pub tier: Tier,
    pub left: u32,
}

/// One of your catches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatchRow {
    pub kind: String,
    pub length_mm: i32,
}

/// The rows of tiers on the screen (1600 x 900 units).
const TOP: i32 = 104;
const ROW_H: i32 = 184;
const ROW_GAP: i32 = 12;
const FRAME_X: i32 = 36;
const FRAME_W: i32 = 1528;
const BOX_X: i32 = 380;
const BOX_W: i32 = 270;
const BOX_GAP: i32 = 20;
const BOX_TOP: i32 = 50;
const BOX_H: i32 = 122;
/// Boxes a row holds.
pub const PER_ROW: usize = 4;

/// The strings the Fishpedia draws, filled.
pub struct Words {
    /// by tier, its name
    pub tier: Vec<(Tier, String)>,
    /// "{count} left", to fill
    pub left: String,
    pub none_left: String,
}

/// A frame in the tier's shape.
pub fn frame(list: &mut List, theme: &Theme, tier: Tier, r: Rect, width: i32) {
    let look = theme.tiers.get(tier);
    let col = c(look.colour);
    match look.frame.as_str() {
        "double" => {
            list.stroke_rect(col, width, r);
            list.stroke_rect(col, width / 2 + 1, r.inset(width * 2 + 2));
        }
        "notched" => {
            let k = (r.h / 8).max(6);
            let (x0, y0, x1, y1) = (r.x, r.y, r.right(), r.bottom());
            list.poly(
                Rgba::NONE,
                col,
                width,
                &[
                    (x0 + k, y0),
                    (x1 - k, y0),
                    (x1, y0 + k),
                    (x1, y1 - k),
                    (x1 - k, y1),
                    (x0 + k, y1),
                    (x0, y1 - k),
                    (x0, y0 + k),
                    (x0 + k, y0),
                ],
            );
        }
        "dashed" => {
            list.dash(&[width * 5, width * 3]);
            list.stroke_rect(col, width, r);
            list.dash(&[]);
        }
        _ => list.stroke_rect(col, width, r),
    }
}

/// A fish's outline, for a fish whose model is not made yet.
fn silhouette(list: &mut List, r: Rect, colour: Rgba) {
    let (cx, cy) = (r.x + r.w / 2 - r.w / 14, r.y + r.h / 2);
    let (hw, hh) = (r.w * 3 / 10, r.h * 2 / 9);
    // a body of twelve points round an ellipse, the tail on the left
    let pts: Vec<(i32, i32)> = (0..12)
        .map(|k| {
            let a = crate::gfx::fx::TURN * k / 12;
            (
                cx + crate::gfx::fx::mul(crate::gfx::fx::cos(a), hw << 16) / 65536,
                cy - crate::gfx::fx::mul(crate::gfx::fx::sin(a), hh << 16) / 65536,
            )
        })
        .collect();
    list.poly(colour, Rgba::NONE, 0, &pts);
    let tail = cx - hw + hw / 10;
    list.poly(
        colour,
        Rgba::NONE,
        0,
        &[(tail, cy), (tail - hw / 2, cy - hh), (tail - hw / 2, cy + hh)],
    );
    // the fin on top
    list.poly(
        colour,
        Rgba::NONE,
        0,
        &[
            (cx - hw / 3, cy - hh + 2),
            (cx, cy - hh - hh / 2),
            (cx + hw / 4, cy - hh + 2),
        ],
    );
}

/// Draws the Fishpedia. `has_model` says whether a model file exists;
/// `ms` is how far into the idle swim the boxes are.
pub fn draw(
    list: &mut List,
    theme: &Theme,
    roster: &Roster,
    rows: &[FishpediaRow],
    words: &Words,
    has_model: &dyn Fn(&str) -> bool,
    ms: u32,
) {
    let font = theme.mono();
    let look = &theme.fishpedia;
    for (k, tier) in TIERS.iter().enumerate() {
        let y = TOP + k as i32 * (ROW_H + ROW_GAP);
        let tl = theme.tiers.get(*tier);
        let col = c(tl.colour);
        frame(list, theme, *tier, Rect::new(FRAME_X, y, FRAME_W, ROW_H), 4);
        let name = words
            .tier
            .iter()
            .find(|(t, _)| t == tier)
            .map_or(String::new(), |(_, n)| n.to_uppercase());
        list.text(col, font, 36, Align::Left, (FRAME_X + 34, y + 44), &name);
        let fish: Vec<&FishpediaRow> = rows.iter().filter(|r| r.tier == *tier).collect();
        let total: u32 = fish.iter().map(|r| r.left).sum();
        list.text(
            col,
            font,
            28,
            Align::Left,
            (FRAME_X + 34, y + ROW_H - 38),
            &words.left.replace("{count}", &total.to_string()),
        );
        for (j, row) in fish.iter().take(PER_ROW).enumerate() {
            let x = BOX_X + j as i32 * (BOX_W + BOX_GAP);
            let b = Rect::new(x, y + BOX_TOP, BOX_W, BOX_H);
            let kind = roster.get(&row.kind);
            let label = kind.map_or(row.kind.clone(), |k| k.name.to_uppercase());
            list.text(
                c(theme.screen.phosphor),
                font,
                24,
                Align::Centre,
                (x + BOX_W / 2, y + 26),
                &label,
            );
            let gone = row.left == 0;
            let dim = if gone { look.gone_pct } else { 100 };
            let model = kind.map_or(row.kind.as_str(), |k| k.model.as_str());
            if has_model(model) {
                list.model(
                    b,
                    model,
                    Some(Anim::Idle),
                    ms,
                    c(look.box_colour).shade(dim).0,
                    dim,
                );
            } else {
                list.fill_rect(c(look.box_colour).shade(dim), b);
                silhouette(list, b.inset(8), c(look.silhouette).shade(dim));
            }
            list.stroke_rect(c(look.box_line), 2, b);
            let count = if gone {
                words.none_left.to_uppercase()
            } else {
                words
                    .left
                    .replace("{count}", &row.left.to_string())
                    .to_uppercase()
            };
            let w = font.width(&count, 24);
            let ink = if gone {
                c(theme.screen.warning)
            } else {
                c(theme.screen.phosphor)
            };
            list.fill_rect(
                c(theme.radar.label_back).alpha(85),
                Rect::new(b.right() - w - 16, b.bottom() - 34, w + 12, 30),
            );
            list.text(
                ink,
                font,
                24,
                Align::Right,
                (b.right() - 10, b.bottom() - 18),
                &count,
            );
        }
    }
}

/// The rows of the catch list.
const CATCH_TOP: i32 = 170;
const CATCH_ROW: i32 = 58;
/// Catch rows that fit; the newest are shown.
pub const CATCH_ROWS: usize = 12;

/// The catch list's strings, filled.
pub struct CatchWords {
    pub heading: String,
    pub none: String,
    /// "{feet} ft", to fill
    pub feet: String,
    pub tier: Vec<(Tier, String)>,
    /// "{count} caught"
    pub count: String,
}

/// Draws your catch list, newest first: each fish's tier (its frame, its
/// colour and its name), its name and its length. No values: the scores
/// stay hidden until the postgame.
pub fn draw_catches(
    list: &mut List,
    theme: &Theme,
    roster: &Roster,
    catches: &[CatchRow],
    words: &CatchWords,
) {
    let font = theme.mono();
    let phos = c(theme.screen.phosphor);
    list.text(
        phos,
        font,
        44,
        Align::Left,
        (60, 132),
        &words.heading.to_uppercase(),
    );
    list.text(
        phos,
        font,
        30,
        Align::Right,
        (1540, 132),
        &words
            .count
            .replace("{count}", &catches.len().to_string())
            .to_uppercase(),
    );
    list.line(c(theme.screen.dim), 3, (60, 160), (1540, 160));
    if catches.is_empty() {
        list.text(
            c(theme.screen.dim).shade(170),
            font,
            32,
            Align::Left,
            (60, 230),
            &words.none,
        );
        return;
    }
    for (k, row) in catches.iter().rev().take(CATCH_ROWS).enumerate() {
        let y = CATCH_TOP + 24 + k as i32 * CATCH_ROW;
        let Some(kind) = roster.get(&row.kind) else {
            continue;
        };
        let col = c(theme.tiers.get(kind.tier).colour);
        frame(list, theme, kind.tier, Rect::new(60, y - 20, 40, 40), 3);
        list.circle(col, Rgba::NONE, 0, (80, y), 9);
        list.text(phos, font, 32, Align::Left, (130, y), &kind.name.to_uppercase());
        let feet = words
            .feet
            .replace("{feet}", &crate::app::gallery::feet(row.length_mm));
        list.text(phos, font, 32, Align::Right, (1060, y), &feet);
        let tier = words
            .tier
            .iter()
            .find(|(t, _)| *t == kind.tier)
            .map_or(String::new(), |(_, n)| n.to_uppercase());
        list.text(col, font, 30, Align::Left, (1160, y), &tier);
    }
}
