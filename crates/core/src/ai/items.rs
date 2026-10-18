//! **Hook for the items system.** Items (the buoy, Shift and the six
//! minigames) are being built alongside the AI. Once they are in the match,
//! this is where an AI seat picks up a buoy on its way (only when the
//! detour is short), uses its item and plays its minigame, all through the
//! same [`SeatInput`] a person makes. Until then it changes nothing.

use crate::sim::input::SeatInput;

use super::senses::Senses;

/// Adds this tick's item actions to `out`. Nothing yet: the match has no
/// items.
pub fn decide(s: &Senses, out: &mut SeatInput) {
    let _ = (s, out);
}
