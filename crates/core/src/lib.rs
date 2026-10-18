//! tool_core: every application rule of the game.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that, along with the dependency
//! allowlist and the reference ban on floats, hash maps and clocks.
//!
//! `api.rs` is the one door the shim calls through; `app.rs` holds the
//! screens and settings, `view.rs` the panels and items the page draws.

pub mod api;
pub mod app;
pub mod glossary;
pub mod map;
pub mod patch_notes;
pub mod strings;
pub mod view;
