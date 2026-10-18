//! `data/controls.json`: which key or mouse button does which action, and
//! how the mouse turns the camera. The page sends raw key codes
//! (`KeyboardEvent.code`), mouse buttons (as `Mouse0`, `Mouse2`) and mouse
//! movement; [`Gather`] turns them into one [`SeatInput`] a tick.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::sim::input::{Actions, SeatInput};
use crate::tuning::Checker;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ControlsFile {
    #[serde(rename = "_note", default)]
    _note: String,
    bindings: BTreeMap<String, Vec<String>>,
    mouse: MouseTuning,
}

/// How the mouse turns the third-person camera.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MouseTuning {
    pub yaw_deg_per_100px: i32,
    pub pitch_deg_per_100px: i32,
    pub pitch_min_deg: i32,
    pub pitch_max_deg: i32,
    pub pitch_start_deg: i32,
}

/// The Settings screen's mouse sensitivity: a share of the controls
/// file's turn, in percent, from `MOUSE_PCT_MIN` to `MOUSE_PCT_MAX` in
/// steps of `MOUSE_PCT_STEP`.
pub const MOUSE_PCT_MIN: u32 = 25;
pub const MOUSE_PCT_MAX: u32 = 300;
pub const MOUSE_PCT_STEP: u32 = 25;
pub const MOUSE_PCT_DEFAULT: u32 = 100;

/// A sensitivity the page sent, kept inside the slider's range and on its
/// steps.
pub fn mouse_pct(pct: u32) -> u32 {
    let pct = pct.clamp(MOUSE_PCT_MIN, MOUSE_PCT_MAX);
    (pct - MOUSE_PCT_MIN) / MOUSE_PCT_STEP * MOUSE_PCT_STEP + MOUSE_PCT_MIN
}

/// A mouse movement as the reader's settings make it: scaled by the
/// sensitivity (percent).
pub fn feel(dx: i32, dy: i32, pct: u32) -> (i32, i32) {
    let scale = |d: i32| (d as i64 * pct as i64 / 100) as i32;
    (scale(dx), scale(dy))
}

/// The checked controls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Controls {
    /// key code to action name
    pub keys: BTreeMap<String, String>,
    pub mouse: MouseTuning,
}

impl Controls {
    /// The action a key or mouse button code does, if any.
    pub fn action(&self, code: &str) -> Option<&str> {
        self.keys.get(code).map(String::as_str)
    }

    /// The codes bound to an action, in file order of the codes' names.
    pub fn codes(&self, action: &str) -> Vec<&str> {
        self.keys
            .iter()
            .filter(|(_, a)| a.as_str() == action)
            .map(|(k, _)| k.as_str())
            .collect()
    }
}

/// Reads and checks the controls file: every name is an action, and no key
/// does two things.
pub fn parse(json: &str) -> Result<Controls, String> {
    let f: ControlsFile =
        serde_json::from_str(json).map_err(|e| format!("The controls file does not read: {e}."))?;
    let mut keys = BTreeMap::new();
    for (action, codes) in &f.bindings {
        if !Actions::NAMES.contains(&action.as_str()) {
            return Err(format!(
                "The controls file binds \"{action}\", which is not an action."
            ));
        }
        for code in codes {
            if let Some(other) = keys.insert(code.clone(), action.clone()) {
                return Err(format!(
                    "The controls file binds {code} to both \"{other}\" and \"{action}\"."
                ));
            }
        }
    }
    let m = &f.mouse;
    let mut c = Checker::new();
    c.range("mouse.yaw_deg_per_100px", m.yaw_deg_per_100px, -720, 720)
        .range("mouse.pitch_deg_per_100px", m.pitch_deg_per_100px, -720, 720)
        .range("mouse.pitch_min_deg", m.pitch_min_deg, -89, 89)
        .range("mouse.pitch_max_deg", m.pitch_max_deg, m.pitch_min_deg, 89)
        .range(
            "mouse.pitch_start_deg",
            m.pitch_start_deg,
            m.pitch_min_deg,
            m.pitch_max_deg,
        );
    c.done()
        .map_err(|e| e.replace("tuning value", "controls value"))?;
    Ok(Controls { keys, mouse: f.mouse })
}

/// The local player's raw input between ticks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Gather {
    held: Actions,
    pressed: Actions,
    released: Actions,
    mouse: [i32; 2],
    /// the pointer over the 2D layer, in its units, while it is there
    pointer: Option<[i32; 2]>,
}

impl Gather {
    /// The pointer moved over the 2D layer (`None`: off it).
    pub fn point(&mut self, at: Option<[i32; 2]>) {
        self.pointer = at;
    }

    /// A key or mouse button went down. Returns the action it does.
    pub fn down(&mut self, controls: &Controls, code: &str) -> Option<String> {
        let action = controls.action(code)?.to_string();
        if !self.held.get(&action) {
            self.pressed.set(&action, true);
        }
        self.held.set(&action, true);
        Some(action)
    }

    /// A key or mouse button came up.
    pub fn up(&mut self, controls: &Controls, code: &str) {
        if let Some(action) = controls.action(code) {
            if self.held.get(action) {
                self.released.set(action, true);
            }
            self.held.set(action, false);
        }
    }

    /// The actions held now.
    pub fn held(&self) -> Actions {
        self.held
    }

    /// A press with no key behind it (an on-screen button).
    pub fn tap(&mut self, action: &str) {
        self.pressed.set(action, true);
    }

    pub fn mouse(&mut self, dx: i32, dy: i32) {
        self.mouse[0] = self.mouse[0].saturating_add(dx);
        self.mouse[1] = self.mouse[1].saturating_add(dy);
    }

    /// Every held action comes up (the page lost the focus).
    pub fn release_all(&mut self) {
        self.released = self.released.or(self.held);
        self.held = Actions::default();
    }

    /// This tick's input; the edges and the mouse start again, the held
    /// actions stay held.
    pub fn take(&mut self) -> SeatInput {
        let input = SeatInput {
            held: self.held,
            pressed: self.pressed,
            released: self.released,
            mouse: self.mouse,
            pointer: self.pointer,
        };
        self.pressed = Actions::default();
        self.released = Actions::default();
        self.mouse = [0, 0];
        input
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"{
        "bindings": {"forward": ["KeyW", "ArrowUp"], "mode": ["KeyQ"]},
        "mouse": {"yaw_deg_per_100px": 25, "pitch_deg_per_100px": 18,
                  "pitch_min_deg": -10, "pitch_max_deg": 60, "pitch_start_deg": 15}
    }"#;

    #[test]
    fn keys_map_to_actions_and_a_tap_inside_a_tick_is_kept() {
        let c = parse(FILE).unwrap();
        assert_eq!(c.action("ArrowUp"), Some("forward"));
        assert_eq!(c.action("KeyZ"), None);
        let mut g = Gather::default();
        g.down(&c, "KeyQ");
        g.up(&c, "KeyQ");
        g.down(&c, "KeyW");
        let i = g.take();
        assert!(i.pressed.mode && i.released.mode && !i.held.mode);
        assert!(i.held.forward && i.pressed.forward);
        // the next tick: W still held, no new edges
        let i = g.take();
        assert!(i.held.forward && !i.pressed.forward && !i.pressed.mode);
        g.release_all();
        let i = g.take();
        assert!(!i.held.forward && i.released.forward);
    }

    #[test]
    fn sensitivity_scales_the_mouse() {
        // 200%: 10 px right and 4 px down turn as 20 and 8
        assert_eq!(feel(10, 4, 200), (20, 8));
        // 50%: 10 right, 4 down -> 5 right, 2 down
        assert_eq!(feel(10, 4, 50), (5, 2));
        assert_eq!(feel(-7, 0, 100), (-7, 0));
        // the slider's own steps: 25 to 300 by 25
        assert_eq!(mouse_pct(0), 25);
        assert_eq!(mouse_pct(130), 125);
        assert_eq!(mouse_pct(999), 300);
    }

    #[test]
    fn an_unknown_action_or_a_key_bound_twice_is_refused() {
        let bad = FILE.replace("\"mode\"", "\"jump\"");
        assert_eq!(
            parse(&bad).unwrap_err(),
            "The controls file binds \"jump\", which is not an action."
        );
        let twice = FILE.replace("\"KeyQ\"", "\"KeyW\"");
        assert!(parse(&twice).unwrap_err().contains("KeyW"));
    }
}
