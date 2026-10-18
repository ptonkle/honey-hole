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
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(strings_json: &str, notes_json: &str) -> Game {
        Game {
            s: tool_core::api::Session::new(strings_json, notes_json),
            frame: Default::default(),
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

    pub fn load_maps(&mut self, list_json: &str) -> String {
        self.s.load_maps(list_json)
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
        self.frame = self.s.frame()
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

    /// 0 for no static world; a new stamp means upload `world_vertices`.
    pub fn world_stamp(&self) -> u32 {
        self.s.world_stamp()
    }

    pub fn world_vertices(&self) -> Vec<f32> {
        floats(self.s.world_vertices())
    }
}
