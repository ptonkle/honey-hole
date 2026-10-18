//! The radar: the map in the approved green look, your own boat and its
//! heading, the item buoy, and the other players as the last ping saw them.
//!
//! The radar reads a plain [`RadarView`] (the boats and the buoy), which
//! the match fills (`Match::radar_view`). A [`Ping`] is one player's ping;
//! the match holds one per seat and steps it each tick. It starts when the
//! cooldown allows, sweeps out in a circle from where the player was,
//! crossing the whole map in the sweep time, and reveals each other player
//! as the sweep passes them, unless they are cloaked. It runs in the
//! match's ticks, with its numbers from `data/tuning.json`'s `radar`
//! section (`tuning::RadarTuning`).

use crate::gfx::fx::{self, ONE};
use crate::map::{Map, P};
pub use crate::tuning::RadarTuning;
use crate::ui2d::theme::{c, RadarLook, Theme};
use crate::ui2d::{Align, Bitmap, List, Rect, Rgba};

/// Milliseconds a tick, the match's.
pub const TICK_MS: i64 = crate::sim::TICK_MS as i64;

impl RadarTuning {
    pub fn cooldown_ticks(&self) -> i64 {
        self.ping_cooldown_ms as i64 / TICK_MS
    }

    pub fn sweep_ticks(&self) -> i64 {
        self.ping_sweep_ms as i64 / TICK_MS
    }
}

/// One boat as the radar may know it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoatView {
    pub at: P,
    /// the way the bow points, in map angle units (0 east, a quarter
    /// turn north), as `map::Spawn::heading`
    pub heading: i32,
    /// the lobby slot, which gives the player colour
    pub slot: usize,
    pub name: String,
    /// hidden from pings (Cloaking Tech)
    pub cloaked: bool,
}

/// What the radar is told: every boat (yours is `me`) and the item buoy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadarView {
    pub me: usize,
    pub boats: Vec<BoatView>,
    pub buoy: Option<P>,
}

/// Where and when a ping saw a boat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sighting {
    pub at: P,
    pub heading: i32,
    pub tick: i64,
}

/// One player's ping and what it has seen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ping {
    /// the tick the last ping started
    pub started: Option<i64>,
    pub centre: P,
    /// the farthest the sweep reaches, mm: the map's farthest corner
    pub reach_mm: i64,
    /// by boat, the last sighting
    pub sightings: Vec<Option<Sighting>>,
    /// by boat, whether this sweep has passed it
    passed: Vec<bool>,
}

impl Default for Ping {
    fn default() -> Ping {
        Ping {
            started: None,
            centre: P::new(0, 0),
            reach_mm: 0,
            sightings: Vec::new(),
            passed: Vec::new(),
        }
    }
}

/// Where a ping stands at a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PingState {
    Ready,
    /// sweeping: done in thousandths of the sweep
    Sweeping(i32),
    /// cooling down: ticks until ready, and thousandths of the wait done
    Cooling(i64, i32),
}

impl Ping {
    pub fn state(&self, tick: i64, t: &RadarTuning) -> PingState {
        let Some(s) = self.started else {
            return PingState::Ready;
        };
        let since = tick - s;
        if since < t.sweep_ticks() {
            PingState::Sweeping((since * 1000 / t.sweep_ticks().max(1)) as i32)
        } else if since < t.cooldown_ticks() {
            PingState::Cooling(
                t.cooldown_ticks() - since,
                (since * 1000 / t.cooldown_ticks().max(1)) as i32,
            )
        } else {
            PingState::Ready
        }
    }

    /// Starts a ping from your boat, if the cooldown allows. `size_mm` is
    /// the map's side.
    pub fn start(&mut self, tick: i64, view: &RadarView, size_mm: i32, t: &RadarTuning) -> bool {
        if self.state(tick, t) != PingState::Ready {
            return false;
        }
        let Some(me) = view.boats.get(view.me) else {
            return false;
        };
        let centre = me.at;
        self.reach_mm = [(0, 0), (size_mm, 0), (0, size_mm), (size_mm, size_mm)]
            .iter()
            .map(|&(x, y)| centre.dist(P::new(x, y)))
            .max()
            .unwrap_or(0);
        self.started = Some(tick);
        self.centre = centre;
        self.passed = vec![false; view.boats.len()];
        self.passed[view.me] = true;
        true
    }

    /// The sweep's radius at `tick` (mm), while it sweeps.
    pub fn radius(&self, tick: i64, t: &RadarTuning) -> Option<i64> {
        match self.state(tick, t) {
            PingState::Sweeping(_) => {
                let since = tick - self.started.unwrap_or(tick);
                Some(self.reach_mm * since / t.sweep_ticks().max(1))
            }
            _ => None,
        }
    }

    /// One tick: each boat the sweep has reached is seen where it is now,
    /// or, if cloaked, its old sighting is cleared.
    pub fn step(&mut self, tick: i64, view: &RadarView, t: &RadarTuning) {
        if self.sightings.len() < view.boats.len() {
            self.sightings.resize(view.boats.len(), None);
        }
        let Some(s) = self.started else {
            return;
        };
        let since = tick - s;
        if since < 0 || since > t.sweep_ticks() {
            return;
        }
        let r = self.reach_mm * since / t.sweep_ticks().max(1);
        self.passed.resize(view.boats.len(), false);
        for (i, b) in view.boats.iter().enumerate() {
            if self.passed[i] || b.at.dist(self.centre) > r {
                continue;
            }
            self.passed[i] = true;
            self.sightings[i] = if b.cloaked {
                None
            } else {
                Some(Sighting {
                    at: b.at,
                    heading: b.heading,
                    tick,
                })
            };
        }
    }
}

// ---- the map picture ----

/// The palette's entries: water deep and shallow, land, shore, then the
/// same four on a darker scanline row.
const DEEP: u8 = 0;
const SHALLOW: u8 = 1;
const LAND: u8 = 2;
const SHORE: u8 = 3;
const SCANNED: u8 = 4;

/// The ground's height at a point (mm), from the cells' middles,
/// bilinear, as `tools/maps/preview.py` samples it.
pub fn height_at(map: &Map, x_mm: i32, y_mm: i32) -> i32 {
    let n = map.n as i64;
    let to_grid = |v: i32| -> i64 {
        let g = v as i64 * 1024 / map.cell_mm as i64 - 512;
        g.clamp(0, (n - 1) * 1024 - 1)
    };
    let (gx, gy) = (to_grid(x_mm), to_grid(y_mm));
    let (x0, y0) = ((gx >> 10) as usize, (gy >> 10) as usize);
    let (tx, ty) = (gx & 1023, gy & 1023);
    let at = |x: usize, y: usize| map.ground[y * map.n + x] as i64;
    let a = at(x0, y0) * (1024 - tx) + at(x0 + 1, y0) * tx;
    let b = at(x0, y0 + 1) * (1024 - tx) + at(x0 + 1, y0 + 1) * tx;
    ((a * (1024 - ty) + b * ty) >> 20) as i32
}

/// The radar picture's id for a map (the page caches the picture by it).
pub fn bitmap_id(map: &Map) -> String {
    format!("radar-{}-{}", map.setting.id(), map.seed)
}

/// The radar's picture of a map: one pixel per block of `block_mm`, land
/// against two water shades with a bright shoreline wherever a block's
/// neighbour is the other kind, every other row darker. Made once a match.
pub fn map_bitmap(map: &Map, look: &RadarLook) -> Bitmap {
    let size = map.size_mm();
    let blocks = (size / look.block_mm) as usize;
    let mut h = vec![0i32; blocks * blocks];
    for by in 0..blocks {
        for bx in 0..blocks {
            // the block's corner, from the top (north) row down
            let x = bx as i32 * look.block_mm + 500;
            let y = size - by as i32 * look.block_mm - 500;
            h[by * blocks + bx] = height_at(map, x, y);
        }
    }
    let wet = |x: usize, y: usize| h[y * blocks + x] < 0;
    let mut pixels = Vec::with_capacity(blocks * blocks);
    for by in 0..blocks {
        for bx in 0..blocks {
            let water = wet(bx, by);
            let edge = [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| {
                let (x, y) = (bx as i32 + dx, by as i32 + dy);
                x >= 0
                    && y >= 0
                    && (x as usize) < blocks
                    && (y as usize) < blocks
                    && wet(x as usize, y as usize) != water
            });
            let kind = if edge {
                SHORE
            } else if !water {
                LAND
            } else if h[by * blocks + bx] < -look.deep_mm {
                DEEP
            } else {
                SHALLOW
            };
            pixels.push(kind + if by % 2 == 1 { SCANNED } else { 0 });
        }
    }
    let base = [look.deep, look.shallow, look.land, look.shore].map(c);
    let mut palette: Vec<Rgba> = base.to_vec();
    palette.extend(base.iter().map(|p| p.shade(look.scan_pct)));
    Bitmap {
        id: bitmap_id(map),
        w: blocks as i32,
        h: blocks as i32,
        pixels,
        palette,
    }
}

// ---- drawing ----

/// Where the map sits on the Fish-Boy's screen (1600 x 900 units).
pub const MAP_RECT: Rect = Rect::new(56, 110, 760, 760);
/// The ping's bar, which a click on starts a ping too.
pub const PING_RECT: Rect = Rect::new(880, 730, 560, 44);
/// The ping's whole button: its label, bar and status.
pub const PING_BUTTON: Rect = Rect::new(870, 670, 640, 180);

/// The square of the map the radar shows: the lake's water and a margin
/// round it, so the lake fills the screen (its lower-left corner and side,
/// mm).
pub fn region(map: &Map, margin_mm: i32) -> (P, i32) {
    let wet: Vec<P> = (0..map.cells.len())
        .filter(|&i| map.cells[i].water())
        .map(|i| map.centre(i))
        .collect();
    let size = map.size_mm();
    if wet.is_empty() {
        return (P::new(0, 0), size);
    }
    let (x0, x1) = (
        wet.iter().map(|p| p.x).min().unwrap_or(0),
        wet.iter().map(|p| p.x).max().unwrap_or(0),
    );
    let (y0, y1) = (
        wet.iter().map(|p| p.y).min().unwrap_or(0),
        wet.iter().map(|p| p.y).max().unwrap_or(0),
    );
    let side = ((x1 - x0).max(y1 - y0) + 2 * margin_mm).min(size);
    let lo = |a: i32, b: i32| ((a + b) / 2 - side / 2).clamp(0, size - side);
    (P::new(lo(x0, x1), lo(y0, y1)), side)
}

/// What the radar screen needs besides the view.
pub struct RadarScene<'a> {
    pub theme: &'a Theme,
    pub map: &'a Map,
    pub bitmap: &'a Bitmap,
    pub view: &'a RadarView,
    pub ping: &'a Ping,
    pub tuning: &'a RadarTuning,
    pub tick: i64,
    /// the square of the map shown (`region`)
    pub region: (P, i32),
    /// the screen's strings, already filled
    pub title: String,
    pub buoy: String,
    pub ping_label: String,
    pub ping_status: String,
    /// by boat, the line under its name in the legend
    pub seen: Vec<String>,
}

impl RadarScene<'_> {
    /// A map point on the screen, in Q16 units.
    fn to_screen_q(&self, p: P) -> (i32, i32) {
        let (lo, side) = (self.region.0, self.region.1 as i64);
        let (dx, dy) = ((p.x - lo.x) as i64, (lo.y as i64 + side) - p.y as i64);
        let x = MAP_RECT.x as i64 * ONE as i64 + dx * MAP_RECT.w as i64 * ONE as i64 / side;
        let y = MAP_RECT.y as i64 * ONE as i64 + dy * MAP_RECT.h as i64 * ONE as i64 / side;
        (x as i32, y as i32)
    }

    fn to_screen(&self, p: P) -> (i32, i32) {
        let (x, y) = self.to_screen_q(p);
        (x >> 16, y >> 16)
    }

    /// Map millimetres in screen units.
    fn units(&self, mm: i64) -> i32 {
        (mm * MAP_RECT.w as i64 / self.region.1.max(1) as i64) as i32
    }

    fn player_colour(&self, slot: usize) -> Rgba {
        let name = crate::app::gallery::PLAYER_COLOURS[slot % 4];
        c(self.theme.radar.players[name])
    }

    /// A name beside a marker, on a dark backing.
    fn label(&self, list: &mut List, at: (i32, i32), text: &str, colour: Rgba) {
        let font = self.theme.mono();
        let size = 26;
        let w = font.width(text, size);
        let x = at.0 + 16;
        list.fill_rect(
            c(self.theme.radar.label_back).alpha(80),
            Rect::new(x - 4, at.1 - 17, w + 8, 34),
        );
        list.text(colour, font, size, Align::Left, (x, at.1), text);
    }

    /// A boat's marker: a dot and its heading.
    fn boat(&self, list: &mut List, at: P, heading: i32, colour: Rgba, big: bool) {
        let (x, y) = self.to_screen(at);
        let (r, len) = if big { (10, 34) } else { (8, 24) };
        let (hx, hy) = (fx::cos(heading), fx::sin(heading));
        let tip = (x + fx::mul(hx, len * ONE) / ONE, y - fx::mul(hy, len * ONE) / ONE);
        list.line(colour, 4, (x, y), tip);
        list.circle(colour, c(self.theme.radar.label_back), 2, (x, y), r);
    }

    pub fn draw(&self, list: &mut List) {
        let t = self.theme;
        let look = &t.radar;
        let font = t.mono();
        let phos = c(t.screen.phosphor);

        // the map
        list.fill_rect(c(look.panel), MAP_RECT);
        list.clip(MAP_RECT);
        // the picture's top-left block is the map's north-west corner
        let covered = self.units(self.bitmap.w as i64 * look.block_mm as i64);
        let corner = self.to_screen(P::new(0, self.map.size_mm()));
        list.bitmap(&self.bitmap.id, Rect::new(corner.0, corner.1, covered, covered));
        for k in 1..look.grid_lines {
            let x = MAP_RECT.x + MAP_RECT.w * k / look.grid_lines;
            let y = MAP_RECT.y + MAP_RECT.h * k / look.grid_lines;
            list.line(c(look.grid), 1, (x, MAP_RECT.y), (x, MAP_RECT.bottom()));
            list.line(c(look.grid), 1, (MAP_RECT.x, y), (MAP_RECT.right(), y));
        }
        let me = self.view.boats.get(self.view.me);
        if let Some(me) = me {
            // range rings round your boat
            let at = self.to_screen(me.at);
            for k in 1..=3 {
                let r = self.units(look.ring_m as i64 * 1000 * k);
                list.circle(Rgba::NONE, c(look.ring).alpha(70), 2, at, r);
            }
        }
        // the sweep: a soft band behind a bright edge
        if let Some(r) = self.ping.radius(self.tick, self.tuning) {
            let at = self.to_screen(self.ping.centre);
            let edge = self.units(r);
            let band = self.units(look.sweep_band_m as i64 * 1000);
            list.radial(
                c(look.sweep).alpha(0),
                c(look.sweep).alpha(look.sweep_pct),
                at,
                edge - band,
                edge,
            );
            list.circle(Rgba::NONE, c(look.sweep).alpha(85), 3, at, edge);
        }
        // the buoy, always
        if let Some(b) = self.view.buoy {
            let at = self.to_screen(b);
            list.circle(Rgba::NONE, phos, 3, at, 12);
            list.circle(phos, Rgba::NONE, 0, at, 4);
            self.label(list, at, &self.buoy, phos);
        }
        // the others as last seen
        for (i, b) in self.view.boats.iter().enumerate() {
            if i == self.view.me {
                continue;
            }
            if let Some(s) = self.ping.sightings.get(i).copied().flatten() {
                let colour = self.player_colour(b.slot);
                self.boat(list, s.at, s.heading, colour, false);
                self.label(list, self.to_screen(s.at), &b.name, colour);
            }
        }
        // you, always, with your heading
        if let Some(me) = me {
            let colour = self.player_colour(me.slot);
            self.boat(list, me.at, me.heading, colour, true);
            self.label(list, self.to_screen(me.at), &me.name, colour);
        }
        list.unclip();
        list.stroke_rect(c(t.screen.dim), 3, MAP_RECT);

        // the legend
        let x = 880;
        list.text(phos, font, 56, Align::Left, (x, 150), &self.title);
        list.line(c(t.screen.dim), 3, (x, 194), (1540, 194));
        let mut y = 240;
        for (i, b) in self.view.boats.iter().enumerate() {
            let colour = self.player_colour(b.slot);
            list.circle(colour, Rgba::NONE, 0, (x + 12, y), 11);
            list.text(colour, font, 34, Align::Left, (x + 40, y), &b.name);
            if let Some(line) = self.seen.get(i) {
                list.text(
                    c(t.screen.dim).shade(160),
                    font,
                    22,
                    Align::Left,
                    (x + 40, y + 32),
                    line,
                );
            }
            y += 78;
        }
        list.circle(Rgba::NONE, phos, 3, (x + 12, y), 11);
        list.circle(phos, Rgba::NONE, 0, (x + 12, y), 4);
        list.text(phos, font, 34, Align::Left, (x + 40, y), &self.buoy);

        // the ping
        let (fill, colour) = match self.ping.state(self.tick, self.tuning) {
            PingState::Ready => (1000, phos),
            PingState::Sweeping(k) => (k, phos),
            PingState::Cooling(_, k) => (k, c(t.screen.dim).shade(170)),
        };
        list.text(
            phos,
            font,
            34,
            Align::Left,
            (PING_RECT.x, PING_RECT.y - 34),
            &self.ping_label,
        );
        list.stroke_rect(phos, 3, PING_RECT);
        let inner = PING_RECT.inset(7);
        list.fill_rect(
            colour,
            Rect::new(inner.x, inner.y, inner.w * fill / 1000, inner.h),
        );
        list.text(
            phos,
            font,
            30,
            Align::Left,
            (PING_RECT.x, PING_RECT.bottom() + 40),
            &self.ping_status,
        );
    }
}
