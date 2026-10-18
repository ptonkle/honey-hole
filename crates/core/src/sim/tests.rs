//! The match core: ticks, the timer, the spawn, boat handling, land and
//! boat collisions, wrecks, the respawn and ghosting. Expected values are
//! worked by hand from the sample tuning (`tuning::tests::sample`): top
//! speed 16000 mm/s, so W adds 16000 × 20 / 5000 = 64 mm/s a tick, coasting
//! takes 16000 × 20 / 6000 = 53 mm/s a tick, full turning is
//! 100° × 65536 / 360 / 50 = 364 units a tick; a ram needs 80% of 16000 =
//! 12800 mm/s and grounding 50% = 8000 mm/s.

use super::*;
use crate::map::{generate, Setting, Spawn};
use crate::tuning::{self, Tuning};

const CELL: i32 = 4000;

fn sample() -> Tuning {
    tuning::tests::sample()
}

/// A square lake `n` cells a side, land only round the edge and in the
/// columns from `wall` east (if any). The spawn is at `at`, facing east.
fn lake(n: usize, wall: Option<usize>, at: P) -> Map {
    let water: Vec<bool> = (0..n * n)
        .map(|i| {
            let (x, y) = (i % n, i / n);
            let edge = x == 0 || y == 0 || x == n - 1 || y == n - 1;
            !edge && wall.is_none_or(|w| x < w)
        })
        .collect();
    let spawn = Spawn {
        at,
        heading: 0,
        dock: P::new(0, at.y),
    };
    Map::from_water(Setting::MichiganLake, 0, n, CELL, &water, spawn)
}

fn setup(t: &Tuning, humans: usize) -> Setup {
    let mut s = Setup::new(t);
    for k in 0..humans {
        s = s.with(k, Occupant::Human, &format!("P{k}"));
    }
    s
}

/// One seat holding these actions (by name), every tick.
fn hold(names: &[&str]) -> SeatInput {
    let mut i = SeatInput::default();
    for n in names {
        i.held.set(n, true);
    }
    i
}

fn press(name: &str) -> SeatInput {
    let mut i = SeatInput::default();
    i.pressed.set(name, true);
    i
}

fn boat(m: &Match, seat: usize) -> &Boat {
    m.boat(seat).unwrap()
}

fn boat_mut(m: &mut Match, seat: usize) -> &mut Boat {
    m.boats[seat].as_mut().unwrap()
}

/// Every event over `ticks` steps with the same inputs each tick.
fn run(m: &mut Match, inputs: &[SeatInput], ticks: u32) -> Vec<MatchEvent> {
    let mut all = Vec::new();
    for _ in 0..ticks {
        all.extend_from_slice(m.step(inputs));
    }
    all
}

#[test]
fn two_matches_with_the_same_inputs_stay_identical() {
    let t = sample();
    let map = generate(Setting::MichiganLake, 0);
    let s = Setup {
        seed: 99,
        ..setup(&t, 4)
    };
    let mut a = Match::new(map.clone(), &t, s.clone()).unwrap();
    let mut b = Match::new(map, &t, s).unwrap();
    // scripted inputs from a seeded stream: throttle, turns, mode switches
    let mut rng = Rng::new(7);
    let (mut log_a, mut log_b) = (Vec::new(), Vec::new());
    let start = a.spawn_slot(0).0;
    let mut farthest = 0;
    for _ in 0..3000 {
        let inputs: Vec<SeatInput> = (0..4)
            .map(|_| {
                let mut i = SeatInput::default();
                i.held.forward = rng.chance(800);
                i.held.back = rng.chance(100);
                i.held.left = rng.chance(300);
                i.held.right = rng.chance(300);
                i.pressed.mode = rng.chance(5);
                i
            })
            .collect();
        log_a.extend_from_slice(a.step(&inputs));
        log_b.extend_from_slice(b.step(&inputs));
        farthest = farthest.max(boat(&a, 0).at().dist(start));
    }
    assert_eq!(a.boats, b.boats);
    assert_eq!(log_a, log_b);
    assert_eq!(a.tick, 3000);
    // and it was a real run: the boats moved off the spawn
    assert!(farthest > 20_000, "{farthest}");
}

#[test]
fn w_reaches_top_speed_in_five_seconds() {
    let t = sample();
    let mut m = Match::new(lake(40, None, P::new(20_000, 80_000)), &t, setup(&t, 1)).unwrap();
    let w = [hold(&["forward"])];
    run(&mut m, &w, 125);
    // halfway through the 5 s, half the speed: 125 × 64
    assert_eq!(boat(&m, 0).forward_speed(), 8000);
    run(&mut m, &w, 124);
    assert_eq!(boat(&m, 0).forward_speed(), 15_936);
    run(&mut m, &w, 1);
    assert_eq!(boat(&m, 0).forward_speed(), 16_000, "top speed at 250 ticks, 5 s");
    run(&mut m, &w, 20);
    assert_eq!(boat(&m, 0).forward_speed(), 16_000, "and no faster");
}

#[test]
fn crossing_half_a_kilometre_from_rest_takes_about_34_seconds() {
    // The accelerating 5 s cover 64 × 0.02 mm × (0 + 1 + ... + 249) =
    // 39 840 mm; the other 460 160 mm at 320 mm a tick take 1438 ticks:
    // 1688 ticks, 33.8 s. (The map's 440 to 640 m spans take 28 to 40 s.)
    let t = sample();
    let mut m = Match::new(lake(140, None, P::new(20_000, 280_000)), &t, setup(&t, 1)).unwrap();
    let w = [hold(&["forward"])];
    let mut ticks = 0;
    while boat(&m, 0).at().x < 520_000 {
        m.step(&w);
        ticks += 1;
    }
    assert!((1686..=1690).contains(&ticks), "{ticks} ticks");
}

#[test]
fn turning_is_slow_at_rest_and_full_from_forty_percent_of_top_speed() {
    let t = sample();
    let mut m = Match::new(lake(40, None, P::new(80_000, 80_000)), &t, setup(&t, 1)).unwrap();
    m.step(&[hold(&["left"])]);
    // at rest: 30% of 364
    assert_eq!(boat(&m, 0).heading, 109);
    let b = boat_mut(&mut m, 0);
    b.heading = 0;
    b.set_vel([6400, 0]); // 40% of top speed
    m.step(&[hold(&["right", "forward"])]);
    assert_eq!(boat(&m, 0).heading, TURN - 364);
}

const TURN: i32 = crate::map::trig::TURN;

/// A boat heading east at `speed` whose bow is 40 mm short of a wall of
/// land at x = 120 m.
fn at_the_wall(speed: i32, t: &Tuning) -> Match {
    let wall_x = 30 * CELL; // cells from 30 east are land
    let mut m = Match::new(lake(40, Some(30), P::new(60_000, 80_000)), t, setup(t, 1)).unwrap();
    let hl = t.boat.half_length_mm + t.boat.half_beam_mm;
    let b = boat_mut(&mut m, 0);
    b.set_at(P::new(wall_x - hl - 40, 80_000));
    b.set_vel([speed, 0]);
    m
}

#[test]
fn land_below_half_top_speed_bounces_the_boat_back() {
    let t = sample();
    let mut m = at_the_wall(7999, &t);
    let before = boat(&m, 0).at();
    let ev = m.step(&[SeatInput::default()]).to_vec();
    assert_eq!(ev, [MatchEvent::Bounced { seat: 0, speed: 7999 }]);
    let b = boat(&m, 0);
    assert!(!b.wrecked());
    assert_eq!(b.at(), before, "it stays where it was, out of the land");
    // back at 30% of 7999 = 2399, less this tick's coasting (53)
    assert_eq!(b.vel(), [-2346, 0]);
}

#[test]
fn land_at_half_top_speed_wrecks_the_boat() {
    let t = sample();
    let mut m = at_the_wall(8000, &t);
    let ev = m.step(&[SeatInput::default()]).to_vec();
    assert_eq!(
        ev,
        [MatchEvent::Wrecked {
            seat: 0,
            cause: WreckCause::Grounded
        }]
    );
    assert!(boat(&m, 0).wrecked());
}

#[test]
fn a_glancing_hit_slides_along_the_shore_losing_speed() {
    let t = sample();
    let mut m = at_the_wall(4000, &t);
    // the hull points east at the wall, sliding north as it hits: only the
    // bow touches, so the land pushes straight west
    boat_mut(&mut m, 0).set_vel([4000, 6000]);
    m.step(&[SeatInput::default()]);
    let b = boat(&m, 0);
    assert!(!b.wrecked(), "4 m/s toward the land is below 8 m/s");
    // back at 30% of 4000 (1200), and 70% of 6000 along the shore (4200);
    // then a tick of coasting (53) along the bow and slip (400) across it
    assert_eq!(b.vel(), [-1200 + 53, 4200 - 400]);
}

/// Seat 0 heading east at `speed`; seat 1 still, broadside (heading north),
/// its middle line 1800 mm past seat 0's bow line: they touch after one
/// tick's move (256 mm or so), straight along the east axis.
fn ram(speed: i32, t: &Tuning) -> Match {
    let mut m = Match::new(lake(40, None, P::new(60_000, 80_000)), t, setup(t, 2)).unwrap();
    let a = boat_mut(&mut m, 0);
    a.set_at(P::new(60_000, 80_000));
    a.heading = 0;
    a.set_vel([speed, 0]);
    let b = boat_mut(&mut m, 1);
    b.set_at(P::new(60_000 + 1000 + 1800, 80_000));
    b.heading = TURN / 4;
    m
}

#[test]
fn a_ram_at_eighty_percent_wrecks_the_other_boat_and_stalls_the_rammer() {
    let t = sample();
    let mut m = ram(12_800, &t);
    let ev = m.step(&[SeatInput::default(); 2]).to_vec();
    assert_eq!(
        ev,
        [
            MatchEvent::Wrecked {
                seat: 1,
                cause: WreckCause::Rammed { by: 0 }
            },
            MatchEvent::Stalled { seat: 0 }
        ]
    );
    assert!(boat(&m, 1).wrecked());
    // stalled for 3 s: W does nothing until tick 1 + 150
    assert_eq!(boat(&m, 0).stall_until, 151);
    run(&mut m, &[hold(&["forward"]), SeatInput::default()], 149);
    assert_eq!(boat(&m, 0).forward_speed(), 0, "the motor is stalled");
    run(&mut m, &[hold(&["forward"]), SeatInput::default()], 2);
    assert!(boat(&m, 0).forward_speed() > 0, "and starts again");
}

#[test]
fn a_ram_at_seventy_nine_percent_only_bumps() {
    let t = sample();
    let mut m = ram(12_640, &t);
    let ev = m.step(&[SeatInput::default(); 2]).to_vec();
    assert!(
        matches!(ev[..], [MatchEvent::Bumped { a: 0, b: 1, .. }]),
        "{ev:?}"
    );
    assert!(!boat(&m, 1).wrecked());
    assert!(boat(&m, 1).vel()[0] > 0, "the other boat is pushed east");
    assert!(boat(&m, 0).vel()[0] < 12_640, "the rammer slows");
    assert_eq!(boat(&m, 0).stall_until, 0);
}

#[test]
fn a_wrecked_boat_respawns_at_the_spawn_after_two_seconds_ghosted() {
    let t = sample();
    let mut m = ram(13_000, &t);
    m.step(&[SeatInput::default(); 2]); // tick 1: seat 1 wrecked
    let none = [SeatInput::default(); 2];
    run(&mut m, &none, 99);
    assert!(boat(&m, 1).wrecked(), "still out at tick 100");
    let ev = m.step(&none).to_vec(); // tick 101 = 1 + 100
    assert!(ev.contains(&MatchEvent::Respawned { seat: 1 }), "{ev:?}");
    let b = boat(&m, 1);
    assert_eq!(b.at(), m.spawn_slot(1).0);
    assert_eq!(b.heading, m.map.spawn.heading);
    assert!(b.ghosted(m.tick));
    assert_eq!(b.ghost_until, 101 + 150, "ghosted for 3 s");

    // a ghost passes through a boat at full speed: no ram, no bump
    let at = boat(&m, 1).at();
    let a = boat_mut(&mut m, 0);
    a.set_at(P::new(at.x - 2800, at.y));
    a.heading = 0;
    a.set_vel([16_000, 0]);
    a.stall_until = 0;
    let ev = run(&mut m, &none, 20);
    assert!(ev
        .iter()
        .all(|e| !matches!(e, MatchEvent::Wrecked { .. } | MatchEvent::Bumped { .. })));
    // the ghosting ends on time
    let left = 251 - m.tick;
    let ev = run(&mut m, &none, left);
    assert!(ev.contains(&MatchEvent::GhostEnded { seat: 1 }));
    assert!(!boat(&m, 1).ghosted(m.tick));
}

#[test]
fn the_timer_ends_the_match_at_zero() {
    let t = sample();
    let s = Setup {
        length_ms: 1000,
        ..setup(&t, 1)
    };
    let mut m = Match::new(lake(40, None, P::new(60_000, 80_000)), &t, s).unwrap();
    assert_eq!(m.remaining_ms(), 1000);
    let ev = run(&mut m, &[SeatInput::default()], 49);
    assert!(!m.ended && ev.is_empty());
    assert_eq!(m.remaining_ms(), 20);
    assert_eq!(m.step(&[SeatInput::default()]), [MatchEvent::MatchEnded]);
    assert!(m.ended);
    assert_eq!(m.remaining_ms(), 0);
    // nothing moves after the end
    assert!(m.step(&[hold(&["forward"])]).is_empty());
    assert_eq!((m.tick, boat(&m, 0).vel()), (50, [0, 0]));
}

#[test]
fn q_switches_to_fishing_keeping_momentum_and_a_d_swivel_the_cast() {
    let t = sample();
    let mut m = Match::new(lake(60, None, P::new(30_000, 120_000)), &t, setup(&t, 1)).unwrap();
    run(&mut m, &[hold(&["forward"])], 100);
    assert_eq!(boat(&m, 0).forward_speed(), 6400);
    let mut q = press("mode");
    q.held.forward = true;
    let ev = m.step(&[q]).to_vec();
    assert_eq!(
        ev,
        [MatchEvent::ModeChanged {
            seat: 0,
            mode: Mode::Fishing
        }]
    );
    // the same speed, less one tick's coasting: W does nothing in fishing mode
    assert_eq!(boat(&m, 0).forward_speed(), 6400 - 53);
    let heading = boat(&m, 0).heading;
    m.step(&[hold(&["left", "forward"])]);
    let b = boat(&m, 0);
    assert_eq!(b.heading, heading, "A swivels the cast, not the boat");
    // 120° a second: 120 × 65536 / 360 / 50 = 436 a tick
    assert_eq!(b.cast_yaw, 436);
    assert_eq!(b.forward_speed(), 6400 - 2 * 53);
    // and it coasts to rest: 6294 / 53 is under 119 more ticks
    run(&mut m, &[SeatInput::default()], 119);
    assert_eq!(boat(&m, 0).speed_mm_per_s(), 0);
    // the cast swivels no further than 135° from the bow (24576)
    run(&mut m, &[hold(&["left"])], 100);
    assert_eq!(boat(&m, 0).cast_yaw, 24_576);
}

#[test]
fn s_brakes_then_reverses_slowly() {
    let t = sample();
    let mut m = Match::new(lake(60, None, P::new(120_000, 120_000)), &t, setup(&t, 1)).unwrap();
    boat_mut(&mut m, 0).set_vel([1600, 0]);
    // braking: 16000 × 20 / 2000 = 160 a tick, 10 ticks to rest
    run(&mut m, &[hold(&["back"])], 10);
    assert_eq!(boat(&m, 0).forward_speed(), 0);
    // reverse: 4000 × 20 / 2000 = 40 a tick, to 4000 at most
    run(&mut m, &[hold(&["back"])], 50);
    assert_eq!(boat(&m, 0).forward_speed(), -2000);
    run(&mut m, &[hold(&["back"])], 200);
    assert_eq!(boat(&m, 0).forward_speed(), -4000);
}

#[test]
fn the_boats_start_side_by_side_at_the_spawn_facing_down_the_lake() {
    let t = sample();
    let map = generate(Setting::MichiganLake, 0);
    let spawn = map.spawn;
    let m = Match::new(map, &t, setup(&t, 4)).unwrap();
    let at: Vec<P> = (0..4).map(|s| boat(&m, s).at()).collect();
    for s in 0..4 {
        assert_eq!(boat(&m, s).heading, spawn.heading);
    }
    // neighbours 5 m apart, the outer pair 15 m apart, around the spawn
    for k in 0..3 {
        assert!((at[k].dist(at[k + 1]) - 5000).abs() <= 2, "{at:?}");
    }
    assert!((at[0].dist(at[3]) - 15_000).abs() <= 3);
    let mid = P::new((at[1].x + at[2].x) / 2, (at[1].y + at[2].y) / 2);
    assert!(mid.dist(spawn.at) <= 2);
    // the first seat is on the left of the heading
    let (c, s) = boat::dir(spawn.heading);
    let left =
        (-(s as i64) * (at[0].x - spawn.at.x) as i64 + c as i64 * (at[0].y - spawn.at.y) as i64) / 65536;
    assert!((left - 7500).abs() <= 3, "{left}");
    // empty seats have no boat
    let m = Match::new(generate(Setting::MichiganLake, 0), &t, setup(&t, 1)).unwrap();
    assert_eq!(m.boats.iter().flatten().count(), 1);
}

#[test]
fn esc_opens_the_menu_and_an_offline_match_waits_under_it() {
    let t = sample();
    let mut m = Match::new(lake(40, None, P::new(60_000, 80_000)), &t, setup(&t, 1)).unwrap();
    assert_eq!(m.step(&[press("cancel")]), [MatchEvent::MenuOpened { seat: 0 }]);
    assert!(m.paused());
    let tick = m.tick;
    run(&mut m, &[hold(&["forward"])], 10);
    assert_eq!(m.tick, tick, "paused offline");
    assert_eq!(boat(&m, 0).vel(), [0, 0]);
    assert_eq!(m.step(&[press("cancel")]), [MatchEvent::MenuClosed { seat: 0 }]);
    assert_eq!(m.tick, tick + 1);
    // online, the match runs on under the menu
    let s = Setup {
        offline: false,
        ..setup(&t, 1)
    };
    let mut m = Match::new(lake(40, None, P::new(60_000, 80_000)), &t, s).unwrap();
    m.step(&[press("menu")]);
    assert!(m.seats[0].menu_open && !m.paused());
    assert_eq!(m.tick, 1);
}

#[test]
fn a_captured_boat_coasts_and_ignores_the_keys() {
    let t = sample();
    let mut m = Match::new(lake(60, None, P::new(30_000, 120_000)), &t, setup(&t, 1)).unwrap();
    let b = boat_mut(&mut m, 0);
    b.set_vel([5000, 0]);
    b.captured = true;
    let mut i = hold(&["forward", "left"]);
    i.pressed.mode = true;
    m.step(&[i]);
    let b = boat(&m, 0);
    assert_eq!(
        (b.forward_speed(), b.heading, b.mode),
        (5000 - 53, 0, Mode::Driving)
    );
}

#[test]
fn nitrous_raises_top_speed_but_not_the_ram_speed() {
    let t = sample();
    let mut m = Match::new(lake(140, None, P::new(20_000, 280_000)), &t, setup(&t, 1)).unwrap();
    boat_mut(&mut m, 0).nitrous = true;
    // 150% of top speed, at 200% of the acceleration: 24000 / 128 = 187.5 ticks
    run(&mut m, &[hold(&["forward"])], 188);
    assert_eq!(boat(&m, 0).forward_speed(), 24_000);
    assert_eq!(m.rules.ram_wreck, 12_800);
}

#[test]
fn every_rule_follows_the_tuning() {
    // the same checks with other numbers: none of them is a literal
    let mut v = tuning::tests::sample_json();
    v["boat"]["top_speed_mm_per_s"] = 10_000.into();
    v["boat"]["accel_ms"] = 2000.into();
    v["boat"]["coast_ms"] = 1000.into();
    v["wreck"]["land_wreck_pct"] = 30.into();
    v["wreck"]["ram_wreck_pct"] = 60.into();
    v["wreck"]["respawn_ms"] = 1000.into();
    v["wreck"]["ghost_ms"] = 500.into();
    v["wreck"]["stall_ms"] = 400.into();
    v["wreck"]["bounce_pct"] = 50.into();
    v["match"]["length_ms"] = 240_000.into();
    v["match"]["spawn_spread_mm"] = 8000.into();
    let t = tuning::parse(&v.to_string()).unwrap();
    let r = Rules::new(&t, &Setup::new(&t));
    // 10000 × 20 / 2000 = 100 a tick; coasting 200 a tick
    assert_eq!((r.top, r.accel, r.coast), (10_000, 100, 200));
    assert_eq!((r.land_wreck, r.ram_wreck), (3000, 6000));
    assert_eq!((r.respawn_ticks, r.ghost_ticks, r.stall_ticks), (50, 25, 20));
    assert_eq!(r.length_ticks, 12_000);

    let mut m = Match::new(lake(40, None, P::new(60_000, 80_000)), &t, setup(&t, 1)).unwrap();
    run(&mut m, &[hold(&["forward"])], 100);
    assert_eq!(boat(&m, 0).forward_speed(), 10_000);
    // grounding at 30% of 10000 wrecks, below bounces back at 50%
    let mut m = at_the_wall(3000, &t);
    m.step(&[SeatInput::default()]);
    assert!(boat(&m, 0).wrecked());
    let mut m = at_the_wall(2999, &t);
    m.step(&[SeatInput::default()]);
    assert_eq!(boat(&m, 0).vel(), [-1499 + 200, 0]);
    // a ram at 60%, a stall of 20 ticks, a respawn after 50 and a ghost of 25
    let mut m = ram(6000, &t);
    let ev = m.step(&[SeatInput::default(); 2]).to_vec();
    assert!(ev.contains(&MatchEvent::Stalled { seat: 0 }));
    assert_eq!(boat(&m, 0).stall_until, 21);
    run(&mut m, &[SeatInput::default(); 2], 50);
    assert!(!boat(&m, 1).wrecked());
    assert_eq!(boat(&m, 1).ghost_until, 51 + 25);
    // 8 m between seats
    let m = Match::new(lake(40, None, P::new(60_000, 80_000)), &t, setup(&t, 2)).unwrap();
    assert_eq!(boat(&m, 0).at().dist(boat(&m, 1).at()), 8000);
    // a setup's own length beats the tuning's
    let s = Setup {
        length_ms: 2000,
        ..setup(&t, 1)
    };
    let m = Match::new(lake(40, None, P::new(60_000, 80_000)), &t, s).unwrap();
    assert_eq!(m.rules.length_ticks, 100);
}

#[test]
fn the_view_draws_each_boat_between_its_last_two_ticks() {
    let t = sample();
    let mut m = Match::new(lake(40, None, P::new(60_000, 80_000)), &t, setup(&t, 1)).unwrap();
    boat_mut(&mut m, 0).set_vel([10_000, 0]);
    m.step(&[SeatInput::default()]);
    // moved 200 mm this tick; halfway is 100 mm on
    let half = m.view(crate::gfx::fx::ONE / 2);
    assert_eq!(half.boats[0].x_mm, 60_000 + 100);
    assert_eq!(m.view(crate::gfx::fx::ONE).boats[0].x_mm, 60_200);
    assert_eq!(half.boats[0].colour, "blue");
    assert_eq!(half.remaining_ms, 480_000 - 20);
}
