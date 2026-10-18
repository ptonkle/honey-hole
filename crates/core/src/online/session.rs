//! One browser's online session: the lobby, the start, and its share of
//! the match. A pure state machine: text comes in through
//! [`Session::receive`], text goes out through [`Session::take_outbox`],
//! and time comes in as `now_ms` arguments.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Serialize;

use super::msg::{Msg, Refusal, Roster, SeatInfo, SetupInfo, Start, StartSeat, Who};
use super::*;
use crate::map::trig;
use crate::sim::remote::{BoatState, Claim, Ram, Role, Verdict, WorldEvent};
use crate::sim::{Boat, Difficulty, Match, MatchEvent, Occupant, SeatSetup, Setup, WreckCause, TICK_MS};

/// Text for the transport to carry: `to: None` is every connected peer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Outgoing {
    pub to: Option<String>,
    pub text: String,
}

/// Where a session stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Status {
    /// a joiner waiting for the host to answer
    Joining,
    Lobby,
    /// the match starts at this time (ms, this browser's clock)
    Starting {
        at_ms: i64,
    },
    Playing,
    Refused {
        refusal: Refusal,
    },
    /// the host left (or went silent): the session is over
    HostLeft,
    /// the transport failed: a `data/strings.json` key (`online.no_relay`...)
    Failed {
        key: String,
    },
    /// this browser left
    Left,
}

/// What a browser needs to build its match when the start time comes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchStart {
    pub setup: Setup,
    pub map_seed: u32,
    /// this browser's seat
    pub seat: usize,
    /// the start, on this browser's clock (ms)
    pub at_ms: i64,
}

#[derive(Debug, Clone, Default)]
struct Peer {
    /// host side: the joiner's seat
    seat: Option<usize>,
    rtt_ms: Option<i64>,
    /// the last few round trips, for a median that a resent ping does
    /// not throw
    rtts: VecDeque<i64>,
    heard_ms: i64,
}

/// A claim the host holds for a moment before judging it.
#[derive(Debug, Clone)]
struct Pending {
    tick: u32,
    seat: usize,
    id: u32,
    claim: Claim,
}

pub struct Session {
    host: bool,
    status: Status,
    name: String,
    version: String,
    tuning: String,
    seat: Option<usize>,
    /// the host's truth, or a joiner's copy of it
    roster: Roster,
    peers: BTreeMap<String, Peer>,
    /// a joiner's host, once its link is up
    host_peer: Option<String>,
    /// a joiner's wishes, repeated until the roster shows them
    want_ready: bool,
    out: Vec<Outgoing>,
    last_hello: i64,
    last_roster: i64,
    last_ping: i64,
    last_retry: i64,
    // ---- the start ----
    start: Option<Start>,
    start_at: Option<i64>,
    start_taken: bool,
    acked: BTreeSet<String>,
    last_start_send: i64,
    // ---- the match ----
    /// by seat, the newest states, oldest first
    states: BTreeMap<usize, VecDeque<BoatState>>,
    last_state_send: i64,
    /// the host's match tick and when it was so, for its Pongs
    host_tick: Option<(u32, i64)>,
    /// a joiner's readings of the host's clock: (round trip, the host's
    /// tick, when the answer came)
    clock: VecDeque<(i64, u32, i64)>,
    /// the reading the clock was last set from
    clock_used: Option<(i64, u32, i64)>,
    rams: Vec<Ram>,
    world: Vec<WorldEvent>,
    verdicts: Vec<Verdict>,
    pending: Vec<Pending>,
    ram_hold: BTreeMap<usize, u32>,
    end_heard: bool,
    end_sent: bool,
    /// seats whose players left mid-match, for the HUD's notice
    gone: Vec<usize>,
}

impl Session {
    fn new(host: bool, name: &str, tuning: &str, now_ms: i64) -> Session {
        Session {
            host,
            status: if host { Status::Lobby } else { Status::Joining },
            name: clean_name(name),
            version: crate::app::VERSION.to_string(),
            tuning: tuning.to_string(),
            seat: host.then_some(0),
            roster: Roster::default(),
            peers: BTreeMap::new(),
            host_peer: None,
            want_ready: false,
            out: Vec::new(),
            last_hello: i64::MIN / 2,
            last_roster: now_ms,
            last_ping: now_ms,
            last_retry: now_ms,
            start: None,
            start_at: None,
            start_taken: false,
            acked: BTreeSet::new(),
            last_start_send: i64::MIN / 2,
            states: BTreeMap::new(),
            last_state_send: i64::MIN / 2,
            host_tick: None,
            clock: VecDeque::new(),
            clock_used: None,
            rams: Vec::new(),
            world: Vec::new(),
            verdicts: Vec::new(),
            pending: Vec::new(),
            ram_hold: BTreeMap::new(),
            end_heard: false,
            end_sent: false,
            gone: Vec::new(),
        }
    }

    /// A host, in seat 1 (index 0), with `seats` lobby seats and this
    /// setup. `tuning` is [`tuning_hash`] of the match numbers.
    pub fn host(name: &str, tuning: &str, seats: usize, setup: SetupInfo, now_ms: i64) -> Session {
        let mut s = Session::new(true, name, tuning, now_ms);
        s.roster.seats = (0..seats.max(1))
            .map(|_| SeatInfo {
                who: Who::Open,
                name: String::new(),
                ready: false,
                ping_ms: None,
            })
            .collect();
        s.roster.seats[0] = SeatInfo {
            who: Who::Host,
            name: s.name.clone(),
            ready: true,
            ping_ms: None,
        };
        s.roster.setup = setup;
        s.fill_ai();
        s.roster.rev = 1;
        s
    }

    /// A joiner, waiting for its link to the host.
    pub fn join(name: &str, tuning: &str, now_ms: i64) -> Session {
        Session::new(false, name, tuning, now_ms)
    }

    pub fn is_host(&self) -> bool {
        self.host
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    /// This browser's seat, once seated.
    pub fn seat(&self) -> Option<usize> {
        self.seat
    }

    pub fn roster(&self) -> &Roster {
        &self.roster
    }

    /// A joiner's round trip to its host, ms.
    pub fn rtt_to_host(&self) -> Option<i64> {
        self.host_peer
            .as_ref()
            .and_then(|h| self.peers.get(h))
            .and_then(|p| p.rtt_ms)
    }

    pub fn take_outbox(&mut self) -> Vec<Outgoing> {
        std::mem::take(&mut self.out)
    }

    fn send(&mut self, to: &str, msg: &Msg) {
        self.out.push(Outgoing {
            to: Some(to.to_string()),
            text: msg.encode(),
        });
    }

    fn broadcast(&mut self, msg: &Msg) {
        self.out.push(Outgoing {
            to: None,
            text: msg.encode(),
        });
    }

    /// Sends to every peer but one (the host passing a joiner's state on).
    fn send_others(&mut self, except: &str, text: &str) {
        let to: Vec<String> = self.peers.keys().filter(|p| *p != except).cloned().collect();
        for p in to {
            self.out.push(Outgoing {
                to: Some(p),
                text: text.to_string(),
            });
        }
    }

    fn over(&self) -> bool {
        matches!(
            self.status,
            Status::Refused { .. } | Status::HostLeft | Status::Failed { .. } | Status::Left
        )
    }

    // ---- transport events ----

    /// A peer's link came up; `host` says it called itself the host.
    pub fn peer_connected(&mut self, peer: &str, host: bool, now_ms: i64) {
        self.peers.entry(peer.to_string()).or_default().heard_ms = now_ms;
        if !self.host && host && self.host_peer.is_none() {
            self.host_peer = Some(peer.to_string());
            self.say_hello(now_ms);
        }
    }

    /// A peer's link closed.
    pub fn peer_left(&mut self, peer: &str, now_ms: i64) {
        let Some(p) = self.peers.remove(peer) else {
            return;
        };
        if !self.host {
            if self.host_peer.as_deref() == Some(peer) && !self.over() {
                self.status = Status::HostLeft;
            }
            return;
        }
        if let Some(seat) = p.seat {
            self.free_seat(seat, now_ms);
        }
    }

    /// The transport failed (`online.no_relay`, `online.no_path`...).
    /// A path missing to one peer is not the end while the host is reached.
    pub fn failed(&mut self, key: &str) {
        if key == "online.no_path" && (self.host || self.host_peer.is_some()) {
            return;
        }
        if !self.over() {
            self.status = Status::Failed { key: key.to_string() };
        }
    }

    /// Leaves: a parting word to everyone, then the page closes the links.
    pub fn leave(&mut self) {
        if !matches!(self.status, Status::Left) {
            self.broadcast(&Msg::Bye);
            self.status = Status::Left;
        }
    }

    fn free_seat(&mut self, seat: usize, _now_ms: i64) {
        if self.start.is_some() {
            if !self.gone.contains(&seat) {
                self.gone.push(seat);
                self.world.push(WorldEvent::Left { seat });
                self.broadcast(&Msg::World {
                    event: WorldEvent::Left { seat },
                });
            }
            return;
        }
        if let Some(s) = self.roster.seats.get_mut(seat) {
            s.who = Who::Open;
            s.name.clear();
            s.ready = false;
            s.ping_ms = None;
        }
        self.fill_ai();
        self.roster_changed();
    }

    // ---- the lobby ----

    fn say_hello(&mut self, now_ms: i64) {
        let Some(h) = self.host_peer.clone() else {
            return;
        };
        self.last_hello = now_ms;
        let hello = Msg::Hello {
            protocol: PROTOCOL,
            version: self.version.clone(),
            tuning: self.tuning.clone(),
            name: self.name.clone(),
        };
        self.send(&h, &hello);
    }

    /// AI players fill the open seats after the people, up to the host's
    /// count.
    fn fill_ai(&mut self) {
        let setup = self.roster.setup.clone();
        let mut k = 0;
        for s in self.roster.seats.iter_mut() {
            if matches!(s.who, Who::Ai { .. }) {
                s.who = Who::Open;
                s.name.clear();
            }
        }
        for s in self.roster.seats.iter_mut() {
            if s.who == Who::Open && k < setup.ai {
                let d = setup
                    .difficulty
                    .get(k)
                    .cloned()
                    .unwrap_or_else(|| "normal".into());
                s.who = Who::Ai { difficulty: d };
                s.ready = true;
                k += 1;
            }
        }
    }

    fn roster_changed(&mut self) {
        self.roster.rev += 1;
        let r = Msg::Roster {
            roster: self.roster.clone(),
        };
        self.broadcast(&r);
    }

    /// The host's match setup changed (the New match choices).
    pub fn set_setup(&mut self, setup: SetupInfo) {
        if !self.host || self.roster.setup == setup {
            return;
        }
        self.roster.setup = setup;
        self.fill_ai();
        self.roster_changed();
    }

    /// A new name (from Settings), for this browser's seat.
    pub fn set_name(&mut self, name: &str) {
        self.name = clean_name(name);
        if self.host {
            if self.roster.seats[0].name != self.name {
                self.roster.seats[0].name = self.name.clone();
                self.roster_changed();
            }
        } else if let Some(h) = self.host_peer.clone() {
            let m = Msg::Name {
                name: self.name.clone(),
            };
            self.send(&h, &m);
        }
    }

    /// A joiner's Ready mark.
    pub fn set_ready(&mut self, ready: bool) {
        if self.host {
            return;
        }
        self.want_ready = ready;
        if let Some(h) = self.host_peer.clone() {
            self.send(&h, &Msg::Ready { ready });
        }
    }

    pub fn ready(&self) -> bool {
        self.want_ready
    }

    /// Whether the host may start: every person seated is Ready.
    pub fn can_start(&self) -> bool {
        self.host && self.start.is_none() && self.roster.seats.iter().all(|s| s.who != Who::Player || s.ready)
    }

    /// The host starts the match: every browser builds it from this, and it
    /// begins [`START_LEAD_MS`] from now.
    pub fn start(
        &mut self,
        map_seed: u32,
        match_seed: u64,
        names: &[String],
        fish: Vec<(String, u32)>,
        now_ms: i64,
    ) -> Result<(), String> {
        if !self.can_start() {
            return Err("online.not_ready".into());
        }
        let mut pool = names.to_vec();
        let seats = self
            .roster
            .seats
            .iter()
            .enumerate()
            .map(|(k, s)| StartSeat {
                who: s.who.clone(),
                name: match &s.who {
                    Who::Ai { .. } if !pool.is_empty() => {
                        let at = (crate::map::rng::mix(match_seed.wrapping_add(k as u64)) % pool.len() as u64)
                            as usize;
                        pool.remove(at)
                    }
                    Who::Ai { .. } => format!("AI {}", k + 1),
                    _ => s.name.clone(),
                },
            })
            .collect();
        let start = Start {
            id: self.roster.rev,
            tuning: self.tuning.clone(),
            map_seed,
            match_seed,
            seats,
            length_ms: self.roster.setup.length_ms,
            buoy_interval_ms: self.roster.setup.buoy_interval_ms,
            lead_ms: START_LEAD_MS,
            fish,
        };
        self.start = Some(start.clone());
        self.start_at = Some(now_ms + START_LEAD_MS);
        self.status = Status::Starting {
            at_ms: now_ms + START_LEAD_MS,
        };
        self.last_start_send = now_ms;
        self.broadcast(&Msg::Start { start });
        Ok(())
    }

    /// The match to build, once its start time has come; given once.
    pub fn take_start(&mut self, now_ms: i64) -> Option<MatchStart> {
        let (start, at) = (self.start.as_ref()?, self.start_at?);
        if self.start_taken || now_ms < at || self.over() {
            return None;
        }
        let seat = self.seat?;
        self.start_taken = true;
        self.status = Status::Playing;
        let seats = start
            .seats
            .iter()
            .map(|s| SeatSetup {
                occupant: match &s.who {
                    Who::Open => Occupant::Empty,
                    Who::Host | Who::Player => Occupant::Human,
                    Who::Ai { difficulty } => {
                        Occupant::Ai(Difficulty::from_key(difficulty).unwrap_or(Difficulty::Normal))
                    }
                },
                name: s.name.clone(),
            })
            .collect();
        Some(MatchStart {
            setup: Setup {
                seats,
                length_ms: start.length_ms,
                buoy_interval_ms: start.buoy_interval_ms,
                offline: false,
                seed: start.match_seed,
                fish: start.fish.clone(),
            },
            map_seed: start.map_seed,
            seat,
            at_ms: at,
        })
    }

    /// Which seats this browser simulates: its own, and on the host the AI
    /// players'.
    pub fn owns(&self, seat: usize) -> bool {
        if self.seat == Some(seat) {
            return true;
        }
        self.host
            && self
                .start
                .as_ref()
                .and_then(|s| s.seats.get(seat))
                .is_some_and(|s| matches!(s.who, Who::Ai { .. }))
    }

    /// Readies a new match for online play: who simulates which boat, and
    /// who judges.
    pub fn prepare(&self, m: &mut Match) {
        let n = m.boats.len();
        m.online.role = if self.host { Role::Host } else { Role::Client };
        m.online.remote = (0..n).map(|s| !self.owns(s)).collect();
        m.online.rams_elsewhere = true;
        m.online.end_elsewhere = !self.host;
    }

    // ---- receiving ----

    /// Text from a peer.
    pub fn receive(&mut self, peer: &str, text: &str, now_ms: i64) {
        let Some(msg) = Msg::decode(text) else {
            return;
        };
        if self.over() {
            return;
        }
        self.peers.entry(peer.to_string()).or_default().heard_ms = now_ms;
        match msg {
            Msg::Ping { at } => {
                let tick = self.host_tick.map(|(t, when)| t + ticks_of(now_ms - when));
                self.send(peer, &Msg::Pong { at, tick })
            }
            Msg::Pong { at, tick } => {
                let rtt = (now_ms - at).max(0);
                if let (Some(tick), false) = (tick, self.host) {
                    self.clock.push_back((rtt, tick, now_ms));
                    if self.clock.len() > RTTS_KEPT {
                        self.clock.pop_front();
                    }
                }
                if let Some(p) = self.peers.get_mut(peer) {
                    p.rtts.push_back(rtt);
                    if p.rtts.len() > RTTS_KEPT {
                        p.rtts.pop_front();
                    }
                    let mut sorted: Vec<i64> = p.rtts.iter().copied().collect();
                    sorted.sort_unstable();
                    p.rtt_ms = Some(sorted[sorted.len() / 2]);
                }
            }
            Msg::State { state } => self.on_state(peer, state, text),
            _ if self.host => self.host_receives(peer, msg, now_ms),
            _ if self.host_peer.as_deref() == Some(peer) => self.joiner_receives(peer, msg, now_ms),
            _ => {}
        }
    }

    fn on_state(&mut self, peer: &str, state: BoatState, text: &str) {
        if self.owns(state.seat) {
            return;
        }
        if self.host {
            // only a seat's own browser speaks for it
            let seat = self.peers.get(peer).and_then(|p| p.seat);
            if seat != Some(state.seat) {
                return;
            }
        }
        let q = self.states.entry(state.seat).or_default();
        if q.back().is_some_and(|b| b.tick >= state.tick) {
            return;
        }
        q.push_back(state);
        while q.len() > STATES_KEPT {
            q.pop_front();
        }
        if self.host {
            // to joiners with no link of their own to the sender (a
            // repeat is dropped as old)
            self.send_others(peer, text);
        }
    }

    fn host_receives(&mut self, peer: &str, msg: Msg, now_ms: i64) {
        let seat = self.peers.get(peer).and_then(|p| p.seat);
        match msg {
            Msg::Hello {
                protocol,
                version,
                tuning,
                name,
            } => {
                if let Some(seat) = seat {
                    self.send(peer, &Msg::Welcome { seat });
                    return;
                }
                let refusal = if protocol != PROTOCOL || version != self.version {
                    Some(Refusal::Version {
                        host: format!("{}/{}", PROTOCOL, self.version),
                        mine: format!("{protocol}/{version}"),
                    })
                } else if tuning != self.tuning {
                    Some(Refusal::Tuning)
                } else if self.start.is_some() {
                    Some(Refusal::Underway)
                } else {
                    None
                };
                let free = self
                    .roster
                    .seats
                    .iter()
                    .position(|s| matches!(s.who, Who::Open | Who::Ai { .. }));
                match (refusal, free) {
                    (Some(refusal), _) => self.send(peer, &Msg::Refused { refusal }),
                    (None, None) => self.send(
                        peer,
                        &Msg::Refused {
                            refusal: Refusal::Full,
                        },
                    ),
                    (None, Some(k)) => {
                        self.roster.seats[k] = SeatInfo {
                            who: Who::Player,
                            name: clean_name(&name),
                            ready: false,
                            ping_ms: None,
                        };
                        self.peers.entry(peer.to_string()).or_default().seat = Some(k);
                        self.fill_ai();
                        self.send(peer, &Msg::Welcome { seat: k });
                        self.roster_changed();
                    }
                }
            }
            Msg::Name { name } => {
                if let Some(k) = seat {
                    let name = clean_name(&name);
                    if self.roster.seats[k].name != name {
                        self.roster.seats[k].name = name;
                        self.roster_changed();
                    }
                }
            }
            Msg::Ready { ready } => {
                if let Some(k) = seat.filter(|_| self.start.is_none()) {
                    if self.roster.seats[k].ready != ready {
                        self.roster.seats[k].ready = ready;
                        self.roster_changed();
                    }
                }
            }
            Msg::StartAck { id } => {
                if self.start.as_ref().is_some_and(|s| s.id == id) {
                    self.acked.insert(peer.to_string());
                }
            }
            Msg::Report { event } => {
                // a joiner speaks only for its own boat
                if let WorldEvent::Wrecked {
                    seat: s,
                    cause: WreckCause::Grounded,
                } = &event
                {
                    if Some(*s) == seat {
                        self.world.push(event.clone());
                        self.send_others(peer, &Msg::World { event }.encode());
                    }
                }
            }
            Msg::Claim {
                seat: s,
                id,
                tick,
                claim,
            } => {
                if Some(s) == seat {
                    self.pending.push(Pending {
                        tick,
                        seat: s,
                        id,
                        claim,
                    });
                }
            }
            Msg::Bye => self.peer_left(peer, now_ms),
            _ => {}
        }
    }

    fn joiner_receives(&mut self, peer: &str, msg: Msg, now_ms: i64) {
        match msg {
            Msg::Welcome { seat } => {
                self.seat = Some(seat);
                if self.status == Status::Joining {
                    self.status = Status::Lobby;
                }
            }
            Msg::Refused { refusal } => self.status = Status::Refused { refusal },
            Msg::Roster { roster } => {
                if roster.rev >= self.roster.rev {
                    self.roster = roster;
                }
            }
            Msg::Start { start } => {
                self.send(peer, &Msg::StartAck { id: start.id });
                if self.start.is_none() && self.seat.is_some() {
                    let half = self.rtt_to_host().unwrap_or(0) / 2;
                    let at = now_ms + (start.lead_ms - half).max(0);
                    self.start = Some(start);
                    self.start_at = Some(at);
                    self.status = Status::Starting { at_ms: at };
                }
            }
            Msg::Rammed { ram } => self.rams.push(ram),
            Msg::World { event } => {
                if let WorldEvent::Left { seat } = &event {
                    if !self.gone.contains(seat) {
                        self.gone.push(*seat);
                    }
                }
                self.world.push(event);
            }
            Msg::Verdict { verdict } => self.verdicts.push(verdict),
            Msg::End => self.end_heard = true,
            Msg::Bye => self.status = Status::HostLeft,
            _ => {}
        }
    }

    // ---- time ----

    /// Every frame: the lobby's repeats, the pings, the start's repeats,
    /// and silence that means a peer has gone.
    pub fn update(&mut self, now_ms: i64) {
        if self.over() {
            return;
        }
        if !self.host && self.seat.is_none() && now_ms - self.last_hello >= RETRY_MS {
            self.say_hello(now_ms);
        }
        if now_ms - self.last_ping >= PING_EVERY_MS {
            self.last_ping = now_ms;
            let ping = Msg::Ping { at: now_ms };
            if self.host {
                self.broadcast(&ping);
                let rtts: Vec<(usize, Option<i64>)> = self
                    .peers
                    .values()
                    .filter_map(|p| p.seat.map(|s| (s, p.rtt_ms)))
                    .collect();
                for (s, rtt) in rtts {
                    self.roster.seats[s].ping_ms = rtt;
                }
            } else if let Some(h) = self.host_peer.clone() {
                self.send(&h, &ping);
            }
        }
        if self.host && self.start.is_none() && now_ms - self.last_roster >= ROSTER_EVERY_MS {
            self.last_roster = now_ms;
            let r = Msg::Roster {
                roster: self.roster.clone(),
            };
            self.broadcast(&r);
        }
        if !self.host && self.seat.is_some() && now_ms - self.last_retry >= RETRY_MS {
            self.last_retry = now_ms;
            if let (Some(h), Some(k)) = (self.host_peer.clone(), self.seat) {
                let shown = self.roster.seats.get(k);
                let ready_off = shown.is_some_and(|s| s.ready != self.want_ready);
                let name_off = shown.is_some_and(|s| s.name != self.name);
                if self.start.is_none() && ready_off {
                    self.send(
                        &h,
                        &Msg::Ready {
                            ready: self.want_ready,
                        },
                    );
                }
                if name_off {
                    self.send(
                        &h,
                        &Msg::Name {
                            name: self.name.clone(),
                        },
                    );
                }
            }
        }
        if self.host && now_ms - self.last_start_send >= RETRY_MS {
            if let Some(start) = self.start.clone() {
                self.last_start_send = now_ms;
                let waiting: Vec<String> = self
                    .peers
                    .iter()
                    .filter(|(id, p)| p.seat.is_some() && !self.acked.contains(*id))
                    .map(|(id, _)| id.clone())
                    .collect();
                for p in waiting {
                    self.send(&p, &Msg::Start { start: start.clone() });
                }
            }
        }
        // silence
        let silent: Vec<String> = self
            .peers
            .iter()
            .filter(|(id, p)| {
                now_ms - p.heard_ms >= SILENT_MS && (self.host || self.host_peer.as_deref() == Some(id))
            })
            .map(|(id, _)| id.clone())
            .collect();
        for p in silent {
            self.peer_left(&p, now_ms);
        }
    }

    // ---- the match ----

    /// After each step of the match: the other boats placed, the host's
    /// verdicts and events applied, this browser's boats sent, and on the
    /// host the rams judged and the claims answered.
    pub fn after_step(&mut self, m: &mut Match, now_ms: i64) {
        if self.start.is_none() {
            return;
        }
        if !self.host {
            // the reading with the shortest round trip is the one least
            // held up on the way (as NTP does)
            let best = self.clock.iter().min_by_key(|(rtt, _, _)| *rtt).copied();
            if let Some(r @ (rtt, tick, at)) = best.filter(|b| Some(*b) != self.clock_used) {
                self.clock_used = Some(r);
                let expected = tick as i64 + (now_ms - at + rtt / 2) / TICK_MS as i64;
                if (expected - m.tick as i64).abs() >= CLOCK_SNAP_TICKS as i64 {
                    m.tick = expected.max(0) as u32;
                }
            }
        } else {
            self.host_tick = Some((m.tick, now_ms));
        }
        self.place_remotes(m);
        for ram in std::mem::take(&mut self.rams) {
            m.apply_ram(&ram);
        }
        for e in std::mem::take(&mut self.world) {
            m.apply_world(&e);
        }
        m.online.incoming.append(&mut self.verdicts);
        if self.host || self.host_peer.is_some() {
            self.report_own_events(m);
        }
        for (seat, id, claim) in std::mem::take(&mut m.online.claims_out) {
            if self.host {
                self.pending.push(Pending {
                    tick: m.tick,
                    seat,
                    id,
                    claim,
                });
            } else if let Some(h) = self.host_peer.clone() {
                let c = Msg::Claim {
                    seat,
                    id,
                    tick: m.tick,
                    claim,
                };
                self.send(&h, &c);
            }
        }
        if self.host {
            self.judge(m);
            for event in std::mem::take(&mut m.online.world_out) {
                self.broadcast(&Msg::World { event });
            }
            if m.ended && !self.end_sent {
                self.end_sent = true;
                self.broadcast(&Msg::End);
            }
        } else {
            m.online.world_out.clear();
            if self.end_heard {
                m.end_now();
            }
        }
        if now_ms - self.last_state_send >= STATE_EVERY_MS {
            // on average STATE_EVERY_MS apart, though ticks are 20 ms
            self.last_state_send = (self.last_state_send + STATE_EVERY_MS).max(now_ms - STATE_EVERY_MS);
            for seat in 0..m.boats.len() {
                if self.owns(seat) {
                    if let Some(state) = m.boat_state(seat) {
                        self.broadcast(&Msg::State { state });
                    }
                }
            }
        }
    }

    /// A grounding of a boat this browser simulates goes to everyone (by
    /// the host).
    fn report_own_events(&mut self, m: &Match) {
        for e in &m.events {
            if let MatchEvent::Wrecked {
                seat,
                cause: WreckCause::Grounded,
            } = e
            {
                if !self.owns(*seat) {
                    continue;
                }
                let event = WorldEvent::Wrecked {
                    seat: *seat,
                    cause: WreckCause::Grounded,
                };
                if self.host {
                    self.broadcast(&Msg::World { event });
                } else if let Some(h) = self.host_peer.clone() {
                    self.send(&h, &Msg::Report { event });
                }
            }
        }
    }

    /// The host: rams between the boats as reported, and the claims whose
    /// moment has passed, in the order they were made.
    fn judge(&mut self, m: &mut Match) {
        let boats: Vec<Option<Boat>> = (0..m.boats.len())
            .map(|seat| {
                if m.boat(seat).is_none() {
                    None
                } else if self.owns(seat) {
                    m.boat(seat).cloned()
                } else {
                    self.latest(seat, m.tick).map(|s| m.boat_from_state(&s))
                }
            })
            .collect();
        let tick = m.tick;
        self.ram_hold.retain(|_, until| *until > tick);
        for ram in m.judge_rams(&boats) {
            if self.ram_hold.contains_key(&ram.victim) {
                continue;
            }
            self.ram_hold.insert(ram.victim, tick + ticks_of(RAM_HOLD_MS));
            m.apply_ram(&ram);
            self.broadcast(&Msg::Rammed { ram });
        }
        let due = tick.saturating_sub(ticks_of(CLAIM_WINDOW_MS));
        let mut ready: Vec<Pending> = Vec::new();
        self.pending.retain(|p| {
            if p.tick <= due {
                ready.push(p.clone());
                false
            } else {
                true
            }
        });
        ready.sort_by_key(|p| (p.tick, p.seat, p.id));
        for p in ready {
            let verdict = m.judge_claim(p.seat, p.id, p.claim);
            m.online.incoming.push(verdict.clone());
            self.broadcast(&Msg::Verdict { verdict });
        }
    }

    /// A remote seat's newest state, carried on to `tick` along its
    /// velocity (for at most [`EXTRAPOLATE_TICKS`]).
    fn latest(&self, seat: usize, tick: u32) -> Option<BoatState> {
        let s = self.states.get(&seat)?.back()?;
        Some(carry(s, tick.saturating_sub(s.tick).min(EXTRAPOLATE_TICKS)))
    }

    /// Every remote boat where its owner said it was [`INTERP_TICKS`] ago,
    /// between the two states either side of then.
    fn place_remotes(&mut self, m: &mut Match) {
        let at = m.tick.saturating_sub(INTERP_TICKS);
        for seat in 0..m.boats.len() {
            if self.owns(seat) || m.boat(seat).is_none() {
                continue;
            }
            if let Some(s) = self.states.get(&seat).and_then(|q| between(q, at)) {
                m.place_remote(&s);
            }
        }
    }

    /// Seats whose players left mid-match.
    pub fn gone(&self) -> &[usize] {
        &self.gone
    }
}

/// Milliseconds to whole ticks.
fn ticks_of(ms: i64) -> u32 {
    (ms / TICK_MS as i64) as u32
}

/// A state carried `dt` ticks on along its velocity.
fn carry(s: &BoatState, dt: u32) -> BoatState {
    let mut c = s.clone();
    let ms = dt as i64 * TICK_MS as i64;
    c.at = [
        s.at[0] + (s.vel[0] as i64 * ms / 1000) as i32,
        s.at[1] + (s.vel[1] as i64 * ms / 1000) as i32,
    ];
    c.tick = s.tick + dt;
    c.ghost = s.ghost.saturating_sub(dt);
    c.stall = s.stall.saturating_sub(dt);
    c
}

/// The state at `tick`, between the buffered states either side of it;
/// past the newest, carried on for at most [`EXTRAPOLATE_TICKS`].
fn between(q: &VecDeque<BoatState>, tick: u32) -> Option<BoatState> {
    let first = q.front()?;
    if tick <= first.tick {
        return Some(first.clone());
    }
    let after = q.iter().position(|s| s.tick > tick);
    let Some(j) = after else {
        let last = q.back()?;
        return Some(carry(last, (tick - last.tick).min(EXTRAPOLATE_TICKS)));
    };
    let (a, b) = (&q[j - 1], &q[j]);
    if a.wrecked != b.wrecked || a.fishing != b.fishing {
        // a respawn or a mode change: no sliding across it
        return Some(carry(a, 0));
    }
    let span = (b.tick - a.tick) as i64;
    let t = (tick - a.tick) as i64;
    let lerp = |x: i32, y: i32| x + ((y - x) as i64 * t / span) as i32;
    let mut s = a.clone();
    s.at = [lerp(a.at[0], b.at[0]), lerp(a.at[1], b.at[1])];
    s.vel = [lerp(a.vel[0], b.vel[0]), lerp(a.vel[1], b.vel[1])];
    let turn = crate::gfx::fx::wrap(b.heading - a.heading) as i64;
    s.heading = trig::wrap(a.heading + (turn * t / span) as i32);
    s.cast_yaw = lerp(a.cast_yaw, b.cast_yaw);
    s.tick = tick;
    s.ghost = a.ghost.saturating_sub(t as u32);
    s.stall = a.stall.saturating_sub(t as u32);
    Some(s)
}
