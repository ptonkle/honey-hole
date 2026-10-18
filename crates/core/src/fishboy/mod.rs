//! The Fish-Boy: the angler's wrist computer, raised to the face (BRIEF.md,
//! "Fish-Boy"; PLAN.md decisions 7 and 8).
//!
//! Tab raises and lowers it, Esc lowers it, and it opens on the last tab
//! viewed. Raised, the camera is first person and the device sits in front
//! of it so its screen spans `screen_pct` (80%) of the view's width or
//! height, whichever binds; the bezel (built to 9/8 of the screen) takes
//! the next 10% and the live world shows round the edge. Raising and
//! lowering slide it up from below the view, keeping the screen square to
//! the camera, so the 2D layer maps onto the screen exactly.
//!
//! The device is drawn by the 3D renderer as a pass of its own over the
//! world (its own camera, the depth cleared); its screen's contents are
//! the 2D draw list (`ui2d`) in a layer mapped onto where the screen part
//! lands (SECOND-ORDER R2.60). The tabs: the radar (`radar.rs`), the
//! Fishpedia and your catch list (`fishpedia.rs`).
//!
//! Whether it is up is the match's (`Match::set_fishboy`: Tab is a
//! `SeatInput` edge, Esc goes through `Match::cancel_layers`, and a raised
//! Fish-Boy makes the boat coast); the app keeps only what is drawn: the
//! tab, the slide up and down, and the pointer. The radar, the Fishpedia
//! and the catch list read the match through plain view structs
//! (`RadarView`, `FishpediaRow`, `CatchRow`).

pub mod fishpedia;
pub mod radar;
#[cfg(test)]
mod tests;

use crate::app::gallery::PLAYER_COLOURS;
use crate::app::{App, Screen};
use crate::gfx::fx::{self, deg, div, mul, ONE};
use crate::gfx::geom::{v3, Affine};
use crate::gfx::render::{self, Camera, Light};
use crate::model::Model;
use crate::roster::{tier_key, Roster, TIERS};
use crate::ui2d::theme::{c, Theme};
use crate::ui2d::{Align, Bitmap, Built, List, Pass, Rect, Rgba, Transform};

use fishpedia::{CatchRow, FishpediaRow};
use radar::{PingState, RadarScene, RadarView};

/// The Fish-Boy's model id.
pub const MODEL: &str = "fishboy";
/// The screen part's size (mm) when no model is loaded: the file's.
const SCREEN_MM: [i32; 2] = [192, 108];

/// The Fish-Boy's tabs, in order along the top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Radar,
    Fishpedia,
    Catch,
}

impl Tab {
    pub const ALL: [Tab; 3] = [Tab::Radar, Tab::Fishpedia, Tab::Catch];

    pub fn key(self) -> &'static str {
        match self {
            Tab::Radar => "radar",
            Tab::Fishpedia => "fishpedia",
            Tab::Catch => "catch",
        }
    }

    pub fn from_key(key: &str) -> Option<Tab> {
        Tab::ALL.into_iter().find(|t| t.key() == key)
    }

    /// The key that picks the tab while the Fish-Boy is up.
    pub fn digit(self) -> &'static str {
        match self {
            Tab::Radar => "Digit1",
            Tab::Fishpedia => "Digit2",
            Tab::Catch => "Digit3",
        }
    }
}

/// The Fish-Boy as drawn, and its data files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FishBoy {
    pub theme: Option<Theme>,
    pub roster: Roster,
    /// raised, or being raised: follows the match's seat
    /// (`SeatState::fishboy`) each frame
    pub open: bool,
    /// the tab shown, kept while it is down
    pub tab: Tab,
    /// the clock (µs) it last started up or down
    pub moved_at: i64,
    /// the device's pixel ratio, hundredths (pointer CSS pixels to buffer)
    pub dpr_pct: i32,
    /// the pointer on the screen, in screen units, and whether a button is
    /// down (for minigames on the Fish-Boy)
    pub pointer: Option<(i32, i32)>,
    pub pointer_down: bool,
    /// the radar's picture of this match's map, made once a match
    pub radar_map: Option<Bitmap>,
}

impl Default for FishBoy {
    fn default() -> FishBoy {
        FishBoy {
            theme: None,
            roster: Roster::default(),
            open: false,
            tab: Tab::Radar,
            moved_at: i64::MIN / 4,
            dpr_pct: 100,
            pointer: None,
            pointer_down: false,
            radar_map: None,
        }
    }
}

/// 3t² − 2t³, for t in 0..=ONE.
fn ease(t: i32) -> i32 {
    let t2 = mul(t, t);
    3 * t2 - 2 * mul(t2, t)
}

/// Where the device and its screen are in the first-person view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Raised {
    /// how far up, ONE when fully raised
    pub amount: i32,
    /// the screen's distance in front of the eye (Q16 metres)
    pub dist: i32,
    /// the screen's middle above the line of sight (negative: below)
    pub lift: i32,
    /// the screen in buffer pixels, from the top left
    pub px: Rect,
    pub camera: Camera,
}

impl FishBoy {
    fn raise_us(&self) -> i64 {
        self.theme
            .as_ref()
            .map_or(1, |t| t.fishboy.raise_ms as i64 * 1000)
            .max(1)
    }

    /// How far raised the device is at `clock`: 0 down, ONE up.
    pub fn amount(&self, clock: i64) -> i32 {
        let dur = self.raise_us();
        let done = (clock - self.moved_at).clamp(0, dur);
        let p = (done * ONE as i64 / dur) as i32;
        if self.open {
            p
        } else {
            ONE - p
        }
    }

    /// Raises it if down, lowers it if up, from wherever it is now.
    pub fn toggle(&mut self, clock: i64) {
        let r = self.amount(clock) as i64;
        self.open = !self.open;
        let from = if self.open { r } else { ONE as i64 - r };
        self.moved_at = clock - from * self.raise_us() / ONE as i64;
        self.pointer_down = false;
    }

    /// Puts it straight down (leaving Play).
    pub fn reset(&mut self) {
        self.open = false;
        self.moved_at = i64::MIN / 4;
        self.pointer_down = false;
    }

    /// The device's place and its screen's pixels in a buffer of `size`,
    /// or `None` while it is down. `screen` is the screen part's size
    /// (Q16 metres).
    pub fn raised(&self, clock: i64, size: [i32; 2], screen: [i32; 2]) -> Option<Raised> {
        let theme = self.theme.as_ref()?;
        let amount = self.amount(clock);
        if amount <= 0 {
            return None;
        }
        let fov = deg(theme.fishboy.fov_deg);
        let tan = div(fx::sin(fov / 2), fx::cos(fov / 2));
        let aspect = render::aspect(size);
        let pct = fx::ratio(theme.fishboy.screen_pct, 100);
        let [sw, sh] = screen;
        let by_h = div(sh, 2 * mul(pct, tan));
        let by_w = div(sw, 2 * mul(mul(pct, aspect), tan));
        let dist = by_h.max(by_w);
        // half the view's height at the screen's distance
        let half = mul(dist, tan);
        let lift = -mul(ONE - ease(amount), 2 * half + sh);
        let (w, h) = (size[0] as i64, size[1] as i64);
        let half_w = mul(half, aspect) as i64;
        let pw = sw as i64 * w / (2 * half_w).max(1);
        let ph = sh as i64 * h / (2 * half as i64).max(1);
        let cy = h / 2 - lift as i64 * h / (2 * half as i64).max(1);
        let px = Rect::new(((w - pw) / 2) as i32, (cy - ph / 2) as i32, pw as i32, ph as i32);
        let camera = Camera {
            eye: v3(0, 0, 0),
            fwd: v3(0, 0, -ONE),
            fov,
            near: dist / 8,
            far: dist * 8,
        };
        Some(Raised {
            amount,
            dist,
            lift,
            px,
            camera,
        })
    }
}

/// The screen part's size (Q16 metres) and its middle, from the model.
fn screen_of(m: &Model) -> ([i32; 2], crate::gfx::geom::V3) {
    let Some(part) = m.parts.iter().position(|p| p.name == "screen") else {
        return (SCREEN_MM.map(|v| fx::ratio(v, 1000)), v3(0, 0, 0));
    };
    let vs: Vec<_> = m.verts.iter().filter(|v| v.1 == part).map(|v| v.0).collect();
    let lo_x = vs.iter().map(|v| v.x).min().unwrap_or(0);
    let hi_x = vs.iter().map(|v| v.x).max().unwrap_or(0);
    let lo_y = vs.iter().map(|v| v.y).min().unwrap_or(0);
    let hi_y = vs.iter().map(|v| v.y).max().unwrap_or(0);
    let mid = m
        .anchor("screen")
        .map_or(v3((lo_x + hi_x) / 2, (lo_y + hi_y) / 2, 0), |a| a.at);
    ([hi_x - lo_x, hi_y - lo_y], mid)
}

/// Land the radar shows round the lake's water.
const RADAR_MARGIN_MM: i32 = 24_000;

/// The tabs' rectangles along the top of the screen.
const TAB_Y: i32 = 22;
const TAB_H: i32 = 62;
const TAB_TEXT: i32 = 34;

impl App {
    fn fishboy_model(&self) -> Option<&Model> {
        self.models.iter().find(|m| m.id == MODEL)
    }

    fn screen_size(&self) -> [i32; 2] {
        self.fishboy_model()
            .map_or(SCREEN_MM.map(|v| fx::ratio(v, 1000)), |m| screen_of(m).0)
    }

    /// The device's place now, while it is up.
    pub fn fishboy_raised(&self) -> Option<Raised> {
        if self.screen != Screen::Play {
            return None;
        }
        self.fishboy.raised(self.clock, self.size, self.screen_size())
    }

    /// While the device is up (or on its way), the world is seen first
    /// person: the eye's height (mm), the field of view and the look down
    /// (angle units), for `App::match_frame`.
    pub fn fishboy_eye(&self) -> Option<(i32, i32, i32)> {
        let theme = self.fishboy.theme.as_ref()?;
        if self.screen != Screen::Play || self.fishboy.amount(self.clock) <= 0 {
            return None;
        }
        let f = &theme.fishboy;
        Some((f.eye_mm, deg(f.fov_deg), deg(f.look_down_deg)))
    }

    /// The screen layer's transform now.
    fn screen_transform(&self) -> Option<Transform> {
        let theme = self.fishboy.theme.as_ref()?;
        let r = self.fishboy_raised()?;
        Some(Transform::onto(theme.fishboy.canvas, r.px))
    }

    fn tab_rects(&self) -> Vec<(Tab, Rect)> {
        let Some(theme) = self.fishboy.theme.as_ref() else {
            return Vec::new();
        };
        let font = theme.mono();
        let mut x = 40;
        Tab::ALL
            .iter()
            .map(|&t| {
                let w = font.width(&self.t(&format!("fishboy.tab.{}", t.key())), TAB_TEXT) + 52;
                let r = Rect::new(x, TAB_Y, w, TAB_H);
                x += w + 16;
                (t, r)
            })
            .collect()
    }

    /// Whether this browser's seat has its Fish-Boy up in the match.
    fn seat_up(&self) -> bool {
        self.play.as_ref().is_some_and(|p| p.game.seats[p.seat].fishboy)
    }

    /// Each frame, after the match's ticks: the device follows the seat
    /// (sliding up or down from where it is), and a new match's map gets
    /// its radar picture. Off Play it is put straight down.
    pub(crate) fn fishboy_advance(&mut self) {
        let up = self.screen == Screen::Play && self.seat_up();
        if self.screen != Screen::Play || self.play.is_none() {
            self.fishboy.reset();
        } else if up != self.fishboy.open {
            self.fishboy.toggle(self.clock);
        }
        let (Some(p), Some(t)) = (self.play.as_ref(), self.fishboy.theme.as_ref()) else {
            self.fishboy.radar_map = None;
            return;
        };
        let id = radar::bitmap_id(&p.game.map);
        if self.fishboy.radar_map.as_ref().is_none_or(|b| b.id != id) {
            self.fishboy.radar_map = Some(radar::map_bitmap(&p.game.map, &t.radar));
        }
    }

    /// A ping: tapped into this seat's next input, so it reaches the match
    /// by the same path as the AI's and an online player's.
    fn fishboy_ping(&mut self) {
        self.tap("ping", false);
    }

    /// A key while the Fish-Boy is up: 1 to 3 pick a tab, and Space on the
    /// Radar tab pings (Space still reaches the match as the cast, which
    /// does nothing while it is up). Returns whether the key is used up
    /// here. Tab and Esc are the match's (`controls.json`).
    pub(crate) fn fishboy_key(&mut self, code: &str) -> bool {
        if self.screen != Screen::Play || !self.seat_up() || self.play.as_ref().is_some_and(|p| p.menu_open())
        {
            return false;
        }
        if let Some(t) = Tab::ALL.into_iter().find(|t| t.digit() == code) {
            self.fishboy.tab = t;
            return true;
        }
        if code == "Space" && self.fishboy.tab == Tab::Radar {
            self.fishboy_ping();
        }
        false
    }

    /// The Fish-Boy's tab while it is up, for the page's `data-fishboy`.
    pub fn fishboy_shown(&self) -> String {
        if self.screen == Screen::Play && self.fishboy.open {
            self.fishboy.tab.key().into()
        } else {
            String::new()
        }
    }

    /// The pointer, in CSS pixels ("x y"), as screen units.
    fn pointer_at(&self, value: &str) -> Result<Option<(i32, i32)>, String> {
        // whole pixels: a fraction after the point is dropped
        let mut it = value
            .split_whitespace()
            .map(|v| v.split('.').next().unwrap_or("").parse::<i64>());
        let (Some(Ok(x)), Some(Ok(y))) = (it.next(), it.next()) else {
            return Err(format!("The pointer \"{value}\" is not two whole numbers."));
        };
        let k = self.fishboy.dpr_pct as i64;
        let (px, py) = ((x * k / 100) as i32, (y * k / 100) as i32);
        Ok(self.screen_transform().map(|t| t.invert(px, py)))
    }

    /// The Fish-Boy's actions and the pointer, and its data files.
    pub(crate) fn fishboy_act(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            "data.ui2d" => {
                self.fishboy.theme = Some(Theme::parse(value)?);
            }
            "data.fish" => {
                self.fishboy.roster = Roster::parse(value)?;
            }
            "fishboy.tab" => {
                self.fishboy.tab =
                    Tab::from_key(value).ok_or_else(|| format!("There is no Fish-Boy tab \"{value}\"."))?;
            }
            "fishboy.ping" if self.seat_up() => self.fishboy_ping(),
            "fishboy.ping" => {}
            "pointer.move" | "pointer.down" | "pointer.up" => {
                let at = self.pointer_at(value)?;
                self.fishboy.pointer = at;
                match name {
                    "pointer.down" => {
                        self.fishboy.pointer_down = true;
                        if let Some((x, y)) = at {
                            self.fishboy_click(x, y);
                        }
                    }
                    "pointer.up" => self.fishboy.pointer_down = false,
                    _ => {}
                }
            }
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    /// A press on the screen at (x, y) screen units: a tab, or the ping.
    fn fishboy_click(&mut self, x: i32, y: i32) {
        if !self.seat_up() || self.play.as_ref().is_some_and(|p| p.menu_open()) {
            return;
        }
        if let Some((t, _)) = self.tab_rects().into_iter().find(|(_, r)| r.contains(x, y)) {
            self.fishboy.tab = t;
            return;
        }
        if self.fishboy.tab == Tab::Radar && radar::PING_BUTTON.contains(x, y) {
            self.fishboy_ping();
        }
    }

    /// The boats and the buoy as this seat's radar draws them: the match's
    /// radar view, with each boat where the drawing has it between ticks.
    pub fn fishboy_radar_view(&self) -> Option<RadarView> {
        let p = self.play.as_ref()?;
        let mut view = p.game.radar_view(p.seat);
        let world = self.play_view()?.world;
        for (b, w) in view.boats.iter_mut().zip(&world.boats) {
            b.at = crate::map::P::new(w.x_mm, w.y_mm);
            b.heading = w.heading;
        }
        Some(view)
    }

    /// Each kind of fish in this match and how many are left, in the
    /// roster's order.
    pub fn fishpedia_rows(&self) -> Vec<FishpediaRow> {
        let Some(p) = self.play.as_ref() else {
            return Vec::new();
        };
        let left = p.game.fish_left();
        self.fishboy
            .roster
            .0
            .iter()
            .filter_map(|k| {
                let n = left.iter().find(|(id, _)| *id == k.id)?.1;
                Some(FishpediaRow {
                    kind: k.id.clone(),
                    tier: k.tier,
                    left: n,
                })
            })
            .collect()
    }

    /// This seat's catches, in the order landed.
    pub fn catch_rows(&self) -> Vec<CatchRow> {
        let Some(p) = self.play.as_ref() else {
            return Vec::new();
        };
        p.game.scores[p.seat]
            .catches
            .iter()
            .map(|c| CatchRow {
                kind: c.kind.clone(),
                length_mm: c.length_mm,
            })
            .collect()
    }

    // ---- drawing ----

    /// This frame's 2D list and the 3D passes that go with it.
    pub fn build_ui2d(&self) -> Built {
        let mut built = Built::default();
        let Some(theme) = self.fishboy.theme.as_ref() else {
            return built;
        };
        if self.screen != Screen::Play {
            return built;
        }
        let list = &mut built.list;
        let mut status = vec![format!("fishboy={}", self.fishboy_shown())];
        match self.fishboy_raised() {
            None => {
                if !self.play.as_ref().is_some_and(|p| p.menu_open()) {
                    // the HUD: how to raise it
                    let hud = theme.hud.canvas;
                    list.layer(Transform::letterbox(
                        hud,
                        Rect::new(0, 0, self.size[0], self.size[1]),
                    ));
                    list.text(
                        c(theme.hud.ink),
                        theme.mono(),
                        30,
                        Align::Centre,
                        (hud[0] / 2, hud[1] - 48),
                        &self.t("hud.fishboy"),
                    );
                }
                status.push("raised=0".into());
            }
            Some(r) => {
                status.push(format!("raised={}", r.amount * 100 / ONE));
                let p = r.px;
                status.push(format!("screen={},{},{},{}", p.x, p.y, p.w, p.h));
                if let Some(p) = self.device_pass(&r) {
                    built.passes.push(p);
                }
                list.layer(Transform::onto(theme.fishboy.canvas, r.px));
                self.draw_screen(list, theme, &mut status);
            }
        }
        for shot in built.list.shots.clone() {
            if let Some(p) = self.shot_pass(theme, &shot) {
                built.passes.push(p);
            }
        }
        status.push(format!("shots={}", built.list.shots.len()));
        built.list.status = status.join(" ");
        built
    }

    fn draw_screen(&self, list: &mut List, theme: &Theme, status: &mut Vec<String>) {
        let [w, h] = theme.fishboy.canvas;
        let s = &theme.screen;
        let font = theme.mono();
        let phos = c(s.phosphor);
        list.fill_rect(c(s.background), Rect::new(0, 0, w, h));

        // the tabs
        for (t, r) in self.tab_rects() {
            let label = self.t(&format!("fishboy.tab.{}", t.key())).to_uppercase();
            let mid = (r.x + r.w / 2, r.y + r.h / 2);
            if t == self.fishboy.tab {
                list.fill_rect(phos, r);
                list.text(c(s.ink), font, TAB_TEXT, Align::Centre, mid, &label);
            } else {
                list.stroke_rect(c(s.dim), 3, r.inset(2));
                list.text(phos, font, TAB_TEXT, Align::Centre, mid, &label);
            }
        }
        list.line(
            c(s.dim).shade(150),
            3,
            (24, TAB_Y + TAB_H + 4),
            (w - 24, TAB_Y + TAB_H + 4),
        );

        match self.fishboy.tab {
            Tab::Radar => {
                self.corner_hint(list, theme);
                if let (Some(p), Some(b), Some(view)) = (
                    self.play.as_ref(),
                    self.fishboy.radar_map.as_ref(),
                    self.fishboy_radar_view(),
                ) {
                    let g = &p.game;
                    let t = &g.tuning.radar;
                    let ping = &g.seats[p.seat].ping;
                    let tick = g.tick as i64;
                    let state = ping.state(tick, t);
                    let secs = |ticks: i64| ((ticks * radar::TICK_MS + 999) / 1000).to_string();
                    let ping_status = match state {
                        PingState::Ready => self.t("radar.ping.ready"),
                        PingState::Sweeping(_) => self.t("radar.ping.sweeping"),
                        PingState::Cooling(left, _) => {
                            self.t("radar.ping.cooling").replace("{seconds}", &secs(left))
                        }
                    };
                    let seen = view
                        .boats
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            if i == view.me {
                                return self.t("radar.you");
                            }
                            match ping.sightings.get(i).copied().flatten() {
                                Some(s) => self.t("radar.seen").replace(
                                    "{seconds}",
                                    &((tick - s.tick) * radar::TICK_MS / 1000).to_string(),
                                ),
                                None => self.t("radar.unseen"),
                            }
                        })
                        .collect();
                    let blips = ping.sightings.iter().filter(|s| s.is_some()).count();
                    status.push(format!(
                        "ping={} blips={blips}",
                        match state {
                            PingState::Ready => "ready",
                            PingState::Sweeping(_) => "sweeping",
                            PingState::Cooling(..) => "cooling",
                        }
                    ));
                    let scene = RadarScene {
                        theme,
                        map: &g.map,
                        bitmap: b,
                        view: &view,
                        ping,
                        tuning: t,
                        tick,
                        region: radar::region(&g.map, RADAR_MARGIN_MM),
                        title: self.t(g.map.setting.name_key()).to_uppercase(),
                        buoy: self.t("radar.buoy"),
                        ping_label: self.t("radar.ping").to_uppercase(),
                        ping_status,
                        seen,
                    };
                    scene.draw(list);
                }
            }
            Tab::Fishpedia => {
                let rows = self.fishpedia_rows();
                let total: u32 = rows.iter().map(|r| r.left).sum();
                list.text(
                    phos,
                    font,
                    32,
                    Align::Right,
                    (w - 50, TAB_Y + TAB_H / 2),
                    &self
                        .t("fishpedia.total")
                        .replace("{count}", &total.to_string())
                        .to_uppercase(),
                );
                let words = fishpedia::Words {
                    tier: self.tier_names(),
                    left: self.t("fishpedia.left"),
                    none_left: self.t("fishpedia.none_left"),
                };
                let has = |id: &str| self.models.iter().any(|m| m.id == id);
                let ms = ((self.clock / 1000) % crate::model::MAX_MS as i64) as u32;
                fishpedia::draw(list, theme, &self.fishboy.roster, &rows, &words, &has, ms);
                status.push(format!("fish_left={total}"));
            }
            Tab::Catch => {
                self.corner_hint(list, theme);
                let rows = self.catch_rows();
                let words = fishpedia::CatchWords {
                    heading: self.t("catch.heading"),
                    none: self.t("catch.none"),
                    feet: self.t("catch.feet"),
                    tier: self.tier_names(),
                    count: self.t("catch.count"),
                };
                fishpedia::draw_catches(list, theme, &self.fishboy.roster, &rows, &words);
                status.push(format!("catches={}", rows.len()));
            }
        }

        // the scanlines over everything
        let every = s.scanline_every;
        let dark = Rgba::solid([0, 0, 0]).alpha(s.scanline_pct);
        let mut y = 0;
        while y < h {
            list.fill_rect(dark, Rect::new(0, y, w, every / 2));
            y += every;
        }
    }

    fn corner_hint(&self, list: &mut List, theme: &Theme) {
        list.text(
            c(theme.screen.dim).shade(170),
            theme.mono(),
            22,
            Align::Right,
            (theme.fishboy.canvas[0] - 50, TAB_Y + TAB_H / 2),
            &self.t("fishboy.keys"),
        );
    }

    fn tier_names(&self) -> Vec<(crate::map::fish::Tier, String)> {
        TIERS
            .iter()
            .map(|t| (*t, self.t(&format!("tier.{}", tier_key(*t)))))
            .collect()
    }

    /// The device over the world: its own camera, the depth cleared.
    fn device_pass(&self, r: &Raised) -> Option<Pass> {
        let theme = self.fishboy.theme.as_ref()?;
        let m = self.fishboy_model()?;
        let look = self.look.as_ref()?;
        let (_, mid) = screen_of(m);
        let place = Affine::translate(v3(0, r.lift, -r.dist)).then(&Affine::translate(mid.neg()));
        let transforms = m.transforms(&m.poses(None, 0));
        let colour = render::rgb(look.players[PLAYER_COLOURS[0]]);
        let mut tris = Vec::new();
        m.draw(&transforms, &place, &m.colours(colour), &mut tris);
        let far = r.camera.far;
        let view = render::view_values(
            &r.camera,
            render::aspect(self.size),
            (far, far * 2),
            &theme.fishboy.light,
            &render::Air::plain(&theme.fishboy.light),
        );
        Some(Pass {
            gl_rect: [0, 0, self.size[0], self.size[1]],
            clear: None,
            view,
            tris,
        })
    }

    /// A model in its box: a side view fitted to the box, nose to the
    /// right.
    fn shot_pass(&self, theme: &Theme, shot: &crate::ui2d::Shot) -> Option<Pass> {
        let m = self.models.iter().find(|m| m.id == shot.model)?;
        let anim = shot.anim.filter(|a| m.animation(*a).is_some());
        let transforms = m.transforms(&m.poses(anim, shot.ms));
        let mut tris = Vec::new();
        m.draw(&transforms, &Affine::IDENTITY, &m.colours([ONE; 3]), &mut tris);
        let aspect = fx::ratio(shot.px.w.max(1), shot.px.h.max(1));
        let (centre, _) = m.ball();
        let half_len = fx::ratio(m.length_mm(), 2000);
        let half_h = fx::ratio(m.height_mm(), 2000);
        let fov = deg(30);
        let fit = div(half_len, aspect).max(half_h);
        let dist = Camera::fit(fov, fit, 112);
        let fwd = v3(ONE, 0, 0);
        let cam = Camera {
            eye: centre.sub(fwd.scale(dist)),
            fwd,
            fov,
            near: dist / 20,
            far: dist * 4,
        };
        let pct = shot.light_pct.clamp(0, 100);
        let dim = |c: [i32; 3]| c.map(|v| v * pct / 100);
        let l = &theme.fishboy.light;
        let light = Light {
            fog: shot.background,
            sun_dir: l.sun_dir,
            sun: dim(l.sun),
            sky: dim(l.sky),
            ground: dim(l.ground),
            glow_fog_pct: l.glow_fog_pct,
        };
        let view = render::view_values(
            &cam,
            aspect,
            (cam.far * 2, cam.far * 3),
            &light,
            &render::Air::plain(&light),
        );
        Some(Pass {
            gl_rect: Pass::gl(shot.px, self.size),
            clear: Some(render::rgb(shot.background)),
            view,
            tris,
        })
    }
}
