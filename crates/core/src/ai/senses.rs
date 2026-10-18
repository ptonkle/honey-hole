//! What one seat can know, and nothing more: the AI's only window on the
//! match (PLAN.md decision 10, "fair play").
//!
//! - its own boat, its Fish-Boy and its own ping's sightings;
//! - the fishing as its seat sees it ([`Match::fishing_view`]): the fish in
//!   sight, its own line and reel bar;
//! - rival boats it can see (within the map's fog distance with no land in
//!   between) or hear (a motor within `ai.hear_mm`), with their lines;
//! - this tick's public events: hook popups and wreck popups, and its own
//!   line's outcomes;
//! - the rules everyone knows: the map's layout, the tuning, the roster.

use crate::fishboy::radar::PingState;
use crate::map::fish::Tier;
use crate::map::{Map, P};
use crate::roster::Roster;
use crate::sim::fishing::view::FishingView;
use crate::sim::fishing::Stage;
use crate::sim::{Boat, Match, MatchEvent, Rules, WreckCause};
use crate::tuning::Tuning;

/// A rival boat in sight or earshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RivalSeen {
    pub seat: usize,
    pub at: P,
    pub heading: i32,
    /// mm/s (east, north), as its wake shows
    pub vel: [i32; 2],
    pub wrecked: bool,
    pub ghosted: bool,
    /// its line, when it has one out
    pub line: Option<Stage>,
}

/// A rival where this seat's ping last saw it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pinged {
    pub seat: usize,
    pub at: P,
    pub tick: u32,
}

/// What happened this tick that the seat knows of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum News {
    /// a hook popup: `seat` hooked a fish of this kind and tier
    Hooked { seat: usize, kind: String, tier: Tier },
    /// a wreck popup
    Wrecked { seat: usize, rammed_by: Option<usize> },
    /// its own line: the fish got away (missed, scared, broke or dropped)
    Lost,
    /// its own line: a catch
    Caught,
}

/// One seat's senses for one tick.
pub struct Senses<'a> {
    pub seat: usize,
    pub tick: u32,
    pub map: &'a Map,
    pub rules: &'a Rules,
    pub tuning: &'a Tuning,
    pub roster: &'a Roster,
    /// its own boat (`None` for an empty seat)
    pub me: Option<&'a Boat>,
    pub fishboy_up: bool,
    pub ping: PingState,
    pub pinged: Vec<Pinged>,
    pub fishing: FishingView,
    pub rivals: Vec<RivalSeen>,
    pub news: Vec<News>,
    /// the match has ended
    pub ended: bool,
}

/// What `seat` senses of `m` now, after the last step.
pub fn sense<'a>(m: &'a Match, seat: usize) -> Senses<'a> {
    let me = m.boat(seat);
    let t = &m.tuning;
    let sight = m.map.setting.params().sight_mm as i64;
    let hear = t.ai.hear_mm as i64;
    let fishing = m.fishing_view(seat);
    let mut rivals = Vec::new();
    if let Some(b) = me.filter(|b| !b.wrecked()) {
        for r in m.boats.iter().flatten().filter(|r| r.seat != seat) {
            let d2 = r.at().dist2(b.at());
            let heard = !r.wrecked() && d2 <= hear * hear;
            let seen = d2 <= sight * sight && !super::nav::land_between(&m.map, b.at(), r.at());
            if !(heard || seen) {
                continue;
            }
            rivals.push(RivalSeen {
                seat: r.seat,
                at: r.at(),
                heading: r.heading,
                vel: r.vel(),
                wrecked: r.wrecked(),
                ghosted: r.ghosted(m.tick),
                line: fishing.lines.iter().find(|l| l.seat == r.seat).map(|l| l.stage),
            });
        }
    }
    let s = &m.seats[seat];
    let order: Vec<usize> = m.boats.iter().flatten().map(|b| b.seat).collect();
    let pinged = s
        .ping
        .sightings
        .iter()
        .enumerate()
        .filter_map(|(k, sg)| {
            let sg = (*sg)?;
            let who = *order.get(k)?;
            (who != seat).then_some(Pinged {
                seat: who,
                at: sg.at,
                tick: sg.tick.max(0) as u32,
            })
        })
        .collect();
    let mut news = Vec::new();
    for e in &m.events {
        match e {
            MatchEvent::Hooked { seat: s, kind, tier, .. } if *s != seat => news.push(News::Hooked {
                seat: *s,
                kind: kind.clone(),
                tier: *tier,
            }),
            MatchEvent::Wrecked { seat: s, cause } => news.push(News::Wrecked {
                seat: *s,
                rammed_by: match cause {
                    WreckCause::Rammed { by } => Some(*by),
                    WreckCause::Grounded => None,
                },
            }),
            MatchEvent::Missed { seat: s }
            | MatchEvent::Scared { seat: s }
            | MatchEvent::LineBroke { seat: s, .. }
            | MatchEvent::FishDropped { seat: s, .. }
                if *s == seat =>
            {
                news.push(News::Lost)
            }
            MatchEvent::Caught { seat: s, .. } if *s == seat => news.push(News::Caught),
            _ => {}
        }
    }
    Senses {
        seat,
        tick: m.tick,
        map: &m.map,
        rules: &m.rules,
        tuning: &m.tuning,
        roster: &m.fishing.roster,
        me,
        fishboy_up: s.fishboy,
        ping: s.ping.state(m.tick as i64, &t.radar),
        pinged,
        fishing,
        rivals,
        news,
        ended: m.ended,
    }
}
