//! The door into the core. The shim holds one `Session` and forwards its
//! calls; each returns a JSON string, `{"ok": ...}` on success or
//! `{"error": "..."}` with a message the page can show as it is.
//!
//! The core describes the screen as a view model (`app::Ui`): the screen's
//! name, its panels and their items. The page builds the DOM from that
//! description without knowing what a panel means. Buttons, settings, keys
//! and screens all come back through one function, `act`, with a dotted
//! name such as `menu.settings`, so a new option needs a new `match` arm in
//! `app.rs` and no new shim function. (After Peter's Mandible.)

use serde::Serialize;

use crate::app::App;

fn ok<T: Serialize>(value: &T) -> String {
    serde_json::json!({ "ok": value }).to_string()
}

fn error(message: &str) -> String {
    serde_json::json!({ "error": message }).to_string()
}

/// The app for one page, or why it could not start.
pub struct Session {
    app: Result<App, String>,
}

impl Session {
    /// Checks the strings file and the patch notes and opens on the Title.
    pub fn new(strings_json: &str, notes_json: &str) -> Session {
        Session {
            app: App::new(strings_json, notes_json),
        }
    }

    /// What the page draws now.
    pub fn ui(&self) -> String {
        match &self.app {
            Ok(app) => ok(&app.ui()),
            Err(e) => error(e),
        }
    }

    /// Applies one named action and returns what the page draws next. A
    /// refused action leaves the app as it was.
    pub fn act(&mut self, name: &str, value: &str) -> String {
        match &mut self.app {
            Ok(app) => match app.act(name, value) {
                Ok(()) => ok(&app.ui()),
                Err(e) => error(&e),
            },
            Err(e) => error(e),
        }
    }

    /// Takes the model files (one `{id: model}` object) and the look file,
    /// and returns what the page draws, or why they were refused.
    pub fn load_models(&mut self, models_json: &str, look_json: &str) -> String {
        match &mut self.app {
            Ok(app) => match app.load_models(models_json, look_json) {
                Ok(()) => ok(&app.ui()),
                Err(e) => error(&e),
            },
            Err(e) => error(e),
        }
    }

    /// Takes the sound recipes (`data/sounds.json`) and the tuning file
    /// (`data/tuning.json`, its audio section), and returns what the page
    /// draws, or why they were refused.
    pub fn load_sounds(&mut self, sounds_json: &str, tuning_json: &str) -> String {
        match &mut self.app {
            Ok(app) => match app.load_sounds(sounds_json, tuning_json) {
                Ok(()) => ok(&app.ui()),
                Err(e) => error(&e),
            },
            Err(e) => error(e),
        }
    }

    /// What the page's synth plays now (`sound::Sound::frame`); the cues
    /// in it are taken, so each plays once.
    pub fn sound(&mut self) -> String {
        match &mut self.app {
            Ok(app) => ok(&app.sound_frame()),
            Err(e) => error(e),
        }
    }

    /// The noise buffer the synth's noise layers play (Q16.16).
    pub fn sound_noise(&self) -> Vec<i32> {
        match &self.app {
            Ok(app) => app.sound_noise(),
            Err(_) => Vec::new(),
        }
    }

    /// The page's clock, in microseconds.
    pub fn advance(&mut self, now: i64) {
        if let Ok(app) = &mut self.app {
            app.advance(now);
        }
    }

    /// The canvas's CSS size and pixel ratio (hundredths) in; the drawing
    /// buffer's size out.
    pub fn resize(&mut self, css_w: i32, css_h: i32, dpr_pct: i32) -> Vec<i32> {
        match &mut self.app {
            Ok(app) => app.resize(css_w, css_h, dpr_pct).to_vec(),
            Err(_) => vec![1, 1],
        }
    }

    /// This frame's draw data (`gfx::render::Frame`).
    pub fn frame(&self) -> crate::gfx::render::Frame {
        match &self.app {
            Ok(app) => app.frame(),
            Err(_) => Default::default(),
        }
    }

    /// The picture-in-picture (`App::inset_frame`): its rectangle, its
    /// border's, and its draw data; empty rectangles when there is none.
    pub fn inset_frame(&self) -> (Vec<i32>, Vec<i32>, crate::gfx::render::Frame) {
        match &self.app {
            Ok(app) => app.inset_frame(),
            Err(_) => Default::default(),
        }
    }

    /// The static world's stamp: 0 for none; a new one means upload
    /// [`Session::world_vertices`] again.
    pub fn world_stamp(&self) -> u32 {
        self.app.as_ref().map_or(0, |a| a.world_stamp())
    }

    /// The static world's vertices (`scene::Lake::vertices`).
    pub fn world_vertices(&self) -> &[i32] {
        match &self.app {
            Ok(app) => app.world.as_ref().map_or(&[], |w| &w.vertices),
            Err(_) => &[],
        }
    }

    /// Every number the renderer needs (`gfx::render::numbers_json`).
    pub fn numbers(&self) -> String {
        crate::gfx::render::numbers_json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTES: &str = r#"[{"version": "0.1.0", "date": "2026-10-17", "notes": ["A."]}]"#;

    fn json(text: &str) -> serde_json::Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn a_session_opens_on_the_title_and_acts_through_the_one_door() {
        let mut s = Session::new(&crate::strings::tests::complete(), NOTES);
        assert_eq!(json(&s.ui())["ok"]["screen"], "title");
        let v = json(&s.act("menu.settings", ""));
        assert_eq!(v["ok"]["screen"], "settings");
    }

    #[test]
    fn a_refused_action_comes_back_as_an_error() {
        let mut s = Session::new(&crate::strings::tests::complete(), NOTES);
        assert_eq!(
            s.act("screen.jump", ""),
            r#"{"error":"Unknown action \"screen.jump\"."}"#
        );
        assert_eq!(json(&s.ui())["ok"]["screen"], "title");
    }

    #[test]
    fn a_bad_strings_file_or_bad_notes_is_wrapped_in_error() {
        let s = Session::new(r#"{"title": "T"}"#, NOTES);
        assert_eq!(s.ui(), r#"{"error":"The strings file has no \"menu.play\"."}"#);
        let s = Session::new(&crate::strings::tests::complete(), "[]");
        assert_eq!(s.ui(), r#"{"error":"The patch notes list no version."}"#);
    }
}
