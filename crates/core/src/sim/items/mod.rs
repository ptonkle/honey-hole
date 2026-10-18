//! Items (BRIEF.md, "Items"; PLAN.md build step 3): the item buoy, its
//! pickup, Shift and the minigames, and the six items' effects. Every
//! number is the `items` section of `data/tuning.json` ([`tuning`]); the
//! minigames are `crate::minigame`'s, stepped here from each seat's input.
//!
//! - **The buoy.** Every `Setup::buoy_interval_ms` (the first a whole
//!   interval in), a buoy spawns at `map::buoy::fair_point` for where the
//!   boats are then (`BuoySpawned`): a similar boat trip from every player,
//!   in open water. `warning_ms` before, the warning (`BuoyWarning`, a
//!   popup), then the countdown's `beats` beats `beat_ms` apart
//!   (`BuoyBeat { left }`, 5 down to 1, the last as it spawns). One buoy at
//!   a time: an unclaimed one moves to the new fair point when the next is
//!   due (`unclaimed_moves`; SECOND-ORDER R4.3).
//! - **Pickup.** A boat whose middle comes within `pickup_radius_mm` takes
//!   it ([`claims::pickup`]) and gets an item drawn from `weights`
//!   ([`claims::draw`]), hidden until then. A seat holds one: a new pickup
//!   replaces the old (`ItemPicked { seat, item, dropped }`).
//! - **Using** (Shift, [`claims::use_item`]). The item is spent and its
//!   minigame starts; the boat coasts ([`Boat::captured`]) while it runs.
//!   On Done the effect applies (`ItemUsed`); Esc quits and wastes it
//!   (`ItemWasted`). Cloaking Tech and Enhanced Radar play on the Fish-Boy,
//!   which comes up for them; the rest on the full-screen layer.
//! - **Effects.** Cloaking Tech: no ping sees the seat for `cloak_ms`, and
//!   every blip of it already on a radar is wiped. Power Reel: the reel's
//!   power is `power_reel_pct` ([`Fishing::reel_power_pct`]) for
//!   `power_reel_ms`. Nitrous: [`Boat::nitrous`] for `nitrous_ms` (rams and
//!   grounding still judge the plain top speed). Teleporter: the radar comes
//!   up to click a destination ([`claims::teleport_target`]), then a
//!   `teleport_charge_ms` charge everyone sees, and the boat is there, at
//!   rest (`Teleported`). High-Quality Camera: offered after each catch
//!   while held (`CameraOffered`); Shift takes the photo (its minigame),
//!   Esc or the offer's time keeps it for later (`CameraSaved`); the photo
//!   makes the last catch worth more ([`Fishing::use_camera`]). Enhanced
//!   Radar: a ping, at once, that also marks the nearest fish of each tier
//!   ([`claims::reveal`]) on the seat's radar until its next ping.
//!
//! [`Boat::captured`]: super::Boat::captured
//! [`Boat::nitrous`]: super::Boat::nitrous
//! [`Fishing::reel_power_pct`]: super::fishing::Fishing::reel_power_pct
//! [`Fishing::use_camera`]: super::fishing::Fishing::use_camera

pub mod claims;
mod online;
pub mod tuning;

#[cfg(test)]
mod tests;

use serde::Serialize;

use super::{ticks, Match, MatchEvent, SeatInput};
use crate::fishboy::radar::{self, RadarView};
use crate::map::fish::Tier;
use crate::map::rng::{mix, Rng};
use crate::map::P;
use crate::minigame::{self, Input, Kind, Layer, Minigame, State};
use tuning::ItemsTuning;

/// The six items. As JSON (events, the sound table) each is its key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Item {
    Cloak,
    PowerReel,
    Nitrous,
    Teleporter,
    Camera,
    EnhancedRadar,
}

impl Item {
    pub const ALL: [Item; 6] = [
        Item::Cloak,
        Item::PowerReel,
        Item::Nitrous,
        Item::Teleporter,
        Item::Camera,
        Item::EnhancedRadar,
    ];

    /// Its key: the tuning's, the strings' (`item.{key}`) and the sound
    /// table's (`use_{key}`).
    pub fn key(self) -> &'static str {
        self.minigame().key()
    }

    pub fn from_key(key: &str) -> Option<Item> {
        Item::ALL.into_iter().find(|i| i.key() == key)
    }

    /// The minigame it plays before it works.
    pub fn minigame(self) -> Kind {
        match self {
            Item::Cloak => Kind::Cloak,
            Item::PowerReel => Kind::PowerReel,
            Item::Nitrous => Kind::Nitrous,
            Item::Teleporter => Kind::Teleporter,
            Item::Camera => Kind::Camera,
            Item::EnhancedRadar => Kind::Radar,
        }
    }
}

/// A minigame being played, and the item it is for.
#[derive(Debug, Clone)]
pub struct Playing {
    pub item: Item,
    pub game: Minigame,
}

/// A teleport charging: where to, and its first and last ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Charge {
    pub to: P,
    pub from: u32,
    pub until: u32,
}

/// A fish an Enhanced Radar marked: where it was then.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark {
    pub fish: usize,
    pub tier: Tier,
    pub at: P,
}

/// One seat's items.
#[derive(Debug, Clone, Default)]
pub struct SeatItems {
    pub held: Option<Item>,
    pub playing: Option<Playing>,
    /// a Teleporter's destination is being picked on the radar
    pub choosing: bool,
    pub charge: Option<Charge>,
    /// each effect runs before this tick
    pub cloak_until: u32,
    pub reel_until: u32,
    pub nitrous_until: u32,
    /// the camera's offer is up before this tick
    pub offer_until: Option<u32>,
    /// the last Enhanced Radar's fish, until the seat's next ping
    pub marks: Vec<Mark>,
    /// items used so far (mixed into each minigame's seed)
    pub uses: u64,
}

impl SeatItems {
    /// Something is under way that Shift may not start over.
    pub fn busy(&self) -> bool {
        self.playing.is_some() || self.choosing || self.charge.is_some() || self.offer_until.is_some()
    }

    /// The seat's keys go to the items (the boat coasts).
    pub fn captures(&self) -> bool {
        self.playing.is_some() || self.choosing
    }

    /// The Fish-Boy may not be put down with Tab: a minigame on its
    /// screen, or a teleport's choice, is using it (Esc ends either).
    pub fn holds_screen(&self) -> bool {
        self.choosing
            || self
                .playing
                .as_ref()
                .is_some_and(|p| p.game.layer() == Layer::FishBoy)
    }
}

/// Every seat's items and the buoy.
#[derive(Debug, Clone)]
pub struct Items {
    pub tuning: ItemsTuning,
    pub interval_ticks: u32,
    /// the buoy in the lake, if any
    pub buoy: Option<P>,
    /// the tick the next buoy is due
    pub next_buoy: u32,
    pub seats: Vec<SeatItems>,
    /// **Test hook**: the next item drawn, instead of the weights' draw
    pub force_next: Option<Item>,
    /// **Test hook**: a minigame is done as soon as it starts
    pub quick: bool,
    /// buoys placed so far: the buoy in the lake is number `spawned`
    /// (online, the host's count, `online.rs`)
    pub spawned: u32,
    /// online: the buoy this browser has claimed already
    pub claimed: Option<u32>,
    seed: u64,
    rng: Rng,
}

impl Items {
    pub fn new(t: &ItemsTuning, interval_ms: i32, seats: usize, seed: u64) -> Items {
        let interval_ticks = ticks(interval_ms).max(1);
        Items {
            tuning: t.clone(),
            interval_ticks,
            buoy: None,
            next_buoy: interval_ticks,
            seats: vec![SeatItems::default(); seats],
            force_next: None,
            quick: false,
            spawned: 0,
            claimed: None,
            seed,
            rng: Rng::stream(seed, 41),
        }
    }

    pub fn cloaked(&self, seat: usize, tick: u32) -> bool {
        self.seats.get(seat).is_some_and(|s| tick < s.cloak_until)
    }

    /// The seat's effects still running, with their ms left, in item
    /// order (the teleport's charge is the Teleporter's).
    pub fn effects(&self, seat: usize, tick: u32) -> Vec<(Item, i32)> {
        let Some(s) = self.seats.get(seat) else {
            return Vec::new();
        };
        let left = |until: u32| (until.saturating_sub(tick) as i64 * super::TICK_MS as i64) as i32;
        let mut out = Vec::new();
        for (item, until) in [
            (Item::Cloak, s.cloak_until),
            (Item::PowerReel, s.reel_until),
            (Item::Nitrous, s.nitrous_until),
        ] {
            if tick < until {
                out.push((item, left(until)));
            }
        }
        if let Some(c) = s.charge {
            out.push((Item::Teleporter, left(c.until)));
        }
        out
    }

    /// The teleport's charge done, in thousandths (0 for none).
    pub fn charge_permille(&self, seat: usize, tick: u32) -> i32 {
        match self.seats.get(seat).and_then(|s| s.charge) {
            Some(c) => {
                let span = (c.until - c.from).max(1) as i64;
                ((tick.saturating_sub(c.from) as i64 * 1000 / span).clamp(1, 1000)) as i32
            }
            None => 0,
        }
    }

    /// The tick the buoy's warning comes, for the next buoy.
    fn warn_tick(&self) -> u32 {
        self.next_buoy.saturating_sub(ticks(self.tuning.warning_ms))
    }

    /// The beat with `left` beats to go (1 is the last, as it spawns).
    fn beat_tick(&self, left: i32) -> u32 {
        self.next_buoy
            .saturating_sub(ticks((left - 1).max(0) * self.tuning.beat_ms))
    }
}

/// The pointer and the main button of a seat's input, for a minigame.
/// Esc is the match's (`Match::cancel_layers`), so it is not passed on.
fn game_input(i: &SeatInput) -> Input {
    Input {
        quit: false,
        ..Input::from_seat(i)
    }
}

impl Match {
    /// **Hook for online play and the debug flag.** Puts the buoy at `at`
    /// now (the next still comes on the clock).
    pub fn place_buoy(&mut self, at: P) {
        self.items.buoy = Some(at);
        self.items.spawned += 1;
        self.items.claimed = None;
        self.events.push(MatchEvent::BuoySpawned {
            x_mm: at.x,
            y_mm: at.y,
        });
    }

    /// Whether a seat's boat coasts for the items or the Fish-Boy.
    pub(super) fn recapture(&mut self, seat: usize) {
        let up = self.seats[seat].fishboy || self.items.seats[seat].captures();
        if let Some(b) = self.boats[seat].as_mut() {
            b.captured = up;
        }
    }

    /// One tick of the items, after the boats and the radar: the buoy's
    /// clock and pickups, each seat's Shift, minigame or teleport choice,
    /// and the effects running.
    pub(super) fn items_step(&mut self, inputs: &[SeatInput]) {
        // a ping of the seat's own ends the last Enhanced Radar's marks
        let pinged: Vec<usize> = self
            .events
            .iter()
            .filter_map(|e| match e {
                MatchEvent::Pinged { seat } => Some(*seat),
                _ => None,
            })
            .collect();
        for seat in pinged {
            self.items.seats[seat].marks.clear();
        }
        self.items_shared_before();
        self.buoy_clock();
        self.pickups();
        for seat in 0..self.items.seats.len() {
            // a remote seat's items are its own browser's
            if self.is_remote(seat) {
                continue;
            }
            let none = SeatInput::default();
            let i = if self.seats[seat].menu_open {
                none
            } else {
                inputs.get(seat).copied().unwrap_or(none)
            };
            self.seat_items(seat, &i);
        }
        self.effects_step();
        // R4.11: a minigame (or a teleport's choice) has the keys, so a
        // fish on the line waits for it (SECOND-ORDER R5.31)
        let hold = self.items.tuning.reel_holds_in_minigame;
        for seat in 0..self.items.seats.len() {
            let held = hold && self.items.seats[seat].captures();
            if let Some(h) = self.fishing.reel_held.get_mut(seat) {
                *h = held;
            }
        }
    }

    /// The warning, the countdown's beats and the spawn.
    fn buoy_clock(&mut self) {
        let tick = self.tick;
        let it = &self.items;
        if it.buoy.is_some() && !it.tuning.unclaimed_moves {
            // it stays until taken; the clock waits
            if tick >= it.warn_tick() {
                self.items.next_buoy = tick + it.interval_ticks;
            }
            return;
        }
        if tick == it.warn_tick() {
            self.events.push(MatchEvent::BuoyWarning);
        }
        for left in (1..=it.tuning.beats).rev() {
            if tick == it.beat_tick(left) {
                self.events.push(MatchEvent::BuoyBeat { left: left as u32 });
            }
        }
        if tick >= it.next_buoy {
            self.items.next_buoy = tick + self.items.interval_ticks;
            let players: Vec<P> = (0..self.boats.len())
                .filter_map(|s| {
                    let b = self.boats[s].as_ref()?;
                    Some(if b.wrecked() { self.spawn_slot(s).0 } else { b.at() })
                })
                .collect();
            let seed = mix(self.items.seed ^ tick as u64);
            // online the host places it, and a joiner waits for its word
            if self.places_buoys() {
                match crate::map::buoy::fair_point(&self.map, &players, seed) {
                    Some(at) => {
                        self.place_buoy(at);
                        self.share_buoy(at);
                    }
                    None => self.items.buoy = None,
                }
            }
        }
    }

    /// The boat that drives through the buoy takes an item.
    fn pickups(&mut self) {
        // online it is a claim to the host (`online.rs`)
        if self.online_pickups() {
            return;
        }
        let boats: Vec<(usize, P)> = self
            .boats
            .iter()
            .flatten()
            .filter(|b| !b.wrecked())
            .map(|b| (b.seat, b.at()))
            .collect();
        let Some(seat) = claims::pickup(self.items.buoy, &boats, self.items.tuning.pickup_radius_mm) else {
            return;
        };
        self.items.buoy = None;
        let item = self.draw_item();
        let dropped = self.items.seats[seat].held.replace(item);
        self.events.push(MatchEvent::ItemPicked { seat, item, dropped });
    }

    /// The item a pickup draws: the test hook's, or from the weights.
    pub(super) fn draw_item(&mut self) -> Item {
        match self.items.force_next.take() {
            Some(i) => i,
            None => {
                let roll = self.items.rng.next_u64() as u32;
                let w = self.items.tuning.weights;
                claims::draw(|i| w.get(i), roll)
            }
        }
    }

    /// One seat's Shift, minigame, teleport choice or camera offer.
    fn seat_items(&mut self, seat: usize, i: &SeatInput) {
        let tick = self.tick;
        if self.items.seats[seat].playing.is_some() {
            self.play_minigame(seat, i);
            return;
        }
        if self.items.seats[seat].choosing {
            if i.pressed.click {
                self.choose_teleport(seat, i.pointer);
            }
            return;
        }
        if let Some(until) = self.items.seats[seat].offer_until {
            if i.pressed.item && self.boat(seat).is_some_and(|b| !b.wrecked()) {
                self.items.seats[seat].offer_until = None;
                self.start_item(seat, Item::Camera);
            } else if tick >= until {
                self.items.seats[seat].offer_until = None;
                self.events.push(MatchEvent::CameraSaved { seat });
            }
            return;
        }
        if !i.pressed.item {
            return;
        }
        let wrecked = self.boat(seat).is_none_or(|b| b.wrecked());
        match claims::use_item(&self.items.seats[seat], wrecked) {
            Ok(item) => self.start_item(seat, item),
            Err(why) => {
                if let Some(item) = self.items.seats[seat].held {
                    self.events.push(MatchEvent::ItemRefused { seat, item, why });
                }
            }
        }
    }

    /// The item is spent and its minigame begins.
    fn start_item(&mut self, seat: usize, item: Item) {
        let s = &mut self.items.seats[seat];
        if s.held == Some(item) {
            s.held = None;
        }
        s.uses += 1;
        let seed = mix(self.items.seed ^ ((seat as u64) << 40) ^ s.uses);
        let game = Minigame::new(item.minigame(), seed, &self.tuning.minigames);
        let on_fishboy = game.layer() == Layer::FishBoy;
        self.items.seats[seat].playing = Some(Playing { item, game });
        self.set_fishboy(seat, on_fishboy);
        self.recapture(seat);
        self.take_cues(seat, item);
        if self.items.quick {
            self.finish_item(seat, item);
        }
    }

    /// A tick of the seat's minigame.
    fn play_minigame(&mut self, seat: usize, i: &SeatInput) {
        let Some(p) = self.items.seats[seat].playing.as_mut() else {
            return;
        };
        let item = p.item;
        p.game.step(&game_input(i));
        let state = p.game.state();
        self.take_cues(seat, item);
        match state {
            State::Playing => {}
            State::Done { .. } => self.finish_item(seat, item),
            State::Quit => self.waste_item(seat),
        }
    }

    /// The minigame's sound cues, as events for this seat.
    fn take_cues(&mut self, seat: usize, item: Item) {
        let Some(p) = self.items.seats[seat].playing.as_mut() else {
            return;
        };
        for cue in p.game.take_cues() {
            let e = match cue {
                minigame::cue::START => MatchEvent::MinigameStarted { seat, item },
                minigame::cue::TICK => MatchEvent::MinigameTick { seat },
                minigame::cue::MISS => MatchEvent::MinigameMiss { seat },
                minigame::cue::SUCCESS => MatchEvent::MinigameDone { seat, item },
                minigame::cue::SHUTTER => MatchEvent::Shutter { seat },
                _ => continue,
            };
            self.events.push(e);
        }
    }

    /// **Hook for the items' Esc.** The minigame quits (the item is
    /// wasted), a teleport's choice ends (wasted too), or the camera's
    /// offer closes (the camera is kept). False with none of these.
    pub(super) fn items_cancel(&mut self, seat: usize) -> bool {
        let s = &mut self.items.seats[seat];
        if s.playing.is_some() || s.choosing {
            self.waste_item(seat);
            return true;
        }
        if s.offer_until.take().is_some() {
            self.events.push(MatchEvent::CameraSaved { seat });
            return true;
        }
        false
    }

    /// The minigame or the teleport's choice ends without its effect.
    fn waste_item(&mut self, seat: usize) {
        let s = &mut self.items.seats[seat];
        let screen = s.holds_screen();
        let item = s
            .playing
            .take()
            .map(|p| p.item)
            .or(s.choosing.then_some(Item::Teleporter));
        s.choosing = false;
        if screen {
            self.set_fishboy(seat, false);
        }
        self.recapture(seat);
        if let Some(item) = item {
            self.events.push(MatchEvent::ItemWasted { seat, item });
        }
    }

    /// The minigame is done: the item's effect.
    fn finish_item(&mut self, seat: usize, item: Item) {
        let tick = self.tick;
        let t = self.items.tuning.clone();
        self.items.seats[seat].playing = None;
        let used = MatchEvent::ItemUsed { seat, item };
        match item {
            Item::Cloak => {
                self.items.seats[seat].cloak_until = tick + ticks(t.cloak_ms);
                self.wipe_blips(seat);
                self.set_fishboy(seat, false);
                self.events.push(used);
            }
            Item::PowerReel => {
                self.items.seats[seat].reel_until = tick + ticks(t.power_reel_ms);
                self.fishing.reel_power_pct[seat] = t.power_reel_pct;
                self.events.push(used);
            }
            Item::Nitrous => {
                self.items.seats[seat].nitrous_until = tick + ticks(t.nitrous_ms);
                if let Some(b) = self.boats[seat].as_mut() {
                    b.nitrous = true;
                }
                self.events.push(used);
            }
            Item::Teleporter => {
                // the radar comes up for the destination; the charge (and
                // `ItemUsed`) follow the click
                self.items.seats[seat].choosing = true;
                self.set_fishboy(seat, true);
                self.events.push(MatchEvent::TeleportChoosing { seat });
            }
            Item::Camera => {
                self.fishing.use_camera(seat, &mut self.scores[seat]);
                self.events.push(used);
            }
            Item::EnhancedRadar => {
                self.set_fishboy(seat, true);
                self.events.push(used);
                self.enhanced_ping(seat);
            }
        }
        if item != Item::Teleporter {
            self.share_use(seat, item);
        }
        self.recapture(seat);
    }

    /// A click on the radar picks the teleport's destination; off the
    /// map's square it does nothing.
    fn choose_teleport(&mut self, seat: usize, pointer: Option<[i32; 2]>) {
        let Some([x, y]) = pointer else {
            return;
        };
        let region = radar::region(&self.map, crate::fishboy::RADAR_MARGIN_MM);
        let Some(want) = radar::map_point(region, x, y) else {
            return;
        };
        let Some(to) = claims::teleport_target(&self.map, want, self.items.tuning.teleport_clear_mm) else {
            return;
        };
        let tick = self.tick;
        let s = &mut self.items.seats[seat];
        s.choosing = false;
        s.charge = Some(Charge {
            to,
            from: tick,
            until: tick + ticks(self.items.tuning.teleport_charge_ms).max(1),
        });
        self.set_fishboy(seat, false);
        self.recapture(seat);
        self.events.push(MatchEvent::ItemUsed {
            seat,
            item: Item::Teleporter,
        });
        self.share_use(seat, Item::Teleporter);
    }

    /// Cloaking Tech: every radar's blip of this seat is wiped now.
    fn wipe_blips(&mut self, seat: usize) {
        let Some(k) = self.boats.iter().flatten().position(|b| b.seat == seat) else {
            return;
        };
        for s in self.seats.iter_mut() {
            if let Some(blip) = s.ping.sightings.get_mut(k) {
                *blip = None;
            }
        }
    }

    /// Enhanced Radar: a ping now, whatever its cooldown, and the nearest
    /// fish of each tier marked.
    fn enhanced_ping(&mut self, seat: usize) {
        let Some(me) = self.boats.iter().flatten().position(|b| b.seat == seat) else {
            return;
        };
        let from = self.boat(seat).map_or(self.map.spawn.at, |b| b.at());
        let view = RadarView {
            me,
            ..self.radar_view(seat)
        };
        let size = self.map.size_mm();
        let t = self.tuning.radar.clone();
        let ping = &mut self.seats[seat].ping;
        ping.started = None;
        if ping.start(self.tick as i64, &view, size, &t) {
            self.events.push(MatchEvent::Pinged { seat });
        }
        let marks: Vec<Mark> = claims::reveal(&self.fishing.fish, from)
            .into_iter()
            .map(|k| {
                let f = &self.fishing.fish[k];
                Mark {
                    fish: k,
                    tier: f.tier,
                    at: f.at,
                }
            })
            .collect();
        let count = marks.len() as u32;
        self.items.seats[seat].marks = marks;
        self.events.push(MatchEvent::Revealed { seat, count });
    }

    /// The effects' ends, nitrous and the Power Reel on the boat and the
    /// reel, and a charged teleport.
    fn effects_step(&mut self) {
        let tick = self.tick;
        for seat in 0..self.items.seats.len() {
            let s = &mut self.items.seats[seat];
            for (item, until) in [
                (Item::Cloak, &mut s.cloak_until),
                (Item::PowerReel, &mut s.reel_until),
                (Item::Nitrous, &mut s.nitrous_until),
            ] {
                if *until != 0 && tick >= *until {
                    *until = 0;
                    self.events.push(if item == Item::Cloak {
                        MatchEvent::CloakEnded { seat }
                    } else {
                        MatchEvent::EffectEnded { seat, item }
                    });
                    // the end's own change; the start's is `finish_item`'s
                    match item {
                        Item::PowerReel => self.fishing.reel_power_pct[seat] = 100,
                        Item::Nitrous => {
                            if let Some(b) = self.boats[seat].as_mut() {
                                b.nitrous = false;
                            }
                        }
                        _ => {}
                    }
                }
            }
            // the cloak travels in this seat's boat state online
            let cloaked = tick < s.cloak_until;
            let remote = self.online.remote.get(seat).copied().unwrap_or(false);
            if let Some(h) = self.online.hidden.get_mut(seat).filter(|_| !remote) {
                *h = cloaked;
            }
            let charged = s.charge.filter(|c| tick >= c.until);
            if let Some(b) = self.boats[seat].as_mut() {
                if let Some(c) = charged {
                    self.items.seats[seat].charge = None;
                    b.set_at(c.to);
                    b.set_vel([0, 0]);
                    b.remember();
                    self.events.push(MatchEvent::Teleported { seat });
                }
            }
        }
    }

    /// After the fishing's tick: a catch with the camera held offers it.
    pub(super) fn items_after_fishing(&mut self) {
        let caught: Vec<usize> = self
            .events
            .iter()
            .filter_map(|e| match e {
                MatchEvent::Caught { seat, .. } => Some(*seat),
                _ => None,
            })
            .collect();
        let until = self.tick + ticks(self.items.tuning.camera_prompt_ms);
        for seat in caught {
            let s = &mut self.items.seats[seat];
            if s.held == Some(Item::Camera) && s.playing.is_none() && !s.choosing {
                s.offer_until = Some(until);
                self.events.push(MatchEvent::CameraOffered { seat });
            }
        }
    }

    /// **Hook for the wreck.** A teleport's charge is lost; nitrous ends.
    pub(super) fn items_wrecked(&mut self, seat: usize) {
        let s = &mut self.items.seats[seat];
        if s.charge.take().is_some() {
            self.events.push(MatchEvent::ItemWasted {
                seat,
                item: Item::Teleporter,
            });
        }
        if s.nitrous_until != 0 {
            s.nitrous_until = 0;
            if let Some(b) = self.boats[seat].as_mut() {
                b.nitrous = false;
            }
            self.events.push(MatchEvent::EffectEnded {
                seat,
                item: Item::Nitrous,
            });
        }
    }
}
