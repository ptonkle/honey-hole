//! Headless playtests: AI-only 4-player matches on Burt Lake, and the
//! balance numbers they come to (PLAN.md decision 10).
//!
//!     cargo run --release --example playtest              8 matches from seed 1
//!     cargo run --release --example playtest -- 20 7     20 matches from seed 7
//!
//!     cargo run --release --example playtest -- 48 1 balance.rammed_swim_mm.legendary=40000
//!                                                    the same with a tuning value changed
//!
//! An argument with `=` sets a value of `data/tuning.json` by its dotted
//! path for this run (the value is JSON: `20000`, `true`); any number of
//! them. The last line of the report, `SUMMARY`, holds the balance numbers
//! of `notes/balance-options.md` in one row.
//!
//! Half the matches are four Normal players, half a mix of difficulties.
//! Run from the repository root (it reads data/). It prints Markdown tables
//! for notes/playtests.md: catches and points by player and tier, how the
//! time is spent, wrecks, hooks lost to rams, the fish left by tier, reel
//! times by tier, the winner's margin, and the AI's cost per tick.

// tables by tier index read most plainly as index loops
#![allow(clippy::needless_range_loop)]

use std::fs;
use std::time::Instant;

use tool_core::ai::{Activity, Crew};
use tool_core::map::fish::Tier;
use tool_core::map::{generate, list, Setting};
use tool_core::roster::Roster;
use tool_core::sim::{Difficulty, Match, MatchEvent, Occupant, Setup, WreckCause};
use tool_core::tuning;

const TIERS: [Tier; 4] = [Tier::Common, Tier::Uncommon, Tier::Rare, Tier::Legendary];
const ACTS: [Activity; 4] = [
    Activity::Fishing,
    Activity::Hunting,
    Activity::Driving,
    Activity::Idle,
];

fn tier_ix(t: Tier) -> usize {
    TIERS.iter().position(|&x| x == t).unwrap_or(0)
}

fn act_ix(a: Activity) -> usize {
    ACTS.iter().position(|&x| x == a).unwrap_or(3)
}

fn diff_ix(d: Difficulty) -> usize {
    Difficulty::ALL.iter().position(|&x| x == d).unwrap_or(1)
}

#[derive(Default, Clone)]
struct SeatStats {
    catches: [u32; 4],
    value: [i64; 4],
    acts: [u64; 4],
    rammed_others: u32,
    was_rammed: u32,
    grounded: u32,
    hooks: u32,
    lost_to_rams: u32,
    /// points of the fish this seat's rams knocked off a line
    denied: i64,
    /// fish it knocked off a line and then landed itself, and their points
    steals: u32,
    stolen: i64,
}

#[derive(Default)]
struct Totals {
    matches: u32,
    by_diff: [SeatStats; 3],
    seats_by_diff: [u32; 3],
    reel_ms: [(i64, u32); 4],
    /// by tier: hooks, then caught, broke, rammed off, dropped otherwise
    outcomes: [[u32; 5]; 4],
    left: [(u32, u32); 4],
    margins: Vec<i64>,
    winners_by_diff: [u32; 3],
    ai_ns: u128,
    ai_ticks: u64,
    ai_worst_ns: u128,
    /// ticks whose four AI players took over 1 ms together
    ai_slow: u64,
    step_ns: u128,
    ticks: u64,
    all: SeatStats,
}

fn add(a: &mut SeatStats, b: &SeatStats) {
    for k in 0..4 {
        a.catches[k] += b.catches[k];
        a.value[k] += b.value[k];
        a.acts[k] += b.acts[k];
    }
    a.rammed_others += b.rammed_others;
    a.was_rammed += b.was_rammed;
    a.grounded += b.grounded;
    a.hooks += b.hooks;
    a.lost_to_rams += b.lost_to_rams;
    a.denied += b.denied;
    a.steals += b.steals;
    a.stolen += b.stolen;
}

fn lineup(k: u32) -> [Difficulty; 4] {
    use Difficulty::*;
    match k % 6 {
        1 => [Hard, Normal, Normal, Easy],
        3 => [Hard, Hard, Easy, Easy],
        5 => [Easy, Normal, Hard, Normal],
        _ => [Normal; 4],
    }
}

fn main() {
    let read = |p: &str| fs::read_to_string(p).unwrap_or_else(|e| panic!("{p}: {e}"));
    let all: Vec<String> = std::env::args().skip(1).collect();
    let (sets, args): (Vec<String>, Vec<String>) = all.into_iter().partition(|a| a.contains('='));
    let matches: u32 = args.first().and_then(|a| a.parse().ok()).unwrap_or(8);
    let base: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(1);
    // a third argument traces that seat's events and goals in each match
    let trace: Option<usize> = args.get(2).and_then(|a| a.parse().ok());
    let mut tv: serde_json::Value = serde_json::from_str(&read("data/tuning.json")).unwrap();
    for set in &sets {
        let (path, value) = set.split_once('=').unwrap();
        let mut at = &mut tv;
        for key in path.split('.') {
            at = at
                .get_mut(key)
                .unwrap_or_else(|| panic!("{path}: no key {key} in data/tuning.json"));
        }
        *at = serde_json::from_str(value).unwrap_or_else(|e| panic!("{set}: {e}"));
        eprintln!("set {path} = {value}");
    }
    let t = tuning::parse(&tv.to_string()).unwrap();
    let roster = Roster::parse(&read("data/fish.json")).unwrap();
    let maps = list::parse(&read("data/maps/michigan-lake.json")).unwrap();
    let names: serde_json::Value = serde_json::from_str(&read("data/names.json")).unwrap();
    let names: Vec<String> = names["ai"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let mut tot = Totals::default();
    println!("| Match | Map | Players (difficulty: points) | Winner's margin |");
    println!("| --- | --- | --- | --- |");
    for k in 0..matches {
        let seed = base + k as u64;
        let map_seed = list::pick(&maps, seed);
        let map = generate(Setting::MichiganLake, map_seed);
        let mut setup = Setup::new(&t);
        setup.seed = seed;
        setup.fish = roster.0.iter().map(|f| (f.id.clone(), f.count)).collect();
        let ups = lineup(k);
        for (s, d) in ups.iter().enumerate() {
            setup = setup.with(s, Occupant::Ai(*d), &names[(seed as usize * 3 + s) % names.len()]);
        }
        let mut m = Match::new(map, &t, setup).unwrap();
        m.stock(&roster).unwrap();
        let start: Vec<(Tier, u32)> = m
            .fish_left()
            .iter()
            .map(|(id, n)| (roster.get(id).unwrap().tier, *n))
            .collect();
        let mut crew = Crew::new(&m);
        let mut st = vec![SeatStats::default(); 4];
        let mut hooked_at = [None::<(u32, Tier)>; 4];
        let mut knocked: Vec<(u32, usize)> = Vec::new();
        while !m.ended {
            let a = Instant::now();
            let inputs = crew.inputs(&m);
            let spent = a.elapsed().as_nanos();
            tot.ai_ns += spent;
            tot.ai_worst_ns = tot.ai_worst_ns.max(spent);
            tot.ai_slow += (spent > 1_000_000) as u64;
            tot.ai_ticks += 4;
            for b in &crew.brains {
                st[b.seat].acts[act_ix(b.activity)] += 1;
            }
            let a = Instant::now();
            let events = m.step(&inputs).to_vec();
            if let Some(ts) = trace {
                for e in &events {
                    let txt = format!("{e:?}");
                    if !txt.starts_with("ReelTick")
                        && (txt.contains(&format!("seat: {ts}"))
                            || txt.starts_with("Hooked")
                            || txt.starts_with("Wrecked"))
                    {
                        let b = m.boat(ts).unwrap();
                        println!(
                            "{:6} {txt}  [at {:?} speed {}]",
                            m.tick,
                            b.at(),
                            b.speed_mm_per_s()
                        );
                    }
                }
                if m.tick.is_multiple_of(100) {
                    if let Some(br) = crew.brain(ts) {
                        let b = m.boat(ts).unwrap();
                        println!(
                            "{:6} goal {:?} {:?} stage {:?} at {:?} speed {} seen {}",
                            m.tick,
                            br.goal,
                            br.activity,
                            m.fishing.lines[ts].stage,
                            b.at(),
                            b.speed_mm_per_s(),
                            m.fishing_view(ts).fish.len()
                        );
                    }
                }
            }
            tot.step_ns += a.elapsed().as_nanos();
            tot.ticks += 1;
            let rammed: Vec<(usize, usize)> = events
                .iter()
                .filter_map(|e| match e {
                    MatchEvent::Wrecked {
                        seat,
                        cause: WreckCause::Rammed { by },
                    } => Some((*seat, *by)),
                    _ => None,
                })
                .collect();
            for e in &events {
                match e {
                    MatchEvent::Wrecked { seat, cause } => match cause {
                        WreckCause::Rammed { by } => {
                            st[*seat].was_rammed += 1;
                            st[*by].rammed_others += 1;
                        }
                        WreckCause::Grounded => st[*seat].grounded += 1,
                    },
                    MatchEvent::Hooked { seat, tier, .. } => {
                        st[*seat].hooks += 1;
                        tot.outcomes[tier_ix(*tier)][0] += 1;
                        hooked_at[*seat] = Some((m.tick, *tier));
                    }
                    MatchEvent::FishDropped { seat, fish } => {
                        let ti = tier_ix(m.fishing.fish[*fish as usize].tier);
                        let rammed_off = rammed.iter().any(|(v, _)| v == seat);
                        tot.outcomes[ti][if rammed_off { 3 } else { 4 }] += 1;
                        if let Some(&(_, by)) = rammed.iter().find(|(v, _)| v == seat) {
                            knocked.push((*fish, by));
                            st[*seat].lost_to_rams += 1;
                            let f = &m.fishing.fish[*fish as usize];
                            let kind = &m.fishing.roster.0[f.kind];
                            st[by].denied += tool_core::sim::fishing::value(
                                kind.value,
                                kind.base_length_mm,
                                f.length_mm,
                                t.fishing.tier_value_pct.get(f.tier),
                            ) as i64;
                        }
                        hooked_at[*seat] = None;
                    }
                    MatchEvent::LineBroke { seat, fish } => {
                        hooked_at[*seat] = None;
                        tot.outcomes[tier_ix(m.fishing.fish[*fish as usize].tier)][2] += 1;
                    }
                    MatchEvent::Caught { seat, tier, fish, .. } => {
                        if knocked.contains(&(*fish, *seat)) {
                            let c = m.scores[*seat].catches.last().map_or(0, |c| c.value);
                            st[*seat].steals += 1;
                            st[*seat].stolen += c as i64;
                        }
                        tot.outcomes[tier_ix(*tier)][1] += 1;
                        if let Some((h, _)) = hooked_at[*seat].take() {
                            let r = &mut tot.reel_ms[tier_ix(*tier)];
                            r.0 += (m.tick - h) as i64 * 20;
                            r.1 += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
        for (s, score) in m.scores.iter().enumerate() {
            for c in &score.catches {
                let tier = roster.get(&c.kind).unwrap().tier;
                st[s].catches[tier_ix(tier)] += 1;
                st[s].value[tier_ix(tier)] += c.value as i64;
            }
        }
        let left = m.fish_left();
        for (i, (_, n)) in left.iter().enumerate() {
            let ti = tier_ix(start[i].0);
            tot.left[ti].0 += *n;
            tot.left[ti].1 += start[i].1;
        }
        let pts: Vec<i64> = st.iter().map(|s| s.value.iter().sum()).collect();
        let mut sorted = pts.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        let margin = if sorted[0] > 0 {
            (sorted[0] - sorted[1]) * 100 / sorted[0]
        } else {
            0
        };
        tot.margins.push(margin);
        let win = pts.iter().position(|&p| p == sorted[0]).unwrap_or(0);
        tot.winners_by_diff[diff_ix(ups[win])] += 1;
        let who: Vec<String> = (0..4).map(|s| format!("{}: {}", ups[s].key(), pts[s])).collect();
        println!("| {} | {} | {} | {}% |", seed, map_seed, who.join(", "), margin);
        for s in 0..4 {
            add(&mut tot.by_diff[diff_ix(ups[s])], &st[s]);
            tot.seats_by_diff[diff_ix(ups[s])] += 1;
            add(&mut tot.all, &st[s]);
        }
        tot.matches += 1;
    }
    report(&tot);
}

fn report(tot: &Totals) {
    let names = ["Common", "Uncommon", "Rare", "Legendary"];
    println!();
    println!("### Catches and points per player, by difficulty (mean per match)");
    println!();
    println!("| Difficulty | Players | Common | Uncommon | Rare | Legendary | Points | Wins |");
    println!("| --- | --- | --- | --- | --- | --- | --- | --- |");
    for (d, diff) in Difficulty::ALL.iter().enumerate() {
        let n = tot.seats_by_diff[d].max(1) as f64;
        let s = &tot.by_diff[d];
        if tot.seats_by_diff[d] == 0 {
            continue;
        }
        let cell = |k: usize| {
            format!(
                "{:.1} ({:.0} pts)",
                s.catches[k] as f64 / n,
                s.value[k] as f64 / n
            )
        };
        println!(
            "| {} | {} | {} | {} | {} | {} | {:.0} | {} |",
            diff.key(),
            tot.seats_by_diff[d],
            cell(0),
            cell(1),
            cell(2),
            cell(3),
            s.value.iter().sum::<i64>() as f64 / n,
            tot.winners_by_diff[d]
        );
    }
    println!();
    println!("### Time spent (share of each player's match)");
    println!();
    println!("| Difficulty | Fishing | Hunting | Driving | Idle |");
    println!("| --- | --- | --- | --- | --- |");
    for (d, diff) in Difficulty::ALL.iter().enumerate() {
        let s = &tot.by_diff[d];
        let all = s.acts.iter().sum::<u64>().max(1) as f64;
        if tot.seats_by_diff[d] == 0 {
            continue;
        }
        println!(
            "| {} | {:.0}% | {:.0}% | {:.0}% | {:.0}% |",
            diff.key(),
            s.acts[0] as f64 * 100.0 / all,
            s.acts[1] as f64 * 100.0 / all,
            s.acts[2] as f64 * 100.0 / all,
            s.acts[3] as f64 * 100.0 / all
        );
    }
    let a = &tot.all;
    let m = tot.matches.max(1) as f64;
    println!();
    println!("### Wrecks, hooks and rams (mean per match, all four players)");
    println!();
    println!("| Hooks | Rams that wrecked | Groundings | Hooks lost to rams | Points knocked off lines |");
    println!("| --- | --- | --- | --- | --- |");
    println!(
        "| {:.1} | {:.2} | {:.2} | {:.2} | {:.0} |",
        a.hooks as f64 / m,
        a.rammed_others as f64 / m,
        a.grounded as f64 / m,
        a.lost_to_rams as f64 / m,
        a.denied as f64 / m
    );
    let fish_min = a.acts[0] as f64 / 50.0 / 60.0;
    let hunt_min = a.acts[1] as f64 / 50.0 / 60.0;
    let pts: i64 = a.value.iter().sum();
    println!();
    println!(
        "Points per minute spent fishing: {:.1}. Points knocked off a rival's line per minute spent hunting: {:.1}; the rammer then landed {} of those fish itself ({:.1} a match, {:.0} points a match).",
        pts as f64 / fish_min.max(0.001),
        if hunt_min > 0.1 { a.denied as f64 / hunt_min } else { 0.0 },
        a.steals,
        a.steals as f64 / m,
        a.stolen as f64 / m
    );
    println!();
    println!("### Fish left at the end, and reel times");
    println!();
    println!("| Tier | Left at the end (of stocked) | Mean reel to a catch |");
    println!("| --- | --- | --- |");
    for k in 0..4 {
        let (l, s) = tot.left[k];
        let (ms, n) = tot.reel_ms[k];
        println!(
            "| {} | {:.1} of {:.1} ({:.0}%) | {} |",
            names[k],
            l as f64 / m,
            s as f64 / m,
            l as f64 * 100.0 / s.max(1) as f64,
            if n == 0 {
                "none".to_string()
            } else {
                format!("{:.1} s ({} reels)", ms as f64 / n as f64 / 1000.0, n)
            }
        );
    }
    println!();
    println!("### What became of each hook (all matches)");
    println!();
    println!("| Tier | Hooks | Landed | Line broke | Rammed off | Dropped (fled, hunted, time up) |");
    println!("| --- | --- | --- | --- | --- | --- |");
    for k in 0..4 {
        let o = tot.outcomes[k];
        println!(
            "| {} | {} | {} | {} | {} | {} |",
            names[k], o[0], o[1], o[2], o[3], o[4]
        );
    }
    let mut margins = tot.margins.clone();
    margins.sort_unstable();
    let mean = margins.iter().sum::<i64>() as f64 / margins.len().max(1) as f64;
    println!();
    println!(
        "Winner's margin over second place: mean {:.0}%, median {}%, closest {}%, widest {}%.",
        mean,
        margins.get(margins.len() / 2).copied().unwrap_or(0),
        margins.first().copied().unwrap_or(0),
        margins.last().copied().unwrap_or(0)
    );
    println!();
    println!(
        "AI cost: {:.1} µs per AI per tick on average; the slowest tick of four AIs {:.0} µs, and {} ticks in {} over 1 ms; the match step {:.1} µs per tick ({} ticks).",
        tot.ai_ns as f64 / tot.ai_ticks.max(1) as f64 / 1e3,
        tot.ai_worst_ns as f64 / 1e3,
        tot.ai_slow,
        tot.ticks,
        tot.step_ns as f64 / tot.ticks.max(1) as f64 / 1e3,
        tot.ticks
    );
    // one row for notes/balance-options.md
    let acts = a.acts.iter().sum::<u64>().max(1) as f64;
    let (left, stocked) = tot.left.iter().fold((0, 0), |(l, s), x| (l + x.0, s + x.1));
    let leg = tot.outcomes[3];
    println!();
    println!(
        "SUMMARY | {} | {:.0} | {:.0} | {:.0} | {} of {} ({:.0}%) | {:.0}% / {}% | {:.0}% | {:.0}% (legendary {:.0}%) | {:.1} | {:.0} | {:.0} |",
        tot.matches,
        pts as f64 / fish_min.max(0.001),
        if hunt_min > 0.1 { a.denied as f64 / hunt_min } else { 0.0 },
        if hunt_min > 0.1 { a.stolen as f64 / hunt_min } else { 0.0 },
        leg[1],
        leg[0],
        leg[1] as f64 * 100.0 / leg[0].max(1) as f64,
        mean,
        margins.get(margins.len() / 2).copied().unwrap_or(0),
        a.acts[1] as f64 * 100.0 / acts,
        left as f64 * 100.0 / stocked.max(1) as f64,
        tot.left[3].0 as f64 * 100.0 / tot.left[3].1.max(1) as f64,
        a.rammed_others as f64 / m,
        pts as f64 / m / 4.0,
        a.denied as f64 / m / 4.0
    );
}
