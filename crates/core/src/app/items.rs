//! Items on screen: the buoy and the items' effects in the world (the
//! giant bobber, the nitrous trail, the teleport's shimmer, your own boat
//! see-through while cloaked), the HUD's item slot, effect timers and the
//! camera's offer, the minigames' layers, the radar's teleport choice and
//! marked fish, the item popups, the pointer for the minigames, and the
//! `?debug=items` flag. Everything here reads the match
//! (`sim::items`) and draws it; no rule is decided here. The drawing's
//! numbers are the `items` section of `data/ui2d.json` ([`ItemsLook`]).
//!
//! The HUD here is drawn straight on the 2D layer's HUD canvas
//! (`hud.canvas`); the match-flow HUD, once merged, gives the slot a place
//! of its own (SECOND-ORDER R4.20).

use serde::Deserialize;

use super::*;
use crate::fishboy::radar::{self, RadarScene};
use crate::gfx::fx::{self, HALF, ONE, QUARTER};
use crate::gfx::geom::{v3, V3};
use crate::gfx::render::{self, Rgb};
use crate::map::P;
use crate::minigame::{Layer, Minigame};
use crate::model::Anim;
use crate::scene::{mm, Instance, Lake};
use crate::sim::fishing::lake::ahead;
use crate::sim::items::{claims, Item, SeatItems};
use crate::sim::view::BoatView;
use crate::ui2d::theme::{c, Theme};
use crate::ui2d::{Align, List, Rect, Rgba, Transform};

use super::play::PlayView;

/// One colour per item, for its icon.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Icons {
    pub cloak: [i32; 3],
    pub power_reel: [i32; 3],
    pub nitrous: [i32; 3],
    pub teleporter: [i32; 3],
    pub camera: [i32; 3],
    pub enhanced_radar: [i32; 3],
}

impl Icons {
    pub fn get(&self, item: Item) -> [i32; 3] {
        match item {
            Item::Cloak => self.cloak,
            Item::PowerReel => self.power_reel,
            Item::Nitrous => self.nitrous,
            Item::Teleporter => self.teleporter,
            Item::Camera => self.camera,
            Item::EnhancedRadar => self.enhanced_radar,
        }
    }
}

/// `data/ui2d.json`'s `items` section. Rectangles are [x, y, w, h] on the
/// HUD canvas; lengths in the world are mm.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemsLook {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// the item slot, bottom right
    pub slot: [i32; 4],
    pub panel: [i32; 3],
    pub edge: [i32; 3],
    pub ink: [i32; 3],
    pub dim: [i32; 3],
    pub key_cap: [i32; 3],
    pub icons: Icons,
    /// each effect timer's row above the slot
    pub effect_h: i32,
    /// the camera's offer, bottom middle
    pub offer: [i32; 4],
    /// your own boat while cloaked: its colour this far toward the tint
    pub cloak_tint: [i32; 3],
    pub cloak_tint_pct: i32,
    /// the nitrous trail behind the boat
    pub trail: [i32; 3],
    pub trail_mm: i32,
    pub trail_width_mm: i32,
    /// the teleport's shimmer round the boat
    pub shimmer: [i32; 3],
    pub shimmer_mm: i32,
    pub shimmer_height_mm: i32,
    /// the teleport's choice on the radar
    pub choose: [i32; 3],
}

/// `?debug=items` puts a buoy this far ahead of the boat (or nearer, the
/// first of these on open water).
const DEBUG_AHEAD_MM: [i32; 3] = [40_000, 25_000, 15_000];

/// A point of an icon's 100 x 100 box in `r`.
fn at(r: Rect, x: i32, y: i32) -> (i32, i32) {
    (r.x + r.w * x / 100, r.y + r.h * y / 100)
}

/// An item's icon in `r`: a small flat picture, drawn in the core.
pub fn draw_icon(list: &mut List, item: Item, r: Rect, colour: Rgba, ink: Rgba) {
    let s = |v: i32| r.w * v / 100;
    let p = |x: i32, y: i32| at(r, x, y);
    match item {
        Item::Cloak => {
            // a boat from above, fading out: see-through, dashed
            let hull = [p(50, 10), p(70, 36), p(68, 86), p(32, 86), p(30, 36)];
            list.poly(colour.alpha(35), Rgba::NONE, 0, &hull);
            list.dash(&[s(9), s(6)]);
            list.poly(Rgba::NONE, colour, s(5), &hull);
            list.dash(&[]);
            for (x, y) in [(20, 24), (82, 58), (18, 70)] {
                list.circle(ink, Rgba::NONE, 0, p(x, y), s(4));
            }
        }
        Item::PowerReel => {
            // a reel turned by its handle, with a spark
            list.circle(colour.alpha(40), colour, s(8), p(44, 58), s(28));
            list.circle(colour, Rgba::NONE, 0, p(44, 58), s(8));
            list.line(colour, s(7), p(44, 58), p(74, 76));
            list.circle(ink, colour, s(4), p(76, 78), s(8));
            list.poly(
                ink,
                Rgba::NONE,
                0,
                &[
                    p(74, 6),
                    p(60, 30),
                    p(72, 30),
                    p(64, 50),
                    p(88, 22),
                    p(76, 22),
                    p(84, 6),
                ],
            );
        }
        Item::Nitrous => {
            // a gas bottle over a flame
            list.round_rect(
                colour,
                ink,
                s(3),
                Rect::new(p(32, 18).0, p(32, 18).1, s(36), s(56)),
                s(12),
            );
            list.fill_rect(ink, Rect::new(p(42, 6).0, p(42, 6).1, s(16), s(13)));
            list.line(ink, s(4), p(40, 40), p(60, 40));
            list.poly(
                c([255, 150, 40]),
                Rgba::NONE,
                0,
                &[p(34, 78), p(50, 98), p(66, 78), p(58, 84), p(50, 76), p(42, 84)],
            );
        }
        Item::Teleporter => {
            // rings round a four-pointed spark
            list.circle(Rgba::NONE, colour, s(6), p(50, 50), s(40));
            list.circle(Rgba::NONE, colour.alpha(70), s(4), p(50, 50), s(26));
            list.poly(
                ink,
                Rgba::NONE,
                0,
                &[
                    p(50, 26),
                    p(56, 44),
                    p(74, 50),
                    p(56, 56),
                    p(50, 74),
                    p(44, 56),
                    p(26, 50),
                    p(44, 44),
                ],
            );
        }
        Item::Camera => {
            // a camera body, its lens and flash
            list.fill_rect(colour, Rect::new(p(30, 22).0, p(30, 22).1, s(20), s(12)));
            list.round_rect(
                colour,
                ink,
                s(3),
                Rect::new(p(12, 32).0, p(12, 32).1, s(76), s(52)),
                s(8),
            );
            list.circle(c([30, 34, 40]), ink, s(5), p(50, 58), s(17));
            list.circle(ink.alpha(70), Rgba::NONE, 0, p(45, 53), s(5));
            list.fill_rect(ink, Rect::new(p(70, 38).0, p(70, 38).1, s(10), s(7)));
        }
        Item::EnhancedRadar => {
            // a radar's sweep finding a fish
            list.circle(colour.alpha(25), colour, s(5), p(50, 50), s(42));
            list.line(colour.alpha(60), s(2), p(8, 50), p(92, 50));
            list.line(colour.alpha(60), s(2), p(50, 8), p(50, 92));
            list.line(colour, s(5), p(50, 50), p(78, 20));
            list.circle(ink, Rgba::NONE, 0, p(34, 66), s(8));
            list.poly(ink, Rgba::NONE, 0, &[p(40, 66), p(52, 58), p(52, 74)]);
        }
    }
}

/// A strip from `a` to `b`, `width` wide, turned to face `eye`.
fn facing_quad(out: &mut Vec<i32>, a: V3, b: V3, eye: V3, width: i32, colour: Rgb) {
    let along = b.sub(a);
    let side = along.cross(eye.sub(a)).norm().scale(width / 2);
    if side == V3::ZERO {
        return;
    }
    let n = along.cross(side).norm();
    let (a0, a1, b0, b1) = (a.sub(side), a.add(side), b.sub(side), b.add(side));
    render::push_tri_glow(out, [a0, a1, b1], n, colour, ONE);
    render::push_tri_glow(out, [a0, b1, b0], n, colour, ONE);
}

/// A flat triangle on the water, seen from above and below.
fn flat(out: &mut Vec<i32>, p: [V3; 3], colour: Rgb) {
    let up = v3(0, ONE, 0);
    render::push_tri_glow(out, p, up, colour, ONE);
    render::push_tri_glow(out, [p[0], p[2], p[1]], up, colour, ONE);
}

/// The largest text size up to `size` at which `text` fits in `width`.
pub fn fit(font: &crate::ui2d::Font, text: &str, size: i32, width: i32) -> i32 {
    let w = font.width(text, size).max(1);
    if w <= width {
        size
    } else {
        (size * width / w).max(8)
    }
}

fn mix(a: [i32; 3], b: [i32; 3], pct: i32) -> [i32; 3] {
    [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * pct / 100)
}

impl App {
    fn items_look(&self) -> Option<&ItemsLook> {
        self.fishboy.theme.as_ref().map(|t| &t.items)
    }

    /// This browser's seat's items.
    pub fn my_items(&self) -> Option<&SeatItems> {
        let p = self.play.as_ref()?;
        p.game.items.seats.get(p.seat)
    }

    /// The minigame this seat is playing, if any.
    pub fn my_minigame(&self) -> Option<&Minigame> {
        self.my_items()?.playing.as_ref().map(|p| &p.game)
    }

    /// A minigame takes the pointer: the page lets it go to point.
    pub fn items_take_pointer(&self) -> bool {
        self.screen == Screen::Play && self.my_minigame().is_some()
    }

    /// Where a minigame is drawn, in buffer pixels: the Fish-Boy's screen
    /// or the whole view.
    fn item_minigame_target(&self, mg: &Minigame) -> Option<Rect> {
        match mg.layer() {
            Layer::FishBoy => self.fishboy_raised().map(|r| r.px),
            Layer::Hud => Some(Rect::new(0, 0, self.size[0], self.size[1])),
        }
    }

    /// The pointer's layer while a minigame plays: its canvas on screen.
    pub(crate) fn items_pointer_transform(&self) -> Option<Transform> {
        let mg = self.my_minigame()?;
        Some(mg.transform(self.item_minigame_target(mg)?))
    }

    /// The pointer over the 2D layer reaches the match in this seat's
    /// input: its place in the layer's units and the main button.
    pub(crate) fn items_pointer(&mut self, name: &str, at: Option<(i32, i32)>) {
        let (Some(controls), Some(p)) = (self.controls.as_ref(), self.play.as_mut()) else {
            return;
        };
        p.gather.point(at.map(|(x, y)| [x, y]));
        match name {
            "pointer.down" => {
                p.gather.down(controls, "Mouse0");
            }
            "pointer.up" => p.gather.up(controls, "Mouse0"),
            _ => {}
        }
    }

    /// After the ticks: the radar tab for a teleport's choice and an
    /// Enhanced Radar, and the debug flags' buoy, item and quick
    /// minigames.
    pub(super) fn items_after_ticks(&mut self, heard: &[serde_json::Value]) {
        let Some(p) = self.play.as_mut() else {
            return;
        };
        let seat = p.seat as u64;
        if heard
            .iter()
            .any(|e| (e["type"] == "TeleportChoosing" || e["type"] == "Revealed") && e["seat"] == seat)
        {
            self.fishboy.tab = crate::fishboy::Tab::Radar;
        }
        let flags = &self.debug_flags;
        if !flags.iter().any(|f| f == "items") {
            return;
        }
        let it = &mut p.game.items;
        it.quick = flags.iter().any(|f| f == "quick");
        if it.force_next.is_none() {
            it.force_next = flags
                .iter()
                .find_map(|f| f.strip_prefix("item.").and_then(Item::from_key));
        }
        if it.buoy.is_some() {
            return;
        }
        let Some(b) = p.game.boat(p.seat).filter(|b| !b.wrecked()) else {
            return;
        };
        let (from, heading) = (b.at(), b.heading);
        if let Some(to) = DEBUG_AHEAD_MM
            .iter()
            .map(|&d| ahead(from, heading, d))
            .find(|&to| p.game.map.wet(to))
        {
            p.game.place_buoy(to);
        }
    }

    // ---- the world ----

    /// The buoy, bobbing where it floats.
    pub(super) fn item_instances<'a>(&'a self, lake: &Lake, view: &PlayView, out: &mut Vec<Instance<'a>>) {
        let Some([x, y]) = view.world.buoy else {
            return;
        };
        let Some(model) = self.models.iter().find(|m| m.id == "item-buoy") else {
            return;
        };
        let ms = ((self.clock.max(0) / 1000) % 60_000) as u32;
        out.push(Instance {
            model,
            place: lake.place(P::new(x, y), 0, 0, 1000),
            anim: Some(Anim::Bob),
            ms,
            colour: [0; 3],
        });
    }

    /// A boat's colour: your own, cloaked, is faded toward the tint, so it
    /// reads as see-through to you (others see it as ever).
    pub(super) fn boat_colour(&self, colour: Rgb, b: &BoatView, me: usize) -> Rgb {
        match self.items_look() {
            Some(l) if b.cloaked && b.seat == me => {
                let tint = render::rgb(l.cloak_tint);
                mix(colour, tint, l.cloak_tint_pct)
            }
            _ => colour,
        }
    }

    /// The nitrous trails and the teleport shimmers, as triangles seen
    /// from `eye`.
    pub(super) fn item_tris(&self, lake: &Lake, view: &PlayView, eye: V3) -> Vec<i32> {
        let mut out = Vec::new();
        let Some(l) = self.items_look() else {
            return out;
        };
        let ms = self.clock.max(0) / 1000;
        for b in view.world.boats.iter().filter(|b| !b.wrecked) {
            let at = P::new(b.x_mm, b.y_mm);
            if b.nitrous {
                self.trail(&mut out, lake, l, at, b.heading, ms);
            }
            if b.charge_permille > 0 {
                self.shimmer(&mut out, lake, l, at, b.charge_permille, ms, eye);
            }
        }
        out
    }

    /// Two flickering tongues of foam and flame on the water behind the
    /// stern, widening as they go.
    fn trail(&self, out: &mut Vec<i32>, lake: &Lake, l: &ItemsLook, at: P, heading: i32, ms: i64) {
        // a plume on the water from the motor back: shock diamonds that
        // flicker with the clock, a pale core inside a coloured flame
        const PROFILE: [i32; 10] = [30, 60, 85, 60, 80, 55, 65, 40, 25, 0];
        let back = heading + HALF;
        let stern = 1900;
        let step = l.trail_mm / (PROFILE.len() as i32 - 1);
        let core = render::rgb([235, 250, 255]);
        let glow = render::rgb(l.trail);
        let station = |k: usize, scale: i32| -> (P, i32) {
            let flick = ((ms / 60 + k as i64 * 7) % 5) as i32 - 2;
            let w = l.trail_width_mm * PROFILE[k] * (100 + flick * 8) / 100 / 100 * scale / 100;
            (ahead(at, back, stern + k as i32 * step), w / 2)
        };
        for (scale, up, from, to) in [(100, 60, glow, glow), (45, 90, core, glow)] {
            for k in 0..PROFILE.len() - 1 {
                let (p0, h0) = station(k, scale);
                let (p1, h1) = station(k + 1, scale);
                let side = |p: P, h: i32, s: i32| lake.world(ahead(p, heading + QUARTER, s * h), up);
                let colour = mix(from, to, k as i32 * 100 / (PROFILE.len() as i32 - 1));
                flat(out, [side(p0, h0, 1), side(p0, h0, -1), side(p1, h1, -1)], colour);
                flat(out, [side(p0, h0, 1), side(p1, h1, -1), side(p1, h1, 1)], colour);
            }
        }
    }

    /// Glowing strands rising round the boat as its teleport charges, and
    /// a ring on the water, turning.
    #[allow(clippy::too_many_arguments)]
    fn shimmer(
        &self,
        out: &mut Vec<i32>,
        lake: &Lake,
        l: &ItemsLook,
        at: P,
        permille: i32,
        ms: i64,
        eye: V3,
    ) {
        let strands = 12;
        let colour = render::rgb(l.shimmer);
        let spin = (ms * fx::TURN as i64 / 2400) as i32;
        let height = l.shimmer_height_mm * (300 + 700 * permille / 1000) / 1000;
        for k in 0..strands {
            let a = spin + k * fx::TURN / strands;
            let base = ahead(at, a, l.shimmer_mm);
            // each strand bobs to its own height
            let wave = ((ms / 90 + k as i64 * 5) % 8) as i32;
            let top = height * (6 + wave) / 13;
            let lo = lake.world(base, 0);
            let hi = lake.world(base, top);
            facing_quad(out, lo, hi, eye, mm(40 + permille / 25), colour);
        }
        // the ring on the water
        let n = 24;
        let (r0, r1) = (l.shimmer_mm - 120, l.shimmer_mm + 120);
        for k in 0..n {
            let (a0, a1) = (spin + k * fx::TURN / n, spin + (k + 1) * fx::TURN / n);
            let p = |a: i32, r: i32| lake.world(ahead(at, a, r), 50);
            flat(out, [p(a0, r0), p(a0, r1), p(a1, r1)], colour);
            flat(out, [p(a0, r0), p(a1, r1), p(a1, r0)], colour);
        }
    }

    // ---- the HUD ----

    fn item_name(&self, item: Item) -> String {
        self.t(&format!("item.{}", item.key()))
    }

    /// The item slot, the effects' timers and the camera's offer on the
    /// HUD canvas, and a minigame over the view; the 2D status words for
    /// the browser check.
    pub(crate) fn draw_items_hud(
        &self,
        list: &mut List,
        theme: &Theme,
        status: &mut Vec<String>,
        slot_too: bool,
    ) {
        let Some(s) = self.my_items() else {
            return;
        };
        let [sx, sy, sw, sh] = theme.items.slot;
        let slot = Rect::new(sx, sy, sw, sh);
        status.push(format!("item={}", s.held.map_or("none", |i| i.key())));
        if slot_too {
            self.draw_slot(list, theme, slot, s.held);
        }
        self.draw_effects_and_offer(list, theme, status);
    }

    /// The item slot of its own (without the flow's HUD).
    fn draw_slot(&self, list: &mut List, theme: &Theme, slot: Rect, held: Option<Item>) {
        let l = &theme.items;
        let font = theme.mono();
        let ink = c(l.ink);
        let [sx, sy, sw, sh] = [slot.x, slot.y, slot.w, slot.h];
        list.round_rect(c(l.panel).alpha(85), c(l.edge), 4, slot, 18);
        let icon = Rect::new(sx + (sw - 104) / 2, sy + 14, 104, 104);
        match held {
            Some(item) => {
                draw_icon(list, item, icon, c(l.icons.get(item)), ink);
                let name = self.item_name(item);
                list.text(
                    ink,
                    font,
                    fit(font, &name, 22, sw - 16),
                    Align::Centre,
                    (sx + sw / 2, sy + sh - 52),
                    &name,
                );
                let key = if item == Item::Camera {
                    self.t("items.after_catch")
                } else {
                    self.t("items.use_key")
                };
                let size = fit(font, &key, 22, sw - 40);
                let w = font.width(&key, size) + 24;
                let cap = Rect::new(sx + (sw - w) / 2, sy + sh - 36, w, 30);
                list.round_rect(c(l.key_cap), Rgba::NONE, 0, cap, 6);
                list.text(
                    c(l.panel),
                    font,
                    size,
                    Align::Centre,
                    (sx + sw / 2, cap.y + 15),
                    &key,
                );
            }
            None => {
                list.round_rect(Rgba::NONE, c(l.dim), 3, icon.inset(12), 14);
                list.text(
                    c(l.dim),
                    font,
                    22,
                    Align::Centre,
                    (sx + sw / 2, sy + sh - 40),
                    &self.t("items.none"),
                );
            }
        }
    }

    /// The effects' timers above the slot, the camera's offer, and a
    /// minigame over the view.
    fn draw_effects_and_offer(&self, list: &mut List, theme: &Theme, status: &mut Vec<String>) {
        let (Some(p), Some(s)) = (self.play.as_ref(), self.my_items()) else {
            return;
        };
        let l = &theme.items;
        let font = theme.mono();
        let ink = c(l.ink);
        let [sx, sy, sw, _] = l.slot;

        // the effects running, stacked above the slot
        let g = &p.game;
        let t = &g.items.tuning;
        let effects = g.items.effects(p.seat, g.tick);
        let mut y = sy - 16 - l.effect_h;
        for &(item, left) in &effects {
            let total = match item {
                Item::Cloak => t.cloak_ms,
                Item::PowerReel => t.power_reel_ms,
                Item::Nitrous => t.nitrous_ms,
                _ => t.teleport_charge_ms,
            };
            let row = Rect::new(sx - 120, y, sw + 120, l.effect_h);
            list.round_rect(c(l.panel).alpha(80), Rgba::NONE, 0, row, 12);
            draw_icon(
                list,
                item,
                Rect::new(row.x + 8, row.y + 6, l.effect_h - 12, l.effect_h - 12),
                c(l.icons.get(item)),
                ink,
            );
            let label = if item == Item::Teleporter {
                self.t("items.charging")
            } else {
                self.item_name(item)
            };
            let tx = row.x + l.effect_h + 4;
            list.text(ink, font, 20, Align::Left, (tx, row.y + 20), &label);
            let secs = self
                .t("items.left")
                .replace("{seconds}", &((left + 999) / 1000).to_string());
            list.text(ink, font, 20, Align::Right, (row.right() - 12, row.y + 20), &secs);
            let bar = Rect::new(tx, row.bottom() - 18, row.right() - 12 - tx, 8);
            list.fill_rect(c(l.dim).alpha(60), bar);
            let fill = (bar.w as i64 * left as i64 / total.max(1) as i64) as i32;
            list.fill_rect(
                c(l.icons.get(item)),
                Rect::new(bar.x, bar.y, fill.clamp(0, bar.w), bar.h),
            );
            y -= l.effect_h + 10;
        }
        let keys: Vec<&str> = effects.iter().map(|(i, _)| i.key()).collect();
        status.push(format!("effects={}", keys.join(",")));
        status.push(format!("buoy={}", g.items.buoy.is_some() as i32));

        // the camera's offer
        if s.offer_until.is_some() {
            self.draw_offer(list, theme);
            status.push("offer=1".into());
        }
        // a minigame over the whole view
        if let Some(mg) = self.my_minigame() {
            status.push(mg.status());
            if mg.layer() == Layer::Hud {
                if let Some(target) = self.item_minigame_target(mg) {
                    list.layer(mg.transform(target));
                    mg.draw(list, theme, &|k| self.t(k));
                }
            }
        }
    }

    /// "Photograph your catch?": Shift uses the camera, Esc saves it.
    fn draw_offer(&self, list: &mut List, theme: &Theme) {
        let l = &theme.items;
        let font = theme.mono();
        let ink = c(l.ink);
        let [x, y, w, h] = l.offer;
        let r = Rect::new(x, y, w, h);
        list.round_rect(c(l.panel).alpha(92), c(l.icons.camera), 4, r, 18);
        draw_icon(
            list,
            Item::Camera,
            Rect::new(x + 20, y + 20, h - 40, h - 40),
            c(l.icons.camera),
            ink,
        );
        let tx = x + h;
        list.text(ink, font, 30, Align::Left, (tx, y + 40), &self.t("camera.offer"));
        let mut cx = tx;
        for (key, what) in [
            ("camera.use_key", "camera.use"),
            ("camera.save_key", "camera.save"),
        ] {
            let k = self.t(key);
            let kw = font.width(&k, 24) + 24;
            let cap = Rect::new(cx, y + h - 64, kw, 38);
            list.round_rect(c(l.key_cap), Rgba::NONE, 0, cap, 8);
            list.text(
                c(l.panel),
                font,
                24,
                Align::Centre,
                (cap.x + kw / 2, cap.y + 19),
                &k,
            );
            let text = self.t(what);
            list.text(ink, font, 24, Align::Left, (cap.right() + 12, cap.y + 19), &text);
            cx = cap.right() + 12 + font.width(&text, 24) + 40;
        }
    }

    /// On the Fish-Boy's radar: the Enhanced Radar's marked fish, and the
    /// teleport's choice (where the pointer would land you).
    pub(crate) fn draw_radar_items(
        &self,
        list: &mut List,
        theme: &Theme,
        scene: &RadarScene,
        status: &mut Vec<String>,
    ) {
        let (Some(p), Some(s)) = (self.play.as_ref(), self.my_items()) else {
            return;
        };
        list.clip(radar::MAP_RECT);
        for m in &s.marks {
            let tier = theme.tiers.get(m.tier);
            let name = self.t(&format!("tier.{}", crate::roster::tier_key(m.tier)));
            scene.mark(list, m.at, c(tier.colour), &name);
        }
        let l = &theme.items;
        if s.choosing {
            let want = self
                .fishboy
                .pointer
                .and_then(|(x, y)| radar::map_point(scene.region, x, y));
            if let Some(to) = want
                .and_then(|w| claims::teleport_target(&p.game.map, w, p.game.items.tuning.teleport_clear_mm))
            {
                let at = scene.screen_point(to);
                list.circle(Rgba::NONE, c(l.choose), 4, at, 22);
                list.line(c(l.choose), 3, (at.0 - 34, at.1), (at.0 + 34, at.1));
                list.line(c(l.choose), 3, (at.0, at.1 - 34), (at.0, at.1 + 34));
            }
        }
        list.unclip();
        if s.choosing {
            let r = radar::MAP_RECT;
            let band = Rect::new(r.x, r.y, r.w, 64);
            list.fill_rect(c(theme.radar.label_back).alpha(85), band);
            list.text(
                c(l.choose),
                theme.mono(),
                26,
                Align::Centre,
                (r.x + r.w / 2, r.y + 22),
                &self.t("teleport.choose"),
            );
            list.text(
                c(theme.screen.dim).shade(170),
                theme.mono(),
                20,
                Align::Centre,
                (r.x + r.w / 2, r.y + 48),
                &self.t("teleport.keys"),
            );
            status.push("choosing=1".into());
            if let Some((x, y)) = self.fishboy.pointer {
                status.push(format!("aim={x},{y}"));
            }
        }
        status.push(format!("marks={}", s.marks.len()));
    }
}

#[cfg(test)]
mod tests;
