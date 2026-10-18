//! The HUD on the 2D layer, in the HUD's 1920 x 1080 canvas: chunky
//! rounded cards with a dark outline and a drop shadow, a rounded display
//! face for words and the Fish-Boy's monospace for numbers (one family
//! with the Fish-Boy, without its green phosphor).
//!
//! - top right: the match's time left, the clock alone;
//! - top centre: the popups, stacked;
//! - bottom right: the radar ping's dish (its ring fills as it cools, the
//!   seconds left inside), the item slot, and over them the map seed when
//!   Settings' "Show map seed" is on;
//! - centre: the countdown, "Lines up" at the end, and a wreck's notice.
//!
//! Kept minimal on purpose (the person, 2026-10-18: "KISS"): no speed
//! gauge, no driving/fishing badge (the angler's pose shows the mode), no
//! key hints (the Title lists the controls), no labels on the cards.
//! TONE.md "Project decisions" has the rule.

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
/// The side of the ping's and the item's square cards.
const SLOT: i32 = 150;

pub fn rgb(c: [i32; 3]) -> Rgba {
    Rgba::solid(c)
}

/// A HUD card: its shadow, then its fill and outline.
pub fn card(list: &mut List, k: &CardLook, r: Rect) {
    card_in(list, k, r, k.fill, k.outline);
}

pub fn card_in(list: &mut List, k: &CardLook, r: Rect, fill: [i32; 3], outline: [i32; 3]) {
    let shadow = Rect::new(r.x + k.shadow, r.y + k.shadow, r.w, r.h);
    list.round_rect(
        rgb([0, 0, 0]).alpha(k.shadow_pct),
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
        // for the checks only: no gauge and no mode badge are drawn
        if let Some(b) = self.my_boat() {
            let kmh = (b.speed_mm_per_s as i64 * 36 / 10_000) as i32;
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
        if self.settings.show_seed {
            let seed = g.map.seed.to_string();
            outlined(
                list,
                l.body(),
                24,
                Align::Right,
                (W - EDGE, H - EDGE - SLOT - 26),
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
            let w = l.body().width(&text, 34) + 120;
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

    /// The time left: the clock alone on its card.
    fn draw_time(&self, list: &mut List, l: &FlowLook, clock: &str, remaining: i32) {
        let r = Rect::new(W - EDGE - 230, EDGE, 230, 96);
        card(list, &l.card, r);
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
            64,
            Align::Centre,
            (r.x + r.w / 2, r.y + r.h / 2 + 2),
            clock,
        );
    }

    /// The fishing line at the bottom centre: one short word when there is
    /// one (too fast, the bite, what was landed), the lure's depth as a bare
    /// number while it sinks or waits, or the cast's charge bar while Space
    /// is held; and while reeling the reel bar and the catch meter on the
    /// right, without labels.
    fn draw_fishing(&self, list: &mut List, l: &FlowLook, f: &crate::app::fishing::FishingHud) {
        let font = l.body();
        let h = &l.hud;
        if let Some(c) = f.charge {
            let r = Rect::new((W - 440) / 2, H - EDGE - 30 - 60, 440, 60);
            card(list, &l.card, r);
            let bar = Rect::new(r.x + 24, r.y + 17, r.w - 48, 26);
            list.round_rect(rgb(h.gauge_face), rgb(l.card.outline), 3, bar, 13);
            let fill = Rect::new(bar.x, bar.y, bar.w * c.clamp(0, 100) / 100, bar.h);
            if fill.w > 0 {
                list.round_rect(rgb(h.gauge_fill), rgb(l.card.outline), 3, fill, 13);
            }
        } else if let Some(text) = f.prompt.as_ref().or(f.depth.as_ref()) {
            let w = font.width(text, 34) + 80;
            let r = Rect::new((W - w) / 2, H - EDGE - 30 - 72, w, 72);
            card(list, &l.card, r);
            list.text(
                rgb(l.card.ink),
                font,
                34,
                Align::Centre,
                (W / 2, r.y + r.h / 2 + 1),
                text,
            );
        }
        let Some(reel) = &f.reel else {
            return;
        };
        // the reel bar and the catch meter, filling from the bottom
        let card_r = Rect::new(W - EDGE - 210, 260, 210, 460);
        card(list, &l.card, card_r);
        let track = Rect::new(card_r.x + 36, card_r.y + 30, 76, 400);
        let meter = Rect::new(card_r.x + 138, card_r.y + 30, 40, 400);
        let y_of = |t: Rect, pct: i32| t.bottom() - t.h * pct.clamp(0, 100) / 100;
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
    }

    /// The radar ping's dish: its ring fills as it cools, with the seconds
    /// left inside; full and green when ready. Returns "ready", "sweeping"
    /// or "cooling".
    fn draw_ping(&self, list: &mut List, l: &FlowLook) -> &'static str {
        let Some(p) = self.play.as_ref() else {
            return "";
        };
        let g = &p.game;
        let t = &g.tuning.radar;
        let state = g.seats[p.seat].ping.state(g.tick as i64, t);
        let r = Rect::new(W - EDGE - SLOT - 16 - SLOT, H - EDGE - SLOT, SLOT, SLOT);
        card(list, &l.card, r);
        let c = (r.x + r.w / 2, r.y + r.h / 2);
        let h = &l.hud;
        list.circle(rgb(h.gauge_face), Rgba::NONE, 0, c, 46);
        list.circle(Rgba::NONE, rgb(h.gauge_tick).alpha(60), 3, c, 28);
        let (word, done, colour) = match state {
            PingState::Ready => ("ready", 1000, h.ping_ready),
            PingState::Sweeping(d) => ("sweeping", d, h.ping_ready),
            PingState::Cooling(_, d) => ("cooling", d, h.ping_wait),
        };
        let from = -TURN / 4;
        let to = from + (TURN as i64 * done as i64 / 1000) as i32;
        list.arc(rgb(colour), 10, c, 46, from, to);
        match state {
            // the sweep's arm going round
            PingState::Sweeping(_) => list.line(rgb(colour), 5, c, polar(c, 44, to)),
            PingState::Cooling(left, _) => {
                let secs = ((left * TICK_MS as i64 + 999) / 1000).to_string();
                let mono = self.fishboy.theme.as_ref().map_or(l.display(), |t| t.mono());
                list.text(rgb(l.card.ink), mono, 34, Align::Centre, c, &secs);
            }
            PingState::Ready => list.circle(rgb(h.gauge_tick), Rgba::NONE, 0, c, 6),
        }
        word
    }

    /// The item slot: the held item's icon and name, or empty (dashed).
    fn draw_item(&self, list: &mut List, l: &FlowLook) {
        let r = Rect::new(W - EDGE - SLOT, H - EDGE - SLOT, SLOT, SLOT);
        card(list, &l.card, r);
        let slot = r.inset(18);
        list.dash(&[12, 8]);
        list.round_rect(rgb(l.hud.item_slot), rgb(l.card.soft), 4, slot, 16);
        list.dash(&[]);
        let Some(item) = self.hud_item() else {
            return;
        };
        // its icon (`app/items.rs`) and its name, inside the slot
        if let Some(t) = self.fishboy.theme.as_ref() {
            let icon = Rect::new(slot.x + slot.w / 2 - 30, slot.y + 4, 60, 60);
            crate::app::items::draw_icon(
                list,
                item,
                icon,
                crate::ui2d::theme::c(t.items.icons.get(item)),
                rgb(l.card.ink),
            );
        }
        let name = self.t(&format!("item.{}", item.key()));
        let size = crate::app::items::fit(l.body(), &name, 16, slot.w - 8);
        list.text(
            rgb(l.card.ink),
            l.body(),
            size,
            Align::Centre,
            (slot.x + slot.w / 2, slot.bottom() - 12),
            &name,
        );
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
    list.round_rect(rgb([0, 0, 0]).alpha(k.shadow_pct), Rgba::NONE, 0, shadow, h / 2);
    list.round_rect(rgb(style.fill), rgb(outline), 6, r, h / 2);
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
        list.round_rect(rgb(c), rgb(k.outline), 3, b, (h - 32) / 2);
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
