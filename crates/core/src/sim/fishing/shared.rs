//! Fishing shared online (`crate::online`, "Adding a claim"): the worked
//! example of claims and world events. Offline none of this runs: the
//! match decides everything itself, as before.
//!
//! - **The fish in the lake are the host's.** Every browser runs the same
//!   lake from the same seed, but each sees only its own lines, so the
//!   fish drift apart. The host sends where each free fish is every
//!   [`FISH_SYNC_TICKS`], all in one message (`WorldEvent::FishPlaces`),
//!   and the other browsers
//!   put them there, except a fish their own line is busy with.
//! - **A hook is a claim.** The local line hooks at once (the 0.75 s
//!   window allows no round trip) and claims the fish
//!   (`Claim::HookFish`). Granted, the host holds the fish for that seat
//!   and every other browser shows it hooked (and raises `Hooked`, for the
//!   popup); a line elsewhere that was after the same fish loses it.
//!   Refused (another seat's claim came first), the local line has already
//!   let it go on hearing the granted one (verdicts come in the order judged).
//! - **A catch is a claim** (`Claim::CatchFish`). Granted, every browser
//!   marks the fish caught and adds the catch to that seat's score, which
//!   the postgame reads.
//! - **A fish that gets away** (the line broke, Esc, Q, a wreck) is set
//!   free: the host releases its hold and tells everyone
//!   (`WorldEvent::FishFreed`); a joiner reports it to the host first.

use super::lake::FishState;
use super::{value, Line};
use crate::sim::remote::{Claim, Role, Thing, WorldEvent};
use crate::sim::{Catch, Match, MatchEvent};

/// How often the host sends the free fish's places: once a second.
pub const FISH_SYNC_TICKS: u32 = 50;

fn fish_thing(fish: u32) -> Thing {
    Thing {
        kind: "fish".into(),
        id: fish,
    }
}

impl Match {
    /// Whether a line this browser plays is busy with this fish (coming to
    /// its lure, or on it).
    fn engaged_here(&self, fish: usize) -> bool {
        self.fishing.lines.iter().enumerate().any(|(seat, l)| {
            !self.is_remote(seat) && (l.fish == Some(fish) || l.interest.is_some_and(|i| i.fish == fish))
        })
    }

    /// Before the lines' tick: the host's verdicts and world events about
    /// fish.
    pub(crate) fn fishing_shared_before(&mut self) {
        if self.online.role == Role::Offline {
            return;
        }
        for v in self.online.verdicts.clone() {
            let local = !self.is_remote(v.seat);
            match v.claim {
                Claim::HookFish { fish } => {
                    let f = fish as usize;
                    if f >= self.fishing.fish.len() {
                        continue;
                    }
                    if v.granted && self.online.role == Role::Host {
                        // the popup, at the host's delay from now
                        let lf = &self.fishing.fish[f];
                        let kind = self.fishing.roster.0.get(lf.kind).map_or(String::new(), |k| k.id.clone());
                        let tier = lf.tier;
                        let tick = self.tick;
                        self.fishing.announcer.queue(tick, v.seat, fish, kind, tier);
                    }
                    if v.granted && !local {
                        // another seat has it: any line here after it lets go
                        for seat in 0..self.fishing.lines.len() {
                            if self.is_remote(seat) {
                                continue;
                            }
                            let l = &self.fishing.lines[seat];
                            if l.fish == Some(f) {
                                self.fishing.lines[seat] = Line::idle();
                                self.events.push(MatchEvent::FishDropped { seat, fish });
                            } else if l.interest.is_some_and(|i| i.fish == f) {
                                self.fishing.lines[seat].interest = None;
                                self.events.push(MatchEvent::Missed { seat });
                            }
                        }
                        let lf = &mut self.fishing.fish[f];
                        lf.state = FishState::Hooked { seat: v.seat };
                        let kind = self
                            .fishing
                            .roster
                            .0
                            .get(lf.kind)
                            .map_or(String::new(), |k| k.id.clone());
                        self.events.push(MatchEvent::Hooked {
                            seat: v.seat,
                            fish,
                            kind,
                            tier: lf.tier,
                        });
                    }
                    // a refused hook here needs nothing more: the granted
                    // claim it lost to, heard first, let this line's fish go
                }
                Claim::CatchFish { fish, length_mm } => {
                    let f = fish as usize;
                    if !v.granted || local || f >= self.fishing.fish.len() {
                        continue;
                    }
                    let lf = &mut self.fishing.fish[f];
                    lf.state = FishState::Caught { seat: v.seat };
                    let (kind, tier) = (lf.kind, lf.tier);
                    let Some(k) = self.fishing.roster.0.get(kind).cloned() else {
                        continue;
                    };
                    let worth = value(
                        k.value,
                        k.base_length_mm,
                        length_mm,
                        self.fishing.tuning.tier_value_pct.get(tier),
                    );
                    if let Some(s) = self.scores.get_mut(v.seat) {
                        s.catches.push(Catch {
                            kind: k.id.clone(),
                            length_mm,
                            value: worth,
                        });
                    }
                    self.events.push(MatchEvent::Caught {
                        seat: v.seat,
                        fish,
                        kind: k.id,
                        tier,
                        length_mm,
                    });
                }
                _ => {}
            }
        }
        for e in self.online.world.clone() {
            match e {
                WorldEvent::FishFreed { fish } => {
                    let f = fish as usize;
                    if self.online.role == Role::Host {
                        self.online.release(&fish_thing(fish));
                        // got away before its popup: none
                        self.fishing.announcer.waiting.retain(|w| w.fish != fish);
                    }
                    if f < self.fishing.fish.len() && !self.engaged_here(f) {
                        let lf = &mut self.fishing.fish[f];
                        if matches!(lf.state, FishState::Hooked { .. }) {
                            lf.state = FishState::Roaming;
                        }
                    }
                }
                WorldEvent::FishPlaces { fish } => {
                    for [id, x, y, depth] in fish {
                        let f = id.max(0) as usize;
                        if f < self.fishing.fish.len() && !self.engaged_here(f) {
                            let lf = &mut self.fishing.fish[f];
                            if matches!(lf.state, FishState::Roaming | FishState::Fleeing { .. }) {
                                lf.at = crate::map::P::new(x, y);
                                lf.depth_mm = depth;
                            }
                        }
                    }
                }
                WorldEvent::Announced {
                    seat,
                    fish,
                    fish_kind: kind,
                    tier,
                    delay_ms,
                } if self.online.role == Role::Client => {
                    self.events.push(MatchEvent::Announced {
                        seat,
                        fish,
                        kind,
                        tier,
                        delay_ms,
                    });
                }
                _ => {}
            }
        }
    }

    /// The host's hook popups due this tick go to every browser (after the
    /// announcer's step, last in the tick).
    pub(crate) fn share_announced(&mut self) {
        if self.online.role != Role::Host {
            return;
        }
        let due: Vec<WorldEvent> = self
            .events
            .iter()
            .filter_map(|e| match e {
                MatchEvent::Announced {
                    seat,
                    fish,
                    kind,
                    tier,
                    delay_ms,
                } => Some(WorldEvent::Announced {
                    seat: *seat,
                    fish: *fish,
                    fish_kind: kind.clone(),
                    tier: *tier,
                    delay_ms: *delay_ms,
                }),
                _ => None,
            })
            .collect();
        for e in due {
            self.broadcast(e);
        }
    }

    /// After the lines' tick (`from` is where its events start): this
    /// browser's hooks and catches go to the host as claims, its freed fish
    /// as reports, and the host sends the free fish's places.
    pub(crate) fn fishing_shared_after(&mut self, from: usize) {
        if self.online.role == Role::Offline {
            return;
        }
        let mine: Vec<MatchEvent> = self.events[from..].to_vec();
        for e in mine {
            match e {
                MatchEvent::Hooked { seat, fish, .. } if !self.is_remote(seat) => {
                    self.claim(seat, Claim::HookFish { fish });
                }
                MatchEvent::Caught {
                    seat,
                    fish,
                    length_mm,
                    ..
                } if !self.is_remote(seat) => {
                    self.claim(seat, Claim::CatchFish { fish, length_mm });
                }
                _ => {}
            }
        }
        for (seat, fish) in std::mem::take(&mut self.fishing.freed) {
            if self.is_remote(seat) {
                continue;
            }
            let event = WorldEvent::FishFreed { fish };
            if self.online.role == Role::Host {
                self.online.release(&fish_thing(fish));
                self.broadcast(event);
            } else {
                self.online.reports_out.push(event);
            }
        }
        if self.online.role == Role::Host && self.tick.is_multiple_of(FISH_SYNC_TICKS) {
            let fish: Vec<[i32; 4]> = self
                .fishing
                .fish
                .iter()
                .filter(|f| matches!(f.state, FishState::Roaming | FishState::Fleeing { .. }))
                .map(|f| [f.id as i32, f.at.x, f.at.y, f.depth_mm])
                .collect();
            if !fish.is_empty() {
                self.broadcast(WorldEvent::FishPlaces { fish });
            }
        }
    }
}
