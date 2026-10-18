//! The screen's 2D layer: a resolution-independent draw list the page
//! replays on a 2D canvas laid over the WebGL canvas (PLAN.md decision 7b).
//!
//! The core draws in a *layer*: a virtual canvas of its own units (the HUD's
//! 1920 x 1080, letterboxed into the window; the Fish-Boy's screen, mapped
//! onto where the device's screen part lands in the view). [`List::layer`]
//! sets the transform from layer units to the drawing buffer's pixels, and
//! every later op is in layer units, so a line width or a font size scales
//! with the layer.
//!
//! The list is two arrays: numbers (Q16, so the shim turns them into floats
//! as it does the 3D frame) and strings (colours, fonts, alignments and
//! text, which ops name by index). Each op is its code followed by a fixed
//! number of arguments ([`Op`]); the page looks the codes up by name in
//! [`numbers_json`], so it holds no number of its own (BUILDER-SPEC D3).
//!
//! Two ops reach past the 2D canvas:
//! - [`List::bitmap`] draws an RGBA image the core made once (the radar's
//!   map), cached by the page under its id;
//! - [`List::model`] draws a named model in a rectangle: the page clears
//!   that rectangle on the 2D canvas (a hole) and the WebGL renderer draws
//!   the model there as a [`Pass`] with its own viewport, scissor and
//!   camera. Ops drawn after it in the list land on top of it.
//!
//! The Fish-Boy's screen uses the same list, projected into the screen's
//! rectangle on the overlay rather than drawn into a texture
//! (SECOND-ORDER R2.60).

pub mod theme;

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::json;

use crate::gfx::fx::{self, int, ONE, TURN};
use crate::gfx::render::{Rgb, VIEW_VALUES};
use crate::model::Anim;

/// A half, to round a Q16 product to the nearest whole.
const HALF_UNIT: i64 = 1 << 15;

/// 2π in Q16, for angles in radians.
const TWO_PI: i64 = 411_775;

/// The draw ops, by code. The arguments follow the code in this order;
/// `s:` marks a string index, every other argument is a Q16 number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// sx, sy, tx, ty: layer units to buffer pixels
    Layer,
    /// s:colour, x, y, w, h
    FillRect,
    /// s:colour, width, x, y, w, h
    StrokeRect,
    /// s:fill, s:stroke, width, x, y, w, h, radius ("" for no fill or stroke)
    RoundRect,
    /// s:colour, width, x1, y1, x2, y2
    Line,
    /// s:fill, s:stroke, width, cx, cy, r
    Circle,
    /// s:colour, width, cx, cy, r, from, to (radians, clockwise on screen)
    Arc,
    /// s:fill, s:stroke, width, n, then n points
    Poly,
    /// s:inner, s:outer, cx, cy, r0, r1: a ring shaded from r0 to r1
    Radial,
    /// s:colour, s:font, s:align, x, y, s:text (y is the text's middle)
    Text,
    /// x, y, w, h: clips until the matching Unclip
    Clip,
    Unclip,
    /// s:id, x, y, w, h: a cached RGBA image, unsmoothed
    Bitmap,
    /// x, y, w, h: a hole the 3D renderer draws a model into
    Hole,
    /// n, then n lengths: the dash of later strokes (n = 0: solid)
    Dash,
    /// alpha: of later ops
    Alpha,
}

impl Op {
    pub const ALL: [Op; 16] = [
        Op::Layer,
        Op::FillRect,
        Op::StrokeRect,
        Op::RoundRect,
        Op::Line,
        Op::Circle,
        Op::Arc,
        Op::Poly,
        Op::Radial,
        Op::Text,
        Op::Clip,
        Op::Unclip,
        Op::Bitmap,
        Op::Hole,
        Op::Dash,
        Op::Alpha,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Op::Layer => "layer",
            Op::FillRect => "fill_rect",
            Op::StrokeRect => "stroke_rect",
            Op::RoundRect => "round_rect",
            Op::Line => "line",
            Op::Circle => "circle",
            Op::Arc => "arc",
            Op::Poly => "poly",
            Op::Radial => "radial",
            Op::Text => "text",
            Op::Clip => "clip",
            Op::Unclip => "unclip",
            Op::Bitmap => "bitmap",
            Op::Hole => "hole",
            Op::Dash => "dash",
            Op::Alpha => "alpha",
        }
    }

    fn code(self) -> i32 {
        Op::ALL.iter().position(|o| *o == self).unwrap_or(0) as i32
    }
}

/// A colour with its opacity in percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba(pub [i32; 3], pub i32);

impl Rgba {
    pub const NONE: Rgba = Rgba([0, 0, 0], 0);

    pub fn solid(c: [i32; 3]) -> Rgba {
        Rgba(c, 100)
    }

    pub fn alpha(self, pct: i32) -> Rgba {
        Rgba(self.0, pct)
    }

    /// Each channel scaled by `pct` percent.
    pub fn shade(self, pct: i32) -> Rgba {
        Rgba(self.0.map(|v| (v * pct / 100).clamp(0, 255)), self.1)
    }

    /// The CSS colour, or "" for a colour with no opacity (nothing drawn).
    pub fn css(self) -> String {
        if self.1 <= 0 {
            return String::new();
        }
        let [r, g, b] = self.0.map(|v| v.clamp(0, 255));
        let a = self.1.clamp(0, 100);
        format!("rgba({r},{g},{b},{}.{:02})", a / 100, a % 100)
    }
}

/// Where a text's anchor point is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Centre,
    Right,
}

impl Align {
    fn css(self) -> &'static str {
        match self {
            Align::Left => "left",
            Align::Centre => "center",
            Align::Right => "right",
        }
    }
}

/// A font role from the theme (`data/ui2d.json`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Font {
    /// a CSS font family list
    pub family: String,
    /// a CSS weight: "normal", "bold" or a number in a string
    pub weight: String,
    /// a glyph's advance, in percent of the size (for laying out labels:
    /// the core cannot measure text)
    pub advance: i32,
}

impl Font {
    /// The CSS font at `size` layer units.
    pub fn css(&self, size: i32) -> String {
        format!("{} {size}px {}", self.weight, self.family)
    }

    /// The width of `text` at `size`, by the advance.
    pub fn width(&self, text: &str, size: i32) -> i32 {
        text.chars().count() as i32 * size * self.advance / 100
    }
}

/// A rectangle, in layer units (or pixels, where said).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }

    /// Shrunk by `d` on every side.
    pub fn inset(&self, d: i32) -> Rect {
        Rect::new(self.x + d, self.y + d, self.w - 2 * d, self.h - 2 * d)
    }

    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }
}

/// A layer's transform: buffer pixel = t + s * layer unit (Q16 scale,
/// whole pixels for the offset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transform {
    pub sx: i32,
    pub sy: i32,
    pub tx: i32,
    pub ty: i32,
}

impl Transform {
    pub const IDENTITY: Transform = Transform {
        sx: ONE,
        sy: ONE,
        tx: 0,
        ty: 0,
    };

    /// A canvas of `canvas` units fitted whole into `target` pixels, its
    /// shape kept and centred (letterboxed).
    pub fn letterbox(canvas: [i32; 2], target: Rect) -> Transform {
        let s = fx::ratio(target.w, canvas[0].max(1)).min(fx::ratio(target.h, canvas[1].max(1)));
        let w = fx::mul(int(canvas[0]), s) >> 16;
        let h = fx::mul(int(canvas[1]), s) >> 16;
        Transform {
            sx: s,
            sy: s,
            tx: target.x + (target.w - w) / 2,
            ty: target.y + (target.h - h) / 2,
        }
    }

    /// A canvas of `canvas` units stretched onto `target` pixels.
    pub fn onto(canvas: [i32; 2], target: Rect) -> Transform {
        Transform {
            sx: fx::ratio(target.w, canvas[0].max(1)),
            sy: fx::ratio(target.h, canvas[1].max(1)),
            tx: target.x,
            ty: target.y,
        }
    }

    /// A layer point in pixels.
    pub fn apply(&self, x: i32, y: i32) -> (i32, i32) {
        (
            self.tx + ((x as i64 * self.sx as i64 + HALF_UNIT) >> 16) as i32,
            self.ty + ((y as i64 * self.sy as i64 + HALF_UNIT) >> 16) as i32,
        )
    }

    /// A pixel point in layer units.
    pub fn invert(&self, px: i32, py: i32) -> (i32, i32) {
        let back = |d: i32, s: i32| ((((d as i64) << 16) + s as i64 / 2) / s.max(1) as i64) as i32;
        let (x, y) = (back(px - self.tx, self.sx), back(py - self.ty, self.sy));
        (x, y)
    }

    /// A layer rectangle in pixels.
    pub fn rect(&self, r: Rect) -> Rect {
        let (x0, y0) = self.apply(r.x, r.y);
        let (x1, y1) = self.apply(r.right(), r.bottom());
        Rect::new(x0, y0, x1 - x0, y1 - y0)
    }
}

/// A model drawn into a rectangle by the 3D renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shot {
    /// in buffer pixels, from the top left
    pub px: Rect,
    /// the model's id
    pub model: String,
    pub anim: Option<Anim>,
    /// how far into the animation, in ms
    pub ms: u32,
    /// the rectangle's background (0-255)
    pub background: [i32; 3],
    /// the model's light, in percent (a fish with none left is dimmed)
    pub light_pct: i32,
}

/// One frame's 2D draw list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct List {
    pub nums: Vec<i32>,
    pub strings: Vec<String>,
    index: BTreeMap<String, i32>,
    transform: Option<Transform>,
    /// the models to draw into holes, in list order
    pub shots: Vec<Shot>,
    /// a few words on what is shown, for the browser check
    /// (`body[data-ui2d]`)
    pub status: String,
}

impl List {
    pub fn new() -> List {
        List::default()
    }

    fn s(&mut self, text: &str) -> i32 {
        if let Some(&i) = self.index.get(text) {
            return int(i);
        }
        let i = self.strings.len() as i32;
        self.strings.push(text.to_string());
        self.index.insert(text.to_string(), i);
        int(i)
    }

    fn op(&mut self, op: Op) {
        self.nums.push(int(op.code()));
    }

    fn n(&mut self, v: &[i32]) {
        self.nums.extend(v.iter().map(|&x| int(x)));
    }

    /// Whether anything is drawn.
    pub fn is_empty(&self) -> bool {
        self.nums.is_empty()
    }

    /// The transform of the layer now drawn in.
    pub fn transform(&self) -> Transform {
        self.transform.unwrap_or(Transform::IDENTITY)
    }

    /// Later ops are in this layer's units.
    pub fn layer(&mut self, t: Transform) {
        self.transform = Some(t);
        self.op(Op::Layer);
        self.nums.extend([t.sx, t.sy, int(t.tx), int(t.ty)]);
    }

    pub fn fill_rect(&mut self, c: Rgba, r: Rect) {
        self.op(Op::FillRect);
        let c = self.s(&c.css());
        self.nums.push(c);
        self.n(&[r.x, r.y, r.w, r.h]);
    }

    pub fn stroke_rect(&mut self, c: Rgba, width: i32, r: Rect) {
        self.op(Op::StrokeRect);
        let c = self.s(&c.css());
        self.nums.push(c);
        self.n(&[width, r.x, r.y, r.w, r.h]);
    }

    pub fn round_rect(&mut self, fill: Rgba, stroke: Rgba, width: i32, r: Rect, radius: i32) {
        self.op(Op::RoundRect);
        let f = self.s(&fill.css());
        let s = self.s(&stroke.css());
        self.nums.extend([f, s]);
        self.n(&[width, r.x, r.y, r.w, r.h, radius]);
    }

    pub fn line(&mut self, c: Rgba, width: i32, a: (i32, i32), b: (i32, i32)) {
        self.op(Op::Line);
        let c = self.s(&c.css());
        self.nums.push(c);
        self.n(&[width, a.0, a.1, b.0, b.1]);
    }

    /// A line with Q16 ends, for points that fall between units.
    pub fn line_q(&mut self, c: Rgba, width: i32, a: (i32, i32), b: (i32, i32)) {
        self.op(Op::Line);
        let c = self.s(&c.css());
        self.nums.extend([c, int(width), a.0, a.1, b.0, b.1]);
    }

    pub fn circle(&mut self, fill: Rgba, stroke: Rgba, width: i32, at: (i32, i32), r: i32) {
        self.op(Op::Circle);
        let f = self.s(&fill.css());
        let s = self.s(&stroke.css());
        self.nums.extend([f, s]);
        self.n(&[width, at.0, at.1, r]);
    }

    /// An arc from angle `from` to `to` (angle units, clockwise on screen
    /// from the right).
    pub fn arc(&mut self, c: Rgba, width: i32, at: (i32, i32), r: i32, from: i32, to: i32) {
        self.op(Op::Arc);
        let c = self.s(&c.css());
        self.nums.push(c);
        self.n(&[width, at.0, at.1, r]);
        self.nums.extend([radians(from), radians(to)]);
    }

    pub fn poly(&mut self, fill: Rgba, stroke: Rgba, width: i32, points: &[(i32, i32)]) {
        self.op(Op::Poly);
        let f = self.s(&fill.css());
        let s = self.s(&stroke.css());
        self.nums.extend([f, s]);
        self.n(&[width, points.len() as i32]);
        for p in points {
            self.n(&[p.0, p.1]);
        }
    }

    pub fn radial(&mut self, inner: Rgba, outer: Rgba, at: (i32, i32), r0: i32, r1: i32) {
        self.op(Op::Radial);
        let a = self.s(&inner.css());
        let b = self.s(&outer.css());
        self.nums.extend([a, b]);
        self.n(&[at.0, at.1, r0.max(0), r1.max(0)]);
    }

    pub fn text(&mut self, c: Rgba, font: &Font, size: i32, align: Align, at: (i32, i32), text: &str) {
        self.op(Op::Text);
        let c = self.s(&c.css());
        let f = self.s(&font.css(size));
        let a = self.s(align.css());
        self.nums.extend([c, f, a]);
        self.n(&[at.0, at.1]);
        let t = self.s(text);
        self.nums.push(t);
    }

    pub fn clip(&mut self, r: Rect) {
        self.op(Op::Clip);
        self.n(&[r.x, r.y, r.w, r.h]);
    }

    pub fn unclip(&mut self) {
        self.op(Op::Unclip);
    }

    pub fn bitmap(&mut self, id: &str, r: Rect) {
        self.op(Op::Bitmap);
        let i = self.s(id);
        self.nums.push(i);
        self.n(&[r.x, r.y, r.w, r.h]);
    }

    /// A model drawn into `r` by the 3D renderer, on `background`, lit
    /// at `light_pct` percent.
    pub fn model(
        &mut self,
        r: Rect,
        model: &str,
        anim: Option<Anim>,
        ms: u32,
        background: [i32; 3],
        light_pct: i32,
    ) {
        self.op(Op::Hole);
        self.n(&[r.x, r.y, r.w, r.h]);
        let px = self.transform().rect(r);
        self.shots.push(Shot {
            px,
            model: model.to_string(),
            anim,
            ms,
            background,
            light_pct,
        });
    }

    pub fn dash(&mut self, lengths: &[i32]) {
        self.op(Op::Dash);
        self.n(&[lengths.len() as i32]);
        self.n(lengths);
    }

    pub fn alpha(&mut self, pct: i32) {
        self.op(Op::Alpha);
        self.nums.push(fx::ratio(pct.clamp(0, 100), 100));
    }

    /// The strings as JSON, with the status: what the page parses.
    pub fn strings_json(&self) -> String {
        json!({ "strings": self.strings, "status": self.status }).to_string()
    }
}

/// Angle units to radians, Q16.
pub fn radians(a: i32) -> i32 {
    (a as i64 * TWO_PI / TURN as i64) as i32
}

/// An RGBA image the core made, which the page draws by its id. The id
/// changes when the image does, so the page's cache stays right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bitmap {
    pub id: String,
    pub w: i32,
    pub h: i32,
    /// palette indices, row by row from the top
    pub pixels: Vec<u8>,
    pub palette: Vec<Rgba>,
}

impl Bitmap {
    /// Four bytes a pixel, as an `ImageData` takes them.
    pub fn rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pixels.len() * 4);
        for &p in &self.pixels {
            let c = self.palette.get(p as usize).copied().unwrap_or(Rgba::NONE);
            out.extend(c.0.map(|v| v.clamp(0, 255) as u8));
            out.push((c.1.clamp(0, 100) * 255 / 100) as u8);
        }
        out
    }
}

/// One 3D drawing into part of the buffer: a model viewport, or the
/// Fish-Boy over the world. Values: the rectangle in GL pixels (from the
/// bottom left), whether to clear the colour, the clear colour, and the
/// view values `gfx::render::view_values` makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pass {
    /// x, y (from the bottom), w, h in pixels
    pub gl_rect: [i32; 4],
    pub clear: Option<Rgb>,
    pub view: Vec<i32>,
    pub tris: Vec<i32>,
}

/// Where each field of a pass's values starts, after the four of the
/// rectangle.
const PASS_CLEAR: usize = 4;
const PASS_COLOUR: usize = 5;
const PASS_VIEW: usize = 8;

impl Pass {
    /// A rectangle in top-left pixels as GL's, in a buffer `size` high.
    pub fn gl(px: Rect, size: [i32; 2]) -> [i32; 4] {
        [px.x, size[1] - px.y - px.h, px.w.max(0), px.h.max(0)]
    }

    /// The pass's values (Q16), laid out as [`numbers_json`] says.
    pub fn values(&self) -> Vec<i32> {
        let mut v: Vec<i32> = self.gl_rect.iter().map(|&x| int(x)).collect();
        v.push(if self.clear.is_some() { ONE } else { 0 });
        v.extend(self.clear.unwrap_or([0; 3]));
        v.extend(&self.view);
        v
    }
}

/// One frame's 2D list and passes, as the shim reads them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Built {
    pub list: List,
    pub passes: Vec<Pass>,
}

impl Built {
    pub fn pass_count(&self) -> u32 {
        self.passes.len() as u32
    }

    pub fn pass_values(&self, k: u32) -> Vec<i32> {
        self.passes.get(k as usize).map_or(Vec::new(), |p| p.values())
    }

    pub fn pass_vertices(&self, k: u32) -> Vec<i32> {
        self.passes.get(k as usize).map_or(Vec::new(), |p| p.tris.clone())
    }

    pub fn pass_vertex_count(&self, k: u32) -> u32 {
        self.passes
            .get(k as usize)
            .map_or(0, |p| crate::gfx::render::vertex_count(&p.tris))
    }
}

/// Every number the page's 2D replayer and its passes need, as JSON: the
/// op codes by name, and where a pass's fields sit.
pub fn numbers_json() -> String {
    let ops: BTreeMap<&str, i32> = Op::ALL.iter().map(|o| (o.name(), o.code())).collect();
    let mut out = json!({
        "ops": ops,
        "pass": {
            "rect": [0, PASS_CLEAR],
            "clear": PASS_CLEAR,
            "colour": [PASS_COLOUR, PASS_VIEW],
            "view": [PASS_VIEW, PASS_VIEW + VIEW_VALUES],
        },
    })
    .to_string();
    // a full turn in radians, for the canvas's arcs: the core holds no
    // float, so it goes in as text, to six places
    let micro = TWO_PI * 1_000_000 / ONE as i64;
    out.pop();
    out.push_str(&format!(
        ",\"full_turn\":{}.{:06}}}",
        micro / 1_000_000,
        micro % 1_000_000
    ));
    out
}

#[cfg(test)]
mod tests;
