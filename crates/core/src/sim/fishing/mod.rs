//! Fishing: the fish in the lake ([`lake`]), each seat's line, and the reel
//! minigame ([`reel`]). Every number is in the `fish` and `fishing`
//! sections of `data/tuning.json` ([`tuning`]).
//!
//! **A seat's line**, one [`Stage`] at a time (fishing mode only):
//!
//! 1. `Idle`. Space starts a charge, unless the boat is faster than
//!    `cast_max_speed_mm_per_s`: then the cast is refused (`CastRefused`).
//! 2. `Charging` while Space is held: the longer the hold (up to
//!    `charge_full_ms`), the further the cast, from `cast_min_mm` to
//!    `cast_max_mm`, in the swivel's direction from the bow. Releasing
//!    casts (`Cast`).
//! 3. `Flying`: the bobber's flight. It lands (`BobberLanded`) where the
//!    cast reached, or at the last water short of land.
//! 4. `Sinking`: the lure sinks at `sink_mm_per_s`. Space stops it at that
//!    depth; the lake bed stops it too (`LureStopped`).
//! 5. `Set`: the lure waits. A roaming fish within the attraction radius
//!    (full when its depth matches the lure's, less as they differ) swims
//!    over. Once there it waits 2–6 s, nibbles 1–3 times (`Nibble`) and
//!    bites (`Bite`). Space before the bite scares it off (`Scared`); Space
//!    within `hook_window_ms` of the bite hooks it (`Hooked`), and later the
//!    fish leaves (`Missed`). Space with no fish about reels the line in.
//! 6. `Reeling`: the reel minigame. The fish is pulled toward the surface
//!    and the boat as the meter fills, struggling as its marker darts. A
//!    full meter lands it (`Caught`, and a [`Catch`] in the seat's score);
//!    an empty one breaks the line (`LineBroke`).
//! 7. `Landed`: the fish is held up for `landed_ms`, then the line is free.
//!
//! **A moving boat** drags the bobber after it, never further than the
//! line's length. Faster than `drag_reel_in_mm_per_s` the line is reeled in;
//! faster than `drag_snap_mm_per_s` with a fish on, the line breaks.
//!
//! **Losing the fish**: Esc ([`Fishing::cancel`]), Q to driving mode, or a
//! wreck drop it (`FishDropped`); a broken line loses it too. A lost fish
//! swims `freed_swim_mm` off and stays in the lake. Only one line can have a
//! fish: a fish swimming to one lure is no longer free for another.
//!
//! **Hooks for other systems**: [`Fishing::reel_power_pct`] (the Power Reel
//! sets 200 for its time), [`Fishing::use_camera`] (the High-Quality
//! Camera's prompt after a catch), [`Fishing::tame`] (a test hook: the next
//! reel's fish holds still near the top of the bar).

pub mod announce;
pub mod lake;
pub mod reel;
pub mod shared;
pub mod strike;
pub mod tuning;
pub mod view;

#[cfg(test)]
mod tests;

use serde::Serialize;

use crate::map::fish::Tier;
use crate::map::rng::Rng;
use crate::map::{Map, P};
use crate::roster::Roster;

use super::boat::{Boat, Mode};
use super::input::SeatInput;
use super::{ticks, Catch, MatchEvent, Score};
use lake::{FishState, LakeFish, School};
use reel::{Outcome, Reel};
use tuning::{FishTuning, FishingTuning};

/// Where a seat's line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Idle,
    Charging,
    Flying,
    Sinking,
    Set,
    Reeling,
    Landed,
}

/// A fish that has noticed a lure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Interest {
    pub fish: usize,
    /// it has reached the lure
    pub arrived: bool,
    /// the tick of its next nibble (or the bite)
    pub next: u32,
    pub nibbles_left: i32,
    /// it is biting; Space up to and on this tick hooks it
    pub biting_until: Option<u32>,
}

/// One seat's line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Line {
    pub stage: Stage,
    /// when the charge began, and the flight's start and end
    pub since: u32,
    pub flew: u32,
    pub lands: u32,
    /// the cast's start (the bow) and where the bobber lands
    pub from: P,
    pub to: P,
    /// the bobber on the water, and the lure under it
    pub bobber: P,
    pub lure_depth_mm: i32,
    /// the line's length from the bow, once the bobber lands
    pub length_mm: i32,
    pub interest: Option<Interest>,
    /// the reel, and the fish on the line
    pub reel: Option<Reel>,
    pub fish: Option<usize>,
    /// where the fish was hooked
    pub hooked_at: P,
    pub hooked_depth_mm: i32,
    /// the next ratchet tick
    pub next_click: u32,
}

impl Line {
    pub fn idle() -> Line {
        Line {
            stage: Stage::Idle,
            since: 0,
            flew: 0,
            lands: 0,
            from: P::new(0, 0),
            to: P::new(0, 0),
            bobber: P::new(0, 0),
            lure_depth_mm: 0,
            length_mm: 0,
            interest: None,
            reel: None,
            fish: None,
            hooked_at: P::new(0, 0),
            hooked_depth_mm: 0,
            next_click: 0,
        }
    }

    /// The bobber is on the water or the lure under it.
    pub fn in_water(&self) -> bool {
        matches!(self.stage, Stage::Sinking | Stage::Set | Stage::Reeling)
    }
}

/// Every seat's fishing and the fish in the lake.
#[derive(Debug, Clone)]
pub struct Fishing {
    pub fish_tuning: FishTuning,
    pub tuning: FishingTuning,
    pub roster: Roster,
    pub fish: Vec<LakeFish>,
    pub schools: Vec<School>,
    /// by seat
    pub lines: Vec<Line>,
    /// **Hook for the items system** (Power Reel): the reel's power by
    /// seat, in percent; 100 is plain.
    pub reel_power_pct: Vec<i32>,
    /// **Hook for the items system** (SECOND-ORDER R5.31): by seat, the
    /// reel holds still this tick (an item's minigame has the keys). The
    /// zone, the fish's marker and the meter wait; a ram still snaps the
    /// line.
    pub reel_held: Vec<bool>,
    /// **Test hook**: the next reel's fish holds still near the top of the
    /// bar, so holding Space lands it.
    pub tame: bool,
    /// catches the High-Quality Camera has already raised, by seat and
    /// catch number
    pub photographed: Vec<(usize, usize)>,
    /// the hook popups still to come (`balance.announce_delay_ms`)
    pub announcer: announce::Announcer,
    /// how far a fish knocked off by a ram swims, by tier
    /// (`balance.rammed_swim_mm`; [`Fishing::new`] makes it
    /// `fish.freed_swim_mm`, and the match sets it)
    pub rammed_swim_mm: tuning::PerTier,
    /// **Hook for online play** (`shared.rs`): fish that got off a line
    /// since the last look, by seat
    pub freed: Vec<(usize, u32)>,
    rng: Rng,
}

/// The seat's rod: the boat's bow.
pub fn bow(b: &Boat, half_length_mm: i32, half_beam_mm: i32) -> P {
    lake::ahead(b.at(), b.heading, half_length_mm + half_beam_mm)
}

/// The points a catch is worth: its kind's value, times its tier's percent,
/// times its length over its kind's base length, rounded.
pub fn value(base_value: i32, base_length_mm: i32, length_mm: i32, tier_pct: i32) -> i32 {
    let num = base_value as i64 * tier_pct as i64 * length_mm as i64;
    let den = 100 * base_length_mm.max(1) as i64;
    ((num + den / 2) / den) as i32
}

/// What [`Fishing::step`] needs of the match.
pub struct World<'a> {
    pub map: &'a Map,
    pub boats: &'a [Option<Boat>],
    /// the boat's hull, for the bow
    pub half_length_mm: i32,
    pub half_beam_mm: i32,
    pub tick: u32,
    /// by seat: online, another browser runs this seat's line, and its
    /// owner's state places it here (`shared.rs`)
    pub remote: &'a [bool],
}

impl Fishing {
    /// No fish yet ([`Fishing::stock`] puts them in) and every line idle.
    pub fn new(fish: &FishTuning, fishing: &FishingTuning, seats: usize, seed: u64) -> Fishing {
        Fishing {
            fish_tuning: fish.clone(),
            tuning: fishing.clone(),
            roster: Roster::default(),
            fish: Vec::new(),
            schools: Vec::new(),
            lines: vec![Line::idle(); seats],
            reel_power_pct: vec![100; seats],
            reel_held: vec![false; seats],
            tame: false,
            photographed: Vec::new(),
            announcer: announce::Announcer::new(tuning::PerTier {
                common: 0,
                uncommon: 0,
                rare: 0,
                legendary: 0,
            }),
            rammed_swim_mm: tuning::PerTier {
                common: fish.freed_swim_mm,
                uncommon: fish.freed_swim_mm,
                rare: fish.freed_swim_mm,
                legendary: fish.freed_swim_mm,
            },
            freed: Vec::new(),
            rng: Rng::stream(seed, 32),
        }
    }

    /// Stocks the lake for a match from the roster, by the match seed.
    pub fn stock(&mut self, map: &Map, roster: &Roster, seed: u64) -> Result<(), String> {
        let (fish, schools) = lake::stock(map, roster, &self.fish_tuning, seed)?;
        self.roster = roster.clone();
        self.fish = fish;
        self.schools = schools;
        Ok(())
    }

    /// How many fish of a kind are still in the lake (public: the Fishpedia).
    pub fn left(&self, kind: usize) -> usize {
        self.fish.iter().filter(|f| f.kind == kind && f.in_lake()).count()
    }

    /// Sets a fish free: it swims off from `from` and stays in the lake.
    fn free(&mut self, map: &Map, fish: usize, from: P) {
        let d = self.fish_tuning.freed_swim_mm;
        self.free_by(map, fish, from, d);
    }

    /// Sets a fish free to swim `swim_mm` off from `from`.
    fn free_by(&mut self, map: &Map, fish: usize, from: P, swim_mm: i32) {
        let f = &mut self.fish[fish];
        if f.in_lake() {
            f.state = FishState::Fleeing {
                to: lake::flee_point_by(map, f.at, from, swim_mm),
            };
        }
    }

    /// Ends a seat's line: any fish on it or coming to it goes free. Says
    /// what was lost, as an event, or nothing for a line that was idle.
    fn end_line(&mut self, map: &Map, seat: usize, rod: P, broke: bool) -> Option<MatchEvent> {
        let line = std::mem::replace(&mut self.lines[seat], Line::idle());
        if let Some(i) = line.interest {
            self.free(map, i.fish, rod);
        }
        match line.stage {
            Stage::Idle | Stage::Landed => None,
            Stage::Charging => None,
            Stage::Reeling => {
                let fish = line.fish.unwrap_or(0);
                self.free(map, fish, rod);
                self.freed.push((seat, fish as u32));
                Some(if broke {
                    MatchEvent::LineBroke {
                        seat,
                        fish: fish as u32,
                    }
                } else {
                    MatchEvent::FishDropped {
                        seat,
                        fish: fish as u32,
                    }
                })
            }
            Stage::Flying | Stage::Sinking | Stage::Set => Some(MatchEvent::LineReeledIn { seat }),
        }
    }

    /// Esc: drops the fish on the line, reels the line in, or stops a
    /// charge. False when the line has nothing to cancel (idle, or a landed
    /// fish being held up, which cannot be lost).
    pub fn cancel(&mut self, map: &Map, seat: usize, rod: P, events: &mut Vec<MatchEvent>) -> bool {
        match self.lines.get(seat).map(|l| l.stage) {
            None | Some(Stage::Idle) | Some(Stage::Landed) => false,
            Some(_) => {
                events.extend(self.end_line(map, seat, rod, false));
                true
            }
        }
    }

    /// A wreck: the fish on the line is lost, and the line comes in.
    pub fn wrecked(&mut self, map: &Map, seat: usize, rod: P, events: &mut Vec<MatchEvent>) {
        if self.lines.get(seat).is_some_and(|l| l.stage != Stage::Landed) {
            events.extend(self.end_line(map, seat, rod, false));
        }
    }

    /// A ram's wreck: as [`Fishing::wrecked`], but the fish on the line
    /// swims `rammed_swim_mm` (by its tier) off, so the rammer cannot land
    /// it at once (`balance.rammed_swim_mm`).
    pub fn rammed(&mut self, map: &Map, seat: usize, rod: P, events: &mut Vec<MatchEvent>) {
        let hooked = self
            .lines
            .get(seat)
            .filter(|l| l.stage == Stage::Reeling)
            .and_then(|l| l.fish);
        self.wrecked(map, seat, rod, events);
        if let Some(fish) = hooked {
            let swim = self.rammed_swim_mm.get(self.fish[fish].tier);
            if let FishState::Fleeing { .. } = self.fish[fish].state {
                self.fish[fish].state = FishState::Roaming;
                self.free_by(map, fish, rod, swim);
            }
        }
    }

    /// **Hook for the items system** (High-Quality Camera): raises the
    /// seat's last catch to `camera_value_pct` of its value, once per
    /// catch. False with no catch, or one already raised.
    pub fn use_camera(&mut self, seat: usize, score: &mut Score) -> bool {
        let Some(n) = score.catches.len().checked_sub(1) else {
            return false;
        };
        if self.photographed.contains(&(seat, n)) {
            return false;
        }
        self.photographed.push((seat, n));
        let c = &mut score.catches[n];
        c.value = (c.value as i64 * self.tuning.camera_value_pct as i64 / 100) as i32;
        true
    }

    /// One tick of fishing, after the boats have moved.
    pub fn step(
        &mut self,
        w: &World,
        inputs: &[SeatInput],
        scores: &mut [Score],
        events: &mut Vec<MatchEvent>,
    ) {
        let tick = w.tick;
        lake::roam(
            w.map,
            &self.fish_tuning,
            &mut self.rng,
            tick,
            &mut self.fish,
            &mut self.schools,
        );
        for seat in 0..self.lines.len() {
            if w.remote.get(seat).copied().unwrap_or(false) {
                continue;
            }
            let none = SeatInput::default();
            let i = inputs.get(seat).copied().unwrap_or(none);
            let Some(b) = w.boats.get(seat).and_then(Option::as_ref) else {
                continue;
            };
            let rod = bow(b, w.half_length_mm, w.half_beam_mm);
            if b.wrecked() {
                continue;
            }
            // Q to driving mode drops the fish and brings the line in
            if b.mode == Mode::Driving {
                if !matches!(self.lines[seat].stage, Stage::Idle | Stage::Landed) {
                    events.extend(self.end_line(w.map, seat, rod, false));
                }
                if self.lines[seat].stage == Stage::Idle {
                    continue;
                }
            }
            // the Fish-Boy or a minigame has the keys
            let i = if b.captured { none } else { i };
            self.seat_step(w, seat, b, rod, &i, scores, events);
        }
        self.move_engaged(w);
    }

    #[allow(clippy::too_many_arguments)]
    fn seat_step(
        &mut self,
        w: &World,
        seat: usize,
        b: &Boat,
        rod: P,
        i: &SeatInput,
        scores: &mut [Score],
        events: &mut Vec<MatchEvent>,
    ) {
        let tick = w.tick;
        let t = &self.tuning;
        let speed = b.speed_mm_per_s();
        let press = i.pressed.cast;
        let line = &mut self.lines[seat];
        match line.stage {
            Stage::Idle => {
                if press {
                    if speed > t.cast_max_speed_mm_per_s {
                        events.push(MatchEvent::CastRefused {
                            seat,
                            speed_mm_per_s: speed,
                        });
                    } else {
                        line.stage = Stage::Charging;
                        line.since = tick;
                        events.push(MatchEvent::Charging { seat });
                        if !i.held.cast {
                            self.release(w, seat, b, rod, events);
                        }
                    }
                }
            }
            Stage::Charging => {
                if !i.held.cast {
                    self.release(w, seat, b, rod, events);
                }
            }
            Stage::Flying => {
                if tick >= line.lands {
                    line.stage = Stage::Sinking;
                    line.bobber = line.to;
                    line.lure_depth_mm = 0;
                    events.push(MatchEvent::BobberLanded {
                        seat,
                        distance_mm: line.length_mm,
                    });
                }
            }
            Stage::Sinking | Stage::Set => {
                if self.drag(w, seat, rod, speed, events) {
                    return;
                }
                let line = &mut self.lines[seat];
                let t = &self.tuning;
                if line.stage == Stage::Sinking {
                    let bed = lake::bed_mm(w.map, line.bobber) - t.bed_clear_mm;
                    line.lure_depth_mm += lake::per_tick(t.sink_mm_per_s);
                    if press || line.lure_depth_mm >= bed {
                        line.lure_depth_mm = line.lure_depth_mm.min(bed).max(0);
                        line.stage = Stage::Set;
                        events.push(MatchEvent::LureStopped {
                            seat,
                            depth_mm: line.lure_depth_mm,
                        });
                    }
                    return;
                }
                self.set_step(w, seat, rod, press, events);
            }
            Stage::Reeling => {
                if speed > t.drag_snap_mm_per_s {
                    events.extend(self.end_line(w.map, seat, rod, true));
                    return;
                }
                if self.reel_held.get(seat).copied().unwrap_or(false) {
                    return;
                }
                let power = self.reel_power_pct[seat];
                let line = &mut self.lines[seat];
                let Some(reel) = line.reel.as_mut() else {
                    return;
                };
                let lift = i.down("cast");
                let out = reel.step(&t.reel, &mut self.rng, tick, lift, power);
                if lift && tick >= line.next_click {
                    line.next_click = tick + ticks(t.reel.tick_ms);
                    events.push(MatchEvent::ReelTick { seat });
                }
                match out {
                    Outcome::Fighting => {}
                    Outcome::Broke => events.extend(self.end_line(w.map, seat, rod, true)),
                    Outcome::Landed => self.land(w, seat, scores, events),
                }
            }
            Stage::Landed => {
                if tick >= line.lands {
                    *line = Line::idle();
                }
            }
        }
    }

    /// Space let go: the cast, as far as the charge reached.
    fn release(&mut self, w: &World, seat: usize, b: &Boat, rod: P, events: &mut Vec<MatchEvent>) {
        let t = &self.tuning;
        let line = &mut self.lines[seat];
        if b.speed_mm_per_s() > t.cast_max_speed_mm_per_s {
            *line = Line::idle();
            events.push(MatchEvent::CastRefused {
                seat,
                speed_mm_per_s: b.speed_mm_per_s(),
            });
            return;
        }
        let power = charge_permille(t, w.tick - line.since);
        let reach = t.cast_min_mm + ((t.cast_max_mm - t.cast_min_mm) as i64 * power as i64 / 1000) as i32;
        let way = b.heading + b.cast_yaw;
        // the last water short of land, in quarter-metre steps back
        let mut d = reach;
        let mut to = lake::ahead(rod, way, d);
        while d > 0 && !(w.map.wet(to) && lake::bed_mm(w.map, to) > t.bed_clear_mm) {
            d = (d - 250).max(0);
            to = lake::ahead(rod, way, d);
        }
        line.stage = Stage::Flying;
        line.from = rod;
        line.to = to;
        line.bobber = rod;
        line.flew = w.tick;
        line.lands = w.tick + ticks(t.cast_flight_ms).max(1);
        line.length_mm = d;
        events.push(MatchEvent::Cast { seat, distance_mm: d });
    }

    /// The boat drags the bobber; too fast, the line comes in. True when it
    /// did.
    fn drag(&mut self, w: &World, seat: usize, rod: P, speed: i32, events: &mut Vec<MatchEvent>) -> bool {
        if speed > self.tuning.drag_reel_in_mm_per_s {
            events.extend(self.end_line(w.map, seat, rod, false));
            return true;
        }
        let line = &mut self.lines[seat];
        let len = line.length_mm as i64;
        if line.bobber.dist(rod) > len {
            let pulled = line.bobber.dist(rod) - len;
            line.bobber = lake::toward(line.bobber, rod, pulled as i32);
        }
        false
    }

    /// The lure waits: a fish comes, nibbles and bites.
    fn set_step(&mut self, w: &World, seat: usize, rod: P, press: bool, events: &mut Vec<MatchEvent>) {
        let tick = w.tick;
        let line = self.lines[seat].clone();
        let Some(mut int) = line.interest else {
            if press {
                events.extend(self.end_line(w.map, seat, rod, false));
                return;
            }
            if let Some(f) = self.notice(&line) {
                self.fish[f].state = FishState::Lured { seat };
                self.lines[seat].interest = Some(Interest {
                    fish: f,
                    arrived: false,
                    next: 0,
                    nibbles_left: self.rng.range(self.tuning.nibbles_min, self.tuning.nibbles_max),
                    biting_until: None,
                });
            }
            return;
        };
        let t = &self.tuning;
        if press {
            match int.biting_until {
                Some(until) if tick <= until => self.hook(w, seat, events),
                // Space before the bite scares the fish off
                _ => {
                    self.lines[seat].interest = None;
                    self.free(w.map, int.fish, rod);
                    events.push(MatchEvent::Scared { seat });
                }
            }
            return;
        }
        if let Some(until) = int.biting_until {
            if tick >= until {
                self.lines[seat].interest = None;
                self.free(w.map, int.fish, rod);
                events.push(MatchEvent::Missed { seat });
            }
            return;
        }
        if !int.arrived {
            let f = &self.fish[int.fish];
            let near = f.at.dist(line.bobber) <= 300 && (f.depth_mm - line.lure_depth_mm).abs() <= 300;
            if near {
                int.arrived = true;
                int.next = tick + ticks(self.rng.range(t.wait_min_ms, t.wait_max_ms));
            }
        } else if tick >= int.next {
            if int.nibbles_left > 0 {
                int.nibbles_left -= 1;
                int.next = tick + ticks(self.rng.range(t.nibble_gap_min_ms, t.nibble_gap_max_ms));
                events.push(MatchEvent::Nibble { seat });
            } else {
                int.biting_until = Some(tick + ticks(t.hook_window_ms));
                events.push(MatchEvent::Bite { seat });
            }
        }
        self.lines[seat].interest = Some(int);
    }

    /// The roaming fish nearest the lure that would come to it, if any:
    /// within the attraction radius, which is full when the fish's depth
    /// matches the lure's and shrinks as they differ.
    fn notice(&self, line: &Line) -> Option<usize> {
        let t = &self.tuning;
        let mut best: Option<(i64, usize)> = None;
        for (k, f) in self.fish.iter().enumerate() {
            if !f.roaming() {
                continue;
            }
            let r = attraction_mm(t, (f.depth_mm - line.lure_depth_mm).abs()) as i64;
            let d2 = f.at.dist2(line.bobber);
            if d2 <= r * r && best.is_none_or(|(b, _)| d2 < b) {
                best = Some((d2, k));
            }
        }
        best.map(|(_, k)| k)
    }

    fn hook(&mut self, w: &World, seat: usize, events: &mut Vec<MatchEvent>) {
        let Some(int) = self.lines[seat].interest.take() else {
            return;
        };
        let f = &mut self.fish[int.fish];
        f.state = FishState::Hooked { seat };
        let tier = f.tier;
        let (at, depth) = (f.at, f.depth_mm);
        let kind = self.roster.0.get(f.kind).map_or(String::new(), |k| k.id.clone());
        let tame = std::mem::take(&mut self.tame);
        let line = &mut self.lines[seat];
        line.stage = Stage::Reeling;
        line.reel = Some(Reel::new(tier, &self.tuning.reel, tame));
        line.fish = Some(int.fish);
        line.hooked_at = at;
        line.hooked_depth_mm = depth;
        line.next_click = w.tick;
        events.push(MatchEvent::Hooked {
            seat,
            fish: int.fish as u32,
            kind,
            tier,
        });
    }

    fn land(&mut self, w: &World, seat: usize, scores: &mut [Score], events: &mut Vec<MatchEvent>) {
        let Some(fish) = self.lines[seat].fish else {
            return;
        };
        let f = &mut self.fish[fish];
        f.state = FishState::Caught { seat };
        let Some(k) = self.roster.0.get(f.kind) else {
            return;
        };
        let worth = value(
            k.value,
            k.base_length_mm,
            f.length_mm,
            self.tuning.tier_value_pct.get(f.tier),
        );
        if let Some(s) = scores.get_mut(seat) {
            s.catches.push(Catch {
                kind: k.id.clone(),
                length_mm: f.length_mm,
                value: worth,
            });
        }
        events.push(MatchEvent::Caught {
            seat,
            fish: fish as u32,
            kind: k.id.clone(),
            tier: f.tier,
            length_mm: f.length_mm,
        });
        let line = &mut self.lines[seat];
        line.stage = Stage::Landed;
        line.reel = None;
        line.lands = w.tick + ticks(self.tuning.landed_ms);
    }

    /// The fish coming to a lure, and the fish on a line, after the lines'
    /// tick: a lured fish swims to the lure; a hooked one is pulled toward
    /// the surface and the boat as the meter fills, swerving with its
    /// marker, and the bobber and lure go with it.
    fn move_engaged(&mut self, w: &World) {
        let approach = lake::per_tick(self.tuning.approach_speed_mm_per_s);
        for seat in 0..self.lines.len() {
            if w.remote.get(seat).copied().unwrap_or(false) {
                continue;
            }
            let line = &self.lines[seat];
            if let Some(int) = line.interest {
                let (to, depth) = (line.bobber, line.lure_depth_mm);
                let f = &mut self.fish[int.fish];
                if int.biting_until.is_some() {
                    // the bite: it strikes the lure and holds it
                    strike::bite(f, to, &self.tuning.strike);
                } else if f.at.dist(to) > 250 {
                    // it stops a nose short of the lure
                    lake::swim(f, to, approach);
                }
                f.depth_mm = crate::gfx::fx::approach(f.depth_mm, depth, approach);
            }
            let (Some(fish), Some(reel)) = (line.fish, line.reel.as_ref()) else {
                continue;
            };
            let Some(b) = w.boats.get(seat).and_then(Option::as_ref) else {
                continue;
            };
            let rod = bow(b, w.half_length_mm, w.half_beam_mm);
            let got = reel.meter as i64;
            let full = reel::FULL as i64;
            // most of the way to the boat when landed, a little each tick
            let k = got * 4 / 5;
            let lerp = |a: i32, b: i32| a + ((b - a) as i64 * k / full) as i32;
            let base = P::new(lerp(line.hooked_at.x, rod.x), lerp(line.hooked_at.y, rod.y));
            // the struggle: sideways by the marker's place on the bar
            let across = crate::map::trig::atan2((rod.y - base.y) as i64, (rod.x - base.x) as i64)
                + crate::map::trig::QUARTER;
            let swerve = ((reel.marker - reel::BAR / 2) as i64 * 3000 / reel::BAR as i64) as i32;
            let to = lake::ahead(base, across, swerve);
            let surface = self.tuning.reel.surface_depth_mm;
            let depth = line.hooked_depth_mm + ((surface - line.hooked_depth_mm) as i64 * got / full) as i32;
            let f = &mut self.fish[fish];
            lake::swim(f, to, lake::per_tick(self.fish_tuning.freed_speed_mm_per_s));
            f.depth_mm = depth;
            let (at, d) = (f.at, f.depth_mm);
            let line = &mut self.lines[seat];
            line.bobber = at;
            line.lure_depth_mm = d;
        }
    }
}

/// The charge, 0 to 1000, after holding Space for `held` ticks.
pub fn charge_permille(t: &FishingTuning, held: u32) -> i32 {
    let full = ticks(t.charge_full_ms).max(1);
    (held.min(full) as i64 * 1000 / full as i64) as i32
}

/// The attraction radius for a fish this far above or below the lure.
pub fn attraction_mm(t: &FishingTuning, depth_gap_mm: i32) -> i32 {
    let gap = depth_gap_mm.min(t.attract_depth_mm) as i64;
    let pct = 100 - (100 - t.attract_far_pct) as i64 * gap / t.attract_depth_mm as i64;
    (t.attract_radius_mm as i64 * pct / 100) as i32
}

/// The fish's tier, as `data/strings.json` names it (`tier.{key}`).
pub fn tier_key(t: Tier) -> &'static str {
    crate::roster::tier_key(t)
}
