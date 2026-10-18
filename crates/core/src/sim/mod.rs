//! The match: one struct that holds everything in play and advances one
//! tick at a time (PLAN.md, contract 3).
//!
//! - **Ticks.** [`TICKS_PER_S`] a second, [`TICK_MS`] each. `Match::step`
//!   takes one [`SeatInput`] per seat and advances one tick. The same map,
//!   tuning, setup and inputs always give the same match: nothing here reads
//!   a clock or a random device, and every number is an integer.
//! - **Units** (contract 1). Positions in whole millimetres on the map's
//!   ground plane (`x` east, `y` north; boats keep micrometres inside so slow
//!   speeds add up), speeds in mm/s, angles in 65536 units a turn with 0
//!   east and a quarter north (the map's convention, `map::trig`), times in
//!   ticks.
//! - **Seats.** A match has the tuning's seat count of lobby slots, each with
//!   its player colour (blue, red, green, yellow, by slot). An empty seat
//!   has no boat.
//! - **Events** (contract 5). Each step's [`MatchEvent`]s are in
//!   [`Match::events`] until the next step. Popups, sounds, the AI and online
//!   messages read them.
//! - **Boats** ([`boat`]): driving and fishing mode, collisions with land and
//!   each other ([`collide`]), wrecks, the respawn, ghosting.
//! - **The view** ([`view`]): plain data for drawing, interpolated between
//!   ticks.
//!
//! **Hooks for other systems**, each a method with its own doc comment:
//! [`Match::cancel_layers`] (Esc's "cancel the current thing", before the
//! in-match menu), [`Match::wreck_drops_fish`] (a wreck loses the fish on
//! the line), [`boat::Boat::nitrous`] (the items system sets it),
//! [`boat::Boat::captured`] (the Fish-Boy and minigames set it: the boat
//! coasts), and [`Score`] (the catches, for the postgame).
//!
//! **Fishing** ([`fishing`]): the fish in the lake and each seat's line,
//! stocked by [`Match::stock`] and stepped after the boats each tick.

pub mod boat;
pub mod collide;
pub mod fishing;
pub mod input;
pub mod view;

#[cfg(test)]
mod tests;

use serde::Serialize;

use crate::map::rng::Rng;
use crate::map::{Map, P};
use crate::tuning::Tuning;

pub use boat::{Boat, Mode};
pub use input::{Actions, SeatInput};

/// Ticks a second (PLAN.md, contract 2).
pub const TICKS_PER_S: i32 = 50;
/// Milliseconds a tick.
pub const TICK_MS: i32 = 1000 / TICKS_PER_S;

/// The player colours by seat (`data/look.json` holds their values).
pub const SEAT_COLOURS: [&str; 4] = crate::app::gallery::PLAYER_COLOURS;

/// How hard an AI player plays (the AI itself comes later).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard];

    pub fn key(self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Normal => "normal",
            Difficulty::Hard => "hard",
        }
    }

    pub fn from_key(key: &str) -> Option<Difficulty> {
        Difficulty::ALL.into_iter().find(|d| d.key() == key)
    }
}

/// Who sits in a seat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Occupant {
    Empty,
    /// a person, at this browser or another
    Human,
    Ai(Difficulty),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SeatSetup {
    pub occupant: Occupant,
    pub name: String,
}

/// What the match setup chose. It overrides the tuning's defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Setup {
    /// one per seat, in slot order
    pub seats: Vec<SeatSetup>,
    pub length_ms: i32,
    /// the time between item buoys (the items system reads it here)
    pub buoy_interval_ms: i32,
    /// an offline match pauses while a person has the in-match menu open
    pub offline: bool,
    /// the match's own seed, for every chance a later system draws
    /// ([`Match::rng`]); the map has its own seed
    pub seed: u64,
}

impl Setup {
    /// Every seat empty, and the tuning's default length and buoy interval.
    pub fn new(tuning: &Tuning) -> Setup {
        Setup {
            seats: (0..tuning.game.seats)
                .map(|_| SeatSetup {
                    occupant: Occupant::Empty,
                    name: String::new(),
                })
                .collect(),
            length_ms: tuning.game.length_ms,
            buoy_interval_ms: tuning.game.buoy_interval_ms,
            offline: true,
            seed: 0,
        }
    }

    /// The same, with `seat` taken.
    pub fn with(mut self, seat: usize, occupant: Occupant, name: &str) -> Setup {
        self.seats[seat] = SeatSetup {
            occupant,
            name: name.to_string(),
        };
        self
    }
}

/// Why a boat was wrecked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WreckCause {
    /// rammed by the boat in this seat
    Rammed { by: usize },
    /// hit land too fast
    Grounded,
}

/// Something that happened this tick. The first group is the match core's;
/// the rest are named in PLAN.md for the systems that will raise them, and
/// their fields are theirs to settle.
///
/// As JSON an event is `{"type": "Wrecked", "seat": 1, ...}`: the sound
/// engine's `data/sounds.json` `events` table names events this way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type")]
pub enum MatchEvent {
    ModeChanged {
        seat: usize,
        mode: Mode,
    },
    /// bounced off land; `speed` toward the land, mm/s
    Bounced {
        seat: usize,
        speed: i32,
    },
    /// two boats bumped; `speed` is how fast they closed, mm/s
    Bumped {
        a: usize,
        b: usize,
        speed: i32,
    },
    Wrecked {
        seat: usize,
        cause: WreckCause,
    },
    /// the rammer's motor stalled
    Stalled {
        seat: usize,
    },
    Respawned {
        seat: usize,
    },
    GhostEnded {
        seat: usize,
    },
    MenuOpened {
        seat: usize,
    },
    MenuClosed {
        seat: usize,
    },
    MatchEnded,
    // fishing (`fishing`): a cast, a bite, a catch
    /// Space was pressed to cast with the boat too fast
    CastRefused {
        seat: usize,
        speed_mm_per_s: i32,
    },
    /// Space went down: the cast is charging
    Charging {
        seat: usize,
    },
    Cast {
        seat: usize,
        distance_mm: i32,
    },
    BobberLanded {
        seat: usize,
        distance_mm: i32,
    },
    /// the lure stopped sinking (Space, or the lake bed)
    LureStopped {
        seat: usize,
        depth_mm: i32,
    },
    Nibble {
        seat: usize,
    },
    /// Space before the bite scared the fish off
    Scared {
        seat: usize,
    },
    Bite {
        seat: usize,
    },
    /// the hook window passed; the fish swam off
    Missed {
        seat: usize,
    },
    Hooked {
        seat: usize,
        fish: u32,
        /// the roster id ("walleye")
        kind: String,
        tier: crate::map::fish::Tier,
    },
    /// the reel's ratchet, while Space lifts the catch zone
    ReelTick {
        seat: usize,
    },
    /// the catch meter emptied, or the boat moved too fast: the fish is lost
    LineBroke {
        seat: usize,
        fish: u32,
    },
    /// Esc, Q or a wreck dropped the fish off the line
    FishDropped {
        seat: usize,
        fish: u32,
    },
    /// the line came in with no fish on it
    LineReeledIn {
        seat: usize,
    },
    Caught {
        seat: usize,
        fish: u32,
        kind: String,
        tier: crate::map::fish::Tier,
        length_mm: i32,
    },
    // raised by later systems
    BuoyWarning,
    BuoySpawned,
    ItemPicked {
        seat: usize,
    },
    ItemUsed {
        seat: usize,
    },
    Pinged {
        seat: usize,
    },
}

/// One landed fish, for the postgame (the fishing system fills it).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Catch {
    /// the fish's model id, e.g. "bluegill"
    pub kind: String,
    pub length_mm: i32,
    /// its points
    pub value: i32,
}

/// One seat's catches, in the order landed. Kept hidden until the postgame.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Score {
    pub catches: Vec<Catch>,
}

/// One seat's state outside its boat.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeatState {
    /// the in-match menu (Settings, Leave match) is open
    pub menu_open: bool,
}

/// The tuning in the units the match uses: ticks, mm/s a tick, angle
/// units a tick. Made once from [`Tuning`] and the [`Setup`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rules {
    pub length_ticks: u32,
    pub top: i32,
    /// speed gained per tick under throttle, mm/s
    pub accel: i32,
    pub reverse: i32,
    pub reverse_accel: i32,
    pub brake: i32,
    pub coast: i32,
    pub side_drag: i32,
    /// angle units a tick at full turn
    pub turn: i32,
    pub turn_at_rest_pct: i32,
    /// the forward speed of full turning, mm/s
    pub turn_full_at: i32,
    pub swivel: i32,
    pub swivel_limit: i32,
    pub half_length: i32,
    pub half_beam: i32,
    pub nitrous_accel_pct: i32,
    pub nitrous_top_pct: i32,
    pub nitrous_turn_pct: i32,
    /// speed toward another boat that wrecks it, mm/s
    pub ram_wreck: i32,
    /// speed toward land that wrecks the boat, mm/s
    pub land_wreck: i32,
    pub stall_ticks: u32,
    pub respawn_ticks: u32,
    pub ghost_ticks: u32,
    pub bounce_pct: i32,
    pub scrape_keep_pct: i32,
    pub bump_pct: i32,
    pub spawn_spread: i32,
}

/// Milliseconds to whole ticks, rounded to the nearest.
pub fn ticks(ms: i32) -> u32 {
    ((ms.max(0) + TICK_MS / 2) / TICK_MS) as u32
}

/// The change a tick for going from 0 to `amount` in `ms`, rounded, at
/// least 1.
fn per_tick(amount: i32, ms: i32) -> i32 {
    let v = (amount as i64 * TICK_MS as i64 + ms as i64 / 2) / ms.max(1) as i64;
    (v as i32).max(1)
}

/// Degrees a second to angle units a tick.
fn deg_per_tick(deg_per_s: i32) -> i32 {
    (deg_per_s as i64 * crate::map::trig::TURN as i64 / 360 / TICKS_PER_S as i64) as i32
}

fn deg(d: i32) -> i32 {
    (d as i64 * crate::map::trig::TURN as i64 / 360) as i32
}

impl Rules {
    pub fn new(t: &Tuning, setup: &Setup) -> Rules {
        let (b, w) = (&t.boat, &t.wreck);
        let top = b.top_speed_mm_per_s;
        Rules {
            length_ticks: ticks(setup.length_ms),
            top,
            accel: per_tick(top, b.accel_ms),
            reverse: b.reverse_speed_mm_per_s,
            reverse_accel: per_tick(b.reverse_speed_mm_per_s, b.reverse_accel_ms),
            brake: per_tick(top, b.brake_ms),
            coast: per_tick(top, b.coast_ms),
            side_drag: per_tick(top, b.side_drag_ms),
            turn: deg_per_tick(b.turn_deg_per_s),
            turn_at_rest_pct: b.turn_at_rest_pct,
            turn_full_at: (top as i64 * b.turn_full_at_pct as i64 / 100) as i32,
            swivel: deg_per_tick(b.swivel_deg_per_s),
            swivel_limit: deg(b.swivel_limit_deg),
            half_length: b.half_length_mm,
            half_beam: b.half_beam_mm,
            nitrous_accel_pct: b.nitrous.accel_pct,
            nitrous_top_pct: b.nitrous.top_speed_pct,
            nitrous_turn_pct: b.nitrous.turn_pct,
            ram_wreck: (top as i64 * w.ram_wreck_pct as i64 / 100) as i32,
            land_wreck: (top as i64 * w.land_wreck_pct as i64 / 100) as i32,
            stall_ticks: ticks(w.stall_ms),
            respawn_ticks: ticks(w.respawn_ms),
            ghost_ticks: ticks(w.ghost_ms),
            bounce_pct: w.bounce_pct,
            scrape_keep_pct: w.scrape_keep_pct,
            bump_pct: w.bump_pct,
            spawn_spread: t.game.spawn_spread_mm,
        }
    }
}

/// The whole match.
#[derive(Debug, Clone)]
pub struct Match {
    pub map: Map,
    pub tuning: Tuning,
    pub rules: Rules,
    pub setup: Setup,
    /// ticks played
    pub tick: u32,
    pub ended: bool,
    /// by seat; `None` for an empty seat
    pub boats: Vec<Option<Boat>>,
    pub seats: Vec<SeatState>,
    /// by seat
    pub scores: Vec<Score>,
    /// this tick's events
    pub events: Vec<MatchEvent>,
    /// the match's chances, from `Setup::seed` (fish, buoys and items draw
    /// from their own `Rng::stream` of the seed, or from this)
    pub rng: Rng,
    /// the fish in the lake and every seat's line
    pub fishing: fishing::Fishing,
}

impl Match {
    /// A match on `map` with these seats. Every boat starts at the map's
    /// spawn, side by side across the heading, facing down the lake.
    pub fn new(map: Map, tuning: &Tuning, setup: Setup) -> Result<Match, String> {
        if setup.seats.is_empty() || setup.seats.len() > SEAT_COLOURS.len() {
            return Err(format!(
                "A match has 1 to {} seats, not {}.",
                SEAT_COLOURS.len(),
                setup.seats.len()
            ));
        }
        if setup.length_ms < TICK_MS {
            return Err("A match must last at least one tick.".into());
        }
        let rules = Rules::new(tuning, &setup);
        let n = setup.seats.len();
        let mut m = Match {
            boats: Vec::with_capacity(n),
            seats: vec![SeatState::default(); n],
            scores: vec![Score::default(); n],
            events: Vec::new(),
            rng: Rng::new(setup.seed),
            fishing: fishing::Fishing::new(&tuning.fish, &tuning.fishing, n, setup.seed),
            map,
            tuning: tuning.clone(),
            rules,
            setup,
            tick: 0,
            ended: false,
        };
        for seat in 0..n {
            let boat = match m.setup.seats[seat].occupant {
                Occupant::Empty => None,
                _ => {
                    let (at, heading) = m.spawn_slot(seat);
                    Some(Boat::new(seat, at, heading))
                }
            };
            m.boats.push(boat);
        }
        Ok(m)
    }

    /// Stocks the lake with the roster's fish (`data/fish.json`), by the
    /// match seed. A match without it has no fish.
    pub fn stock(&mut self, roster: &crate::roster::Roster) -> Result<(), String> {
        self.fishing.stock(&self.map, roster, self.setup.seed)
    }

    /// The bow of a seat's boat, where its line goes out.
    pub fn rod(&self, seat: usize) -> P {
        self.boat(seat).map_or(self.map.spawn.at, |b| {
            fishing::bow(b, self.rules.half_length, self.rules.half_beam)
        })
    }

    /// Where a seat's boat starts and respawns: the map's spawn, moved
    /// across the heading so the seats stand `spawn_spread` apart, the
    /// first seat on the left.
    pub fn spawn_slot(&self, seat: usize) -> (P, i32) {
        let s = self.map.spawn;
        let n = self.setup.seats.len() as i64;
        let (c, sn) = boat::dir(s.heading);
        // left of the heading is (-sin, cos)
        let off = (2 * seat as i64 - (n - 1)) * self.rules.spawn_spread as i64 / 2;
        let left = -off;
        let x = s.at.x as i64 + (-(sn as i64) * left) / boat::ONE as i64;
        let y = s.at.y as i64 + (c as i64 * left) / boat::ONE as i64;
        (P::new(x as i32, y as i32), s.heading)
    }

    /// The time left, in ms.
    pub fn remaining_ms(&self) -> i32 {
        (self.rules.length_ticks.saturating_sub(self.tick) as i64 * TICK_MS as i64) as i32
    }

    /// Whether the match is waiting on a menu: an offline match stops while
    /// a person has the in-match menu open.
    pub fn paused(&self) -> bool {
        self.setup.offline
            && self
                .seats
                .iter()
                .zip(&self.setup.seats)
                .any(|(s, set)| s.menu_open && set.occupant == Occupant::Human)
    }

    /// Advances one tick. `inputs` holds one per seat, in seat order; a
    /// missing one is no input. Returns this tick's events.
    pub fn step(&mut self, inputs: &[SeatInput]) -> &[MatchEvent] {
        self.events.clear();
        if self.ended {
            return &self.events;
        }
        let none = SeatInput::default();
        let input = |seat: usize| inputs.get(seat).copied().unwrap_or(none);

        // the in-match menu answers even while the match is paused
        for seat in 0..self.seats.len() {
            let i = input(seat);
            if i.pressed.cancel {
                self.cancel(seat);
            }
            if i.pressed.menu {
                self.toggle_menu(seat);
            }
        }
        if self.paused() {
            return &self.events;
        }

        self.tick += 1;
        self.respawn_and_unghost();
        for seat in 0..self.boats.len() {
            let i = input(seat);
            self.drive(seat, &i);
        }
        collide::boats(self);
        for seat in 0..self.boats.len() {
            let i = input(seat);
            if let Some(b) = self.boats[seat].as_mut() {
                b.throttle(&self.rules, &i, self.tick);
            }
        }
        let world = fishing::World {
            map: &self.map,
            boats: &self.boats,
            half_length_mm: self.rules.half_length,
            half_beam_mm: self.rules.half_beam,
            tick: self.tick,
        };
        let all: Vec<SeatInput> = (0..self.seats.len()).map(input).collect();
        self.fishing.step(&world, &all, &mut self.scores, &mut self.events);
        if self.tick >= self.rules.length_ticks {
            self.ended = true;
            self.events.push(MatchEvent::MatchEnded);
        }
        &self.events
    }

    /// Esc: the layers above the boat cancel their current thing first
    /// (a minigame, the Fish-Boy, the fish on the line); with nothing to
    /// cancel, the in-match menu opens, and Esc in the menu closes it.
    pub fn cancel(&mut self, seat: usize) {
        if self.seats[seat].menu_open || !self.cancel_layers(seat) {
            self.toggle_menu(seat);
        }
    }

    /// **Hook for the fishing, items and Fish-Boy systems.** Cancel this
    /// seat's current thing, top layer first, and return true; return false
    /// when there is nothing to cancel (Esc then opens the menu). The
    /// fishing layer: a fish on the line is dropped, a line out is reeled
    /// in, a charge stops.
    pub fn cancel_layers(&mut self, seat: usize) -> bool {
        let rod = self.rod(seat);
        self.fishing.cancel(&self.map, seat, rod, &mut self.events)
    }

    fn toggle_menu(&mut self, seat: usize) {
        let s = &mut self.seats[seat];
        s.menu_open = !s.menu_open;
        self.events.push(if s.menu_open {
            MatchEvent::MenuOpened { seat }
        } else {
            MatchEvent::MenuClosed { seat }
        });
    }

    fn respawn_and_unghost(&mut self) {
        for seat in 0..self.boats.len() {
            let slot = self.spawn_slot(seat);
            let (tick, rules) = (self.tick, &self.rules);
            let Some(b) = self.boats[seat].as_mut() else {
                continue;
            };
            if b.wrecked_until.is_some_and(|t| tick >= t) {
                b.respawn(slot.0, slot.1, tick + rules.ghost_ticks);
                self.events.push(MatchEvent::Respawned { seat });
            } else if b.ghost_until == tick && tick > 0 {
                self.events.push(MatchEvent::GhostEnded { seat });
            }
        }
    }

    /// Steering, the mode switch and the move with its land collision.
    fn drive(&mut self, seat: usize, i: &SeatInput) {
        let tick = self.tick;
        let Some(b) = self.boats[seat].as_mut() else {
            return;
        };
        b.remember();
        if b.wrecked() {
            return;
        }
        if i.pressed.mode && !b.captured {
            b.mode = match b.mode {
                Mode::Driving => Mode::Fishing,
                Mode::Fishing => Mode::Driving,
            };
            self.events.push(MatchEvent::ModeChanged { seat, mode: b.mode });
        }
        b.steer(&self.rules, i);
        match collide::land(&self.map, &self.rules, b) {
            collide::LandHit::Clear => {}
            collide::LandHit::Bounced(speed) => self.events.push(MatchEvent::Bounced { seat, speed }),
            collide::LandHit::Wrecked => self.wreck(seat, WreckCause::Grounded, tick),
        }
    }

    /// Wrecks a seat's boat: it sits out the respawn delay.
    pub(crate) fn wreck(&mut self, seat: usize, cause: WreckCause, tick: u32) {
        let respawn = tick + self.rules.respawn_ticks;
        if let Some(b) = self.boats[seat].as_mut() {
            b.wreck(respawn);
        }
        self.wreck_drops_fish(seat);
        self.events.push(MatchEvent::Wrecked { seat, cause });
    }

    /// **Hook for the fishing system.** A wrecked player loses the fish on
    /// the line (BRIEF: "they do not lose anything except if they were
    /// reeling in a fish"); landed fish in [`Score`] stay. The line comes
    /// in and the fish on it swims free.
    pub fn wreck_drops_fish(&mut self, seat: usize) {
        let rod = self.rod(seat);
        self.fishing.wrecked(&self.map, seat, rod, &mut self.events);
    }

    /// The boat in a seat, if the seat is taken.
    pub fn boat(&self, seat: usize) -> Option<&Boat> {
        self.boats.get(seat).and_then(Option::as_ref)
    }
}
