//! The AI players (PLAN.md decision 10): each AI seat's [`Brain`] reads
//! only what its seat could know ([`senses`]) and makes the same
//! [`SeatInput`] a person would, so it plays by the same rules.
//!
//! A simple utility AI. Each tick a brain:
//!
//! 1. **remembers** what it senses: rivals in sight or earshot, its own
//!    ping's sightings, hook and wreck popups, its own line's outcomes;
//! 2. **decides** its [`Goal`] ([`Brain::decide`]): flee a rival coming
//!    fast while it fishes ([`hunt::should_flee`]), hunt a rival who
//!    hooked a fish worth the trip ([`hunt::worth_hunting`]), fish the best
//!    fish in sight for the time it would take ([`fish::choose`]), or
//!    explore the lake for fish;
//! 3. **acts**: drives a path over the map's grid ([`nav`]), casts, stops
//!    the lure at the fish's depth, hooks after a reaction delay, plays the
//!    reel bar ([`fish`]), rams at speed ([`hunt`]), raises the Fish-Boy now
//!    and then to ping, goes for the item buoy when the trip is worth it,
//!    uses its item at its moment and plays the item's minigame with its
//!    hand on the pointer ([`items`], [`hand`]).
//!
//! The difficulty (`data/tuning.json` `ai.skill`) sets the reaction, the
//! reel's lag and noise, the cast's aim, the aggression, the evasion and
//! how often it pings. Every chance is drawn from the match seed, so a
//! match with the same seed and the same human inputs plays the same.
//!
//! [`Crew`] holds every AI seat's brain and the shared path grid; the app
//! asks it for the AI seats' inputs each tick ([`Crew::inputs`]).

pub mod fish;
pub mod hand;
pub mod hunt;
pub mod items;
pub mod nav;
pub mod senses;
pub mod tuning;

#[cfg(test)]
mod item_tests;
#[cfg(test)]
mod tests;

use crate::map::rng::Rng;
use crate::map::P;
use crate::sim::fishing::Stage;
use crate::sim::input::SeatInput;
use crate::sim::{ticks, Boat, Difficulty, Match, Mode, Occupant};

use nav::Nav;
use senses::{News, Senses};
use tuning::Skill;

/// What a brain is after.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Goal {
    /// driving about the lake looking for fish
    Explore { to: P },
    /// this fish (its id), seen
    Fish { fish: u32 },
    /// ramming this seat before its reel ends
    Hunt { seat: usize, until: u32 },
    /// away from a rival, until this tick
    Flee { to: P, until: u32 },
    /// through the item buoy, there
    Buoy { at: P },
}

/// How a brain spends a tick, for the playtests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    Fishing,
    Hunting,
    Driving,
    Idle,
}

/// A rival's hook, as the popup told it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookNews {
    pub tick: u32,
    pub kind: String,
    pub tier: crate::map::fish::Tier,
    /// the hunt was weighed
    pub decided: bool,
}

/// The reel hand: what it last saw of the bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReelHand {
    marks: [i32; 64],
    seen: u32,
    zone_prev: i32,
    offset: i32,
    offset_until: u32,
}

impl Default for ReelHand {
    fn default() -> ReelHand {
        ReelHand {
            marks: [500; 64],
            seen: 0,
            zone_prev: -1,
            offset: 0,
            offset_until: 0,
        }
    }
}

/// One AI seat.
#[derive(Debug, Clone)]
pub struct Brain {
    pub seat: usize,
    pub difficulty: Difficulty,
    pub skill: Skill,
    rng: Rng,
    pub goal: Goal,
    pub activity: Activity,
    path: Vec<P>,
    path_goal: P,
    path_tick: u32,
    // fishing
    /// the cast's Space is let go on this tick
    cast_release: Option<u32>,
    /// this cast's miss of the aim, once drawn
    aim: Option<i32>,
    /// the depth to stop the lure at
    lure_depth: i32,
    hook_at: Option<u32>,
    lure_set: Option<u32>,
    reeling_in: bool,
    /// fish it failed at, and how often
    tries: Vec<(u32, i32)>,
    /// ticks it has been near its stop point without a cast
    stalled: u32,
    reel: ReelHand,
    // the radar
    next_ping: u32,
    ping_until: Option<u32>,
    // rivals, by seat
    known: Vec<Option<(P, [i32; 2], u32)>>,
    pub hooks: Vec<Option<HookNews>>,
    threat_rolled: Vec<bool>,
    // driving
    slow_since: Option<u32>,
    reverse_until: u32,
    /// a ram's run-up point, after a run-in too slow
    runup: Option<P>,
    /// when each 64 m square of the lake was last passed
    explored: Vec<u32>,
    /// the items: its hand on the minigames, and its plans
    pub items: items::Mind,
}

/// The explored squares' side.
const EXPLORE_CELL_MM: i32 = 64_000;

impl Brain {
    pub fn new(seat: usize, difficulty: Difficulty, m: &Match) -> Brain {
        let skill = m.tuning.ai.skill.get(difficulty).clone();
        let mut rng = Rng::stream(m.setup.seed, 0xA1 + seat as u64);
        let first_ping = ticks(skill.ping_every_ms / 2 + rng.range(0, skill.ping_every_ms / 2));
        let n = m.setup.seats.len();
        let side = (m.map.size_mm() / EXPLORE_CELL_MM + 1) as usize;
        Brain {
            seat,
            difficulty,
            skill,
            rng,
            goal: Goal::Explore { to: m.map.spawn.at },
            activity: Activity::Idle,
            path: Vec::new(),
            path_goal: P::new(i32::MIN / 2, 0),
            path_tick: 0,
            cast_release: None,
            aim: None,
            lure_depth: 0,
            hook_at: None,
            lure_set: None,
            reeling_in: false,
            tries: Vec::new(),
            stalled: 0,
            reel: ReelHand::default(),
            next_ping: first_ping,
            ping_until: None,
            known: vec![None; n],
            hooks: vec![None; n],
            threat_rolled: vec![false; n],
            slow_since: None,
            reverse_until: 0,
            runup: None,
            explored: vec![0; side * side],
            items: items::Mind::new(m.setup.seed, seat),
        }
    }

    fn tried(&self, fish: u32) -> i32 {
        self.tries.iter().find(|(f, _)| *f == fish).map_or(0, |t| t.1)
    }

    fn try_failed(&mut self, fish: u32) {
        match self.tries.iter_mut().find(|(f, _)| *f == fish) {
            Some(t) => t.1 += 1,
            None => self.tries.push((fish, 1)),
        }
    }

    /// Forgets the line's plans (after a wreck, or a line that came in).
    fn reset_line(&mut self) {
        self.cast_release = None;
        self.hook_at = None;
        self.lure_set = None;
        self.reeling_in = false;
        self.reel = ReelHand::default();
    }

    /// One tick: the input for the next step.
    pub fn think(&mut self, s: &Senses, nav: &Nav) -> SeatInput {
        let mut out = SeatInput::default();
        let Some(b) = s.me else {
            return out;
        };
        if s.ended {
            return out;
        }
        self.remember(s);
        if b.wrecked() {
            self.activity = Activity::Idle;
            self.reset_line();
            self.path.clear();
            self.runup = None;
            if !matches!(self.goal, Goal::Explore { .. }) {
                self.goal = Goal::Explore { to: s.map.spawn.at };
            }
            return out;
        }
        // a minigame, or a teleport's spot on the radar: the hand
        if let Some(i) = items::during(self, s) {
            self.activity = self.activity_now(s, b, &i);
            return i;
        }
        if s.fishboy_up {
            self.activity = if s.fishing.stage == Stage::Idle {
                Activity::Idle
            } else {
                Activity::Fishing
            };
            let tug = s.fishing.biting || s.fishing.lines.iter().any(|l| l.seat == s.seat && l.tug);
            if tug || self.ping_until.is_none_or(|t| s.tick >= t) {
                out.pressed.fishboy = true;
                self.ping_until = None;
            }
            return out;
        }
        self.decide(s, nav, b);
        if self.want_ping(s) {
            out.pressed.fishboy = true;
            out.pressed.ping = true;
            self.ping_until = Some(s.tick + ticks(s.tuning.radar.ping_sweep_ms));
            self.next_ping = s.tick + ticks(self.skill.ping_every_ms);
            return out;
        }
        match self.goal {
            Goal::Hunt { seat, .. } => hunt::chase(self, s, nav, b, seat, &mut out),
            Goal::Flee { to, .. } => {
                if b.mode == Mode::Fishing {
                    out.pressed.mode = true;
                } else {
                    self.drive_to(s, nav, b, to, s.rules.top, false, &mut out);
                }
            }
            Goal::Fish { fish } => fish::act(self, s, nav, b, fish, &mut out),
            Goal::Buoy { at } => {
                if b.mode == Mode::Fishing {
                    // a quiet line comes in
                    out.pressed.mode = true;
                } else {
                    self.drive_to(s, nav, b, at, s.rules.top, false, &mut out);
                }
            }
            Goal::Explore { to } => {
                if b.mode == Mode::Fishing && s.fishing.stage == Stage::Idle {
                    out.pressed.mode = true;
                } else if b.mode == Mode::Fishing {
                    // a line still out (a landed fish held up): see it through
                    fish::tend_line(self, s, b, &mut out);
                } else {
                    self.drive_to(s, nav, b, to, s.rules.top, false, &mut out);
                }
            }
        }
        items::decide(self, s, b, &mut out);
        self.activity = self.activity_now(s, b, &out);
        out
    }

    fn activity_now(&self, s: &Senses, b: &Boat, out: &SeatInput) -> Activity {
        if b.mode == Mode::Fishing || s.fishing.stage != Stage::Idle {
            return Activity::Fishing;
        }
        if matches!(self.goal, Goal::Hunt { .. }) {
            return Activity::Hunting;
        }
        if b.speed_mm_per_s() < 500 && !out.held.forward && !out.held.back {
            return Activity::Idle;
        }
        Activity::Driving
    }

    /// Takes in this tick's senses.
    fn remember(&mut self, s: &Senses) {
        for r in &s.rivals {
            self.known[r.seat] = Some((r.at, r.vel, s.tick));
        }
        for p in &s.pinged {
            if self.known[p.seat].is_none_or(|k| k.2 < p.tick) {
                self.known[p.seat] = Some((p.at, [0, 0], p.tick));
            }
        }
        if let Some(b) = s.me {
            let side = (s.map.size_mm() / EXPLORE_CELL_MM + 1) as usize;
            let at = b.at();
            let (x, y) = (
                (at.x.max(0) / EXPLORE_CELL_MM) as usize,
                (at.y.max(0) / EXPLORE_CELL_MM) as usize,
            );
            if x < side && y < side {
                self.explored[y * side + x] = s.tick.max(1);
            }
        }
        for n in &s.news {
            match n {
                News::Hooked {
                    seat,
                    kind,
                    tier,
                    ago_ms,
                } => {
                    self.hooks[*seat] = Some(HookNews {
                        // when the hook was, by the popup's known delay
                        tick: s.tick.saturating_sub(ticks(*ago_ms)),
                        kind: kind.clone(),
                        tier: *tier,
                        decided: false,
                    });
                }
                News::Wrecked { seat, .. } => {
                    if *seat < self.hooks.len() {
                        self.hooks[*seat] = None;
                    }
                    if let Goal::Hunt { seat: target, .. } = self.goal {
                        if target == *seat || *seat == self.seat {
                            self.goal = Goal::Explore { to: s.map.spawn.at };
                            self.runup = None;
                        }
                    }
                }
                News::Lost => {
                    if let Goal::Fish { fish } = self.goal {
                        self.try_failed(fish);
                    }
                    self.reset_line();
                }
                News::Caught => {
                    self.reset_line();
                    self.goal = Goal::Explore { to: s.map.spawn.at };
                }
            }
        }
    }

    /// Whether to raise the Fish-Boy and ping now: a rival's hook worth
    /// weighing with the rival nowhere known, or its usual ping while the
    /// lure waits quietly or it drives about.
    fn want_ping(&self, s: &Senses) -> bool {
        if s.ping != crate::fishboy::radar::PingState::Ready {
            return false;
        }
        let stage = s.fishing.stage;
        if matches!(
            stage,
            Stage::Charging | Stage::Flying | Stage::Reeling | Stage::Sinking
        ) {
            return false;
        }
        let tug = s.fishing.biting || s.fishing.lines.iter().any(|l| l.seat == s.seat && l.tug);
        if tug {
            return false;
        }
        // the boat coasts with the Fish-Boy up: only with room to coast
        if let Some(b) = s.me {
            let v = b.speed_mm_per_s() as i64;
            let coast = (s.rules.coast * crate::sim::TICKS_PER_S).max(1) as i64;
            let room = (v * v / (2 * coast)) as i32 + 10_000;
            let v2 = b.vel();
            let way = if v > 500 {
                crate::map::trig::atan2(v2[1] as i64, v2[0] as i64)
            } else {
                b.heading
            };
            if v > 500 && nav::land_ahead(s.map, b.at(), way, room) < room {
                return false;
            }
        }
        let keep = ticks(s.tuning.ai.sighting_keep_ms);
        let need = self.hooks.iter().enumerate().any(|(seat, h)| {
            h.as_ref().is_some_and(|h| {
                !h.decided && self.known[seat].is_none_or(|k| k.2 + keep < s.tick || k.2 < h.tick)
            })
        });
        if need && stage == Stage::Idle {
            return true;
        }
        let quiet = match self.goal {
            Goal::Explore { .. } => stage == Stage::Idle,
            Goal::Fish { .. } => stage == Stage::Set && !fish::fish_near_lure(s, 2500),
            _ => false,
        };
        quiet && s.tick >= self.next_ping
    }

    /// The goal for this tick.
    fn decide(&mut self, s: &Senses, nav: &Nav, b: &Boat) {
        let t = &s.tuning.ai;
        let stage = s.fishing.stage;
        let at = b.at();
        // a goal that ran out
        match self.goal {
            Goal::Hunt { seat, until } => {
                let off = s
                    .rivals
                    .iter()
                    .find(|r| r.seat == seat)
                    .is_some_and(|r| r.line != Some(Stage::Reeling) || r.ghosted);
                if s.tick >= until || off {
                    self.goal = Goal::Explore { to: at };
                    self.runup = None;
                    self.hooks[seat] = None;
                }
            }
            Goal::Flee { until, .. } if s.tick >= until => self.goal = Goal::Explore { to: at },
            // taken, or moved to a new fair point
            Goal::Buoy { at: buoy } if s.items.buoy != Some(buoy) => {
                self.goal = Goal::Explore { to: at };
                self.path.clear();
            }
            _ => {}
        }
        // a rival bearing down while the line is out
        if !matches!(self.goal, Goal::Flee { .. } | Goal::Hunt { .. }) && stage != Stage::Idle {
            for r in &s.rivals {
                match hunt::should_flee(s, b, r) {
                    Some(from) if !self.threat_rolled[r.seat] => {
                        self.threat_rolled[r.seat] = true;
                        if self.rng.chance(self.skill.evade_pct as u32 * 10) {
                            let to = hunt::flee_point(s, nav, at, from);
                            self.goal = Goal::Flee {
                                to,
                                until: s.tick + ticks(t.flee_ms),
                            };
                            self.path.clear();
                            return;
                        }
                    }
                    Some(_) => {}
                    None => self.threat_rolled[r.seat] = false,
                }
            }
        }
        // a rival's hook worth the trip (not with a fish at its own lure)
        let busy = stage == Stage::Reeling
            || (stage == Stage::Set && (s.fishing.biting || fish::fish_near_lure(s, 2500)));
        if !matches!(self.goal, Goal::Hunt { .. } | Goal::Flee { .. }) && !busy {
            for seat in 0..self.hooks.len() {
                let Some(h) = self.hooks[seat].clone() else {
                    continue;
                };
                let expire = h.tick + ticks(t.rival_reel_ms.get(h.tier) * 2);
                if s.tick > expire {
                    self.hooks[seat] = None;
                    continue;
                }
                if h.decided {
                    continue;
                }
                let keep = ticks(t.sighting_keep_ms);
                let Some((where_, _, seen)) = self.known[seat].filter(|k| k.2 + keep >= s.tick) else {
                    // unknown: wait for a ping, unless one cannot come
                    if s.ping != crate::fishboy::radar::PingState::Ready && s.tick > h.tick + ticks(6000) {
                        if let Some(h) = self.hooks[seat].as_mut() {
                            h.decided = true;
                        }
                    }
                    continue;
                };
                // a sighting older than the hook only counts once a ping has had its chance
                if seen < h.tick && s.tick < h.tick + ticks(s.tuning.radar.ping_sweep_ms + 500) {
                    continue;
                }
                if let Some(n) = self.hooks[seat].as_mut() {
                    n.decided = true;
                }
                let since = ((s.tick - h.tick) as i64 * crate::sim::TICK_MS as i64) as i32;
                if hunt::worth_hunting(s, &self.skill, at, where_, &h.kind, h.tier, since) {
                    let until = h.tick + ticks(t.rival_reel_ms.get(h.tier) * 13 / 10);
                    self.goal = Goal::Hunt { seat, until };
                    self.path.clear();
                    self.runup = None;
                    return;
                }
            }
        }
        // the item buoy, when worth the trip
        if let Some(buoy) = s.items.buoy {
            if !matches!(self.goal, Goal::Buoy { .. }) && items::buoy_worth(self, s, b, buoy) {
                self.goal = Goal::Buoy { at: buoy };
                self.path.clear();
                return;
            }
        }
        // fish in sight
        if matches!(self.goal, Goal::Explore { .. } | Goal::Fish { .. }) && stage == Stage::Idle {
            let current = match self.goal {
                Goal::Fish { fish } => Some(fish),
                _ => None,
            };
            match fish::choose(self, s, at, current) {
                Some(f) => {
                    if current != Some(f) {
                        self.stalled = 0;
                        self.path.clear();
                    }
                    self.goal = Goal::Fish { fish: f };
                }
                None => {
                    // a fish an Enhanced Radar marked
                    if let Some(to) = items::mark_to_visit(s, at) {
                        if !matches!(self.goal, Goal::Explore { to: g } if g == to) {
                            self.goal = Goal::Explore { to };
                            self.path.clear();
                        }
                        return;
                    }
                    if current.is_some()
                        || matches!(self.goal, Goal::Explore { to } if to.dist(at) < t.waypoint_mm as i64 * 2)
                    {
                        let to = self.explore_point(s, nav, at);
                        self.goal = Goal::Explore { to };
                        self.path.clear();
                    }
                }
            }
        }
    }

    /// Somewhere to look for fish: an open spot of water, far from where
    /// it has lately been.
    fn explore_point(&mut self, s: &Senses, nav: &Nav, at: P) -> P {
        let side = (s.map.size_mm() / EXPLORE_CELL_MM + 1) as usize;
        let mut best: Option<(i64, P)> = None;
        for _ in 0..24 {
            let c = self.rng.below((nav.n * nav.n) as u32) as usize;
            if !nav.open[c] {
                continue;
            }
            let p = nav.centre(c);
            let d = p.dist(at);
            if d < 50_000 {
                continue;
            }
            let (x, y) = ((p.x / EXPLORE_CELL_MM) as usize, (p.y / EXPLORE_CELL_MM) as usize);
            let last = self.explored.get(y * side + x).copied().unwrap_or(0);
            let age = (s.tick - last.min(s.tick)).min(ticks(240_000)) as i64;
            // stale squares first, then nearer ones
            let score = age * 1000 - d / 2 + self.rng.below(20_000) as i64;
            if best.is_none_or(|(b, _)| score > b) {
                best = Some((score, p));
            }
        }
        best.map_or(s.map.spawn.at, |(_, p)| p)
    }

    /// Drives toward `to` along a path, at up to `want` mm/s; with `stop`,
    /// slowing to stop there.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn drive_to(
        &mut self,
        s: &Senses,
        nav: &Nav,
        b: &Boat,
        to: P,
        want: i32,
        stop: bool,
        out: &mut SeatInput,
    ) {
        let t = &s.tuning.ai;
        let r = s.rules;
        let at = b.at();
        if s.tick < self.reverse_until {
            out.held.back = true;
            out.held.left = true;
            return;
        }
        let moved = self.path_goal.dist(to) > 10_000;
        // a fresh path when the goal moves, or now and then when the next
        // waypoint is no longer in clear water from here (a bounce, a dodge)
        let stale = s.tick >= self.path_tick + ticks(3000)
            && self
                .path
                .first()
                .is_some_and(|&w| !nav::clear_line(s.map, at, w, 0));
        if moved || self.path.is_empty() || stale {
            self.path = nav.path(s.map, at, to).unwrap_or_else(|| vec![to]);
            self.path_goal = to;
            self.path_tick = s.tick;
        }
        while self.path.len() > 1 && self.path[0].dist(at) < t.waypoint_mm as i64 {
            self.path.remove(0);
        }
        if self.path.len() > 1
            && s.tick % 10 == self.seat as u32
            && nav::clear_line(s.map, at, self.path[1], t.path_roomy_mm)
        {
            self.path.remove(0);
        }
        let next = self.path[0];
        // nitrous: the boat's top speed is higher for its time
        let mut want = if b.nitrous && want >= r.top {
            nav::pct(r.top, r.nitrous_top_pct)
        } else {
            want
        };
        if stop {
            let left: i64 = self.path.windows(2).map(|w| w[0].dist(w[1])).sum::<i64>() + at.dist(next);
            want = want.min(nav::stop_speed(left as i32 - 3000, nav::decel(r, t), 0));
        }
        // slow before a sharp bend at the next waypoint
        if self.path.len() > 1 {
            let bend = crate::gfx::fx::wrap(nav::bearing(next, self.path[1]) - nav::bearing(at, next)).abs();
            let corner = if bend > crate::map::trig::TURN / 6 {
                nav::pct(r.top, 45)
            } else if bend > crate::map::trig::TURN / 12 {
                nav::pct(r.top, 70)
            } else {
                r.top
            };
            let d = (at.dist(next) as i32 - t.waypoint_mm).max(0);
            want = want.min(nav::stop_speed(d, nav::decel(r, t), corner));
        }
        nav::steer(s.map, r, t, b, next, want, false, out);
        // stuck against something: back off and look again
        let trying = want > 2000 && out.held.forward;
        if trying && b.speed_mm_per_s() < 600 {
            let since = *self.slow_since.get_or_insert(s.tick);
            if s.tick > since + ticks(1500) {
                self.reverse_until = s.tick + ticks(800);
                self.slow_since = None;
                self.path.clear();
            }
        } else {
            self.slow_since = None;
        }
    }
}

/// Every AI seat of a match and the shared path grid.
#[derive(Debug, Clone)]
pub struct Crew {
    pub nav: Nav,
    pub brains: Vec<Brain>,
}

impl Crew {
    /// A brain for each AI seat of `m`.
    pub fn new(m: &Match) -> Crew {
        let brains = m
            .setup
            .seats
            .iter()
            .enumerate()
            .filter_map(|(seat, s)| match s.occupant {
                Occupant::Ai(d) => Some(Brain::new(seat, d, m)),
                _ => None,
            })
            .collect();
        Crew {
            nav: Nav::new(&m.map, &m.tuning.ai),
            brains,
        }
    }

    /// One input per seat for the next step: the AI seats' own, and none
    /// for the rest (the app puts in the people's).
    pub fn inputs(&mut self, m: &Match) -> Vec<SeatInput> {
        let mut out = vec![SeatInput::default(); m.seats.len()];
        for b in self.brains.iter_mut() {
            let s = senses::sense(m, b.seat);
            out[b.seat] = b.think(&s, &self.nav);
        }
        out
    }

    /// The brain of a seat, if an AI plays it.
    pub fn brain(&self, seat: usize) -> Option<&Brain> {
        self.brains.iter().find(|b| b.seat == seat)
    }
}
