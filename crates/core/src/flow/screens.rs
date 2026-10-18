//! The postgame screen: the reveal drawn on the 2D layer over the lake,
//! and its buttons (Skip during the reveal; Rematch and Back to Title
//! once the ranking is up).
//!
//! Each player is a column headed by their name in their colour and their
//! total, counting up. Their catches come in one after another as cards,
//! each in its tier's frame (the Fishpedia's shapes: double, notched,
//! dashed, plain) with the fish swimming in a box, its name, its length
//! in feet to two places, its tier by name, and its points. Then the
//! ranking: places, ties sharing one, and the winner named.

use crate::app::gallery::feet;
use crate::app::App;
use crate::fishboy::fishpedia;
use crate::roster::tier_key;
use crate::sim::SEAT_COLOURS;
use crate::ui2d::theme::Theme;
use crate::ui2d::{Align, List, Rect, Rgba};
use crate::view::{Item, Panel};

use super::hud::{card, card_in, outlined, rgb, H, W};
use super::look::FlowLook;
use super::postgame::{places, ranking, Column, Shown};

/// The columns' area.
const TOP: i32 = 150;
const SIDE: i32 = 60;
const GAP: i32 = 28;
const HEAD_H: i32 = 128;

impl App {
    /// A player's colour on the HUD's light cards: the radar's brighter
    /// one, so it reads on the dark backdrop too.
    fn player_rgb(&self, seat: usize) -> [i32; 3] {
        let name = SEAT_COLOURS[seat.min(SEAT_COLOURS.len() - 1)];
        self.fishboy
            .theme
            .as_ref()
            .and_then(|t| t.radar.players.get(name).copied())
            .unwrap_or([200, 200, 200])
    }

    fn points(&self, n: i64) -> String {
        self.t("postgame.points").replace("{points}", &n.to_string())
    }

    /// The postgame on the 2D layer.
    pub(crate) fn postgame_ui2d(&self, list: &mut List, theme: &Theme, status: &mut Vec<String>) {
        let (Some(l), Some(pg)) = (self.flow.look.as_ref(), self.flow.postgame.as_ref()) else {
            return;
        };
        let ms = pg.ms(self.clock);
        let shown = pg.reveal.at(ms);
        let pl = &l.postgame;
        // the backdrop over the whole window, letterbox and all
        let t = list.transform();
        list.layer(crate::ui2d::Transform::IDENTITY);
        list.fill_rect(
            rgb(pl.backdrop).alpha(pl.backdrop_pct),
            Rect::new(0, 0, self.size[0], self.size[1]),
        );
        list.layer(t);
        outlined(
            list,
            l.display(),
            72,
            Align::Centre,
            (W / 2, 80),
            &self.t("postgame.heading"),
            rgb(l.card.light),
            rgb(l.card.outline),
        );
        let cols = &pg.reveal.columns;
        let n = cols.len().max(1) as i32;
        let col_w = ((W - 2 * SIDE - GAP * (n - 1)) / n).min(440);
        let x0 = (W - (col_w * n + GAP * (n - 1))) / 2;
        let most = cols.iter().map(|c| c.catches.len()).max().unwrap_or(0).max(1) as i32;
        let area = H - TOP - HEAD_H - 40 - if shown.ranking { 0 } else { 70 };
        let row_h = (area / most - 10).clamp(34, 104);
        for (k, c) in cols.iter().enumerate() {
            let x = x0 + k as i32 * (col_w + GAP);
            self.draw_column(
                list,
                l,
                theme,
                c,
                &shown,
                k,
                Rect::new(x, TOP, col_w, HEAD_H),
                row_h,
            );
        }
        status.push(format!(
            "postgame={} cards={} totals={}",
            if shown.ranking { "ranking" } else { "reveal" },
            shown
                .cards
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(","),
            shown
                .totals
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(","),
        ));
        if shown.ranking {
            let places = self.draw_ranking(list, l, cols);
            status.push(format!("places={places}"));
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_column(
        &self,
        list: &mut List,
        l: &FlowLook,
        theme: &Theme,
        c: &Column,
        shown: &Shown,
        k: usize,
        head: Rect,
        row_h: i32,
    ) {
        let colour = self.player_rgb(c.seat);
        let now = shown.current == Some(k);
        // the player being revealed stands out
        let head_r = if now {
            Rect::new(head.x, head.y - 10, head.w, head.h)
        } else {
            head
        };
        card_in(
            list,
            &l.card,
            head_r,
            l.card.fill,
            if now { colour } else { l.card.outline },
        );
        list.circle(
            rgb(colour),
            rgb(l.card.outline),
            3,
            (head_r.x + 34, head_r.y + 36),
            13,
        );
        list.text(
            rgb(l.card.ink),
            l.body(),
            32,
            Align::Left,
            (head_r.x + 58, head_r.y + 38),
            &c.name,
        );
        let mono = self.fishboy.theme.as_ref().map_or(l.display(), |t| t.mono());
        list.text(
            rgb(l.card.ink),
            mono,
            50,
            Align::Centre,
            (head_r.x + head_r.w / 2, head_r.y + 92),
            &self.points(shown.totals[k]),
        );
        let n = shown.cards[k];
        let mut y = head.bottom() + 16;
        if c.catches.is_empty() && (shown.ranking || shown.current.is_some_and(|cur| cur >= k)) {
            outlined(
                list,
                l.body(),
                28,
                Align::Centre,
                (head.x + head.w / 2, y + 30),
                &self.t("postgame.none"),
                rgb(l.card.light),
                rgb(l.card.outline),
            );
        }
        for f in &c.catches[..n] {
            let r = Rect::new(head.x, y, head.w, row_h);
            self.draw_catch(list, l, theme, f, r);
            y += row_h + 10;
        }
    }

    /// One catch: its tier's frame, the fish in a box (when the row is
    /// tall enough), its name, length, tier and points.
    fn draw_catch(&self, list: &mut List, l: &FlowLook, theme: &Theme, f: &crate::sim::Catch, r: Rect) {
        let kind = self.fishboy.roster.get(&f.kind);
        let tier = kind.map(|k| k.tier);
        list.round_rect(rgb(l.postgame.card), Rgba::NONE, 0, r, 10);
        if let Some(t) = tier {
            fishpedia::frame(list, theme, t, r.inset(3), 5);
        }
        let mut x = r.x + 18;
        let big = r.h >= 64;
        if big {
            if let Some(k) = kind {
                let b = Rect::new(r.x + 12, r.y + 12, (r.h - 24) * 16 / 10, r.h - 24);
                if self.models.iter().any(|m| m.id == k.model) {
                    let swim = ((self.clock / 1000) % crate::model::MAX_MS as i64) as u32;
                    list.model(
                        b,
                        &k.model,
                        Some(crate::model::Anim::Idle),
                        swim,
                        l.postgame.fish_box,
                        100,
                    );
                } else {
                    list.fill_rect(rgb(l.postgame.fish_box), b);
                }
                x = b.right() + 14;
            }
        }
        let name = kind.map_or(f.kind.clone(), |k| k.name.clone());
        let tier_name = tier.map_or(String::new(), |t| self.t(&format!("tier.{}", tier_key(t))));
        let length = self.t("catch.feet").replace("{feet}", &feet(f.length_mm));
        let ink = rgb(l.card.ink);
        let (size, small) = if big {
            (28, 22)
        } else {
            ((r.h * 45 / 100).min(26), (r.h * 38 / 100).min(20))
        };
        // the points at the right; the words shrink to fit beside them
        let mono = self.fishboy.theme.as_ref().map_or(l.display(), |t| t.mono());
        let value = format!("+{}", f.value);
        let value_w = mono.width(&value, size + 4);
        list.text(
            ink,
            mono,
            size + 4,
            Align::Right,
            (r.right() - 18, r.y + r.h / 2),
            &value,
        );
        let room = (r.right() - 18 - value_w - 12 - x).max(1);
        let font = l.body();
        let fit = |text: &str, size: i32| size.min(size * room / font.width(text, size).max(1)).max(10);
        if big {
            list.text(
                ink,
                font,
                fit(&name, size),
                Align::Left,
                (x, r.y + r.h * 36 / 100),
                &name,
            );
            let sub = format!("{length} · {tier_name}");
            list.text(
                rgb(l.card.soft),
                font,
                fit(&sub, small),
                Align::Left,
                (x, r.y + r.h * 70 / 100),
                &sub,
            );
        } else {
            let line = format!("{name} · {length} · {tier_name}");
            list.text(
                ink,
                font,
                fit(&line, size),
                Align::Left,
                (x, r.y + r.h / 2),
                &line,
            );
        }
    }

    /// The final ranking over the columns. Returns the places in seat
    /// order, for the browser check.
    fn draw_ranking(&self, list: &mut List, l: &FlowLook, cols: &[Column]) -> String {
        let order = ranking(cols);
        let rows = order.len() as i32;
        let w = 900;
        let h = 150 + rows * 92;
        let r = Rect::new((W - w) / 2, (H - h) / 2 + 40, w, h);
        card(list, &l.card, r);
        let firsts: Vec<&Column> = order.iter().filter(|(p, _)| *p == 1).map(|(_, c)| *c).collect();
        let heading = if firsts.len() == 1 {
            self.t("postgame.winner").replace("{player}", &firsts[0].name)
        } else {
            self.t("postgame.tie")
        };
        list.text(
            rgb(l.card.ink),
            l.display(),
            50,
            Align::Centre,
            (W / 2, r.y + 64),
            &heading,
        );
        let mono = self.fishboy.theme.as_ref().map_or(l.display(), |t| t.mono());
        for (i, (place, c)) in order.iter().enumerate() {
            let y = r.y + 130 + i as i32 * 92;
            let row = Rect::new(r.x + 30, y, w - 60, 78);
            let fill = if *place == 1 {
                l.postgame.winner
            } else {
                l.postgame.card
            };
            card_in(list, &l.card, row, fill, l.card.outline);
            list.text(
                rgb(l.card.ink),
                l.display(),
                40,
                Align::Left,
                (row.x + 24, row.y + 40),
                &self.t(&format!("postgame.place.{place}")),
            );
            list.circle(
                rgb(self.player_rgb(c.seat)),
                rgb(l.card.outline),
                3,
                (row.x + 150, row.y + 39),
                14,
            );
            list.text(
                rgb(l.card.ink),
                l.body(),
                34,
                Align::Left,
                (row.x + 178, row.y + 40),
                &c.name,
            );
            list.text(
                rgb(l.card.ink),
                mono,
                38,
                Align::Right,
                (row.right() - 24, row.y + 40),
                &self.points(c.total()),
            );
        }
        let totals: Vec<i64> = cols.iter().map(Column::total).collect();
        places(&totals)
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// The postgame's buttons.
    pub(crate) fn postgame_panels(&self) -> Vec<Panel> {
        let items = if self.postgame_ranked() {
            vec![self.button("postgame.rematch"), self.button("postgame.title")]
        } else {
            vec![Item::quiet_button("postgame.skip", self.t("postgame.skip"))]
        };
        vec![Panel::new("postgame", String::new(), items)]
    }
}
