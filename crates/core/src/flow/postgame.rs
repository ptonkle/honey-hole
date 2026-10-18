//! The postgame's reveal, as plain numbers: who shows what when, and the
//! final ranking. Scores stay hidden until here ("a la Mario Party"): each
//! player's catches are revealed one after another, in seat order, their
//! total ticking up with each card, then the ranking.
//!
//! Times are ms from the postgame's start ([`Reveal::at`]); the drawing
//! (`flow/screens.rs`) and the tally sound read the same [`Shown`].

use crate::sim::{Catch, Score};

use super::look::PostgameLook;

/// One player in the postgame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub seat: usize,
    pub name: String,
    pub catches: Vec<Catch>,
}

impl Column {
    pub fn total(&self) -> i64 {
        self.catches.iter().map(|c| c.value as i64).sum()
    }
}

/// The reveal's timetable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reveal {
    pub columns: Vec<Column>,
    /// when each card shows, by column
    pub card_ms: Vec<Vec<i64>>,
    /// when each column's turn starts
    pub starts: Vec<i64>,
    /// when the ranking shows
    pub rank_ms: i64,
    pub count_ms: i64,
}

/// What shows at one moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    /// cards shown, by column
    pub cards: Vec<usize>,
    /// each column's total as drawn (counting up to the newest card)
    pub totals: Vec<i64>,
    /// the column being revealed now, if any
    pub current: Option<usize>,
    pub ranking: bool,
}

/// Places for these totals: 1 for the best, and tied totals share a
/// place, the next one skipping (two firsts, then a third).
pub fn places(totals: &[i64]) -> Vec<usize> {
    totals
        .iter()
        .map(|t| 1 + totals.iter().filter(|o| *o > t).count())
        .collect()
}

/// The columns in ranking order: by place, then seat.
pub fn ranking(columns: &[Column]) -> Vec<(usize, &Column)> {
    let totals: Vec<i64> = columns.iter().map(Column::total).collect();
    let mut out: Vec<(usize, &Column)> = places(&totals).into_iter().zip(columns).collect();
    out.sort_by_key(|(place, c)| (*place, c.seat));
    out
}

/// The postgame's players: every taken seat with its catches, in seat order.
pub fn columns(names: &[(usize, String)], scores: &[Score]) -> Vec<Column> {
    names
        .iter()
        .map(|(seat, name)| Column {
            seat: *seat,
            name: name.clone(),
            catches: scores.get(*seat).map_or(Vec::new(), |s| s.catches.clone()),
        })
        .collect()
}

impl Reveal {
    /// The timetable for these columns: the first card after `first_ms`,
    /// one every `card_ms`, `gap_ms` more between players (a player with
    /// no catch still takes a card's time, shown as none), and the ranking
    /// `rank_ms` after the last.
    pub fn new(columns: Vec<Column>, look: &PostgameLook) -> Reveal {
        let (card, gap) = (look.card_ms as i64, look.gap_ms as i64);
        let mut t = look.first_ms as i64;
        let mut card_ms = Vec::new();
        let mut starts = Vec::new();
        for c in &columns {
            starts.push(t);
            let times: Vec<i64> = (0..c.catches.len() as i64).map(|k| t + k * card).collect();
            t += card * c.catches.len().max(1) as i64 + gap;
            card_ms.push(times);
        }
        Reveal {
            columns,
            card_ms,
            starts,
            rank_ms: t - gap + look.rank_ms as i64,
            count_ms: look.count_ms as i64,
        }
    }

    /// What shows `ms` after the start.
    pub fn at(&self, ms: i64) -> Shown {
        let mut cards = Vec::new();
        let mut totals = Vec::new();
        let mut current = None;
        for (k, (c, times)) in self.columns.iter().zip(&self.card_ms).enumerate() {
            let n = times.iter().filter(|t| **t <= ms).count();
            let mut total: i64 = c.catches[..n].iter().map(|f| f.value as i64).sum();
            if n > 0 {
                // the newest card's points count up
                let since = ms - times[n - 1];
                if since < self.count_ms {
                    let v = c.catches[n - 1].value as i64;
                    total -= v - v * since / self.count_ms;
                }
            }
            if self.starts[k] <= ms {
                current = Some(k);
            }
            cards.push(n);
            totals.push(total);
        }
        let ranking = ms >= self.rank_ms;
        if ranking {
            // every card up and counted
            for (k, c) in self.columns.iter().enumerate() {
                cards[k] = c.catches.len();
                totals[k] = c.total();
            }
            current = None;
        }
        Shown {
            cards,
            totals,
            current,
            ranking,
        }
    }

    /// Cards that showed after `from` and by `to` (one tally tick each).
    pub fn cards_between(&self, from: i64, to: i64) -> usize {
        self.card_ms
            .iter()
            .flatten()
            .filter(|t| **t > from && **t <= to && **t < self.rank_ms)
            .count()
    }
}
