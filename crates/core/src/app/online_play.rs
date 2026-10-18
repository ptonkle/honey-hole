//! Online play in the app: the Play screen's choice (Offline or Online),
//! the Online screen (Host, or Join by room code), the Lobby, and the
//! wiring between the page's transport (`web/online.js`, through
//! `web/net.js`) and the protocol (`crate::online`).
//!
//! - **The page's side.** `net_event(kind, peer, text, host)` for each
//!   transport event; `net_code(which, code)` for a pasted code the
//!   transport made; and every frame `net_pump(now)`, whose answer holds the
//!   text to send (`out`, each `{to, text}`, `to: null` for everyone) and
//!   what to ask of the transport (`requests`, each `{op, ...}`: host_room,
//!   join_room, host_code, invite_code, join_code, accept_reply, leave).
//!   The page sends `out` before it carries out `requests`, so a parting
//!   word goes out before the links close (Ape Race's N14).
//! - **Pasted codes.** With `flag.codes` (the page's `?codes`, for the
//!   automated online check) the Online screen and the Lobby also offer the
//!   pasted-code path, which needs no relay.
//! - **The match.** The host's New match choices are its setup (the same
//!   `lobby.*` actions); when the start time comes every browser builds the
//!   match from the host's start, and `app/play.rs` runs it, calling
//!   `Session::after_step` after each tick.

use serde::Serialize;
use serde_json::{json, Value};

use super::*;
use crate::map::rng::mix;
use crate::map::{generate, Setting};
use crate::online::{self, Refusal, Session, SetupInfo, Status, Who};
use crate::sim::{self, Match};
use crate::view::{Choice, TableRow};

#[cfg(test)]
mod tests;

/// The online part of the app.
#[derive(Default)]
pub struct OnlinePlay {
    pub session: Option<Session>,
    /// the room code, typed or made
    pub room: String,
    /// the pasted-code path is offered (`flag.codes`)
    pub codes: bool,
    /// this browser's own code to give (the host's invite, a joiner's reply)
    pub my_code: String,
    /// a code typed in (a joiner's invite, the host's reply)
    pub their_code: String,
    /// a problem to show on the Online screen: a strings key and its
    /// placeholders
    pub problem: Option<(String, Vec<(String, String)>)>,
    /// what the transport is asked to do next
    pub requests: Vec<Value>,
}

/// What `net_pump` hands the page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pump {
    pub out: Vec<online::Outgoing>,
    pub requests: Vec<Value>,
    /// the screen may have changed: draw `ui` again
    pub redraw: bool,
    /// in an online match, whether another browser's boat has left its
    /// spawn on this screen (the online check reads it)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_moved: Option<bool>,
}

/// How far a boat must go from its spawn slot to count as moved.
const MOVED_MM: i64 = 3000;

/// The longest pasted code a field takes (a compressed session
/// description is a few hundred characters).
const CODE_FIELD_MAX: u32 = 8000;

impl App {
    fn now_ms(&self) -> i64 {
        self.clock / 1000
    }

    fn tuning_text(&self) -> String {
        self.tuning.as_ref().map(online::tuning_hash).unwrap_or_default()
    }

    /// The host's setup, from the New match choices.
    fn setup_info(&self) -> SetupInfo {
        let Some(l) = self.lobby.as_ref() else {
            return SetupInfo::default();
        };
        SetupInfo {
            ai: l.ai,
            difficulty: l.difficulty.iter().map(|d| d.key().to_string()).collect(),
            length_ms: l.length_ms,
            buoy_interval_ms: l.buoy_interval_ms,
            seed: l.seed.clone(),
        }
    }

    fn request(&mut self, op: &str, extra: Value) {
        let mut r = json!({ "op": op });
        if let (Some(r), Value::Object(e)) = (r.as_object_mut(), extra) {
            r.extend(e);
        }
        self.online.requests.push(r);
    }

    fn problem(&mut self, key: &str) {
        self.online.problem = Some((key.to_string(), Vec::new()));
    }

    fn open_session(&mut self, host: bool) -> Result<(), String> {
        let hash = self.tuning_text();
        if hash.is_empty() {
            return Err("The tuning file is not loaded yet.".into());
        }
        let now = self.now_ms();
        let name = self.player_name();
        let seats = self.tuning.as_ref().map_or(4, |t| t.game.seats as usize);
        self.online.session = Some(if host {
            Session::host(&name, &hash, seats, self.setup_info(), now)
        } else {
            Session::join(&name, &hash, now)
        });
        self.online.problem = None;
        self.online.my_code.clear();
        self.go(Screen::OnlineLobby);
        Ok(())
    }

    /// The Online screen's and the Lobby's controls (`online.*`).
    pub(super) fn online_act(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            "online.name" => {
                self.set_name(value);
                let n = self.player_name();
                if let Some(s) = self.online.session.as_mut() {
                    s.set_name(&n);
                }
            }
            "online.room" => self.online.room = value.trim().to_string(),
            "online.host" => {
                let code = online::room_code(mix(self.clock as u64 ^ 0x726f_6f6d));
                self.open_session(true)?;
                self.online.room = code.clone();
                self.request("host_room", json!({ "room": online::room_name(&code) }));
            }
            "online.join" => match online::clean_code(&self.online.room) {
                Some(code) => {
                    self.open_session(false)?;
                    self.online.room = code.clone();
                    self.request("join_room", json!({ "room": online::room_name(&code) }));
                }
                None => {
                    self.online.problem = Some((
                        "online.bad_room".into(),
                        vec![("{n}".into(), online::CODE_LEN.to_string())],
                    ))
                }
            },
            "online.host_code" => {
                self.open_session(true)?;
                self.online.room.clear();
                self.request("host_code", json!({}));
            }
            "online.invite" | "online.reply" => self.online.their_code = value.trim().to_string(),
            "online.join_code" => {
                let invite = self.online.their_code.clone();
                self.open_session(false)?;
                self.online.room.clear();
                self.online.their_code.clear();
                self.request("join_code", json!({ "invite": invite }));
            }
            "online.connect" => {
                let reply = std::mem::take(&mut self.online.their_code);
                self.request("accept_reply", json!({ "reply": reply }));
            }
            "online.next_invite" => {
                self.online.my_code.clear();
                self.request("invite_code", json!({}));
            }
            "online.ready" => {
                if let Some(s) = self.online.session.as_mut() {
                    let r = !s.ready();
                    s.set_ready(r);
                }
            }
            "online.start" => self.start_online()?,
            "online.leave" => {
                self.leave_online();
                self.go(Screen::Online);
            }
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    /// The host's New match choices changed: they are its setup.
    pub(super) fn online_setup_changed(&mut self) {
        let info = self.setup_info();
        if let Some(s) = self.online.session.as_mut() {
            s.set_setup(info);
        }
    }

    /// Leaves the online session: a parting word, then the links close.
    pub(super) fn leave_online(&mut self) {
        if let Some(s) = self.online.session.as_mut() {
            s.leave();
            self.request("leave", json!({}));
        }
    }

    fn start_online(&mut self) -> Result<(), String> {
        let list = self.maps.clone().ok_or("The map list file is not loaded yet.")?;
        let match_seed = mix(self.clock as u64 ^ 0x6f6e_6c69_6e65);
        let map_seed = self.map_seed(&list, match_seed)?;
        let names = self.names.clone();
        // the host's starting fish counts, as offline (`App::setup`)
        let fish = self.setup(match_seed)?.fish;
        let now = self.now_ms();
        let s = self.online.session.as_mut().ok_or("There is no online lobby.")?;
        s.start(map_seed, match_seed, &names, fish, now)
            .map_err(|k| self.t(&k))
    }

    // ---- the page's side ----

    /// A transport event: connected, message, disconnected or error.
    pub fn net_event(&mut self, kind: &str, peer: &str, text: &str, host: bool) {
        let now = self.now_ms();
        let Some(s) = self.online.session.as_mut() else {
            if kind == "error" {
                self.problem(text);
            }
            return;
        };
        match kind {
            "connected" => s.peer_connected(peer, host, now),
            "message" => s.receive(peer, text, now),
            "disconnected" => s.peer_left(peer, now),
            "error" => s.failed(text),
            _ => {}
        }
    }

    /// A pasted code the transport made: the host's invite or a joiner's
    /// reply, to show for copying.
    pub fn net_code(&mut self, _which: &str, code: &str) {
        self.online.my_code = code.to_string();
    }

    /// Every frame: the clock, the session's repeats and its start, and
    /// what the page sends and asks of the transport.
    pub fn net_pump(&mut self, now_us: i64) -> Pump {
        self.advance(now_us);
        let now = self.now_ms();
        let mut out = Vec::new();
        let mut start = None;
        let mut over = false;
        if let Some(s) = self.online.session.as_mut() {
            s.update(now);
            if self.play.is_none() {
                start = s.take_start(now);
            }
            out = s.take_outbox();
            over = matches!(s.status(), Status::Left);
            if matches!(s.status(), Status::HostLeft) {
                if let Some(p) = self.play.as_mut() {
                    p.game.end_now();
                }
            }
        }
        if over {
            self.online.session = None;
        }
        if let Some(st) = start {
            if let Err(e) = self.begin_online_match(st) {
                self.online.problem = Some((e, Vec::new()));
            }
        }
        let redraw =
            self.online.session.is_some() || matches!(self.screen, Screen::Online | Screen::OnlineLobby);
        Pump {
            out,
            requests: std::mem::take(&mut self.online.requests),
            redraw,
            remote_moved: self.remote_moved(),
        }
    }

    /// Whether a boat another browser simulates is 3 m or more from its
    /// spawn slot, in an online match.
    fn remote_moved(&self) -> Option<bool> {
        self.online.session.as_ref()?;
        let g = &self.play.as_ref()?.game;
        Some((0..g.boats.len()).any(|seat| {
            g.is_remote(seat)
                && g.boat(seat).is_some_and(|b| {
                    let (at, s) = (b.at(), g.spawn_slot(seat).0);
                    crate::map::trig::hypot((at.x - s.x) as i64, (at.y - s.y) as i64) >= MOVED_MM
                })
        }))
    }

    fn begin_online_match(&mut self, st: online::MatchStart) -> Result<(), String> {
        let t = self.tuning.clone().ok_or("The tuning file is not loaded yet.")?;
        let pitch = self.controls.as_ref().map_or(0, |c| c.mouse.pitch_start_deg);
        let map = generate(Setting::MichiganLake, st.map_seed);
        let _ = self.show_world(&map);
        let mut game = Match::new(map, &t, st.setup)?;
        // the lake's fish, from the same seed and roster everywhere
        // (`sim::fishing`); the host's verdicts settle who hooks which
        if !self.fishboy.roster.0.is_empty() {
            game.stock(&self.fishboy.roster)?;
        }
        if let Some(s) = self.online.session.as_ref() {
            s.prepare(&mut game);
        }
        self.play = Some(play::Play::begin(game, st.seat, pitch, st.at_ms * 1000));
        self.go(Screen::Play);
        Ok(())
    }

    /// A line for the HUD about the other players: who left, or that the
    /// host did.
    pub fn online_notice(&self) -> Option<String> {
        let s = self.online.session.as_ref()?;
        if matches!(s.status(), Status::HostLeft) {
            return Some(self.t("online.host_left"));
        }
        let p = self.play.as_ref()?;
        let names: Vec<String> = s
            .gone()
            .iter()
            .filter_map(|&k| p.game.setup.seats.get(k).map(|x| x.name.clone()))
            .collect();
        (!names.is_empty()).then(|| self.t("online.left_match").replace("{name}", &names.join(", ")))
    }

    // ---- panels ----

    pub(super) fn play_menu_panels(&self) -> Vec<Panel> {
        vec![Panel::new(
            "menu",
            self.t("play_menu.heading"),
            vec![
                self.button("menu.offline"),
                self.button("menu.online"),
                self.button("menu.back"),
            ],
        )]
    }

    fn problem_text(&self) -> String {
        match &self.online.problem {
            Some((key, args)) => {
                let mut t = self.t(key);
                for (k, v) in args {
                    t = t.replace(k, v);
                }
                t
            }
            None => String::new(),
        }
    }

    fn code_field(&self, id: &str, value: &str) -> Item {
        Item::Text {
            id: id.into(),
            label: self.t(id),
            value: value.to_string(),
            max: CODE_FIELD_MAX,
            placeholder: String::new(),
        }
    }

    pub(super) fn online_panels(&self) -> Vec<Panel> {
        let mut items = vec![
            Item::Text {
                id: "online.name".into(),
                label: self.t("settings.name"),
                value: self.settings.name.clone(),
                max: play::NAME_MAX,
                placeholder: self.t("name.default"),
            },
            self.button("online.host"),
            Item::Text {
                id: "online.room".into(),
                label: self.t("online.room"),
                value: self.online.room.clone(),
                max: online::CODE_LEN as u32 + 2,
                placeholder: String::new(),
            },
            self.button("online.join"),
        ];
        if self.online.codes {
            items.push(self.button("online.host_code"));
            items.push(self.code_field("online.invite", &self.online.their_code));
            items.push(self.button("online.join_code"));
        }
        items.push(self.button("menu.back"));
        let mut p = Panel::new("online", self.t("online.heading"), items);
        p.sub = self.problem_text();
        vec![p]
    }

    fn status_text(&self, s: &Session) -> String {
        match s.status() {
            Status::Joining => self.t("online.joining"),
            Status::Lobby if s.is_host() && !s.can_start() => self.t("online.waiting"),
            Status::Lobby => String::new(),
            Status::Starting { .. } | Status::Playing => self.t("online.starting"),
            Status::Refused { refusal } => match refusal {
                Refusal::Full => self.t("online.refused.full"),
                Refusal::Underway => self.t("online.refused.underway"),
                Refusal::Tuning => self.t("online.refused.tuning"),
                Refusal::Version { host, mine } => self
                    .t("online.refused.version")
                    .replace("{host}", host)
                    .replace("{mine}", mine),
            },
            Status::HostLeft => self.t("online.host_left"),
            Status::Failed { key } => self.t(key),
            Status::Left => String::new(),
        }
    }

    pub(super) fn online_lobby_panels(&self) -> Vec<Panel> {
        let Some(s) = self.online.session.as_ref() else {
            return vec![Panel::new(
                "online_lobby",
                self.t("online_lobby.heading"),
                vec![self.button("online.leave")],
            )];
        };
        let roster = s.roster();
        let rows = roster
            .seats
            .iter()
            .enumerate()
            .map(|(k, seat)| {
                let colour = self.t(&format!("colour.{}", sim::SEAT_COLOURS[k.min(3)]));
                let mine = s.seat() == Some(k);
                let name = if mine {
                    self.t("lobby.you").replace("{name}", &seat.name)
                } else {
                    seat.name.clone()
                };
                let state = match &seat.who {
                    Who::Open => self.t("online.state.open"),
                    Who::Host => self.t("online.state.host"),
                    Who::Player if seat.ready => self.t("online.state.ready"),
                    Who::Player => self.t("online.state.not_ready"),
                    Who::Ai { difficulty } => self
                        .t("lobby.ai_player")
                        .replace("{difficulty}", &self.t(&format!("difficulty.{difficulty}"))),
                };
                let ping = seat
                    .ping_ms
                    .map(|ms| self.t("online.ping").replace("{ms}", &ms.to_string()))
                    .unwrap_or_default();
                TableRow {
                    cells: vec![colour, name, state, ping],
                    marked: mine,
                }
            })
            .collect();
        let mut items = vec![Item::Table {
            head: vec![
                self.t("online_lobby.colour"),
                self.t("online_lobby.name"),
                self.t("online_lobby.state"),
                self.t("online_lobby.ping"),
            ],
            rows,
        }];
        if s.is_host() {
            items.extend(self.host_setup_items(roster));
            let mut start = Item::button("online.start", self.t("lobby.start"));
            if let Item::Button { disabled, .. } = &mut start {
                *disabled = !s.can_start();
            }
            items.push(start);
        } else {
            items.push(Item::List {
                rows: self.setup_rows(&roster.setup),
            });
            if s.seat().is_some() {
                let key = if s.ready() {
                    "online.unready"
                } else {
                    "online.ready"
                };
                items.push(Item::button("online.ready", self.t(key)));
            }
        }
        if self.online.codes {
            if s.is_host() {
                items.push(self.code_field("online.invite", &self.online.my_code));
                items.push(self.code_field("online.reply", &self.online.their_code));
                items.push(self.button("online.connect"));
                items.push(self.button("online.next_invite"));
            } else {
                items.push(self.code_field("online.reply", &self.online.my_code));
            }
        }
        items.push(self.button("online.leave"));
        let mut p = Panel::new("online_lobby", self.t("online_lobby.heading"), items);
        let mut sub = Vec::new();
        if !self.online.room.is_empty() {
            sub.push(self.t("online_lobby.room").replace("{code}", &self.online.room));
        }
        let status = self.status_text(s);
        if !status.is_empty() {
            sub.push(status);
        }
        p.sub = sub.join(" · ");
        vec![p]
    }

    /// The host's New match choices, for the seats left to AI players.
    fn host_setup_items(&self, roster: &online::Roster) -> Vec<Item> {
        let (Some(t), Some(lobby)) = (self.tuning.as_ref(), self.lobby.as_ref()) else {
            return Vec::new();
        };
        let seats = t.game.seats as usize;
        let mut items = vec![Item::Select {
            id: "lobby.ai".into(),
            label: self.t("lobby.ai"),
            value: lobby.ai.to_string(),
            options: (0..seats)
                .map(|n| Choice {
                    value: n.to_string(),
                    text: n.to_string(),
                    disabled: false,
                })
                .collect(),
        }];
        let ai_seats = roster
            .seats
            .iter()
            .enumerate()
            .filter(|(_, s)| matches!(s.who, Who::Ai { .. }))
            .map(|(k, _)| k);
        for (i, k) in ai_seats.enumerate() {
            items.push(Item::Select {
                id: format!("lobby.ai.{}", i + 1),
                label: self.t("lobby.difficulty").replace(
                    "{colour}",
                    &self.t(&format!("colour.{}", sim::SEAT_COLOURS[k.min(3)])),
                ),
                value: lobby.difficulty[i].key().into(),
                options: sim::Difficulty::ALL
                    .iter()
                    .map(|d| Choice {
                        value: d.key().into(),
                        text: self.t(&format!("difficulty.{}", d.key())),
                        disabled: false,
                    })
                    .collect(),
            });
        }
        items.push(self.time_choices(
            "lobby.length",
            "lobby.length",
            lobby.length_ms,
            &t.game.length_choices_ms,
        ));
        items.push(self.time_choices(
            "lobby.buoy",
            "lobby.buoy",
            lobby.buoy_interval_ms,
            &t.game.buoy_interval_choices_ms,
        ));
        items.push(Item::Text {
            id: "lobby.seed".into(),
            label: self.t("lobby.seed"),
            value: lobby.seed.clone(),
            max: 10,
            placeholder: self.t("lobby.seed_random"),
        });
        items
    }

    /// The host's setup as a joiner sees it.
    fn setup_rows(&self, s: &SetupInfo) -> Vec<[String; 2]> {
        let span = |ms: i32| {
            if ms % 60_000 == 0 {
                self.t("units.min").replace("{n}", &(ms / 60_000).to_string())
            } else {
                self.t("units.s").replace("{n}", &(ms / 1000).to_string())
            }
        };
        vec![
            [self.t("lobby.ai"), s.ai.to_string()],
            [self.t("lobby.length"), span(s.length_ms)],
            [self.t("lobby.buoy"), span(s.buoy_interval_ms)],
            [
                self.t("lobby.seed"),
                if s.seed.is_empty() {
                    self.t("lobby.seed_random")
                } else {
                    s.seed.clone()
                },
            ],
        ]
    }
}
