//! The six item minigames (BRIEF.md, "Items"): short WarioWare-style games
//! an item plays before it takes effect. Each is a deterministic state
//! machine on its own virtual canvas (`minigames.canvas`, 1600 x 900, the
//! Fish-Boy screen's), stepped one tick ([`TICK_MS`]) at a time.
//!
//! - A minigame cannot be failed: playing badly only makes it take longer.
//! - Esc ([`Input::quit`]) quits it; the caller wastes the item.
//! - It opens with a one-line instruction banner (`minigames.banner_ms`),
//!   then play. Input during the banner is ignored, except Esc.
//! - Randomness comes only from the seed passed in.
//! - Its numbers are `data/tuning.json`'s `minigames` ([`tuning`]); its
//!   look is `data/ui2d.json`'s `minigames` ([`look`]).
//!
//! # Using one (the items system, the Minigames screen)
//!
//! ```text
//! let mut mg = Minigame::new(Kind::PowerReel, seed, &tuning.minigames);
//! // each tick:
//! mg.step(&Input { pointer, held, pressed, released, quit });
//! for cue in mg.take_cues() { sound.play(cue) }   // minigame_start, ...
//! match mg.state() { State::Playing => {}, State::Done { ms_taken } => use the item,
//!                    State::Quit => waste the item }
//! // each frame:
//! let target = match mg.layer() { Layer::FishBoy => the Fish-Boy screen's pixels,
//!                                 Layer::Hud => the whole buffer };
//! let t = mg.transform(target);      // canvas units to pixels (letterboxed)
//! list.layer(t);
//! mg.draw(&mut list, &theme, &|key| strings.t(key));
//! ```
//!
//! The pointer is in canvas units: `t.invert(px, py)`. A caller that sees
//! pointer moves between ticks may also pass each through
//! [`Minigame::point`], so a fast circle round the reel or a fast stroke
//! of the eraser is followed in full rather than sampled once a tick.
//! While [`Minigame::hides_cursor`] is true (the camera's viewfinder), the
//! page hides its own cursor: the minigame draws one.
//!
//! The sounds are cue names the sound engine knows ([`cue`]); the caller
//! plays them for this player only.

pub mod look;
pub mod tuning;

mod atoms;
mod beats;
mod camera;
mod cloak;
mod focus;
mod frame;
mod reel;
pub(crate) mod shape;
pub mod sight;
#[cfg(test)]
mod tests;

use crate::map::rng::Rng;
use crate::sim::input::SeatInput;
use crate::sim::TICK_MS;
use crate::ui2d::theme::{c, Theme};
use crate::ui2d::{Align, Font, List, Rect, Transform};

use look::Look;
use tuning::MinigameTuning;

/// The sound cues a minigame asks for (`data/sounds.json`).
pub mod cue {
    /// the banner comes up
    pub const START: &str = "minigame_start";
    /// a step forward: a turn of the reel, an atom binned, a circle hit,
    /// another share of the boat erased
    pub const TICK: &str = "minigame_tick";
    /// a wrong bin, a circle missed
    pub const MISS: &str = "minigame_miss";
    /// the minigame is done
    pub const SUCCESS: &str = "minigame_success";
    /// a photograph taken
    pub const SHUTTER: &str = "camera_shutter";
}

/// The six minigames, one per item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Cloak,
    PowerReel,
    Nitrous,
    Teleporter,
    Camera,
    Radar,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Cloak,
        Kind::PowerReel,
        Kind::Nitrous,
        Kind::Teleporter,
        Kind::Camera,
        Kind::Radar,
    ];

    /// The item's key, as the sound events name it (`ItemUsed`'s `item`).
    pub fn key(self) -> &'static str {
        match self {
            Kind::Cloak => "cloak",
            Kind::PowerReel => "power_reel",
            Kind::Nitrous => "nitrous",
            Kind::Teleporter => "teleporter",
            Kind::Camera => "camera",
            Kind::Radar => "enhanced_radar",
        }
    }

    pub fn from_key(key: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.key() == key)
    }

    /// Where it is drawn: Cloaking Tech and Enhanced Radar on the Fish-Boy's
    /// screen, the rest over the whole view.
    pub fn layer(self) -> Layer {
        match self {
            Kind::Cloak | Kind::Radar => Layer::FishBoy,
            _ => Layer::Hud,
        }
    }

    fn salt(self) -> u64 {
        Kind::ALL.iter().position(|k| *k == self).unwrap_or(0) as u64 + 1
    }
}

/// The 2D layer a minigame is drawn on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// the Fish-Boy's screen, raised (`fishboy/`)
    FishBoy,
    /// the full-screen layer over the view
    Hud,
}

/// One tick's input. The pointer is in canvas units, `None` while off the
/// layer; `pressed` and `released` are the main button's edges since the
/// last tick (a press and a release inside one tick show in both).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Input {
    pub pointer: Option<(i32, i32)>,
    pub held: bool,
    pub pressed: bool,
    pub released: bool,
    /// Esc
    pub quit: bool,
}

impl Input {
    /// A match seat's input: the main button and Esc. The pointer must
    /// already be in the minigame's canvas units (the caller converts it).
    pub fn from_seat(s: &SeatInput) -> Input {
        Input {
            pointer: s.pointer.map(|[x, y]| (x, y)),
            held: s.down("click"),
            pressed: s.pressed.click,
            released: s.released.click,
            quit: s.pressed.cancel,
        }
    }
}

/// Where a minigame stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Playing,
    /// finished; `ms_taken` counts play after the banner
    Done {
        ms_taken: u32,
    },
    /// Esc: the item is wasted
    Quit,
}

/// A step's results for the game: the cues it asks for.
pub(crate) struct Ctx<'a> {
    pub cues: &'a mut Vec<&'static str>,
}

/// What a game draws with.
pub(crate) struct Paint<'a> {
    pub list: &'a mut List,
    pub look: &'a Look,
    pub font: &'a Font,
    pub t: &'a dyn Fn(&str) -> String,
    pub canvas: [i32; 2],
    /// where the pointer is, for a drawn cursor
    pub pointer: Option<(i32, i32)>,
    pub held: bool,
}

#[derive(Debug, Clone)]
enum Game {
    Cloak(cloak::Cloak),
    Reel(reel::Reel),
    Atoms(atoms::Atoms),
    Beats(beats::Beats),
    Camera(camera::Shoot),
    Focus(focus::Focus),
}

#[derive(Debug, Clone)]
pub struct Minigame {
    pub kind: Kind,
    canvas: [i32; 2],
    banner_ticks: u32,
    /// ticks since it began, the banner included
    ticks: u32,
    state: State,
    game: Game,
    cues: Vec<&'static str>,
    pointer: Option<(i32, i32)>,
    held: bool,
}

/// Milliseconds to whole ticks, at least one.
pub(crate) fn ticks_of(ms: i32) -> u32 {
    crate::sim::ticks(ms).max(1)
}

/// Units a second to thousandths of a unit a tick.
pub(crate) fn per_tick_milli(per_s: i32) -> i32 {
    (per_s as i64 * 1000 * TICK_MS as i64 / 1000) as i32
}

/// The instruction line's middle, from the top, and the progress line's,
/// from the bottom (canvas units).
pub(crate) const TOP_LINE: i32 = 56;
pub(crate) const BOTTOM_LINE: i32 = 44;
/// The banner's text keeps this far from the canvas's sides.
const BANNER_MARGIN: i32 = 60;

impl Minigame {
    pub fn new(kind: Kind, seed: u64, t: &MinigameTuning) -> Minigame {
        let mut rng = Rng::stream(seed, kind.salt());
        let canvas = t.canvas;
        let game = match kind {
            Kind::Cloak => Game::Cloak(cloak::Cloak::new(&t.cloak, canvas)),
            Kind::PowerReel => Game::Reel(reel::Reel::new(&t.power_reel, canvas)),
            Kind::Nitrous => Game::Atoms(atoms::Atoms::new(&t.nitrous, canvas, &mut rng)),
            Kind::Teleporter => Game::Beats(beats::Beats::new(&t.teleporter, canvas, rng)),
            Kind::Camera => Game::Camera(camera::Shoot::new(&t.camera, canvas, rng)),
            Kind::Radar => Game::Focus(focus::Focus::new(&t.enhanced_radar, canvas, &mut rng)),
        };
        Minigame {
            kind,
            canvas,
            banner_ticks: crate::sim::ticks(t.banner_ms),
            ticks: 0,
            state: State::Playing,
            game,
            cues: vec![cue::START],
            pointer: None,
            held: false,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn layer(&self) -> Layer {
        self.kind.layer()
    }

    pub fn canvas(&self) -> [i32; 2] {
        self.canvas
    }

    /// Whether the instruction banner is up (play has not begun).
    pub fn in_banner(&self) -> bool {
        self.ticks < self.banner_ticks
    }

    /// Ticks of play so far, after the banner.
    pub fn play_ticks(&self) -> u32 {
        self.ticks.saturating_sub(self.banner_ticks)
    }

    /// The canvas fitted whole into `target` (pixels), letterboxed.
    pub fn transform(&self, target: Rect) -> Transform {
        Transform::letterbox(self.canvas, target)
    }

    /// The minigame draws its own cursor now, so the page hides its own.
    pub fn hides_cursor(&self) -> bool {
        matches!(self.game, Game::Camera(_)) && self.state == State::Playing
    }

    /// The cues asked for since the last call.
    pub fn take_cues(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.cues)
    }

    /// How far along: (done, needed). The cloak counts percent erased.
    pub fn progress(&self) -> (i32, i32) {
        match &self.game {
            Game::Cloak(g) => (g.erased_pct(), g.t.done_pct),
            Game::Reel(g) => (g.turns_done(), g.t.turns),
            Game::Atoms(g) => (g.binned(), g.total()),
            Game::Beats(g) => (g.hits, g.t.hits),
            Game::Camera(g) => (g.photos, g.t.photos),
            Game::Focus(g) => (g.within(), focus::SLIDERS as i32),
        }
    }

    /// One tick.
    pub fn step(&mut self, input: &Input) {
        if self.state != State::Playing {
            return;
        }
        if input.quit {
            self.state = State::Quit;
            return;
        }
        self.pointer = input.pointer;
        self.held = input.held;
        self.ticks += 1;
        if self.ticks <= self.banner_ticks {
            return;
        }
        let mut ctx = Ctx { cues: &mut self.cues };
        let finished = match &mut self.game {
            Game::Cloak(g) => g.step(&mut ctx, input),
            Game::Reel(g) => g.step(&mut ctx, input),
            Game::Atoms(g) => g.step(&mut ctx, input),
            Game::Beats(g) => g.step(&mut ctx, input),
            Game::Camera(g) => g.step(&mut ctx, input),
            Game::Focus(g) => g.step(&mut ctx, input),
        };
        if finished {
            self.state = State::Done {
                ms_taken: self.play_ticks() * TICK_MS as u32,
            };
            // the radar's success sounds as the picture snaps sharp
            if !matches!(self.game, Game::Focus(_)) {
                self.cues.push(cue::SUCCESS);
            }
        }
    }

    /// A pointer move between ticks, with the main button `held` or not:
    /// the reel and the eraser follow every one.
    pub fn point(&mut self, at: (i32, i32), held: bool) {
        if self.state != State::Playing || self.in_banner() {
            return;
        }
        match &mut self.game {
            Game::Reel(g) => {
                let mut ctx = Ctx { cues: &mut self.cues };
                g.sample(&mut ctx, Some(at));
            }
            Game::Cloak(g) if held => {
                let mut ctx = Ctx { cues: &mut self.cues };
                g.stroke(&mut ctx, at);
            }
            Game::Cloak(g) => g.lift(),
            _ => {}
        }
    }

    /// A few words on what is shown, for the browser check.
    pub fn status(&self) -> String {
        let phase = match self.state {
            State::Playing if self.in_banner() => "banner",
            State::Playing => "play",
            State::Done { .. } => "done",
            State::Quit => "quit",
        };
        let (done, total) = self.progress();
        let mut out = format!("mg={} phase={phase} done={done} total={total}", self.kind.key());
        // a target a scripted pointer can aim at, in canvas units
        let target = match &self.game {
            Game::Atoms(g) => g.pickable().map(|(n, p)| {
                let bin = g.bins[if n { 0 } else { 1 }];
                format!(
                    " atom={},{} bin={},{}",
                    p.0,
                    p.1,
                    bin.x + bin.w / 2,
                    bin.y + bin.h / 2
                )
            }),
            Game::Camera(g) => g.visible().map(|p| format!(" fish={},{}", p.0, p.1)),
            _ => None,
        };
        out.push_str(&target.unwrap_or_default());
        out
    }

    /// The minigame in canvas units; the caller has set the layer
    /// ([`Minigame::transform`]).
    pub fn draw(&self, list: &mut List, theme: &Theme, t: &dyn Fn(&str) -> String) {
        let look = &theme.minigames;
        let mut p = Paint {
            list,
            look,
            font: theme.mono(),
            t,
            canvas: self.canvas,
            pointer: self.pointer,
            held: self.held,
        };
        let [w, h] = self.canvas;
        p.list.clip(Rect::new(0, 0, w, h));
        match &self.game {
            Game::Cloak(g) => g.draw(&mut p),
            Game::Reel(g) => g.draw(&mut p),
            Game::Atoms(g) => g.draw(&mut p),
            Game::Beats(g) => g.draw(&mut p),
            Game::Camera(g) => g.draw(&mut p),
            Game::Focus(g) => g.draw(&mut p),
        }
        if self.layer() == Layer::Hud {
            frame::draw(p.list, &look.frame, self.canvas);
        }
        let instruction = self.instruction(t);
        if self.in_banner() && self.state == State::Playing {
            // on the Fish-Boy the banner is the device's own phosphor
            let device = (self.layer() == Layer::FishBoy).then_some(&theme.screen);
            let font = match device {
                Some(_) => theme.mono(),
                None => theme.fonts.get("banner").unwrap_or(theme.mono()),
            };
            self.draw_banner(&mut p, font, &instruction, device);
        } else {
            p.list.text(
                c(look.text),
                p.font,
                look.text_size,
                Align::Centre,
                (w / 2, TOP_LINE),
                &instruction,
            );
            self.draw_progress(&mut p);
        }
        p.list.text(
            c(look.hint),
            p.font,
            look.hint_size,
            Align::Left,
            (24, h - BOTTOM_LINE),
            &t("minigame.quit_hint"),
        );
        // the camera's viewfinder over everything
        if let Game::Camera(g) = &self.game {
            if self.state == State::Playing {
                g.draw_viewfinder(&mut p);
            }
        }
        p.list.unclip();
    }

    /// The instruction, with its count filled in.
    pub fn instruction(&self, t: &dyn Fn(&str) -> String) -> String {
        let (_, total) = self.progress();
        let count = match self.kind {
            Kind::Cloak => 0,
            _ => total,
        };
        t(&format!("minigame.{}", self.kind.key())).replace("{count}", &count.to_string())
    }

    fn draw_progress(&self, p: &mut Paint) {
        let look = p.look;
        let [w, h] = p.canvas;
        let (done, total) = self.progress();
        let text = match self.kind {
            Kind::Cloak => (p.t)("minigame.erased").replace("{percent}", &done.to_string()),
            Kind::Radar => return,
            _ => (p.t)("minigame.progress")
                .replace("{done}", &done.to_string())
                .replace("{total}", &total.to_string()),
        };
        // a bar, then the words
        let bar = Rect::new(w / 2 - 200, h - BOTTOM_LINE - 10, 400, 20);
        p.list
            .round_rect(c(look.bar_back), crate::ui2d::Rgba::NONE, 0, bar, 10);
        let fill = bar.w * done.clamp(0, total.max(1)) / total.max(1);
        if fill > 0 {
            p.list.round_rect(
                c(look.bar),
                crate::ui2d::Rgba::NONE,
                0,
                Rect::new(bar.x, bar.y, fill.max(20), bar.h),
                10,
            );
        }
        p.list.text(
            c(look.text),
            p.font,
            look.progress_size,
            Align::Left,
            (bar.right() + 24, h - BOTTOM_LINE),
            &text,
        );
    }

    fn draw_banner(
        &self,
        p: &mut Paint,
        font: &Font,
        text: &str,
        device: Option<&crate::ui2d::theme::ScreenLook>,
    ) {
        let b = &p.look.banner;
        let (band_c, edge, ink, shade) = match device {
            Some(s) => (s.dim, s.phosphor, s.phosphor, s.background),
            None => (b.band, b.edge, b.ink, b.shadow),
        };
        let [w, h] = p.canvas;
        p.list
            .fill_rect(c([0, 0, 0]).alpha(b.dim_pct), Rect::new(0, 0, w, h));
        let pop_ticks = ticks_of(b.pop_ms) as i32;
        let grown = (self.ticks as i32).min(pop_ticks);
        // grows from 60% to full, a WarioWare pop
        // no wider than the canvas, less a margin each side
        let chars = text.chars().count().max(1) as i32;
        let fit = (w - 2 * BANNER_MARGIN) * 100 / (chars * font.advance.max(1));
        let size = b.size.min(fit) * (60 + 40 * grown / pop_ticks) / 100;
        let band = Rect::new(0, h / 2 - b.band_h / 2, w, b.band_h);
        p.list.fill_rect(c(band_c), band);
        let f = &p.look.frame;
        let stripes = f.stripe_h * f.stripes.len() as i32;
        for y in [band.y - stripes - 4, band.bottom() + 4] {
            p.list.fill_rect(c(edge), Rect::new(0, y - 4, w, stripes + 8));
            // the magazine's triple stripe in an ink keyline, above and
            // below; plain phosphor bars on the device
            if device.is_none() {
                crate::flow::print::stripes(p.list, &f.stripes, 0, y, w, f.stripe_h);
            }
        }
        let shadow = size / 16;
        p.list.text(
            c(shade),
            font,
            size,
            Align::Centre,
            (w / 2 + shadow, h / 2 + shadow),
            text,
        );
        p.list
            .text(c(ink), font, size, Align::Centre, (w / 2, h / 2), text);
    }
}
