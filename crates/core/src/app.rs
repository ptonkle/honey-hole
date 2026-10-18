//! The app: which screen is up, the reader's settings, and what the page
//! shows (`ui`). Everything changes through `act(name, value)`, so a later
//! sync model (online play, a replay) can drive the same app from inputs
//! alone: no state lives in the page, and nothing here reads a clock or
//! draws a random number.
//!
//! The screens and the pause menu belong to the person at this screen.
//! The game itself is not designed yet; Play is a placeholder screen with
//! the pause menu over it, and holds no player of its own, so nothing
//! assumes there is one player.

use serde::Serialize;

use crate::model::Model;
use crate::patch_notes::{self, Entry};
use crate::strings::{self, Strings};
use crate::view::{Item, Panel};

pub mod gallery;
pub mod lake;
mod menus;
mod notes;
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
    /// the game; a placeholder until the game is designed
    Play,
    /// each model turning, for a look at the art (`app/gallery.rs`)
    Gallery,
    /// a map drawn in 3D with a camera tour, for debugging (`app/lake.rs`)
    Lake,
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
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            volume: 80,
            text_size: TextSize::Medium,
            notes_minor: false,
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
    pub panels: Vec<Panel>,
}

pub struct App {
    strings: Strings,
    notes: Vec<Entry>,
    pub settings: Settings,
    pub screen: Screen,
    /// the pause menu is open over Play. Whether the game stops under it
    /// is the sync model's to decide, once there is a game.
    pub paused: bool,
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
    /// the Lake preview's choices
    pub preview: lake::Preview,
    /// the lake drawn now, built once per map (`scene::Lake`); the page
    /// uploads its static vertices when its stamp changes
    pub world: Option<crate::scene::Lake>,
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
            paused: false,
            models: Vec::new(),
            look: None,
            gallery: gallery::Gallery::default(),
            clock: 0,
            size: [1, 1],
            scene_unavailable: false,
            preview: lake::Preview::default(),
            world: None,
        })
    }

    pub(crate) fn t(&self, key: &str) -> String {
        self.strings.t(key)
    }

    // ---- screens ----

    fn go(&mut self, screen: Screen) {
        self.screen = screen;
        self.paused = false;
    }

    /// Escape opens and closes the pause menu in Play, and goes back to
    /// the Title from any other screen.
    fn escape(&mut self) {
        match self.screen {
            Screen::Play => self.paused = !self.paused,
            Screen::Settings | Screen::Notes | Screen::Gallery | Screen::Lake => self.go(Screen::Title),
            Screen::Title => {}
        }
    }

    /// The version in the corner opens What's New from the menus, not
    /// from Play.
    pub fn version_opens_notes(&self) -> bool {
        matches!(
            self.screen,
            Screen::Title | Screen::Settings | Screen::Notes | Screen::Gallery | Screen::Lake
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
        match name {
            "key" => self.key(value),
            "menu.play" => self.go(Screen::Play),
            "menu.settings" => self.go(Screen::Settings),
            "menu.notes" => self.go(Screen::Notes),
            "menu.gallery" => self.go(Screen::Gallery),
            "menu.lake" => self.open_lake(),
            "scene.unavailable" => self.scene_unavailable = true,
            "version" if self.version_opens_notes() => self.go(Screen::Notes),
            "version" => {}
            "menu.back" | "menu.quit" => self.go(Screen::Title),
            "menu.pause" if self.screen == Screen::Play => self.paused = true,
            "menu.pause" => {}
            "menu.resume" => self.paused = false,
            "notes.minor" => self.settings.notes_minor = value == "on",
            _ if name.starts_with("settings.") => self.settings_act(name, value)?,
            _ if name.starts_with("gallery.") => self.gallery_act(name, value)?,
            _ if name.starts_with("lake.") => self.lake_act(name, value)?,
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    /// The page's clock, in microseconds. It only goes forward.
    pub fn advance(&mut self, now: i64) {
        self.clock = self.clock.max(now);
    }

    /// The canvas's size in CSS pixels and the device's pixel ratio in
    /// hundredths; returns the drawing buffer's size.
    pub fn resize(&mut self, css_w: i32, css_h: i32, dpr_pct: i32) -> [i32; 2] {
        self.size = crate::gfx::render::buffer_size(css_w, css_h, dpr_pct);
        self.size
    }

    // ---- what the page shows ----

    /// This frame's draw data (`gfx::render::Frame`): the screen's 3D
    /// scene, or nothing on a screen without one.
    pub fn frame(&self) -> crate::gfx::render::Frame {
        match self.screen {
            Screen::Gallery => self.gallery_frame(),
            Screen::Lake => self.lake_frame(),
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
            Screen::Play => self.play_panels(),
            Screen::Gallery => self.gallery_panels(),
            Screen::Lake => self.lake_panels(),
        };
        Ui {
            screen: self.screen,
            title: self.t("title"),
            version: format!("v{VERSION}"),
            version_opens: self.version_opens_notes(),
            text_size: self.settings.text_size,
            volume: self.settings.volume,
            scene: matches!(self.screen, Screen::Gallery | Screen::Lake),
            panels,
        }
    }
}
