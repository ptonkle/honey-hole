//! **Hooks for online play** (`crate::online`, PLAN.md decision 11). The
//! match itself does no input or output; these let an online session run
//! it as one browser's share of a shared match:
//!
//! - **Remote seats.** A seat whose boat another browser simulates (another
//!   person's, or the host's AI) is *remote* here: [`Match::step`] leaves its
//!   boat alone, and the session places it from its owner's reports with
//!   [`Match::place_remote`].
//! - **Rams judged elsewhere.** Online, a contact at ram speed is the host's
//!   to judge from the boats' reported states ([`Match::judge_rams`]); the
//!   boats pass through each other until its verdict arrives
//!   ([`Match::apply_ram`]). Slower contacts still bump here.
//! - **The end judged elsewhere.** A client's match does not end itself on
//!   time; the host's end message ends it ([`Match::end_now`]).
//! - **Claims, verdicts and world events.** A shared thing (a fish, a buoy,
//!   an item's effect on others) is decided once, by the host. A system asks
//!   with [`Match::claim`] and reads the answers in [`Hooks::verdicts`]
//!   during a later step; the host's systems tell everyone about a change
//!   with [`Match::broadcast`], read on every other browser in
//!   [`Hooks::world`]. Offline the match is its own host: a claim is judged
//!   at once, and a broadcast goes nowhere (the system already did it).
//!   `crate::online` documents how a system adds a claim.

use serde::{Deserialize, Serialize};

use super::boat::{Boat, Mode, ONE, UM};
use super::{collide, Match, MatchEvent, WreckCause};

/// Which part this browser plays in the match.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// no other browser: the match decides everything itself
    #[default]
    Offline,
    /// online, and the referee of every shared thing
    Host,
    /// online; shared things are the host's to decide
    Client,
}

/// A shared thing two players can contend for, named by kind and number.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Thing {
    pub kind: String,
    pub id: u32,
}

/// A request to the host for a shared thing (PLAN.md decision 11). Each
/// system adds its own kinds here; `crate::online` says how.
///
/// As JSON a claim is `{"kind": "hook_fish", "fish": 7}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Claim {
    /// this seat's lure hooked fish `fish` (the fishing system)
    HookFish { fish: u32 },
    /// this seat landed fish `fish`, of this length (the fishing system)
    CatchFish { fish: u32, length_mm: i32 },
    /// this seat drove through item buoy `buoy` (the items system)
    PickBuoy { buoy: u32 },
    /// this seat used its item; `effect` is the item's own data (a
    /// teleporter's target, say), read by the items system
    UseItem { item: String, effect: serde_json::Value },
    /// a claim for tests of the claim machinery: the thing `thing`
    #[cfg(test)]
    Stub { thing: u32 },
}

impl Claim {
    /// The thing this claim contends for, if two players could want it at
    /// once: the host gives a held thing to nobody else until it is
    /// released ([`Hooks::release`]).
    pub fn thing(&self) -> Option<Thing> {
        let t = |kind: &str, id: u32| {
            Some(Thing {
                kind: kind.to_string(),
                id,
            })
        };
        match self {
            Claim::HookFish { fish } => t("fish", *fish),
            // landing needs the hook first: the same thing, already held
            Claim::CatchFish { fish, .. } => t("fish", *fish),
            Claim::PickBuoy { buoy } => t("buoy", *buoy),
            Claim::UseItem { .. } => None,
            #[cfg(test)]
            Claim::Stub { thing } => t("stub", *thing),
        }
    }
}

/// The host's answer to a claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    /// the claiming seat
    pub seat: usize,
    /// the claim's number, from the claiming seat's own count
    pub id: u32,
    pub claim: Claim,
    pub granted: bool,
}

/// The host deciding a claim on its own match. The default gives a thing
/// to the first seat that claims it, until released; a system that needs
/// more (the fish is still free, the seat is near the buoy) sets its own
/// with [`Hooks::referee`].
pub type Referee = fn(&Match, usize, &Claim) -> bool;

/// The default referee: a claim without a thing is granted; a thing held
/// by another seat is refused.
pub fn first_come(m: &Match, seat: usize, claim: &Claim) -> bool {
    match claim.thing() {
        None => true,
        Some(thing) => m.online.holder(&thing).is_none_or(|s| s == seat),
    }
}

/// Something the host tells every browser, outside the boats' states.
/// Systems add their own kinds; `crate::online` says how.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorldEvent {
    /// a boat was wrecked (the host passes on a client's grounding; rams
    /// travel as their own message)
    Wrecked { seat: usize, cause: WreckCause },
    /// a seat's player left the match: its boat is gone
    Left { seat: usize },
    /// a fish got off a line and is free again (the fishing system)
    FishFreed { fish: u32 },
    /// a fish is at a new place, mm, and depth, mm (the fishing system)
    FishMoved { fish: u32, at: [i32; 2], depth: i32 },
    /// an item buoy came up at a place, mm (the items system)
    BuoySpawned { buoy: u32, at: [i32; 2] },
    /// a popup for every player: a `data/strings.json` key and its
    /// placeholders, in order
    Popup { key: String, args: Vec<[String; 2]> },
}

/// What one boat looks like to the other browsers, sent by its owner
/// about 20 times a second. Short field names keep it small on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoatState {
    #[serde(rename = "s")]
    pub seat: usize,
    /// the owner's match tick when it was so
    #[serde(rename = "k")]
    pub tick: u32,
    /// the middle of the hull, mm (east, north)
    #[serde(rename = "p")]
    pub at: [i32; 2],
    /// mm/s (east, north)
    #[serde(rename = "v")]
    pub vel: [i32; 2],
    #[serde(rename = "h")]
    pub heading: i32,
    #[serde(rename = "f", default)]
    pub fishing: bool,
    #[serde(rename = "c", default)]
    pub cast_yaw: i32,
    #[serde(rename = "w", default)]
    pub wrecked: bool,
    /// ticks of ghosting left
    #[serde(rename = "g", default)]
    pub ghost: u32,
    /// ticks of motor stall left
    #[serde(rename = "l", default)]
    pub stall: u32,
    #[serde(rename = "n", default)]
    pub nitrous: bool,
    /// **Hook for the items system** (Cloaking Tech): kept off the radar
    #[serde(rename = "x", default)]
    pub hidden: bool,
}

/// The online part of a match. Offline it stays at its default and
/// changes nothing.
#[derive(Debug, Clone, Default)]
pub struct Hooks {
    pub role: Role,
    /// by seat: another browser simulates this seat's boat
    pub remote: Vec<bool>,
    /// by seat: Cloaking Tech (the items system sets it; it travels in the
    /// boat's state)
    pub hidden: Vec<bool>,
    /// contacts at ram speed wait for the host's judgement
    pub rams_elsewhere: bool,
    /// the match does not end itself on time
    pub end_elsewhere: bool,
    /// the host's referee; `None` is [`first_come`]
    pub referee: Option<Referee>,
    /// the things held, and by which seat (the host's own record)
    pub holds: Vec<(Thing, usize)>,
    /// this step's answers to claims, for the systems to read
    pub verdicts: Vec<Verdict>,
    /// this step's world events from the host, for the systems to read
    pub world: Vec<WorldEvent>,
    /// answers and events that arrived since the last step
    pub incoming: Vec<Verdict>,
    pub world_in: Vec<WorldEvent>,
    /// claims and broadcasts made this step, for the session to send
    pub claims_out: Vec<(usize, u32, Claim)>,
    pub world_out: Vec<WorldEvent>,
    /// each seat's claim count, for numbering
    pub claim_count: Vec<u32>,
}

impl Hooks {
    /// Who holds a thing, on the host.
    pub fn holder(&self, thing: &Thing) -> Option<usize> {
        self.holds.iter().find(|(t, _)| t == thing).map(|(_, s)| *s)
    }

    /// Lets a held thing go (a fish freed, a buoy gone), on the host.
    pub fn release(&mut self, thing: &Thing) {
        self.holds.retain(|(t, _)| t != thing);
    }

    /// Whether another browser simulates this seat.
    pub fn is_remote(&self, seat: usize) -> bool {
        self.remote.get(seat).copied().unwrap_or(false)
    }

    /// Whether shared things are the host's to decide here.
    pub fn is_client(&self) -> bool {
        self.role == Role::Client
    }
}

/// A ram the host judged: `by` wrecked `victim`. When only `by` was fast
/// enough its motor stalls and it loses `toward` mm/s along `dir` (Q16).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ram {
    pub victim: usize,
    pub by: usize,
    pub stall: bool,
    pub toward: i32,
    pub dir: [i64; 2],
}

impl Match {
    /// Moves last step's arrivals into this step's views (the start of
    /// [`Match::step`]).
    pub(crate) fn take_arrivals(&mut self) {
        let h = &mut self.online;
        h.verdicts = std::mem::take(&mut h.incoming);
        h.world = std::mem::take(&mut h.world_in);
    }

    pub fn is_remote(&self, seat: usize) -> bool {
        self.online.is_remote(seat)
    }

    /// A system asks for a shared thing for `seat`. Offline it is judged
    /// at once, and the answer is in [`Hooks::verdicts`] during the next
    /// step; online the host answers a moment later, and the answer shows
    /// there the step after it arrives. Returns the claim's number.
    pub fn claim(&mut self, seat: usize, claim: Claim) -> u32 {
        let h = &mut self.online;
        if h.claim_count.len() <= seat {
            h.claim_count.resize(seat + 1, 0);
        }
        h.claim_count[seat] += 1;
        let id = h.claim_count[seat];
        if self.online.role == Role::Offline {
            let v = self.judge_claim(seat, id, claim);
            self.online.incoming.push(v);
        } else {
            // online the host judges every claim the same way, its own
            // included, a moment after it was made (`crate::online`)
            self.online.claims_out.push((seat, id, claim));
        }
        id
    }

    /// The referee's answer to one claim, holding its thing if granted.
    pub fn judge_claim(&mut self, seat: usize, id: u32, claim: Claim) -> Verdict {
        let referee = self.online.referee.unwrap_or(first_come);
        let granted = referee(self, seat, &claim);
        if granted {
            if let Some(thing) = claim.thing() {
                if self.online.holder(&thing).is_none() {
                    self.online.holds.push((thing, seat));
                }
            }
        }
        Verdict {
            seat,
            id,
            claim,
            granted,
        }
    }

    /// The host's systems tell every browser about a shared change.
    /// Offline it goes nowhere.
    pub fn broadcast(&mut self, event: WorldEvent) {
        if self.online.role == Role::Host {
            self.online.world_out.push(event);
        }
    }

    /// The state other browsers are sent for a seat this browser
    /// simulates.
    pub fn boat_state(&self, seat: usize) -> Option<BoatState> {
        let b = self.boat(seat)?;
        let left = |until: u32| until.saturating_sub(self.tick);
        Some(BoatState {
            seat,
            tick: self.tick,
            at: [b.at().x, b.at().y],
            vel: b.vel(),
            heading: b.heading,
            fishing: b.mode == Mode::Fishing,
            cast_yaw: b.cast_yaw,
            wrecked: b.wrecked(),
            ghost: left(b.ghost_until),
            stall: left(b.stall_until),
            nitrous: b.nitrous,
            hidden: self.online.hidden.get(seat).copied().unwrap_or(false),
        })
    }

    /// Puts a remote seat's boat where its owner's state says, from where
    /// it was (for drawing between ticks). A mode change shows as an event.
    pub fn place_remote(&mut self, s: &BoatState) {
        let tick = self.tick;
        let Some(b) = self.boats.get_mut(s.seat).and_then(Option::as_mut) else {
            return;
        };
        b.remember();
        let mode = if s.fishing { Mode::Fishing } else { Mode::Driving };
        let changed = b.mode != mode;
        set_from_state(b, s, tick);
        if changed {
            self.events.push(MatchEvent::ModeChanged { seat: s.seat, mode });
        }
        if self.online.hidden.len() <= s.seat {
            self.online.hidden.resize(s.seat + 1, false);
        }
        self.online.hidden[s.seat] = s.hidden;
    }

    /// A boat as a state says it is, for the host's judging.
    pub fn boat_from_state(&self, s: &BoatState) -> Boat {
        let mut b = Boat::new(s.seat, crate::map::P::new(s.at[0], s.at[1]), s.heading);
        set_from_state(&mut b, s, self.tick);
        b
    }

    /// The host judges rams between these boats (by seat; this browser's
    /// own from its match, the others' from their latest states), with the
    /// match's own rules (`wreck` in the tuning).
    pub fn judge_rams(&self, boats: &[Option<Boat>]) -> Vec<Ram> {
        let mut rams = Vec::new();
        let r = &self.rules;
        for i in 0..boats.len() {
            for j in i + 1..boats.len() {
                let (Some(a), Some(b)) = (boats[i].as_ref(), boats[j].as_ref()) else {
                    continue;
                };
                let Some(c) = collide::contact(a, b, r, self.tick) else {
                    continue;
                };
                let (a_rams, b_rams) = (c.a_toward >= r.ram_wreck, c.b_toward >= r.ram_wreck);
                if a_rams {
                    rams.push(Ram {
                        victim: j,
                        by: i,
                        stall: !b_rams,
                        toward: c.a_toward,
                        dir: c.d,
                    });
                }
                if b_rams {
                    rams.push(Ram {
                        victim: i,
                        by: j,
                        stall: !a_rams,
                        toward: c.b_toward,
                        dir: [-c.d[0], -c.d[1]],
                    });
                }
            }
        }
        rams
    }

    /// A ram the host judged, on this browser: a victim this browser
    /// simulates is wrecked, a rammer it simulates stalls, and a remote
    /// seat's part shows as its events (for popups and sounds).
    pub fn apply_ram(&mut self, ram: &Ram) {
        let tick = self.tick;
        let cause = WreckCause::Rammed { by: ram.by };
        if self.is_remote(ram.victim) {
            self.events.push(MatchEvent::Wrecked {
                seat: ram.victim,
                cause,
            });
        } else if self.boat(ram.victim).is_some_and(|b| !b.wrecked()) {
            self.wreck(ram.victim, cause, tick);
        }
        if !ram.stall {
            return;
        }
        if !self.is_remote(ram.by) {
            let stall = tick + self.rules.stall_ticks;
            if let Some(b) = self.boats.get_mut(ram.by).and_then(Option::as_mut) {
                b.stall_until = stall;
                let mut v = b.vel();
                v[0] -= (ram.toward as i64 * ram.dir[0] / ONE as i64) as i32;
                v[1] -= (ram.toward as i64 * ram.dir[1] / ONE as i64) as i32;
                b.set_vel(v);
            }
        }
        self.events.push(MatchEvent::Stalled { seat: ram.by });
    }

    /// A world event about the boats, on this browser; the rest go to the
    /// systems through [`Hooks::world`] next step.
    pub fn apply_world(&mut self, e: &WorldEvent) {
        match e {
            WorldEvent::Wrecked { seat, cause } => self.events.push(MatchEvent::Wrecked {
                seat: *seat,
                cause: *cause,
            }),
            WorldEvent::Left { seat } => {
                if let Some(b) = self.boats.get_mut(*seat) {
                    *b = None;
                }
            }
            _ => {}
        }
        self.online.world_in.push(e.clone());
    }

    /// Ends the match now (the host's end message).
    pub fn end_now(&mut self) {
        if !self.ended {
            self.ended = true;
            self.events.push(MatchEvent::MatchEnded);
        }
    }
}

fn set_from_state(b: &mut Boat, s: &BoatState, tick: u32) {
    b.pos_um = [s.at[0] as i64 * UM, s.at[1] as i64 * UM];
    b.set_vel(s.vel);
    b.heading = s.heading;
    b.mode = if s.fishing { Mode::Fishing } else { Mode::Driving };
    b.cast_yaw = s.cast_yaw;
    // a remote boat respawns when its owner says, not on a tick of ours
    b.wrecked_until = s.wrecked.then_some(u32::MAX);
    b.ghost_until = tick + s.ghost;
    b.stall_until = tick + s.stall;
    b.nitrous = s.nitrous;
}
