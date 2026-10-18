//! A simulated transport for the online tests: browsers ("nodes") joined by
//! links with latency, jitter and loss, on one millisecond clock.
//!
//! The real channel (`web/online.js`) is reliable and ordered, so a lost
//! packet is sent again and everything behind it on that link waits:
//! `loss_pct` models that as a resend after [`RESEND_MS`]. `drop_pct` drops
//! messages outright, for the parts of the protocol that must survive a
//! link failing under them (the lobby's repeats). Every chance comes from
//! a seed, so a failing run can be run again.

use std::collections::{BTreeMap, BTreeSet};

use super::session::Status;
use super::*;
use crate::map::rng::Rng;
use crate::map::{Map, Setting, Spawn, P};
use crate::sim::{Match, MatchEvent, SeatInput, TICK_MS};
use crate::tuning::Tuning;

/// A lost packet's resend, after this long (an SCTP retransmission).
pub const RESEND_MS: i64 = 200;
const CELL: i32 = 4000;

#[derive(Debug, Clone, Copy)]
pub struct LinkSpec {
    pub latency_ms: i64,
    pub jitter_ms: i64,
    pub loss_pct: u64,
    pub drop_pct: u64,
}

impl LinkSpec {
    pub const CLEAN: LinkSpec = LinkSpec {
        latency_ms: 30,
        jitter_ms: 0,
        loss_pct: 0,
        drop_pct: 0,
    };

    /// A poor home connection: 60 ms ± 40, 5% lost and resent.
    pub const ROUGH: LinkSpec = LinkSpec {
        latency_ms: 60,
        jitter_ms: 40,
        loss_pct: 5,
        drop_pct: 0,
    };
}

struct Packet {
    from: usize,
    to: usize,
    text: String,
    at: i64,
}

pub struct Node {
    pub id: String,
    pub session: Session,
    pub game: Option<Match>,
    /// held for this node's own seat every tick
    pub input: SeatInput,
    next_tick: i64,
    /// every event its match raised, with the tick
    pub log: Vec<(u32, MatchEvent)>,
    /// every verdict its systems saw
    pub verdicts: Vec<Verdict>,
    /// claims to make: (at this node's tick, claim)
    pub plans: Vec<(u32, Claim)>,
    /// inputs to press: (at this node's tick, the input)
    pub presses: Vec<(u32, SeatInput)>,
    pub gone: bool,
}

pub struct Loop {
    pub nodes: Vec<Node>,
    pub now: i64,
    pub spec: LinkSpec,
    rng: Rng,
    queue: Vec<Packet>,
    /// the last delivery time of each directed link, to keep it ordered
    last: BTreeMap<(usize, usize), i64>,
    /// pairs with no direct path (two joiners behind strict NATs)
    pub cut: BTreeSet<(usize, usize)>,
    pub tuning: Tuning,
    pub map: Map,
    /// the fish every browser stocks its lake with
    pub roster: Option<crate::roster::Roster>,
    /// the host's 3-2-1-Go
    pub countdown_ms: i32,
}

/// An open square lake, 160 m a side, with the spawn on the west side.
pub fn open_lake() -> Map {
    let n = 40;
    let water: Vec<bool> = (0..n * n)
        .map(|i| {
            let (x, y) = (i % n, i / n);
            x != 0 && y != 0 && x != n - 1 && y != n - 1
        })
        .collect();
    let at = P::new(20_000, 80_000);
    let spawn = Spawn {
        at,
        heading: 0,
        dock: P::new(0, at.y),
    };
    Map::from_water(Setting::MichiganLake, 0, n, CELL, &water, spawn)
}

pub fn setup_info(t: &Tuning, ai: usize) -> SetupInfo {
    SetupInfo {
        ai,
        difficulty: vec!["easy".into(), "normal".into(), "hard".into()],
        length_ms: t.game.length_ms,
        buoy_interval_ms: t.game.buoy_interval_ms,
        seed: String::new(),
    }
}

impl Loop {
    /// A host and `joiners` joiners, every link `spec`, chances from `seed`.
    pub fn new(joiners: usize, ai: usize, spec: LinkSpec, seed: u64) -> Loop {
        let tuning = crate::tuning::tests::sample();
        let hash = tuning_hash(&tuning);
        let seats = tuning.game.seats as usize;
        let mut nodes = vec![Node::new(
            "host",
            Session::host("Hana", &hash, seats, setup_info(&tuning, ai), 0),
        )];
        for k in 1..=joiners {
            nodes.push(Node::new(
                &format!("joiner{k}"),
                Session::join(&format!("Player {k}"), &hash, 0),
            ));
        }
        Loop {
            nodes,
            now: 0,
            spec,
            rng: Rng::new(seed),
            queue: Vec::new(),
            last: BTreeMap::new(),
            cut: BTreeSet::new(),
            tuning,
            map: open_lake(),
            roster: None,
            countdown_ms: 0,
        }
    }

    /// The joiners arrive one at a time, each seated (or refused) before
    /// the next, so joiner `k` sits in seat `k`: every link to the ones
    /// already there comes up (but the cut ones).
    pub fn connect(&mut self) {
        for k in 1..self.nodes.len() {
            for j in 0..k {
                if self.is_cut(j, k) {
                    continue;
                }
                let now = self.now;
                let (jid, kid) = (self.nodes[j].id.clone(), self.nodes[k].id.clone());
                self.nodes[k].session.peer_connected(&jid, j == 0, now);
                self.nodes[j].session.peer_connected(&kid, false, now);
            }
            self.run_until(3000, |l| {
                let s = &l.nodes[k].session;
                s.seat().is_some() || matches!(s.status(), Status::Refused { .. })
            });
        }
    }

    fn is_cut(&self, a: usize, b: usize) -> bool {
        self.cut.contains(&(a, b)) || self.cut.contains(&(b, a))
    }

    fn index(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    fn post(&mut self, from: usize, to: usize, text: String) {
        if self.is_cut(from, to) || self.nodes[to].gone {
            return;
        }
        let s = self.spec;
        if s.drop_pct > 0 && self.rng.next_u64() % 100 < s.drop_pct {
            return;
        }
        let mut at = self.now + s.latency_ms;
        if s.jitter_ms > 0 {
            at += (self.rng.next_u64() % (2 * s.jitter_ms as u64 + 1)) as i64 - s.jitter_ms;
        }
        if s.loss_pct > 0 && self.rng.next_u64() % 100 < s.loss_pct {
            at += RESEND_MS;
        }
        // reliable and ordered: nothing overtakes what was sent before it
        let last = self.last.entry((from, to)).or_insert(0);
        at = at.max(*last).max(self.now + 1);
        *last = at;
        self.queue.push(Packet { from, to, text, at });
    }

    /// The outboxes onto the links.
    fn send_all(&mut self) {
        for from in 0..self.nodes.len() {
            if self.nodes[from].gone {
                continue;
            }
            for o in self.nodes[from].session.take_outbox() {
                match o.to {
                    Some(id) => {
                        if let Some(to) = self.index(&id) {
                            self.post(from, to, o.text);
                        }
                    }
                    None => {
                        for to in 0..self.nodes.len() {
                            if to != from {
                                self.post(from, to, o.text.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    /// Runs `ms` milliseconds.
    pub fn run(&mut self, ms: i64) {
        for _ in 0..ms {
            self.now += 1;
            let now = self.now;
            let mut due = Vec::new();
            self.queue.retain_mut(|p| {
                if p.at <= now {
                    due.push(Packet {
                        from: p.from,
                        to: p.to,
                        text: std::mem::take(&mut p.text),
                        at: p.at,
                    });
                    false
                } else {
                    true
                }
            });
            due.sort_by_key(|p| p.at);
            for p in due {
                if self.nodes[p.to].gone {
                    continue;
                }
                let from = self.nodes[p.from].id.clone();
                self.nodes[p.to].session.receive(&from, &p.text, now);
            }
            let map = self.map.clone();
            let tuning = self.tuning.clone();
            let roster = self.roster.clone();
            for node in self.nodes.iter_mut().filter(|n| !n.gone) {
                node.session.update(now);
                node.tick(now, &map, &tuning, roster.as_ref());
            }
            self.send_all();
        }
    }

    /// Runs until `done` holds or `ms` pass; whether it held.
    pub fn run_until(&mut self, ms: i64, done: impl Fn(&Loop) -> bool) -> bool {
        for _ in 0..ms {
            if done(self) {
                return true;
            }
            self.run(1);
        }
        done(self)
    }

    /// Joiners seated, every one Ready, the host's Start pressed.
    pub fn lobby_and_start(&mut self) {
        self.lobby_and_start_with(Vec::new());
    }

    /// The same, with these fish in the lake.
    pub fn lobby_and_start_with(&mut self, fish: Vec<(String, u32)>) {
        self.connect();
        let n = self.nodes.len();
        assert!(
            self.run_until(5000, |l| l.nodes[1..].iter().all(|j| j.session.seat().is_some())),
            "every joiner is seated"
        );
        for k in 1..n {
            self.nodes[k].session.set_ready(true);
        }
        assert!(
            self.run_until(5000, |l| l.nodes[0].session.can_start()),
            "every Ready reaches the host"
        );
        let now = self.now;
        self.nodes[0]
            .session
            .start(
                7,
                1234,
                &["Walt".into(), "Gus".into(), "Rita".into()],
                fish,
                self.countdown_ms,
                now,
            )
            .unwrap();
        assert!(
            self.run_until(5000, |l| l.nodes.iter().all(|n| n.game.is_some())),
            "every browser starts its match"
        );
    }

    /// A browser closes: its links close at once on every other.
    pub fn close(&mut self, k: usize, say_bye: bool) {
        if say_bye {
            self.nodes[k].session.leave();
            self.send_all();
        }
        self.nodes[k].gone = true;
        let id = self.nodes[k].id.clone();
        let now = self.now;
        for (j, node) in self.nodes.iter_mut().enumerate() {
            if j != k && !node.gone {
                node.session.peer_left(&id, now);
            }
        }
    }
}

impl Node {
    fn new(id: &str, session: Session) -> Node {
        Node {
            id: id.into(),
            session,
            game: None,
            input: SeatInput::default(),
            next_tick: 0,
            log: Vec::new(),
            verdicts: Vec::new(),
            plans: Vec::new(),
            presses: Vec::new(),
            gone: false,
        }
    }

    /// The ticks this node's clock has reached, as the app runs them.
    fn tick(&mut self, now: i64, map: &Map, tuning: &Tuning, roster: Option<&crate::roster::Roster>) {
        if self.game.is_none() {
            let Some(start) = self.session.take_start(now) else {
                return;
            };
            let mut m = Match::new(map.clone(), tuning, start.setup).unwrap();
            if let Some(r) = roster {
                m.stock(r).unwrap();
            }
            self.session.prepare(&mut m);
            self.game = Some(m);
            self.next_tick = start.at_ms + TICK_MS as i64;
        }
        let seat = self.session.seat().unwrap();
        while now >= self.next_tick {
            self.next_tick += TICK_MS as i64;
            let m = self.game.as_mut().unwrap();
            let mut inputs = vec![SeatInput::default(); m.seats.len()];
            inputs[seat] = self.input;
            let tick = m.tick;
            for (t, i) in &self.presses {
                if *t == tick {
                    inputs[seat] = *i;
                }
            }
            let due: Vec<Claim> = self
                .plans
                .iter()
                .filter(|(t, _)| *t == tick)
                .map(|(_, c)| c.clone())
                .collect();
            for c in due {
                m.claim(seat, c);
            }
            m.step(&inputs);
            self.session.after_step(m, now);
            self.verdicts.extend(m.online.verdicts.iter().cloned());
            for e in &m.events {
                self.log.push((m.tick, e.clone()));
            }
        }
    }

    pub fn game(&self) -> &Match {
        self.game.as_ref().expect("a match")
    }

    pub fn game_mut(&mut self) -> &mut Match {
        self.game.as_mut().expect("a match")
    }
}
