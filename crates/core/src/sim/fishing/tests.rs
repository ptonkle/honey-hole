//! Fishing. Expected values are worked by hand from the sample tuning
//! (`tuning::tests::fishing_sample`): a full charge is 1000 ms, 50 ticks,
//! from 4 m to 24 m, so each tick of charge adds 400 mm; the hook window is
//! 760 ms, 38 ticks; the boat's bow is 1000 + 850 = 1850 mm ahead of its
//! middle; a perfect reel takes 3 / 5 / 10 / 20 s by tier.

use super::lake::{FishState, LakeFish, School};
use super::reel::{Outcome, Reel, BAR, FULL};
use super::*;
use crate::map::{Setting, Spawn};
use crate::sim::{Match, MatchEvent, Mode, Occupant, Setup, WreckCause};
use crate::tuning::{self, Tuning};

const CELL: i32 = 4000;
/// Where the test boat floats, facing east, in the middle of a 160 m lake.
const AT: P = P { x: 60_000, y: 80_000 };
/// Its bow.
const BOW_X: i32 = AT.x + 1850;

const ROSTER: &str = r#"{"fish": [
    {"id": "bluegill", "name": "Bluegill", "tier": "common", "model": "bluegill", "base_length_mm": 200, "value": 10, "count": 16},
    {"id": "walleye", "name": "Walleye", "tier": "uncommon", "model": "walleye", "base_length_mm": 400, "value": 20, "count": 10},
    {"id": "northern-pike", "name": "Northern pike", "tier": "rare", "model": "northern-pike", "base_length_mm": 600, "value": 30, "count": 6},
    {"id": "muskellunge", "name": "Muskellunge", "tier": "legendary", "model": "muskellunge", "base_length_mm": 1000, "value": 50, "count": 3}
]}"#;

fn sample() -> Tuning {
    tuning::tests::sample()
}

fn roster() -> Roster {
    Roster::parse(ROSTER).unwrap()
}

/// A square lake 40 cells a side with land round the edge.
fn lake_map() -> Map {
    let n = 40;
    let water: Vec<bool> = (0..n * n)
        .map(|i| {
            let (x, y) = (i % n, i / n);
            !(x == 0 || y == 0 || x == n - 1 || y == n - 1)
        })
        .collect();
    let spawn = Spawn {
        at: AT,
        heading: 0,
        dock: P::new(0, AT.y),
    };
    Map::from_water(Setting::MichiganLake, 0, n, CELL, &water, spawn)
}

/// A match with `humans` seats, no fish stocked, every boat in fishing
/// mode at rest. Seat 0 sits at `AT` facing east; seat 1 30 m north of it.
fn fresh(humans: usize) -> Match {
    let t = sample();
    let mut s = Setup::new(&t);
    for k in 0..humans {
        s = s.with(k, Occupant::Human, &format!("P{k}"));
    }
    let mut m = Match::new(lake_map(), &t, s).unwrap();
    m.fishing.roster = roster();
    m.fishing.fish_tuning.roam_radius_mm = 0;
    for k in 0..humans {
        let b = m.boats[k].as_mut().unwrap();
        b.set_at(P::new(AT.x, AT.y + 30_000 * k as i32));
        b.prev_um = b.pos_um;
        b.heading = 0;
        b.mode = Mode::Fishing;
    }
    m
}

/// Puts a fish of roster kind `kind` in the lake, as a school of one.
fn put_fish(m: &mut Match, kind: usize, at: P, depth_mm: i32, length_mm: i32) -> usize {
    let tier = m.fishing.roster.0[kind].tier;
    m.fishing.schools.push(School {
        at,
        home: at,
        target: at,
        rest_until: 0,
    });
    let id = m.fishing.fish.len();
    m.fishing.fish.push(LakeFish {
        id: id as u32,
        kind,
        tier,
        length_mm,
        at,
        depth_mm,
        home_depth_mm: depth_mm,
        heading: 0,
        school: m.fishing.schools.len() - 1,
        offset: P::new(0, 0),
        state: FishState::Roaming,
        phase_ms: 0,
    });
    id
}

fn input(pressed: &[&str], held: &[&str], released: &[&str]) -> SeatInput {
    let mut i = SeatInput::default();
    for n in pressed {
        i.pressed.set(n, true);
    }
    for n in held {
        i.held.set(n, true);
    }
    for n in released {
        i.released.set(n, true);
    }
    i
}

fn space_down() -> SeatInput {
    input(&["cast"], &["cast"], &[])
}

fn space_held() -> SeatInput {
    input(&[], &["cast"], &[])
}

fn space_up() -> SeatInput {
    input(&[], &[], &["cast"])
}

fn tap(name: &str) -> SeatInput {
    input(&[name], &[], &[name])
}

/// Steps with seat 0's input (the others idle); returns its events.
fn step0(m: &mut Match, i: SeatInput) -> Vec<MatchEvent> {
    let mut inputs = vec![SeatInput::default(); m.seats.len()];
    inputs[0] = i;
    m.step(&inputs).to_vec()
}

/// Steps idle until an event matches, at most `limit` ticks; the events of
/// that tick, or a panic.
fn until(m: &mut Match, limit: u32, want: impl Fn(&MatchEvent) -> bool) -> Vec<MatchEvent> {
    for _ in 0..limit {
        let e = step0(m, SeatInput::default());
        if e.iter().any(&want) {
            return e;
        }
    }
    panic!("nothing came in {limit} ticks; line {:?}", m.fishing.lines[0].stage)
}

/// A tap of Space: the shortest cast, 4 m from the bow.
fn cast_short(m: &mut Match) {
    step0(m, tap("cast"));
}

/// Seat 0 casts 4 m to a fish waiting just past where the bobber lands,
/// at a depth the lure reaches, and the lure is stopped there. Returns the
/// fish.
fn lure_set_by_fish(m: &mut Match, kind: usize) -> usize {
    let f = put_fish(m, kind, P::new(BOW_X + 4000 + 800, AT.y), 1000, 300);
    cast_short(m);
    until(m, 200, |e| matches!(e, MatchEvent::BobberLanded { .. }));
    // 1 m/s: the lure is a metre down after 50 ticks
    while m.fishing.lines[0].lure_depth_mm < 1000 {
        step0(m, SeatInput::default());
    }
    let e = step0(m, tap("cast"));
    assert!(e.iter().any(|e| matches!(e, MatchEvent::LureStopped { seat: 0, .. })), "{e:?}");
    f
}

fn is_bite(e: &MatchEvent) -> bool {
    matches!(e, MatchEvent::Bite { seat: 0 })
}

fn is_hooked(e: &MatchEvent) -> bool {
    matches!(e, MatchEvent::Hooked { seat: 0, .. })
}

/// Seat 0 with a fish of `kind` on the line, its reel tamed (the marker
/// holds still at nine tenths up).
fn hooked(m: &mut Match, kind: usize) -> usize {
    let f = lure_set_by_fish(m, kind);
    until(m, 1000, is_bite);
    m.fishing.tame = true;
    let e = step0(m, tap("cast"));
    assert!(e.iter().any(is_hooked), "{e:?}");
    f
}

#[test]
fn a_cast_is_refused_above_the_speed_limit() {
    let mut m = fresh(1);
    // the boat coasts 53 mm/s a tick: 1300 is still over 1000 when the
    // line looks
    m.boats[0].as_mut().unwrap().set_vel([1300, 0]);
    let e = step0(&mut m, space_down());
    assert!(e.iter().any(|e| matches!(e, MatchEvent::CastRefused { seat: 0, .. })), "{e:?}");
    assert_eq!(m.fishing.lines[0].stage, Stage::Idle);
    assert!(m.fishing_view(0).too_fast);
    // at 900 the charge starts
    m.boats[0].as_mut().unwrap().set_vel([900, 0]);
    let e = step0(&mut m, space_down());
    assert!(e.iter().any(|e| matches!(e, MatchEvent::Charging { seat: 0 })), "{e:?}");
    assert_eq!(m.fishing.lines[0].stage, Stage::Charging);
}

#[test]
fn a_cast_needs_fishing_mode() {
    let mut m = fresh(1);
    m.boats[0].as_mut().unwrap().mode = Mode::Driving;
    step0(&mut m, space_down());
    assert_eq!(m.fishing.lines[0].stage, Stage::Idle);
}

/// The cast's distance after holding Space for `held` ticks.
fn cast_after(held: u32) -> i32 {
    let mut m = fresh(1);
    step0(&mut m, space_down());
    for _ in 1..held {
        step0(&mut m, space_held());
    }
    let e = step0(&mut m, space_up());
    e.iter()
        .find_map(|e| match e {
            MatchEvent::Cast { distance_mm, .. } => Some(*distance_mm),
            _ => None,
        })
        .expect("a cast")
}

#[test]
fn the_cast_goes_further_the_longer_space_is_held() {
    // 4 m plus 400 mm a tick of charge, up to 24 m at 50 ticks
    assert_eq!(cast_after(1), 4400);
    assert_eq!(cast_after(10), 8000);
    assert_eq!(cast_after(25), 14_000);
    assert_eq!(cast_after(50), 24_000);
    assert_eq!(cast_after(80), 24_000);
}

#[test]
fn the_bobber_lands_in_the_swivel_direction_and_the_lure_sinks_to_where_space_stops_it() {
    let mut m = fresh(1);
    // swivelled a quarter turn left: the cast goes north
    m.boats[0].as_mut().unwrap().cast_yaw = crate::map::trig::QUARTER;
    cast_short(&mut m);
    let l = &m.fishing.lines[0];
    assert_eq!(l.stage, Stage::Flying);
    assert!((l.to.x - BOW_X).abs() < 60 && (l.to.y - (AT.y + 4000)).abs() < 60, "{:?}", l.to);
    // a 1 s flight
    let e = until(&mut m, 60, |e| matches!(e, MatchEvent::BobberLanded { .. }));
    assert_eq!(m.tick, 51, "{e:?}");
    assert_eq!(m.fishing.lines[0].stage, Stage::Sinking);
    for _ in 0..30 {
        step0(&mut m, SeatInput::default());
    }
    // 20 mm a tick for 30 ticks, then one more with Space
    let e = step0(&mut m, tap("cast"));
    assert!(e.contains(&MatchEvent::LureStopped { seat: 0, depth_mm: 620 }), "{e:?}");
    assert_eq!(m.fishing.lines[0].stage, Stage::Set);
}

#[test]
fn the_lake_bed_stops_the_lure() {
    let mut m = fresh(1);
    cast_short(&mut m);
    let e = until(&mut m, 2000, |e| matches!(e, MatchEvent::LureStopped { .. }));
    let bed = lake::bed_mm(&m.map, m.fishing.lines[0].bobber);
    assert!(e.contains(&MatchEvent::LureStopped {
        seat: 0,
        depth_mm: bed - 200
    }));
}

#[test]
fn nibbles_come_before_the_bite_after_a_wait() {
    let mut m = fresh(1);
    let f = lure_set_by_fish(&mut m, 0);
    // the fish notices the lure on the next tick
    assert_eq!(m.fishing.fish[f].state, FishState::Roaming);
    step0(&mut m, SeatInput::default());
    assert_eq!(m.fishing.fish[f].state, FishState::Lured { seat: 0 });
    let mut nibbles = Vec::new();
    let mut arrived = None;
    loop {
        let e = step0(&mut m, SeatInput::default());
        if arrived.is_none() && m.fishing.lines[0].interest.is_some_and(|i| i.arrived) {
            arrived = Some(m.tick);
        }
        if e.contains(&MatchEvent::Nibble { seat: 0 }) {
            nibbles.push(m.tick);
        }
        if e.iter().any(is_bite) {
            break;
        }
        assert!(m.tick < 2000, "no bite");
    }
    // 1 to 3 nibbles, the first 2 to 6 s after the fish got there
    assert!((1..=3).contains(&nibbles.len()), "{nibbles:?}");
    let wait = nibbles[0] - arrived.unwrap();
    assert!((100..=300).contains(&wait), "{wait} ticks");
    assert!(m.fishing_view(0).biting);
}

#[test]
fn space_before_the_bite_scares_the_fish_off() {
    let mut m = fresh(1);
    let f = lure_set_by_fish(&mut m, 0);
    until(&mut m, 1000, |e| matches!(e, MatchEvent::Nibble { seat: 0 }));
    let e = step0(&mut m, tap("cast"));
    assert!(e.contains(&MatchEvent::Scared { seat: 0 }), "{e:?}");
    assert!(matches!(m.fishing.fish[f].state, FishState::Fleeing { .. }));
    assert_eq!(m.fishing.lines[0].stage, Stage::Set, "the line stays out");
}

/// A match at the bite, and the tick it bit on.
fn at_bite() -> (Match, u32) {
    let mut m = fresh(1);
    lure_set_by_fish(&mut m, 0);
    until(&mut m, 1000, is_bite);
    let t = m.tick;
    (m, t)
}

#[test]
fn space_within_the_hook_window_hooks_and_a_tick_later_misses() {
    // the window is 38 ticks: Space on the 38th tick after the bite hooks
    let (mut m, bit) = at_bite();
    let mut late = m.clone();
    for _ in 0..37 {
        step0(&mut m, SeatInput::default());
    }
    assert_eq!(m.tick, bit + 37);
    let e = step0(&mut m, tap("cast"));
    assert!(e.iter().any(is_hooked), "{e:?}");
    assert_eq!(m.fishing.lines[0].stage, Stage::Reeling);
    // a tick later the fish has gone
    let mut gone = Vec::new();
    for _ in 0..38 {
        gone.extend(step0(&mut late, SeatInput::default()));
    }
    assert!(gone.contains(&MatchEvent::Missed { seat: 0 }), "{gone:?}");
    let e = step0(&mut late, tap("cast"));
    assert!(!e.iter().any(is_hooked), "{e:?}");
}

#[test]
fn a_hook_names_the_fish_and_its_tier() {
    let mut m = fresh(1);
    let f = hooked(&mut m, 2);
    let hooked: Vec<MatchEvent> = m.events.clone();
    assert!(hooked.contains(&MatchEvent::Hooked {
        seat: 0,
        fish: f as u32,
        kind: "northern-pike".into(),
        tier: Tier::Rare
    }));
    assert_eq!(m.fishing.fish[f].state, FishState::Hooked { seat: 0 });
    let v = serde_json::to_value(&hooked[0]).unwrap();
    assert_eq!(v["tier"], "Rare");
}

/// Ticks a perfect reel takes: the zone kept on the marker every tick.
fn perfect_ticks(tier: Tier) -> u32 {
    let t = sample().fishing.reel;
    let mut rng = Rng::new(9);
    let mut r = Reel::new(tier, &t, false);
    for tick in 1..100_000 {
        r.zone = (r.marker - r.zone_h / 2).clamp(0, BAR - r.zone_h);
        r.zone_v = 0;
        if r.step(&t, &mut rng, tick, false, 100) == Outcome::Landed {
            // the zone moved under gravity this tick; it is still on the marker
            return tick;
        }
        assert_ne!(r.meter, 0, "a perfect reel broke");
    }
    panic!("never landed");
}

#[test]
fn a_perfect_reel_takes_the_tiers_time() {
    // 3, 5, 10 and 20 s are 150, 250, 500 and 1000 ticks, within 10%
    for (tier, want) in [
        (Tier::Common, 150),
        (Tier::Uncommon, 250),
        (Tier::Rare, 500),
        (Tier::Legendary, 1000),
    ] {
        let got = perfect_ticks(tier);
        assert!(got * 10 >= want * 9 && got * 10 <= want * 11, "{tier:?}: {got} ticks");
    }
}

#[test]
fn the_power_reel_doubles_the_reel() {
    let t = sample().fishing.reel;
    let r = Reel::new(Tier::Common, &t, false);
    // 700 000 of the meter in 150 ticks: 4667 a tick
    assert_eq!(r.fill_per_tick(&t, 100), 4667);
    assert_eq!(r.fill_per_tick(&t, 200), 9334);
}

#[test]
fn the_meter_drains_out_of_the_zone_and_the_line_breaks_only_when_empty() {
    let t = sample().fishing.reel;
    let mut rng = Rng::new(1);
    // tamed, the marker sits at 900 of 1000; the zone falls away from it
    let mut r = Reel::new(Tier::Common, &t, true);
    let mut last = r.meter;
    let mut tick = 0;
    loop {
        tick += 1;
        let out = r.step(&t, &mut rng, tick, false, 100);
        if r.in_zone() {
            assert!(r.meter > last);
        } else {
            // full to empty in 5 s: 4000 a tick
            assert_eq!(r.meter, (last - 4000).max(0));
        }
        if out == Outcome::Broke {
            assert_eq!(r.meter, 0);
            break;
        }
        assert!(r.meter > 0);
        last = r.meter;
    }
    // from 300 000 at 4000 a tick: 75 ticks, give or take the first ticks in the zone
    assert!((70..=90).contains(&tick), "{tick}");
}

#[test]
fn holding_space_on_a_tamed_fish_lands_it() {
    let t = sample().fishing.reel;
    let mut rng = Rng::new(1);
    let mut r = Reel::new(Tier::Common, &t, true);
    let mut tick = 0;
    while r.step(&t, &mut rng, tick, true, 100) == Outcome::Fighting {
        tick += 1;
        assert!(tick < 1000);
    }
    assert_eq!(r.meter, FULL);
}

#[test]
fn a_broken_line_loses_the_fish_which_stays_in_the_lake_and_swims_away() {
    let mut m = fresh(1);
    let f = hooked(&mut m, 0);
    let was = m.fishing.fish[f].at;
    let e = until(&mut m, 500, |e| matches!(e, MatchEvent::LineBroke { seat: 0, .. }));
    assert!(e.contains(&MatchEvent::LineBroke {
        seat: 0,
        fish: f as u32
    }));
    assert_eq!(m.fishing.lines[0].stage, Stage::Idle);
    assert!(m.scores[0].catches.is_empty());
    // it swims 12 m off at 3 m/s, then settles
    for _ in 0..400 {
        step0(&mut m, SeatInput::default());
    }
    let fish = &m.fishing.fish[f];
    assert!(fish.in_lake());
    assert_eq!(fish.state, FishState::Roaming);
    let moved = fish.at.dist(was);
    assert!((9000..=14_000).contains(&moved), "moved {moved} mm");
    assert_eq!(m.fishing.left(0), 1, "the count left is unchanged");
}

#[test]
fn landing_scores_the_fish_at_its_value_and_it_is_never_caught_twice() {
    let mut m = fresh(1);
    let f = hooked(&mut m, 0);
    m.fishing.fish[f].length_mm = 300;
    let mut got = Vec::new();
    for _ in 0..400 {
        got.extend(step0(&mut m, space_held()));
        if !m.scores[0].catches.is_empty() {
            break;
        }
    }
    // a 300 mm bluegill of base 200 and value 10, common (100%): 15
    assert_eq!(
        m.scores[0].catches,
        vec![Catch {
            kind: "bluegill".into(),
            length_mm: 300,
            value: 15
        }]
    );
    assert!(got.contains(&MatchEvent::Caught {
        seat: 0,
        fish: f as u32,
        kind: "bluegill".into(),
        tier: Tier::Common,
        length_mm: 300
    }));
    assert_eq!(m.fishing.left(0), 0);
    assert_eq!(m.fishing.lines[0].stage, Stage::Landed);
    assert!(m.fishing_view(0).fish.iter().all(|s| s.id != f as u32));
    // held up for 2 s, then the line is free again; no lure finds it
    for _ in 0..100 {
        step0(&mut m, SeatInput::default());
    }
    assert_eq!(m.fishing.lines[0].stage, Stage::Idle);
    cast_short(&mut m);
    for _ in 0..600 {
        step0(&mut m, SeatInput::default());
    }
    assert!(m.fishing.lines[0].interest.is_none());
}

#[test]
fn a_value_is_base_times_tier_times_length_factor() {
    // a bluegill of 255 mm (base 170, value 10), common: 10 × 1 × 1.5 = 15
    assert_eq!(value(10, 170, 255, 100), 15);
    // a muskie at 1.5 times its 950 mm, value 55, legendary 5×: 412.5, rounded up
    assert_eq!(value(55, 950, 1425, 500), 413);
    // a walleye at its base length, uncommon 2×
    assert_eq!(value(25, 430, 430, 200), 50);
}

#[test]
fn the_camera_raises_the_last_catch_once() {
    let mut m = fresh(1);
    let mut s = Score::default();
    assert!(!m.fishing.use_camera(0, &mut s));
    s.catches.push(Catch {
        kind: "walleye".into(),
        length_mm: 430,
        value: 50,
    });
    assert!(m.fishing.use_camera(0, &mut s));
    assert_eq!(s.catches[0].value, 75);
    assert!(!m.fishing.use_camera(0, &mut s));
    assert_eq!(s.catches[0].value, 75);
}

/// The fish is dropped: it swims off and the line is idle.
fn assert_dropped(m: &Match, f: usize, e: &[MatchEvent]) {
    assert!(e.contains(&MatchEvent::FishDropped {
        seat: 0,
        fish: f as u32
    }), "{e:?}");
    assert!(matches!(m.fishing.fish[f].state, FishState::Fleeing { .. }));
    assert_eq!(m.fishing.lines[0].stage, Stage::Idle);
}

#[test]
fn a_wreck_drops_the_fish() {
    let mut m = fresh(1);
    let f = hooked(&mut m, 0);
    m.events.clear();
    let tick = m.tick;
    m.wreck(0, WreckCause::Grounded, tick);
    let e = m.events.clone();
    assert_dropped(&m, f, &e);
}

#[test]
fn esc_drops_the_fish_and_then_opens_the_menu() {
    let mut m = fresh(1);
    let f = hooked(&mut m, 0);
    let e = step0(&mut m, tap("cancel"));
    assert_dropped(&m, f, &e);
    assert!(!m.seats[0].menu_open, "the first Esc only drops the fish");
    step0(&mut m, tap("cancel"));
    assert!(m.seats[0].menu_open);
}

#[test]
fn esc_reels_a_line_in() {
    let mut m = fresh(1);
    cast_short(&mut m);
    let e = step0(&mut m, tap("cancel"));
    assert!(e.contains(&MatchEvent::LineReeledIn { seat: 0 }), "{e:?}");
    assert_eq!(m.fishing.lines[0].stage, Stage::Idle);
    assert!(!m.seats[0].menu_open);
}

#[test]
fn q_to_driving_drops_the_fish() {
    let mut m = fresh(1);
    let f = hooked(&mut m, 0);
    let e = step0(&mut m, tap("mode"));
    assert_dropped(&m, f, &e);
    assert_eq!(m.boats[0].as_ref().unwrap().mode, Mode::Driving);
}

#[test]
fn a_moving_boat_drags_the_bobber_and_too_fast_reels_the_line_in() {
    let mut m = fresh(1);
    cast_short(&mut m);
    until(&mut m, 200, |e| matches!(e, MatchEvent::BobberLanded { .. }));
    let len = m.fishing.lines[0].length_mm as i64;
    // drifting backward at 2 m/s pulls the bobber along, the line taut
    for _ in 0..20 {
        m.boats[0].as_mut().unwrap().set_vel([-2000, 0]);
        step0(&mut m, SeatInput::default());
    }
    let l = &m.fishing.lines[0];
    assert!((l.bobber.dist(m.rod(0)) - len).abs() <= 2, "{} vs {len}", l.bobber.dist(m.rod(0)));
    assert!(l.bobber.x < BOW_X + 4000 - 500, "{:?}", l.bobber);
    // at 4 m/s the line comes in
    m.boats[0].as_mut().unwrap().set_vel([-4000, 0]);
    let e = step0(&mut m, SeatInput::default());
    assert!(e.contains(&MatchEvent::LineReeledIn { seat: 0 }), "{e:?}");
}

#[test]
fn too_fast_with_a_fish_on_breaks_the_line() {
    let mut m = fresh(1);
    let f = hooked(&mut m, 0);
    m.boats[0].as_mut().unwrap().set_vel([-6000, 0]);
    let e = step0(&mut m, space_held());
    assert!(e.contains(&MatchEvent::LineBroke {
        seat: 0,
        fish: f as u32
    }), "{e:?}");
}

#[test]
fn two_seats_cannot_hook_the_same_fish() {
    let mut m = fresh(2);
    // seat 1 floats beside seat 0; both lures land by the one fish
    {
        let b = m.boats[1].as_mut().unwrap();
        b.set_at(P::new(AT.x, AT.y + 1000));
        b.prev_um = b.pos_um;
    }
    let f = put_fish(&mut m, 0, P::new(BOW_X + 4800, AT.y + 500), 1000, 300);
    let both = vec![tap("cast"), tap("cast")];
    m.step(&both);
    let mut hooks = Vec::new();
    let mut lured_by = Vec::new();
    for _ in 0..1500 {
        // each seat stops its lure a metre down and strikes at its bite
        let mut inputs = vec![SeatInput::default(); 2];
        for (seat, slot) in inputs.iter_mut().enumerate() {
            let l = &m.fishing.lines[seat];
            let v = m.fishing_view(seat);
            if (l.stage == Stage::Sinking && l.lure_depth_mm >= 1000) || v.biting {
                *slot = tap("cast");
            }
        }
        let e = m.step(&inputs).to_vec();
        hooks.extend(e.into_iter().filter(|e| matches!(e, MatchEvent::Hooked { .. })));
        if let FishState::Lured { seat } = m.fishing.fish[f].state {
            lured_by.push(seat);
        }
        if !hooks.is_empty() {
            break;
        }
    }
    assert_eq!(hooks.len(), 1, "{hooks:?}");
    lured_by.dedup();
    assert_eq!(lured_by.len(), 1, "the fish went to one lure only: {lured_by:?}");
    let other = 1 - lured_by[0];
    assert!(m.fishing.lines[other].interest.is_none_or(|i| i.fish != f));
}

#[test]
fn a_fish_comes_to_a_lure_at_its_depth_from_further_away() {
    let t = sample().fishing;
    // 6 m at the same depth, 30% of it 3 m or more apart, in proportion between
    assert_eq!(attraction_mm(&t, 0), 6000);
    assert_eq!(attraction_mm(&t, 1500), 3900);
    assert_eq!(attraction_mm(&t, 3000), 1800);
    assert_eq!(attraction_mm(&t, 9000), 1800);
}

#[test]
fn fish_show_close_to_the_boat_and_closer_still_when_deep() {
    let t = sample().fish;
    assert_eq!(view::sight_mm(&t, 500), 40_000);
    assert_eq!(view::sight_mm(&t, 12_000), 25_000);
    assert_eq!(view::sight_mm(&t, 6500), 32_500);
    let mut m = fresh(1);
    let shallow = put_fish(&mut m, 0, P::new(AT.x + 35_000, AT.y), 1000, 200);
    let deep = put_fish(&mut m, 3, P::new(AT.x - 30_000, AT.y), 12_000, 1000);
    let near_deep = put_fish(&mut m, 3, P::new(AT.x, AT.y + 24_000), 12_000, 1000);
    let seen: Vec<u32> = m.fishing_view(0).fish.iter().map(|f| f.id).collect();
    assert!(seen.contains(&(shallow as u32)));
    assert!(!seen.contains(&(deep as u32)));
    assert!(seen.contains(&(near_deep as u32)));
}

#[test]
fn stocking_follows_the_counts_schools_and_lengths() {
    let map = crate::map::generate(Setting::MichiganLake, 0);
    let real = Roster::parse(include_str!("../../../../../data/fish.json")).unwrap();
    let t = sample();
    let (fish, schools) = lake::stock(&map, &real, &t.fish, 5).unwrap();
    for (k, kind) in real.0.iter().enumerate() {
        let n = fish.iter().filter(|f| f.kind == k).count() as u32;
        // within 20% either way, rounded (a lone schooling fish makes two)
        let (lo, hi) = (kind.count * 8 / 10, (kind.count * 12 + 9) / 10);
        assert!(n >= lo && n <= hi.max(2), "{}: {n} of {}", kind.id, kind.count);
    }
    for (s, _) in schools.iter().enumerate() {
        let members: Vec<&LakeFish> = fish.iter().filter(|f| f.school == s).collect();
        if members[0].tier.schools() {
            assert!((2..=4).contains(&members.len()), "a school of {}", members.len());
        } else {
            assert_eq!(members.len(), 1);
        }
    }
    for f in &fish {
        let base = real.0[f.kind].base_length_mm;
        assert!(f.length_mm >= base && f.length_mm <= base * 3 / 2, "{f:?}");
    }
    // the same seed stocks the same lake; another does not
    assert_eq!(lake::stock(&map, &real, &t.fish, 5).unwrap().0, fish);
    assert_ne!(lake::stock(&map, &real, &t.fish, 6).unwrap().0, fish);
}

#[test]
fn schools_roam_together_and_slowly() {
    let mut m = fresh(1);
    m.fishing.fish_tuning.roam_radius_mm = 10_000;
    let map = crate::map::generate(Setting::MichiganLake, 0);
    m.map = map;
    m.stock(&roster()).unwrap();
    let start: Vec<P> = m.fishing.fish.iter().map(|f| f.at).collect();
    for _ in 0..500 {
        step0(&mut m, SeatInput::default());
    }
    for (f, was) in m.fishing.fish.iter().zip(&start) {
        // 10 s at 0.8 m/s at most, and never far from home
        assert!(f.at.dist(*was) <= 8_100, "{} mm", f.at.dist(*was));
        let s = &m.fishing.schools[f.school];
        assert!(s.at.dist(s.home) <= 10_000);
        assert!(m.map.wet(f.at));
    }
    assert!(m.fishing.fish.iter().zip(&start).any(|(f, w)| f.at != *w), "nothing moved");
}

/// A whole little fishing match driven by a scripted angler: cast toward
/// the nearest fish in sight, stop the lure at its depth, strike at the
/// bite, and reel by keeping the zone under the marker.
fn scripted(seed: u64) -> (Vec<MatchEvent>, Vec<Score>) {
    let t = sample();
    let map = crate::map::generate(Setting::MichiganLake, 3);
    let mut s = Setup::new(&t).with(0, Occupant::Human, "A");
    s.seed = seed;
    let mut m = Match::new(map, &t, s).unwrap();
    m.stock(&roster()).unwrap();
    m.boats[0].as_mut().unwrap().mode = Mode::Fishing;
    // float the boat 8 m west of the first fish, facing it
    let f = m.fishing.fish[0].at;
    let b = m.boats[0].as_mut().unwrap();
    b.set_at(P::new(f.x - 8000, f.y));
    b.prev_um = b.pos_um;
    b.heading = 0;
    let want = m.fishing.fish[0].depth_mm;
    let mut all = Vec::new();
    for _ in 0..3000 {
        let v = m.fishing_view(0);
        let l = &m.fishing.lines[0];
        let i = match (v.stage, &v.reel) {
            (Stage::Idle, _) => tap("cast"),
            (Stage::Sinking, _) if l.lure_depth_mm >= want => tap("cast"),
            (Stage::Set, _) if v.biting => tap("cast"),
            (Stage::Reeling, Some(r)) if r.zone + r.zone_h / 2 < r.fish => space_held(),
            _ => SeatInput::default(),
        };
        all.extend(step0(&mut m, i));
    }
    (all, m.scores.clone())
}

#[test]
fn the_same_inputs_give_the_same_catches() {
    let (a, sa) = scripted(41);
    let (b, sb) = scripted(41);
    assert_eq!(a, b);
    assert_eq!(sa, sb);
    assert!(a.iter().any(|e| matches!(e, MatchEvent::Hooked { .. })), "the script never hooked");
}

/// A steady player, who holds Space while the zone's middle (led a little
/// by its speed) is under the marker, lands nearly every fish of every tier
/// on the shipped tuning, in at most three times the tier's perfect time.
/// Measured on 2026-10-18: every one of 200 landed per tier, in 3.0, 5.3,
/// 12.8 and 47 s on average (perfect 3, 5, 10, 20 s); a person reacting
/// later will do worse (SECOND-ORDER R3.12).
#[test]
fn a_steady_player_lands_every_tier_in_fair_time() {
    let t = crate::tuning::parse(include_str!("../../../../../data/tuning.json")).unwrap();
    let r = &t.fishing.reel;
    for tier in [Tier::Common, Tier::Uncommon, Tier::Rare, Tier::Legendary] {
        let (mut landed, mut ticks_sum) = (0u32, 0u32);
        for seed in 0..200 {
            let mut rng = Rng::new(seed);
            let mut reel = Reel::new(tier, r, false);
            let mut tick = 0;
            loop {
                tick += 1;
                let mid = reel.zone + reel.zone_h / 2 + reel.zone_v / 5;
                match reel.step(r, &mut rng, tick, mid < reel.marker, 100) {
                    Outcome::Landed => {
                        landed += 1;
                        ticks_sum += tick;
                        break;
                    }
                    Outcome::Broke => break,
                    Outcome::Fighting => {}
                }
            }
        }
        let mean_ms = ticks_sum / landed.max(1) * 20;
        let perfect = r.perfect_ms.get(tier) as u32;
        assert!(landed >= 180, "{tier:?}: landed {landed} of 200");
        assert!(mean_ms <= 3 * perfect, "{tier:?}: {mean_ms} ms on average");
    }
}
