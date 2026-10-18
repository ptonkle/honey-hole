//! What one seat does in one tick (PLAN.md, contract 4). People and AI
//! players make the same `SeatInput`: the app builds it from the page's raw
//! keys and mouse through `data/controls.json` (`controls.rs`), the AI
//! builds it from what it sees.
//!
//! Every action has three views: `held` (down for this tick), `pressed`
//! (went down since the last tick) and `released` (came up since the last
//! tick). A press and release inside one tick shows in both edges, so a quick
//! tap is never lost.

use serde::{Deserialize, Serialize};

/// One flag per action. Field names are the action names in
/// `data/controls.json`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Actions {
    /// W: throttle in driving mode
    pub forward: bool,
    /// S: brake, then reverse
    pub back: bool,
    /// A: turn left, or swivel the cast left in fishing mode
    pub left: bool,
    /// D: turn right, or swivel right
    pub right: bool,
    /// Q: switch between driving and fishing mode
    pub mode: bool,
    /// Tab: raise or lower the Fish-Boy (its layer, later)
    pub fishboy: bool,
    /// Esc: cancel the current thing; with nothing to cancel, the in-match
    /// menu ([`super::Match::cancel`])
    pub cancel: bool,
    /// Shift: use the item (items, later)
    pub item: bool,
    /// Space: cast, stop the bobber, hook, reel (fishing, later)
    pub cast: bool,
    /// the main mouse button: minigames and the radar (later)
    pub click: bool,
    /// the other mouse button
    pub alt_click: bool,
    /// opens or closes the in-match menu directly (its on-screen button;
    /// no key is bound to it)
    pub menu: bool,
    /// a radar ping, with the Fish-Boy up (the app taps it for Space on
    /// the Radar tab or a click on the ping; no key is bound to it)
    pub ping: bool,
}

impl Actions {
    /// Every action name, in field order.
    pub const NAMES: [&'static str; 13] = [
        "forward",
        "back",
        "left",
        "right",
        "mode",
        "fishboy",
        "cancel",
        "item",
        "cast",
        "click",
        "alt_click",
        "menu",
        "ping",
    ];

    fn slot(&mut self, name: &str) -> Option<&mut bool> {
        Some(match name {
            "forward" => &mut self.forward,
            "back" => &mut self.back,
            "left" => &mut self.left,
            "right" => &mut self.right,
            "mode" => &mut self.mode,
            "fishboy" => &mut self.fishboy,
            "cancel" => &mut self.cancel,
            "item" => &mut self.item,
            "cast" => &mut self.cast,
            "click" => &mut self.click,
            "alt_click" => &mut self.alt_click,
            "menu" => &mut self.menu,
            "ping" => &mut self.ping,
            _ => return None,
        })
    }

    /// Sets an action by name; false for a name that is not an action.
    pub fn set(&mut self, name: &str, on: bool) -> bool {
        match self.slot(name) {
            Some(s) => {
                *s = on;
                true
            }
            None => false,
        }
    }

    /// An action by name; false for a name that is not an action.
    pub fn get(&self, name: &str) -> bool {
        let mut copy = *self;
        copy.slot(name).is_some_and(|s| *s)
    }

    /// Both sets' flags together.
    pub fn or(self, o: Actions) -> Actions {
        let mut out = self;
        for n in Actions::NAMES {
            out.set(n, self.get(n) || o.get(n));
        }
        out
    }
}

/// One seat's input for one tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SeatInput {
    pub held: Actions,
    pub pressed: Actions,
    pub released: Actions,
    /// the mouse's movement since the last tick, in CSS pixels (x right,
    /// y down). The app turns the camera with it; the underwater camera and
    /// minigames will read it here.
    pub mouse: [i32; 2],
    /// where the pointer is in the current 2D layer (minigames, the radar),
    /// in that layer's own units, which the 2D layer defines; `None` while
    /// no layer takes the pointer
    pub pointer: Option<[i32; 2]>,
    /// where the seat's camera looks, turned from the bow (65536 a turn,
    /// left positive): in fishing mode a cast with the swivel centred goes
    /// this way (SECOND-ORDER R7.74). The AI leaves it 0, the bow.
    pub look: i32,
}

impl SeatInput {
    /// Held this tick or pressed since the last: a tap shorter than a tick
    /// still counts as held for one tick.
    pub fn down(&self, name: &str) -> bool {
        self.held.get(name) || self.pressed.get(name)
    }
}
