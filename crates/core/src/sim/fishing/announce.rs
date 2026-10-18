//! The hook popup's announcement: `MatchEvent::Announced`, raised
//! `balance.announce_delay_ms` (by tier) after a `Hooked`. The popup, its
//! sound for the other players and the AI's hunt all read `Announced`, not
//! `Hooked`, so a delay hides a hook from everyone at once.
//!
//! - With no delay (as shipped) the announcement follows its `Hooked` in the
//!   same tick, straight after it in the event list: the game is as it was.
//! - A fish landed or lost before its time (`Caught`, `LineBroke`,
//!   `FishDropped` for the same seat and fish) is never announced; the
//!   landing has its own popup.
//! - It works on the tick's events only, so a `Hooked` raised by any system
//!   is announced.

use serde::Serialize;

use super::tuning::PerTier;
use crate::map::fish::Tier;
use crate::sim::{ticks, MatchEvent};

/// A hook waiting for its announcement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Waiting {
    pub due: u32,
    pub seat: usize,
    pub fish: u32,
    pub kind: String,
    pub tier: Tier,
    pub delay_ms: i32,
}

/// The hooks not yet announced, and the delays by tier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Announcer {
    pub delay_ms: PerTier,
    pub waiting: Vec<Waiting>,
}

impl Announcer {
    pub fn new(delay_ms: PerTier) -> Announcer {
        Announcer {
            delay_ms,
            waiting: Vec::new(),
        }
    }

    /// After a tick's fishing: drops the waiting hooks whose fish came off
    /// or in, queues this tick's hooks, and adds the announcements due.
    pub fn step(&mut self, tick: u32, events: &mut Vec<MatchEvent>) {
        let mut out = Vec::with_capacity(events.len() + 1);
        for e in events.drain(..) {
            match &e {
                MatchEvent::Caught { seat, fish, .. }
                | MatchEvent::LineBroke { seat, fish }
                | MatchEvent::FishDropped { seat, fish } => {
                    self.waiting.retain(|w| !(w.seat == *seat && w.fish == *fish));
                    out.push(e);
                }
                MatchEvent::Hooked {
                    seat,
                    fish,
                    kind,
                    tier,
                } => {
                    let delay_ms = self.delay_ms.get(*tier);
                    let w = Waiting {
                        due: tick + ticks(delay_ms),
                        seat: *seat,
                        fish: *fish,
                        kind: kind.clone(),
                        tier: *tier,
                        delay_ms,
                    };
                    out.push(e);
                    if w.due <= tick {
                        out.push(announced(w));
                    } else {
                        self.waiting.push(w);
                    }
                }
                _ => out.push(e),
            }
        }
        let (due, wait): (Vec<Waiting>, Vec<Waiting>) = std::mem::take(&mut self.waiting)
            .into_iter()
            .partition(|w| w.due <= tick);
        self.waiting = wait;
        out.extend(due.into_iter().map(announced));
        *events = out;
    }
}

fn announced(w: Waiting) -> MatchEvent {
    MatchEvent::Announced {
        seat: w.seat,
        fish: w.fish,
        kind: w.kind,
        tier: w.tier,
        delay_ms: w.delay_ms,
    }
}
