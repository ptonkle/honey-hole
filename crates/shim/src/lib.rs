//! tool_shim: forwards strings between the page and `tool_core::api`.
//!
//! Builder requirement: the shim decides nothing. Each method is one call
//! into the core and a conversion of its arguments or result. The core
//! speaks Q16.16 integers; WebGL wants `f32`, so the frame's values are
//! divided by the core's own `ONE` on the way out. The page's float clock
//! (milliseconds) becomes whole microseconds, and the pixel ratio whole
//! hundredths. `crates/core/tests/boundary.rs` fails if a branch, a loop or
//! the `?` operator appears here; if a feature seems to need one, the rule
//! belongs in the core.

use tool_core::gfx::fx;
use wasm_bindgen::prelude::*;

const ONE: f32 = fx::ONE as f32;

fn floats(v: &[i32]) -> Vec<f32> {
    v.iter().map(|x| *x as f32 / ONE).collect()
}

/// The page's one session with the core.
#[wasm_bindgen]
pub struct Game {
    s: tool_core::api::Session,
    frame: tool_core::gfx::render::Frame,
    /// the picture-in-picture's rectangle, its border's, and its frame
    inset: (Vec<i32>, Vec<i32>, tool_core::gfx::render::Frame),
    ui2d: tool_core::ui2d::Built,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(strings_json: &str, notes_json: &str) -> Game {
        Game {
            s: tool_core::api::Session::new(strings_json, notes_json),
            frame: Default::default(),
            inset: Default::default(),
            ui2d: Default::default(),
        }
    }

    pub fn ui(&self) -> String {
        self.s.ui()
    }

    pub fn act(&mut self, name: &str, value: &str) -> String {
        self.s.act(name, value)
    }

    pub fn load_models(&mut self, models_json: &str, look_json: &str) -> String {
        self.s.load_models(models_json, look_json)
    }

    pub fn load_sounds(&mut self, sounds_json: &str, tuning_json: &str) -> String {
        self.s.load_sounds(sounds_json, tuning_json)
    }

    /// This frame's cues and loops for the synth (`web/audio.js`).
    pub fn sound(&mut self) -> String {
        self.s.sound()
    }

    /// The synth's noise samples, -1 to 1.
    pub fn sound_noise(&self) -> Vec<f32> {
        floats(&self.s.sound_noise())
    }

    pub fn numbers(&self) -> String {
        self.s.numbers()
    }

    pub fn advance(&mut self, now_ms: f64) {
        self.s.advance((now_ms * 1000.0) as i64)
    }

    /// CSS size and device pixel ratio in; drawing-buffer size out.
    pub fn resize(&mut self, css_w: f64, css_h: f64, dpr: f64) -> Vec<i32> {
        self.s.resize(css_w as i32, css_h as i32, (dpr * 100.0) as i32)
    }

    /// Builds this frame; read it with `view`, `vertices`, `vertex_count`,
    /// `statics`, `groups` and `instances`.
    pub fn build_frame(&mut self) {
        self.frame = self.s.frame();
        self.inset = self.s.inset_frame();
    }

    /// The picture-in-picture's rectangle in drawing-buffer pixels from the
    /// bottom left (x, y, width, height), empty when there is none; its
    /// border's; and its draw data, read as the main frame's is.
    pub fn inset_rect(&self) -> Vec<i32> {
        self.inset.0.clone()
    }

    pub fn inset_border(&self) -> Vec<i32> {
        self.inset.1.clone()
    }

    pub fn inset_view(&self) -> Vec<f32> {
        floats(&self.inset.2.view)
    }

    pub fn inset_vertices(&self) -> Vec<f32> {
        floats(&self.inset.2.tris)
    }

    pub fn inset_vertex_count(&self) -> u32 {
        tool_core::gfx::render::vertex_count(&self.inset.2.tris)
    }

    pub fn inset_statics(&self) -> Vec<i32> {
        self.inset.2.statics.to_vec()
    }

    pub fn inset_groups(&self) -> Vec<i32> {
        self.inset.2.groups.clone()
    }

    pub fn inset_instances(&self) -> Vec<f32> {
        floats(&self.inset.2.instances)
    }

    pub fn view(&self) -> Vec<f32> {
        floats(&self.frame.view)
    }

    pub fn vertices(&self) -> Vec<f32> {
        floats(&self.frame.tris)
    }

    pub fn vertex_count(&self) -> u32 {
        tool_core::gfx::render::vertex_count(&self.frame.tris)
    }

    /// The static world's terrain and water ranges (first vertex, count).
    pub fn statics(&self) -> Vec<i32> {
        self.frame.statics.to_vec()
    }

    /// The instanced groups, `group` values each (`numbers`).
    pub fn groups(&self) -> Vec<i32> {
        self.frame.groups.clone()
    }

    pub fn instances(&self) -> Vec<f32> {
        floats(&self.frame.instances)
    }

    /// The see-through triangles (a ghosted boat) and their view values.
    pub fn ghost_vertices(&self) -> Vec<f32> {
        floats(&self.frame.ghost)
    }

    pub fn ghost_vertex_count(&self) -> u32 {
        tool_core::gfx::render::vertex_count(&self.frame.ghost)
    }

    pub fn ghost_view(&self) -> Vec<f32> {
        floats(&self.frame.ghost_view)
    }

    /// 0 for no static world; a new stamp means upload `world_vertices`.
    pub fn world_stamp(&self) -> u32 {
        self.s.world_stamp()
    }

    pub fn world_vertices(&self) -> Vec<f32> {
        floats(self.s.world_vertices())
    }

    /// Builds this frame's 2D list and passes; read them with the
    /// `ui2d_*` and `pass_*` methods.
    pub fn build_ui2d(&mut self) {
        self.ui2d = self.s.build_ui2d()
    }

    pub fn ui2d_numbers(&self) -> String {
        self.s.ui2d_numbers()
    }

    pub fn ui2d_values(&self) -> Vec<f32> {
        floats(&self.ui2d.list.nums)
    }

    /// The list's strings and status, as JSON.
    pub fn ui2d_strings(&self) -> String {
        self.ui2d.list.strings_json()
    }

    pub fn pass_count(&self) -> u32 {
        self.ui2d.pass_count()
    }

    pub fn pass_values(&self, k: u32) -> Vec<f32> {
        floats(&self.ui2d.pass_values(k))
    }

    pub fn pass_vertices(&self, k: u32) -> Vec<f32> {
        floats(&self.ui2d.pass_vertices(k))
    }

    pub fn pass_vertex_count(&self, k: u32) -> u32 {
        self.ui2d.pass_vertex_count(k)
    }

    pub fn bitmap_rgba(&self, id: &str) -> Vec<u8> {
        self.s.bitmap_rgba(id)
    }

    pub fn bitmap_size(&self, id: &str) -> Vec<i32> {
        self.s.bitmap_size(id)
    }
}
