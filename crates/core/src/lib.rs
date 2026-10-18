//! tool_core: every application rule of the game.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that, along with the dependency
//! allowlist and the reference ban on floats, hash maps and clocks.
//!
//! `api.rs` is the one door the shim calls through; `app.rs` holds the
//! screens and settings, `view.rs` the panels and items the page draws,
//! `model.rs` the model files and `gfx` what the 3D renderer reads.
//! `sim` is the match (boats, the timer, events), `tuning.rs` its numbers
//! from `data/tuning.json`, `controls.rs` the keys from `data/controls.json`.

pub mod ai;
pub mod api;
pub mod app;
pub mod controls;
pub mod fishboy;
pub mod flow;
pub mod gfx;
pub mod glossary;
pub mod map;
pub mod minigame;
pub mod model;
pub mod online;
pub mod patch_notes;
pub mod roster;
pub mod scene;
pub mod sim;
pub mod sound;
pub mod strings;
pub mod tuning;
pub mod ui2d;
pub mod view;
