//! The messages between browsers, as JSON text: one object with its type
//! in `t`, e.g. `{"t":"state","s":1,"k":340,...}`. A message that does not
//! read is dropped (a peer on another protocol never gets this far: the
//! room name and the Hello carry [`super::PROTOCOL`]).

use serde::{Deserialize, Serialize};

use crate::sim::remote::{BoatState, Claim, Ram, Verdict, WorldEvent};

/// Why the host would not seat a joiner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "why", rename_all = "snake_case")]
pub enum Refusal {
    /// every seat has a person in it
    Full,
    /// a match is under way
    Underway,
    /// another protocol or game version
    Version { host: String, mine: String },
    /// the same version, but other match numbers (`data/tuning.json`)
    Tuning,
}

/// Who sits in a lobby seat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "who", rename_all = "snake_case")]
pub enum Who {
    Open,
    Host,
    Player,
    /// an AI player, by difficulty key (`sim::Difficulty::key`)
    Ai {
        difficulty: String,
    },
}

/// One lobby seat, as the host tells everyone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatInfo {
    #[serde(flatten)]
    pub who: Who,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub ready: bool,
    /// the host's last round trip to this seat's browser, ms
    #[serde(default)]
    pub ping_ms: Option<i64>,
}

/// The host's match setup: the offline New match's choices.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupInfo {
    /// AI players wanted (fewer when people fill the seats)
    pub ai: usize,
    /// by AI player, in seat order: difficulty keys
    pub difficulty: Vec<String>,
    pub length_ms: i32,
    pub buoy_interval_ms: i32,
    /// typed digits, or empty for a random checked layout
    pub seed: String,
}

/// The lobby as the host keeps it and tells everyone.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Roster {
    /// rises with every change
    pub rev: u32,
    pub seats: Vec<SeatInfo>,
    pub setup: SetupInfo,
}

/// One seat of a starting match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartSeat {
    #[serde(flatten)]
    pub who: Who,
    #[serde(default)]
    pub name: String,
}

/// Everything a browser needs to build the same match as the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Start {
    /// a number for this start, so a repeat is known
    pub id: u32,
    pub tuning: String,
    pub map_seed: u32,
    pub match_seed: u64,
    pub seats: Vec<StartSeat>,
    pub length_ms: i32,
    pub buoy_interval_ms: i32,
    /// the match begins this long after the host sent this
    pub lead_ms: i64,
    /// how many of each kind of fish the match starts with
    /// (`sim::Setup::fish`), the host's, so every browser counts the same
    #[serde(default)]
    pub fish: Vec<(String, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Msg {
    // ---- the lobby ----
    /// joiner → host, repeated until seated or refused
    Hello {
        protocol: u32,
        version: String,
        tuning: String,
        name: String,
    },
    /// host → joiner: your seat
    Welcome {
        seat: usize,
    },
    Refused {
        refusal: Refusal,
    },
    /// host → everyone, on a change and every second
    Roster {
        roster: Roster,
    },
    /// joiner → host, repeated until the roster shows it
    Name {
        name: String,
    },
    Ready {
        ready: bool,
    },
    /// host → everyone, repeated to each joiner until acknowledged
    Start {
        start: Start,
    },
    StartAck {
        id: u32,
    },
    // ---- every half second, both ways ----
    Ping {
        at: i64,
    },
    /// `tick`: the host's match tick as it answers, once a match runs
    Pong {
        at: i64,
        #[serde(default)]
        tick: Option<u32>,
    },
    // ---- the match ----
    /// a boat's owner → everyone (the host passes joiners' on as well)
    State {
        #[serde(flatten)]
        state: BoatState,
    },
    /// host → everyone: a ram it judged
    Rammed {
        ram: Ram,
    },
    /// a joiner → host: something about its own boat (a grounding)
    Report {
        event: WorldEvent,
    },
    /// host → everyone
    World {
        event: WorldEvent,
    },
    /// anyone → host: a claim, made at this match tick
    Claim {
        seat: usize,
        id: u32,
        tick: u32,
        claim: Claim,
    },
    /// host → everyone: the answer to a claim
    Verdict {
        verdict: Verdict,
    },
    /// host → everyone: the match is over
    End,
    /// anyone → everyone: this browser is leaving
    Bye,
}

impl Msg {
    pub fn encode(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn decode(text: &str) -> Option<Msg> {
        serde_json::from_str(text).ok()
    }
}
