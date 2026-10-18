//! The app: which screen is up, the reader's settings, and what the page
//! shows (`ui`). Everything changes through `act(name, value)`, so a later
//! sync model (online play, a replay) can drive the same app from inputs
//! alone: no state lives in the page, and nothing here reads a clock or
//! draws a random number.
//!
//! The screens belong to the person at this screen. The match itself is
//! `sim::Match`, which knows nothing of screens; `app/play.rs` sets one up
//! (the Lobby screen), feeds it this browser's input and draws it (Play).

use serde::Serialize;

use crate::model::Model;
use crate::patch_notes::{self, Entry};
use crate::strings::{self, Strings};
use crate::view::{Item, Panel};

pub mod fishing;
pub mod gallery;
pub mod lake;
mod menus;
pub mod minigames;
mod notes;
pub mod play;
pub mod sounds;
#[cfg(test)]
mod tests;

/// The game's version: major.minor.patch. The newest entry of
/// `data/patch-notes.json` must be this one (`tests/data_files.rs`).
pub const VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Screen {
    Title,
    Settings,
    /// What's New: each version's notes (`patch_notes.rs`)
    Notes,
    /// the offline match setup, "New match" (`app/play.rs`)
    Lobby,
    /// a match (`app/play.rs`)
    Play,
    /// each model turning, for a look at the art (`app/gallery.rs`)
    Gallery,
    /// every sound, played on a click (`app/sounds.rs`)
    Sounds,
    /// a map drawn in 3D with a camera tour, for debugging (`app/lake.rs`)
    Lake,
    /// each item's minigame on its own, for trying them (`app/minigames.rs`)
    Minigames,
}

/// How large the menus' text is drawn. The page scales its text by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextSize {
    Small,
    Medium,
    Large,
}

impl TextSize {
    pub const ALL: [TextSize; 3] = [TextSize::Small, TextSize::Medium, TextSize::Large];

    pub fn key(self) -> &'static str {
        match self {
            TextSize::Small => "small",
            TextSize::Medium => "medium",
            TextSize::Large => "large",
        }
    }

    pub fn from_key(key: &str) -> Option<TextSize> {
        TextSize::ALL.into_iter().find(|t| t.key() == key)
    }
}

/// The volume slider's range and step, in percent.
pub const VOLUME_MAX: u32 = 100;
pub const VOLUME_STEP: u32 = 5;

/// The reader's settings. They last for this visit: the page keeps no
/// browser storage (BUILDER-SPEC D17).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// 0 to `VOLUME_MAX`, a multiple of `VOLUME_STEP`
    pub volume: u32,
    pub text_size: TextSize,
    /// What's New lists minor versions too
    pub notes_minor: bool,
    /// the person's name in matches; empty for the stand-in name
    pub name: String,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            volume: 80,
            text_size: TextSize::Medium,
            notes_minor: false,
            name: String::new(),
        }
    }
}

/// Everything the page draws.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ui {
    pub screen: Screen,
    /// the document title
    pub title: String,
    /// shown in the corner, as "v0.1.0"
    pub version: String,
    /// whether clicking the version opens What's New
    pub version_opens: bool,
    pub text_size: TextSize,
    /// for the sounds the game will play, 0 to `VOLUME_MAX`
    pub volume: u32,
    /// whether the page draws the 3D scene (`frame`) behind the panels
    pub scene: bool,
    /// the Fish-Boy's tab while it is up, else "" (`fishboy/`)
    pub fishboy: String,
    /// the 2D layer takes the pointer (the Fish-Boy is up): the page lets
    /// the pointer go and sends its place over the layer (`pointer.*`)
    pub pointer: bool,
    /// the page keeps keys and the mouse from the browser (Tab moves no
    /// focus; a click locks the pointer) and sends them all here
    pub capture: bool,
    /// the page asks for the view model each frame (a match's clock moves)
    pub live: bool,
    /// the page hides its cursor over the 2D layer, which draws its own
    /// (the camera minigame's viewfinder)
    pub hide_cursor: bool,
    pub panels: Vec<Panel>,
}

pub struct App {
    strings: Strings,
    notes: Vec<Entry>,
    pub settings: Settings,
    pub screen: Screen,
    /// Settings was opened from the in-match menu: Back returns to Play
    pub settings_return: bool,
    /// the match files from the page (`data_act`)
    pub tuning: Option<crate::tuning::Tuning>,
    pub controls: Option<crate::controls::Controls>,
    pub maps: Option<crate::map::list::MapList>,
    pub names: Vec<String>,
    /// the match setup being chosen, once the tuning is in
    pub lobby: Option<play::Lobby>,
    /// the match being played
    pub play: Option<play::Play>,
    /// the models, in gallery order, and the look they are drawn in
    /// (`load_models`)
    pub models: Vec<Model>,
    pub look: Option<gallery::Look>,
    pub gallery: gallery::Gallery,
    /// the page's clock, in microseconds (`advance`)
    pub clock: i64,
    /// the drawing buffer's size, in pixels (`resize`)
    pub size: [i32; 2],
    /// the page could not start WebGL 2
    pub scene_unavailable: bool,
    /// the Fish-Boy, its data and the demo match behind it (`fishboy/`)
    pub fishboy: crate::fishboy::FishBoy,
    /// the sound recipes and the cues waiting for the page (`load_sounds`)
    pub sound: Option<crate::sound::Sound>,
    pub sounds: sounds::SoundsScreen,
    /// the Lake preview's choices
    pub preview: lake::Preview,
    /// the lake drawn now, built once per map (`scene::Lake`); the page
    /// uploads its static vertices when its stamp changes
    pub world: Option<crate::scene::Lake>,
    /// the Minigames screen's minigame and results
    pub minigames: minigames::MinigamesScreen,
    /// the page's `?debug=fishing`: a fish by every bobber (`app/fishing.rs`)
    pub debug_fishing: bool,
}

impl App {
    /// Checks the strings file and the patch notes, and opens on the Title.
    pub fn new(strings_json: &str, notes_json: &str) -> Result<App, String> {
        let strings = strings::parse(strings_json).map_err(|e| e.message())?;
        let notes = patch_notes::parse(notes_json)?;
        Ok(App {
            strings,
            notes,
            settings: Settings::default(),
            screen: Screen::Title,
            settings_return: false,
            tuning: None,
            controls: None,
            maps: None,
            names: Vec::new(),
            lobby: None,
            play: None,
            models: Vec::new(),
            look: None,
            gallery: gallery::Gallery::default(),
            clock: 0,
            size: [1, 1],
            scene_unavailable: false,
            fishboy: crate::fishboy::FishBoy::default(),
            sound: None,
            sounds: sounds::SoundsScreen::default(),
            preview: lake::Preview::default(),
            world: None,
            minigames: minigames::MinigamesScreen::default(),
            debug_fishing: false,
        })
    }

    pub(crate) fn t(&self, key: &str) -> String {
        self.strings.t(key)
    }

    // ---- screens ----

    fn go(&mut self, screen: Screen) {
        self.screen = screen;
    }

    /// Back, and Escape off Play: to the Title, or from Settings opened in
    /// a match back to the match. (In Play, Escape is the match's own
    /// "cancel", `controls.json`.)
    fn back(&mut self) {
        if self.screen == Screen::Settings && self.settings_return && self.play.is_some() {
            self.settings_return = false;
            self.go(Screen::Play);
        } else {
            self.settings_return = false;
            self.go(Screen::Title);
        }
    }

    fn escape(&mut self) {
        match self.screen {
            Screen::Play => {}
            Screen::Settings
            | Screen::Notes
            | Screen::Gallery
            | Screen::Lobby
            | Screen::Sounds
            | Screen::Lake => self.back(),
            Screen::Minigames => self.minigames_escape(),
            Screen::Title => {}
        }
    }

    /// The version in the corner opens What's New from the menus, not
    /// from Play.
    pub fn version_opens_notes(&self) -> bool {
        matches!(
            self.screen,
            Screen::Title
                | Screen::Settings
                | Screen::Notes
                | Screen::Gallery
                | Screen::Lobby
                | Screen::Sounds
                | Screen::Lake
                | Screen::Minigames
        )
    }

    /// A key went down (`KeyboardEvent.code`). The core decides what each
    /// key does; a key with no use is ignored.
    fn key(&mut self, code: &str) {
        if code == "Escape" {
            self.escape();
        }
    }

    /// The one door for buttons, settings, keys and screens. `name` is a
    /// dotted action name, `value` the control's new value.
    pub fn act(&mut self, name: &str, value: &str) -> Result<(), String> {
        if name.starts_with("menu.") {
            self.click();
        }
        if name == "key" && self.fishboy_key(value) {
            return Ok(());
        }
        if self.match_input(name, value)? {
            return Ok(());
        }
        match name {
            "key" => self.key(value),
            "key.up" | "keys.release" | "mouse.down" | "mouse.up" | "mouse.move" => {}
            "menu.play" => self.go(Screen::Lobby),
            "menu.settings" => {
                self.settings_return = self.screen == Screen::Play;
                self.go(Screen::Settings);
            }
            "menu.notes" => self.go(Screen::Notes),
            "menu.gallery" => self.go(Screen::Gallery),
            "menu.sounds" => self.go(Screen::Sounds),
            "menu.lake" => self.open_lake(),
            "menu.minigames" => self.go(Screen::Minigames),
            "scene.unavailable" => self.scene_unavailable = true,
            "version" if self.version_opens_notes() => self.go(Screen::Notes),
            "version" => {}
            "menu.back" => self.back(),
            "menu.quit" => self.leave_match(),
            "menu.pause" if self.screen == Screen::Play => self.tap("menu", false),
            "menu.pause" => {}
            "menu.resume" => self.tap("menu", true),
            "notes.minor" => self.settings.notes_minor = value == "on",
            "settings.name" => self.set_name(value),
            "debug.flags" => self.debug_act(value),
            "data.ui2d" | "data.fish" => self.fishboy_act(name, value)?,
            _ if name.starts_with("data.") => self.data_act(name, value)?,
            _ if name.starts_with("lobby.") => self.lobby_act(name, value)?,
            _ if name.starts_with("settings.") => self.settings_act(name, value)?,
            _ if name.starts_with("gallery.") => self.gallery_act(name, value)?,
            _ if name.starts_with("minigames.")
                || (name.starts_with("pointer.") && self.screen == Screen::Minigames) =>
            {
                self.minigames_act(name, value)?
            }
            _ if name.starts_with("fishboy.") || name.starts_with("pointer.") => {
                self.fishboy_act(name, value)?
            }
            _ if name.starts_with("sounds.") => self.sounds_act(name, value)?,
            _ if name.starts_with("lake.") => self.lake_act(name, value)?,
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    /// The page's clock, in microseconds. It only goes forward.
    pub fn advance(&mut self, now: i64) {
        self.clock = self.clock.max(now);
        self.run_ticks(self.clock);
        self.fishboy_advance();
        self.minigames_advance();
    }

    /// The canvas's size in CSS pixels and the device's pixel ratio in
    /// hundredths; returns the drawing buffer's size.
    pub fn resize(&mut self, css_w: i32, css_h: i32, dpr_pct: i32) -> [i32; 2] {
        self.size = crate::gfx::render::buffer_size(css_w, css_h, dpr_pct);
        self.fishboy.dpr_pct = dpr_pct;
        self.size
    }

    // ---- what the page shows ----

    /// This frame's draw data (`gfx::render::Frame`): the screen's 3D
    /// scene, or nothing on a screen without one.
    pub fn frame(&self) -> crate::gfx::render::Frame {
        match self.screen {
            Screen::Gallery => self.gallery_frame(),
            Screen::Lake => self.lake_frame(),
            Screen::Play => self.match_frame().unwrap_or_default(),
            _ => crate::gfx::render::Frame::default(),
        }
    }

    /// The static world's stamp, 0 for none: the page uploads its
    /// vertices again when it changes.
    pub fn world_stamp(&self) -> u32 {
        self.world.as_ref().map_or(0, |w| w.stamp())
    }

    pub fn ui(&self) -> Ui {
        let panels = match self.screen {
            Screen::Title => self.title_panels(),
            Screen::Settings => self.settings_panels(),
            Screen::Notes => self.notes_panels(),
            Screen::Lobby => self.lobby_panels(),
            Screen::Play => self.play_panels(),
            Screen::Gallery => self.gallery_panels(),
            Screen::Sounds => self.sounds_panels(),
            Screen::Lake => self.lake_panels(),
            Screen::Minigames => self.minigames_panels(),
        };
        Ui {
            screen: self.screen,
            title: self.t("title"),
            version: format!("v{VERSION}"),
            version_opens: self.version_opens_notes(),
            text_size: self.settings.text_size,
            volume: self.settings.volume,
            scene: matches!(
                self.screen,
                Screen::Gallery | Screen::Play | Screen::Lake | Screen::Minigames
            ),
            fishboy: self.fishboy_shown(),
            pointer: self.fishboy_shown() != ""
                || (self.screen == Screen::Minigames && self.minigames.game.is_some()),
            capture: self.screen == Screen::Play
                && self
                    .play
                    .as_ref()
                    .is_some_and(|p| !p.menu_open() && !p.game.ended),
            live: (self.screen == Screen::Play && self.play.is_some()) || self.screen == Screen::Minigames,
            hide_cursor: self.minigames_hide_cursor(),
            panels,
        }
    }
}
