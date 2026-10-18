//! The HUD on the 2D layer, in the HUD's 1920 x 1080 canvas: chunky
//! rounded cards with a dark outline and a drop shadow, a rounded display
//! face for words and the Fish-Boy's monospace for numbers (one family
//! with the Fish-Boy, without its green phosphor).
//!
//! - top right: the match's time left;
//! - top centre: the popups, stacked;
//! - bottom left: the speed gauge and the driving/fishing badge;
//! - bottom right: the radar ping's cooldown, the item slot, and under them
//!   the map seed when Settings' "Show map seed" is on;
//! - bottom centre: the key hints, fading after the first minute;
//! - centre: the countdown, "Lines up" at the end, and a wreck's notice.

use crate::app::App;
use crate::fishboy::radar::PingState;
use crate::gfx::fx::{self, TURN};
use crate::sim::view::BoatView;
use crate::sim::{Mode, TICK_MS};
use crate::ui2d::theme::Theme;
use crate::ui2d::{Align, Font, List, Rect, Rgba};

use super::look::{CardLook, FlowLook};
use super::popups::Popup;

pub const W: i32 = 1920;
pub const H: i32 = 1080;
/// The gap between the HUD's cards and the canvas's edge.
const EDGE: i32 = 36;

pub fn rgb(c: [i32; 3]) -> Rgba {
    Rgba::solid(c)
}

/// A HUD card, a box from the magazine: its offset shadow, its paper
/// and keyline, the triple stripe across its top and a dot screen fading
/// into its bottom-right corner (`flow/print.rs`).
pub fn card(list: &mut List, k: &CardLook, r: Rect) {
    card_in(list, k, r, k.fill, k.outline);
    // clear of the rounded corners at the stripe's depth
    let inset = k.outline_w / 2 + k.radius / 3;
    super::print::stripes(
        list,
        &k.stripes,
        r.x + inset,
        r.y + k.stripe_at,
        r.w - 2 * inset,
        k.stripe_h,
    );
    super::print::dot_corner(
        list,
        rgb(k.dots).alpha(k.dots_pct),
        r.inset(k.outline_w + 4),
        r.w * k.dots_w_pct / 100,
        r.h * k.dots_h_pct / 100,
        k.dot_step,
    );
}

/// A plain box in `fill`: the offset shadow, the fill and the keyline.
pub fn card_in(list: &mut List, k: &CardLook, r: Rect, fill: [i32; 3], outline: [i32; 3]) {
    let shadow = Rect::new(r.x + k.shadow, r.y + k.shadow, r.w, r.h);
    list.round_rect(
        rgb(k.shadow_colour).alpha(k.shadow_pct),
        Rgba::NONE,
        0,
        shadow,
        k.radius,
    );
    list.round_rect(
        rgb(fill).alpha(k.fill_pct),
        rgb(outline),
        k.outline_w,
        r,
        k.radius,
    );
}

/// Text with a dark outline round it, to read over the lake.
#[allow(clippy::too_many_arguments)]
pub fn outlined(
    list: &mut List,
    font: &Font,
    size: i32,
    align: Align,
    at: (i32, i32),
    text: &str,
    fill: Rgba,
    outline: Rgba,
) {
    let d = (size / 18).max(2);
    for (dx, dy) in [
        (-d, 0),
        (d, 0),
        (0, -d),
        (0, d),
        (-d, -d),
        (d, d),
        (-d, d),
        (d, -d),
    ] {
        list.text(outline, font, size, align, (at.0 + dx, at.1 + dy), text);
    }
    list.text(fill, font, size, align, at, text);
}

/// A point `r` from `c` at angle `a` (angle units, clockwise on screen
/// from the right).
pub fn polar(c: (i32, i32), r: i32, a: i32) -> (i32, i32) {
    (
        c.0 + ((r as i64 * fx::cos(a) as i64) >> 16) as i32,
        c.1 + ((r as i64 * fx::sin(a) as i64) >> 16) as i32,
    )
}

fn deg(d: i32) -> i32 {
    (d as i64 * TURN as i64 / 360) as i32
}

/// A key cap: the key's name in a small raised box; returns its width.
pub fn key_cap(list: &mut List, l: &FlowLook, at: (i32, i32), key: &str, size: i32) -> i32 {
    let font = l.body();
    let w = font.width(key, size) + size;
    let h = size * 3 / 2;
    let r = Rect::new(at.0, at.1 - h / 2, w, h);
    list.round_rect(
        rgb([0, 0, 0]).alpha(l.card.shadow_pct),
        Rgba::NONE,
        0,
        Rect::new(r.x, r.y + 4, w, h),
        8,
    );
    list.round_rect(rgb(l.hud.key_cap), rgb(l.card.outline), 3, r, 8);
    list.text(
        rgb(l.card.ink),
        font,
        size,
        Align::Centre,
        (r.x + w / 2, at.1),
        key,
    );
    w
}

/// A small fish: body and tail, facing right.
pub fn fish_icon(list: &mut List, at: (i32, i32), s: i32, fill: Rgba, ink: Rgba) {
    let (x, y) = at;
    let body = [
        (x + s, y),
        (x + s * 6 / 10, y - s * 4 / 10),
        (x, y - s * 45 / 100),
        (x - s * 5 / 10, y - s * 2 / 10),
        (x - s * 5 / 10, y + s * 2 / 10),
        (x, y + s * 45 / 100),
        (x + s * 6 / 10, y + s * 4 / 10),
    ];
    list.poly(fill, ink, 3, &body);
    let tail = [
        (x - s * 45 / 100, y),
        (x - s, y - s * 45 / 100),
        (x - s * 85 / 100, y),
        (x - s, y + s * 45 / 100),
    ];
    list.poly(fill, ink, 3, &tail);
    list.circle(ink, Rgba::NONE, 0, (x + s * 55 / 100, y - s / 10), (s / 9).max(2));
}

/// A star of a burst.
pub fn burst_icon(list: &mut List, at: (i32, i32), s: i32, fill: Rgba, ink: Rgba) {
    let pts: Vec<(i32, i32)> = (0..16)
        .map(|k| polar(at, if k % 2 == 0 { s } else { s * 5 / 10 }, k * TURN / 16))
        .collect();
    list.poly(fill, ink, 3, &pts);
}

/// The item buoy's giant bobber: red over white.
pub fn bobber_icon(list: &mut List, at: (i32, i32), s: i32, ink: Rgba) {
    list.circle(rgb([250, 250, 246]), ink, 3, at, s * 7 / 10);
    list.poly(
        rgb([226, 56, 44]),
        Rgba::NONE,
        0,
        &(0..=12)
            .map(|k| polar(at, s * 7 / 10, TURN / 2 + k * TURN / 24))
            .collect::<Vec<_>>(),
    );
    list.circle(Rgba::NONE, ink, 3, at, s * 7 / 10);
    list.line(ink, 5, (at.0, at.1 - s * 7 / 10), (at.0, at.1 - s));
}

/// A little boat for the driving badge.
fn boat_icon(list: &mut List, at: (i32, i32), s: i32, fill: Rgba, ink: Rgba) {
    let (x, y) = at;
    list.poly(
        fill,
        ink,
        3,
        &[
            (x - s, y - s / 5),
            (x + s, y - s / 5),
            (x + s * 6 / 10, y + s * 4 / 10),
            (x - s * 7 / 10, y + s * 4 / 10),
        ],
    );
    list.line(
        ink,
        4,
        (x - s * 7 / 10, y - s / 5),
        (x - s * 7 / 10, y - s * 7 / 10),
    );
}

/// A hook for the fishing badge.
fn hook_icon(list: &mut List, at: (i32, i32), s: i32, ink: Rgba) {
    let (x, y) = at;
    list.line(ink, 5, (x + s / 4, y - s), (x + s / 4, y + s / 5));
    list.arc(ink, 5, (x, y + s / 5), s / 4, 0, TURN / 2);
    list.line(ink, 5, (x - s / 4, y + s / 5), (x - s / 4, y - s / 5));
}

impl App {
    /// **Hook for the items system.** The item this browser's seat holds,
    /// for the HUD's item slot; `None` draws the slot empty.
    pub fn hud_item(&self) -> Option<crate::sim::items::Item> {
        self.my_items()?.held
    }

    /// My boat as drawn now.
    fn my_boat(&self) -> Option<BoatView> {
        let v = self.play_view()?;
        v.world.boats.into_iter().find(|b| b.seat == v.seat)
    }

    /// The HUD under the menus and the Fish-Boy. Returns false (drawing
    /// nothing) when the flow's look is not loaded.
    pub(crate) fn flow_hud(&self, list: &mut List, status: &mut Vec<String>) -> bool {
        let (Some(l), Some(p)) = (self.flow.look.as_ref(), self.play.as_ref()) else {
            return false;
        };
        let g = &p.game;
        status.push("hud=1".into());
        let remaining = g.remaining_ms();
        let clock = crate::app::play::clock_text(remaining);
        self.draw_time(list, l, &clock, remaining);
        status.push(format!("time={clock}"));
        if let Some(b) = self.my_boat() {
            let kmh = (b.speed_mm_per_s as i64 * 36 / 10_000) as i32;
            self.draw_gauge(list, l, kmh);
            self.draw_mode(list, l, b.mode);
            status.push(format!("speed={kmh} mode={}", mode_key(b.mode)));
        }
        match self.fishing_hud() {
            Some(f) => {
                self.draw_fishing(list, l, &f);
                status.push(format!("fishing={}", f.key));
                if let Some(c) = f.charge {
                    status.push(format!("charge={c}"));
                }
                if let Some(r) = &f.reel {
                    status.push(format!("reel={},{},{},{}", r.zone[0], r.zone[1], r.fish, r.meter));
                }
            }
            None => status.push("fishing=".into()),
        }
        let ping = self.draw_ping(list, l);
        status.push(format!("hud_ping={ping}"));
        self.draw_item(list, l);
        let hints = self.draw_hints(list, l);
        status.push(format!("hints={hints}"));
        if self.settings.show_seed {
            let seed = g.map.seed.to_string();
            outlined(
                list,
                l.body(),
                24,
                Align::Right,
                (W - EDGE, H - EDGE - 176 - 26),
                &self.t("hud.seed").replace("{seed}", &seed),
                rgb(l.hud.seed),
                rgb(l.card.outline),
            );
            status.push(format!("seed={seed}"));
        } else {
            status.push("seed=".into());
        }
        true
    }

    /// Over everything on Play, the Fish-Boy too: a wreck's flash, the
    /// countdown, the end, and the popups.
    pub(crate) fn flow_overlay(&self, list: &mut List, theme: &Theme, status: &mut Vec<String>) {
        let (Some(l), Some(p)) = (self.flow.look.as_ref(), self.play.as_ref()) else {
            return;
        };
        let g = &p.game;
        // my wreck: the flash, and where I come back
        if let Some(b) = self.flow.bursts.iter().rev().find(|b| b.seat == p.seat) {
            let age = ((self.clock - b.born_us) / 1000) as i32;
            let h = &l.hud;
            if age < h.wreck_flash_ms {
                let pct = h.wreck_flash_pct * (h.wreck_flash_ms - age) / h.wreck_flash_ms;
                list.fill_rect(rgb(h.wreck_flash).alpha(pct), Rect::new(0, 0, W, H));
            }
        }
        if self.my_boat().is_some_and(|b| b.wrecked) {
            let text = self.t("hud.wrecked");
            let w = l.body().width(&text, 34) + 80;
            card(list, &l.card, Rect::new((W - w) / 2, H / 2 + 200, w, 76));
            list.text(
                rgb(l.card.ink),
                l.body(),
                34,
                Align::Centre,
                (W / 2, H / 2 + 238),
                &text,
            );
            status.push("wrecked=1".into());
        }
        // the countdown, then "Go"
        let n = &l.countdown;
        let since_go = g.tick as i64 * TICK_MS as i64;
        let shown = if g.countdown > 0 {
            let ms = g.countdown as i64 * TICK_MS as i64;
            let left = (ms + 999) / 1000;
            // each number pops in from the start of its second
            let into = 1000 - (ms - (left - 1) * 1000);
            Some((left.to_string(), n.number, into, left.to_string()))
        } else if g.setup.countdown_ms > 0 && since_go < n.go_ms as i64 && !g.ended {
            Some((self.t("hud.go"), n.go, since_go, "go".to_string()))
        } else {
            None
        };
        match &shown {
            Some((text, colour, into, word)) => {
                let pop = (*into).clamp(0, n.pop_ms as i64) as i32;
                // from 160% down to full size
                let size = n.size * (160 - 60 * pop / n.pop_ms) / 100;
                outlined(
                    list,
                    l.display(),
                    size,
                    Align::Centre,
                    (W / 2, H * 42 / 100),
                    text,
                    rgb(*colour),
                    rgb(l.card.outline),
                );
                status.push(format!("countdown={word}"));
            }
            None => status.push("countdown=".into()),
        }
        if g.ended {
            outlined(
                list,
                l.display(),
                n.size * 6 / 10,
                Align::Centre,
                (W / 2, H * 40 / 100),
                &self.t("hud.lines_up"),
                rgb(n.end),
                rgb(l.card.outline),
            );
            outlined(
                list,
                l.body(),
                40,
                Align::Centre,
                (W / 2, H * 40 / 100 + n.size * 6 / 10),
                &self.t("hud.ended"),
                rgb(l.card.light),
                rgb(l.card.outline),
            );
            status.push("ended=1".into());
        }
        let count = self.draw_popups(list, l, theme);
        status.push(format!("popups={count}"));
        if let Some(first) = self.flow.popups.shown().next() {
            status.push(format!("popup={}", first.style));
        }
        let keys: Vec<&str> = self.flow.popups.shown().map(|p| p.key.as_str()).collect();
        status.push(format!("popup_keys={}", keys.join(",")));
    }

    fn draw_time(&self, list: &mut List, l: &FlowLook, clock: &str, remaining: i32) {
        let r = Rect::new(W - EDGE - 290, EDGE + 54, 290, 132);
        card(list, &l.card, r);
        list.text(
            rgb(l.card.soft),
            l.body(),
            24,
            Align::Centre,
            (r.x + r.w / 2, r.y + 28),
            &self.t("hud.time_left").to_uppercase(),
        );
        // the last 30 s in the countdown's warning colour
        let ink = if remaining <= 30_000 {
            l.countdown.end
        } else {
            l.card.ink
        };
        let mono = self.fishboy.theme.as_ref().map_or(l.display(), |t| t.mono());
        list.text(
            rgb(ink),
            mono,
            70,
            Align::Centre,
            (r.x + r.w / 2, r.y + 82),
            clock,
        );
    }

    /// The speed gauge: a 270° dial from 0 to the nitrous top speed, the
    /// part past the plain top speed marked, the needle, and the km/h.
    fn draw_gauge(&self, list: &mut List, l: &FlowLook, kmh: i32) {
        let h = &l.hud;
        let r = Rect::new(EDGE, H - EDGE - 262, 300, 262);
        card(list, &l.card, r);
        let c = (r.x + r.w / 2, r.y + 138);
        let rad = 106;
        let (top, boost) = self.play.as_ref().map_or((58, 87), |p| {
            let t = p.game.rules.top as i64;
            let n = t * p.game.rules.nitrous_top_pct as i64 / 100;
            ((t * 36 / 10_000) as i32, (n * 36 / 10_000) as i32)
        });
        // the dial's top: the nitrous speed, up to a whole 10
        let max = ((boost + 9) / 10 * 10).max(10);
        let at = |v: i32| deg(135) + (deg(270) as i64 * v.clamp(0, max) as i64 / max as i64) as i32;
        list.circle(rgb(h.gauge_face), Rgba::NONE, 0, c, rad + 14);
        list.arc(rgb(h.gauge_tick).alpha(55), 16, c, rad, at(0), at(top));
        list.arc(rgb(h.gauge_boost).alpha(70), 16, c, rad, at(top), at(max));
        let fill = if kmh > top { h.gauge_boost } else { h.gauge_fill };
        if kmh > 0 {
            list.arc(rgb(fill), 16, c, rad, at(0), at(kmh));
        }
        let mut v = 0;
        while v <= max {
            let a = at(v);
            list.line(rgb(h.gauge_tick), 4, polar(c, rad - 22, a), polar(c, rad - 10, a));
            v += 10;
        }
        let a = at(kmh);
        list.line(rgb(l.card.outline), 12, c, polar(c, rad - 16, a));
        list.line(rgb(h.needle), 6, c, polar(c, rad - 18, a));
        list.circle(rgb(h.needle), rgb(l.card.outline), 4, c, 14);
        let mono = self.fishboy.theme.as_ref().map_or(l.display(), |t| t.mono());
        list.text(
            rgb(l.card.ink),
            mono,
            54,
            Align::Centre,
            (c.0, c.1 + 64),
            &kmh.to_string(),
        );
        list.text(
            rgb(l.card.soft),
            l.body(),
            22,
            Align::Centre,
            (c.0, c.1 + 100),
            &self.t("hud.kmh"),
        );
    }

    /// The mode badge beside the gauge: a boat or a hook, the word, and
    /// the mode's colour.
    fn draw_mode(&self, list: &mut List, l: &FlowLook, mode: Mode) {
        let (colour, word) = match mode {
            Mode::Driving => (l.hud.driving, self.t("hud.driving")),
            Mode::Fishing => (l.hud.fishing, self.t("hud.fishing")),
        };
        let word = word.to_uppercase();
        let w = l.display().width(&word, 34) + 116;
        let r = Rect::new(EDGE + 300 + 20, H - EDGE - 262, w, 84);
        card_in(list, &l.card, r, colour, l.card.outline);
        let ink = rgb(l.card.outline);
        let icon = (r.x + 50, r.y + r.h / 2);
        match mode {
            Mode::Driving => boat_icon(list, icon, 28, rgb(l.card.fill), ink),
            Mode::Fishing => hook_icon(list, icon, 26, ink),
        }
        list.text(
            rgb(l.card.ink),
            l.display(),
            34,
            Align::Left,
            (r.x + 90, r.y + r.h / 2 + 2),
            &word,
        );
    }

    /// The fishing line's prompt at the bottom centre (with the lure's
    /// depth, the cast's charge or whether the fish is in the zone), and
    /// while reeling the reel bar and the catch meter on the right.
    fn draw_fishing(&self, list: &mut List, l: &FlowLook, f: &crate::app::fishing::FishingHud) {
        let font = l.body();
        let h = &l.hud;
        let second = match (&f.depth, &f.reel) {
            (Some(d), _) => Some(d.clone()),
            (None, Some(r)) => Some(self.t(if r.in_zone {
                "fishing.in_zone"
            } else {
                "fishing.out_of_zone"
            })),
            _ => None,
        };
        let tall = if second.is_some() || f.charge.is_some() {
            118
        } else {
            76
        };
        let w = (font.width(&f.prompt, 30) + 80).max(560);
        let r = Rect::new((W - w) / 2, H - EDGE - 30 - tall, w, tall);
        card(list, &l.card, r);
        list.text(
            rgb(l.card.ink),
            font,
            30,
            Align::Centre,
            (W / 2, r.y + 38),
            &f.prompt,
        );
        if let Some(c) = f.charge {
            let bar = Rect::new(r.x + 40, r.y + 70, r.w - 80, 26);
            list.round_rect(rgb(h.gauge_face), rgb(l.card.outline), 3, bar, 13);
            let fill = Rect::new(bar.x, bar.y, bar.w * c.clamp(0, 100) / 100, bar.h);
            if fill.w > 0 {
                list.round_rect(rgb(h.gauge_fill), rgb(l.card.outline), 3, fill, 13);
            }
        } else if let Some(t) = &second {
            list.text(rgb(l.card.soft), font, 24, Align::Centre, (W / 2, r.y + 84), t);
        }
        let Some(reel) = &f.reel else {
            return;
        };
        // the reel bar and the catch meter, filling from the bottom
        let card_r = Rect::new(W - EDGE - 230, 250, 230, 520);
        card(list, &l.card, card_r);
        let track = Rect::new(card_r.x + 40, card_r.y + 70, 76, 400);
        let meter = Rect::new(card_r.x + 150, card_r.y + 70, 40, 400);
        let y_of = |t: Rect, pct: i32| t.bottom() - t.h * pct.clamp(0, 100) / 100;
        list.text(
            rgb(l.card.soft),
            font,
            22,
            Align::Centre,
            (track.x + track.w / 2, card_r.y + 36),
            &self.t("fishing.reel"),
        );
        list.round_rect(rgb(h.gauge_face), rgb(l.card.outline), 4, track, 12);
        let (z0, z1) = (y_of(track, reel.zone[1]), y_of(track, reel.zone[0]));
        let zone_c = if reel.in_zone { h.ping_ready } else { h.ping_wait };
        list.round_rect(
            rgb(zone_c).alpha(80),
            rgb(l.card.outline),
            3,
            Rect::new(track.x + 6, z0, track.w - 12, z1 - z0),
            8,
        );
        let tier = self
            .fishboy
            .theme
            .as_ref()
            .map_or(h.needle, |t| t.tiers.get(reel.tier).colour);
        fish_icon(
            list,
            (track.x + track.w / 2, y_of(track, reel.fish)),
            26,
            rgb(tier),
            rgb(l.card.outline),
        );
        list.round_rect(rgb(h.gauge_face), rgb(l.card.outline), 4, meter, 10);
        let top = y_of(meter, reel.meter);
        let colour = if reel.meter < 25 {
            h.gauge_boost
        } else {
            h.gauge_fill
        };
        if meter.bottom() - top > 4 {
            list.round_rect(
                rgb(colour),
                Rgba::NONE,
                0,
                Rect::new(meter.x + 4, top, meter.w - 8, meter.bottom() - top - 4),
                8,
            );
        }
        list.text(
            rgb(l.card.ink),
            font,
            20,
            Align::Centre,
            (card_r.x + card_r.w / 2, card_r.bottom() - 26),
            &self.t("fishing.meter"),
        );
    }

    /// The radar ping's cooldown: a dish whose ring fills as it cools.
    /// Returns "ready", "sweeping" or "cooling".
    fn draw_ping(&self, list: &mut List, l: &FlowLook) -> &'static str {
        let Some(p) = self.play.as_ref() else {
            return "";
        };
        let g = &p.game;
        let t = &g.tuning.radar;
        let state = g.seats[p.seat].ping.state(g.tick as i64, t);
        let r = Rect::new(W - EDGE - 176 - 20 - 176, H - EDGE - 176, 176, 176);
        card(list, &l.card, r);
        let c = (r.x + r.w / 2, r.y + 74);
        let h = &l.hud;
        list.circle(rgb(h.gauge_face), Rgba::NONE, 0, c, 50);
        list.circle(Rgba::NONE, rgb(h.gauge_tick).alpha(60), 3, c, 30);
        list.circle(rgb(h.gauge_tick), Rgba::NONE, 0, c, 6);
        let secs = |ticks: i64| ((ticks * TICK_MS as i64 + 999) / 1000).to_string();
        let (word, label, done, colour) = match state {
            PingState::Ready => ("ready", self.t("hud.ping_ready"), 1000, h.ping_ready),
            PingState::Sweeping(d) => ("sweeping", self.t("hud.pinging"), d, h.ping_ready),
            PingState::Cooling(left, d) => (
                "cooling",
                self.t("hud.ping_wait").replace("{seconds}", &secs(left)),
                d,
                h.ping_wait,
            ),
        };
        let from = -TURN / 4;
        list.arc(
            rgb(colour),
            10,
            c,
            50,
            from,
            from + (TURN as i64 * done as i64 / 1000) as i32,
        );
        if word == "sweeping" {
            // the sweep's arm going round
            list.line(
                rgb(colour),
                5,
                c,
                polar(c, 48, from + (TURN as i64 * done as i64 / 1000) as i32),
            );
        }
        list.text(
            rgb(l.card.ink),
            l.body(),
            24,
            Align::Centre,
            (c.0, r.y + 150),
            &label,
        );
        word
    }

    /// The item slot, with the Shift key's cap: empty until the items
    /// system fills [`App::hud_item`].
    fn draw_item(&self, list: &mut List, l: &FlowLook) {
        let r = Rect::new(W - EDGE - 176, H - EDGE - 176, 176, 176);
        card(list, &l.card, r);
        let slot = Rect::new(r.x + 34, r.y + 22, 108, 92);
        list.dash(&[12, 8]);
        list.round_rect(rgb(l.hud.item_slot), rgb(l.card.soft), 4, slot, 16);
        list.dash(&[]);
        match self.hud_item() {
            Some(item) => {
                // its icon (`app/items.rs`) and its name, inside the slot
                if let Some(t) = self.fishboy.theme.as_ref() {
                    let icon = Rect::new(slot.x + slot.w / 2 - 32, slot.y + 4, 64, 64);
                    crate::app::items::draw_icon(
                        list,
                        item,
                        icon,
                        crate::ui2d::theme::c(t.items.icons.get(item)),
                        rgb(l.card.ink),
                    );
                }
                let name = self.t(&format!("item.{}", item.key()));
                let size = crate::app::items::fit(l.body(), &name, 18, slot.w - 8);
                list.text(
                    rgb(l.card.ink),
                    l.body(),
                    size,
                    Align::Centre,
                    (slot.x + slot.w / 2, slot.y + slot.h - 12),
                    &name,
                );
            }
            None => list.text(
                rgb(l.card.soft),
                l.body(),
                22,
                Align::Centre,
                (slot.x + slot.w / 2, slot.y + slot.h / 2),
                &self.t("hud.item_none"),
            ),
        }
        // the camera waits for a catch (`app/items.rs`)
        let key = if self.hud_item() == Some(crate::sim::items::Item::Camera) {
            self.t("items.after_catch")
        } else {
            self.t("hint.item_key")
        };
        let w = l.body().width(&key, 20) + 20;
        key_cap(list, l, (r.x + (r.w - w) / 2, r.y + 150), &key, 20);
    }

    /// The key hints along the bottom, fading out once `flow.hints_ms` of
    /// match clock has passed. Returns their opacity in percent.
    fn draw_hints(&self, list: &mut List, l: &FlowLook) -> i32 {
        let Some(p) = self.play.as_ref() else {
            return 0;
        };
        let into = p.game.tick as i64 * TICK_MS as i64;
        let after = p.game.tuning.flow.hints_ms as i64;
        let fade = l.hud.hint_fade_ms as i64;
        let pct = if into < after {
            100
        } else {
            (100 - (into - after) * 100 / fade).max(0) as i32
        };
        if pct <= 0 {
            return 0;
        }
        let hints = ["mode", "fishboy", "item", "menu"]
            .map(|k| (self.t(&format!("hint.{k}_key")), self.t(&format!("hint.{k}"))));
        let size = 22;
        let font = l.body();
        let widths: Vec<i32> = hints
            .iter()
            .map(|(k, w)| font.width(k, size) + size + 12 + font.width(w, size))
            .collect();
        let gap = 40;
        let total: i32 = widths.iter().sum::<i32>() + gap * (hints.len() as i32 - 1);
        let mut x = (W - total) / 2;
        let y = H - 22;
        list.alpha(pct);
        for ((k, w), width) in hints.iter().zip(&widths) {
            let cap = key_cap(list, l, (x, y), k, size);
            outlined(
                list,
                font,
                size,
                Align::Left,
                (x + cap + 12, y),
                w,
                rgb(l.card.light),
                rgb(l.card.outline),
            );
            x += width + gap;
        }
        list.alpha(100);
        pct
    }

    /// The popups at the top centre, sliding in and fading out. Returns
    /// how many are drawn.
    fn draw_popups(&self, list: &mut List, l: &FlowLook, theme: &Theme) -> usize {
        let p = &l.popups;
        let life = self.tuning.as_ref().map_or(2000, |t| t.flow.popup_ms) as i64;
        let mut y = EDGE;
        let mut n = 0;
        for pop in self.flow.popups.shown() {
            let age = (self.clock - pop.shown_us.unwrap_or(self.clock)) / 1000;
            let slide = (age.min(p.slide_ms as i64) * 100 / p.slide_ms as i64) as i32;
            let fade = ((life - age).clamp(0, p.fade_ms as i64) * 100 / p.fade_ms as i64) as i32;
            let pct = slide.min(fade);
            list.alpha(pct);
            let tier = pop.tier.map(|t| {
                self.t(&format!("tier.{}", crate::roster::tier_key(t)))
                    .to_uppercase()
            });
            let h = popup(list, l, theme, pop, y - (100 - slide) * 40 / 100, tier);
            list.alpha(100);
            y += h + 14;
            n += 1;
        }
        n
    }
}

fn mode_key(m: Mode) -> &'static str {
    match m {
        Mode::Driving => "driving",
        Mode::Fishing => "fishing",
    }
}

/// One popup at `y`: its icon, its words and, for a fish, a badge naming
/// the tier in the tier's colour. Returns its height.
fn popup(list: &mut List, l: &FlowLook, theme: &Theme, pop: &Popup, y: i32, tier: Option<String>) -> i32 {
    let Some(style) = l.popups.styles.get(&pop.style) else {
        return 0;
    };
    let tier_look = pop
        .tier
        .filter(|_| style.tier)
        .map(|t| (t, theme.tiers.get(t).colour));
    let outline = tier_look.map_or(style.outline, |(_, c)| c);
    let font = l.body();
    let size = 36;
    let h = 78;
    let badge = tier.filter(|_| tier_look.is_some());
    let badge_w = badge.as_ref().map_or(0, |w| l.display().width(w, 24) + 34);
    let icon_w = if style.icon == "none" { 0 } else { 70 };
    let w = icon_w + font.width(&pop.text, size) + 60 + badge_w;
    let r = Rect::new((W - w) / 2, y, w, h);
    let k = &l.card;
    let shadow = Rect::new(r.x + k.shadow, r.y + k.shadow, r.w, r.h);
    // a magazine box: rounded corners, a solid offset shadow, and the
    // tier's colour as a keyline inside the outline
    list.round_rect(
        rgb(k.shadow_colour).alpha(k.shadow_pct),
        Rgba::NONE,
        0,
        shadow,
        k.radius,
    );
    list.round_rect(rgb(style.fill), rgb(k.outline), k.outline_w + 2, r, k.radius);
    list.round_rect(
        Rgba::NONE,
        rgb(outline),
        4,
        r.inset(k.outline_w + 4),
        k.radius - 6,
    );
    let ink = rgb(style.ink);
    let mid = r.y + h / 2;
    let icon_at = (r.x + 30 + 18, mid);
    let accent = rgb(tier_look.map_or(style.outline, |(_, c)| c));
    match style.icon.as_str() {
        "fish" => fish_icon(list, icon_at, 26, accent, rgb(k.outline)),
        "burst" => burst_icon(list, icon_at, 26, accent, rgb(k.outline)),
        "bobber" => bobber_icon(list, icon_at, 26, rgb(k.outline)),
        _ => {}
    }
    list.text(
        ink,
        font,
        size,
        Align::Left,
        (r.x + 30 + icon_w, mid + 1),
        &pop.text,
    );
    if let (Some(word), Some((_, c))) = (badge, tier_look) {
        let b = Rect::new(r.right() - 24 - badge_w + 10, r.y + 16, badge_w - 10, h - 32);
        list.round_rect(rgb(c), rgb(k.outline), 3, b, 8);
        list.text(
            rgb(k.outline),
            l.display(),
            24,
            Align::Centre,
            (b.x + b.w / 2, mid + 1),
            &word,
        );
    }
    h
}
