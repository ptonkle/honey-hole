//! Online play for up to four browsers (PLAN.md decision 11): the lobby,
//! the start, and each browser's share of one match. The transport is
//! `web/online.js` (WebRTC introduced by room code over pinned Nostr
//! relays, or by pasted codes; copied from Ape Race and Mandible): it only
//! carries text. This module is a pure state machine ([`Session`]): text in
//! through `receive`, text out through `take_outbox`, time as `now_ms`.
//!
//! # The sync model: host-authoritative world, client-owned boats
//!
//! This game's players meet in two ways: their boats collide (a ram wrecks
//! the slower one), and they contend for shared things (a fish only one
//! line can hook, a buoy only one boat can pick up, an item used on
//! others). So:
//!
//! - **Each browser simulates its own boat** with no delay, and sends its
//!   state ([`BoatState`]) about 20 times a second ([`STATE_EVERY_MS`]) to
//!   every peer; the host passes each joiner's on to the other joiners too,
//!   so two joiners with no direct link still see each other (Ape Race's
//!   T71: with STUN only, joiners can fail to reach each other while both
//!   reach the host). A repeat is dropped as old. Every other boat is drawn
//!   where its owner said it was [`INTERP_TICKS`] ticks (100 ms) ago,
//!   between the two states either side of then, or carried on along its
//!   velocity for at most [`EXTRAPOLATE_TICKS`] when states are late.
//! - **The host runs the AI players** and sends their states like its own.
//! - **Rams are the host's to judge**, from the reported states (its own
//!   boats as they are, the others' newest state carried on to now), with
//!   the match's own rules (`wreck` in the tuning,
//!   `sim::collide::contact`). Until the verdict ([`Ram`]) arrives, boats
//!   that meet at ram speed pass through each other. Every browser applies
//!   the verdict: the victim's owner wrecks it, the rammer's owner stalls
//!   it, the rest show the popup. **Land wrecks** are judged by each owner
//!   for its own boat and reported, and the host tells everyone.
//! - **The host owns the timer and the end.** Its start message carries
//!   the map seed, the match seed, the seats, the fish counts and a hash of
//!   the match numbers ([`tuning_hash`]); every browser starts its match
//!   [`START_LEAD_MS`] after it was sent, less half its round trip. The
//!   host answers each ping with its match tick; a joiner reads the host's
//!   clock from the answer with the shortest of its last round trips (as
//!   NTP does: it is the one least held up), and jumps to it when it is
//!   [`CLOCK_SNAP_TICKS`] or more off. A joiner's match does not end
//!   itself; the host's end message ends it.
//! - **Shared things are claims.** See below.
//!
//! The host leaving ends the match for everyone ([`Status::HostLeft`]):
//! moving the host would mean moving the fish, the buoy and the claims
//! held. A joiner leaving takes its boat out of the match
//! ([`WorldEvent::Left`]); in the lobby its seat opens again.
//!
//! # Adding a claim (fishing, items, the buoy)
//!
//! A claim is a request for something two players could both want. The
//! host judges every claim, its own included, [`CLAIM_WINDOW_MS`] after it
//! was made, in the order made (by match tick, then seat), so two claims a
//! few milliseconds apart are settled the same way whatever the network
//! did. Every browser hears every verdict.
//!
//! 1. Add a variant to [`Claim`] (`sim/remote.rs`) with the fields the host
//!    needs to judge it, e.g. `HookFish { fish }`.
//! 2. Give it its [`Thing`] in `Claim::thing` if two players can contend
//!    for it (a fish, a buoy); a granted claim holds that thing for its
//!    seat until the system calls `m.online.release(&thing)` on the host
//!    (the fish is landed or freed, the buoy is gone).
//! 3. If first come, first served is not the whole rule (the fish must
//!    still be free and near the lure), set a [`Referee`] on the host's
//!    match: `m.online.referee = Some(my_rule)`, a `fn(&Match, seat,
//!    &Claim) -> bool` that reads the host's match. Call
//!    `sim::remote::first_come` from it to keep the holding rule.
//! 4. In the system's step, when the local player acts, call
//!    `m.claim(seat, Claim::HookFish { fish })` instead of deciding, and
//!    show the effect only once a verdict for it is in `m.online.verdicts`
//!    (`granted`, with the claim and the seat; the claim's number is what
//!    `claim` returned). Every browser sees every verdict, so the others'
//!    effects (a rival hooked fish 7: it is no longer free) come the same
//!    way. Offline, `claim` is answered at once (the next step), so one
//!    code path serves both. The fishing system's hook and catch are the
//!    worked example (`sim/fishing/`, `online_hooks.rs`).
//! 5. Changes only the host decides (a freed fish swims off, a buoy comes
//!    up) are world events: add a variant to [`WorldEvent`], call
//!    `m.broadcast(event)` on the host (`m.online.role == Role::Host`,
//!    or `Offline`, where it goes nowhere), and apply it in the system's
//!    step from `m.online.world` on the other browsers. A shared system on
//!    a client (`Role::Client`) applies what the host says and does not
//!    decide.
//! 6. Add a loopback test in `online/tests.rs` beside
//!    `two_claims_for_one_thing_are_settled_the_same_way_everywhere`.
//!
//! **The Fish-Boy's radar** reads the match: remote boats are in it like
//! any other, placed from their states, so the radar draws them with no
//! online code. Cloaking Tech sets `m.online.hidden[seat]` on the owner's
//! browser; it travels in the boat's state and the radar leaves a hidden
//! boat out. A radar ping that others should see is a world event.
//!
//! # For the app (app/online_play.rs)
//!
//! 1. **Open:** [`Session::host`] (seat 1) or [`Session::join`]; the page
//!    gets a room name ([`room_name`]) or the pasted-code requests.
//! 2. **Transport events:** `connected` → [`Session::peer_connected`],
//!    `message` → [`Session::receive`], `disconnected` →
//!    [`Session::peer_left`], `error` → [`Session::failed`].
//! 3. **Every frame:** [`Session::update`], then send [`Session::take_outbox`].
//! 4. **Lobby:** the host's setup through [`Session::set_setup`]; names
//!    through [`Session::set_name`]; a joiner's [`Session::set_ready`]; the
//!    host's [`Session::start`].
//! 5. **Start:** [`Session::take_start`] gives the [`MatchStart`] once its
//!    time comes; build the match from it, then [`Session::prepare`] it.
//! 6. **Each tick:** step the match with this browser's inputs, then
//!    [`Session::after_step`].
//! 7. **Leaving:** [`Session::leave`], send the outbox, then close the links.

pub mod msg;
mod session;

#[cfg(test)]
mod loopback;
#[cfg(test)]
mod tests;

pub use msg::{Msg, Refusal, Roster, SeatInfo, SetupInfo, Who};
pub use session::{MatchStart, Outgoing, Session, Status};

pub use crate::sim::remote::{BoatState, Claim, Ram, Referee, Thing, Verdict, WorldEvent};

/// The protocol version. Different versions never meet: the room name and
/// the Hello carry it.
pub const PROTOCOL: u32 = 1;
/// How often a joiner repeats a Hello, a Ready mark or a name until the
/// host shows it, and the host a start until each joiner acknowledges it.
pub const RETRY_MS: i64 = 500;
/// How often the host sends the lobby to everyone.
pub const ROSTER_EVERY_MS: i64 = 1000;
/// How often a joiner pings the host and the host every joiner.
pub const PING_EVERY_MS: i64 = 500;
/// How often a boat's owner sends its state (20 a second).
pub const STATE_EVERY_MS: i64 = 50;
/// How far behind its owner another boat is drawn: 5 ticks, 100 ms.
pub const INTERP_TICKS: u32 = 5;
/// The longest a late boat is carried on along its velocity: 200 ms.
pub const EXTRAPOLATE_TICKS: u32 = 10;
/// A joiner this many ticks or more off the host's clock jumps to it.
pub const CLOCK_SNAP_TICKS: u32 = 2;
/// The match starts this long after the host's start message.
pub const START_LEAD_MS: i64 = 1000;
/// A peer not heard from for this long has gone.
pub const SILENT_MS: i64 = 8000;
/// After judging a ram the host judges no other on that victim for this
/// long (its wreck is on its way).
pub const RAM_HOLD_MS: i64 = 1000;
/// The host judges a claim this long after it was made, so claims made
/// close together are settled in the order made, not the order they came.
pub const CLAIM_WINDOW_MS: i64 = 150;
/// Round trips kept a peer; the median of them is the round trip.
pub const RTTS_KEPT: usize = 8;
/// States kept a boat.
pub const STATES_KEPT: usize = 30;
/// The room code's length and letters (no 0, O, 1, I: easy to read out).
pub const CODE_LEN: usize = 6;
pub const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// A room code from a seed.
pub fn room_code(seed: u64) -> String {
    let mut z = seed;
    (0..CODE_LEN)
        .map(|_| {
            z = crate::map::rng::mix(z.wrapping_add(0x9E37_79B9_7F4A_7C15));
            CODE_ALPHABET[(z % CODE_ALPHABET.len() as u64) as usize] as char
        })
        .collect()
}

/// A typed room code, cleaned: upper case, its letters only; `None` when
/// it is not a whole code.
pub fn clean_code(text: &str) -> Option<String> {
    let code: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    (code.len() == CODE_LEN && code.bytes().all(|b| CODE_ALPHABET.contains(&b))).then_some(code)
}

/// The transport's room for a code: this game and protocol only.
pub fn room_name(code: &str) -> String {
    format!("honey-hole-p{PROTOCOL}-{code}")
}

/// A name as other players see it: no control characters, trimmed, at
/// most the name length the Settings field takes.
pub fn clean_name(name: &str) -> String {
    let s: String = name
        .chars()
        .filter(|c| !c.is_control())
        .take(crate::app::play::NAME_MAX as usize)
        .collect();
    s.trim().to_string()
}

/// A short hash of the match numbers, so browsers with different
/// `data/tuning.json` files are told apart (FNV-1a over the parsed values).
pub fn tuning_hash(t: &crate::tuning::Tuning) -> String {
    let text = format!("{t:?}");
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}
