//! The AI and the items: the buoy, each item's moment and the hand on the
//! minigames. Expected values are worked by hand from the shipped
//! `ai.items` tuning (`data/tuning.json`, read by `tuning::tests::ai_sample`)
//! and the sample match tuning: top speed 16 m/s, so a buoy trip is
//! 1.3 x d / 12.8 m/s + 1.5 s; a buoy is worth 25 s with no item and 6 s
//! with one; Nitrous from 200 m (120 m on a hunt), the Teleporter from
//! 180 m with 14 s of the rival's reel left.

use super::senses::sense;
use super::*;
use crate::map::{generate, Map, Setting, Spawn};
use crate::minigame::{Input, Kind, Minigame, State};
use crate::roster::Roster;
use crate::sim::fishing::lake::{FishState, LakeFish, School};
use crate::sim::items::Item;
use crate::sim::{Catch, MatchEvent, Occupant, Setup};
use crate::tuning::{self, Tuning};

const FISH: &str = include_str!("../../../../data/fish.json");

fn sample() -> Tuning {
    tuning::tests::sample()
}

/// A square lake 60 cells (240 m) a side, land round the edge, the spawn
/// at (60 m, 120 m) facing east.
fn lake() -> Map {
    let n = 60;
    let water: Vec<bool> = (0..n * n)
        .map(|i| {
            let (x, y) = (i % n, i / n);
            !(x == 0 || y == 0 || x == n - 1 || y == n - 1)
        })
        .collect();
    let spawn = Spawn {
        at: P::new(60_000, 120_000),
        heading: 0,
        dock: P::new(0, 120_000),
    };
    Map::from_water(Setting::MichiganLake, 0, n, 4000, &water, spawn)
}

fn one_ai(d: Difficulty, extra: &[Occupant]) -> (Match, Crew) {
    let t = sample();
    let mut s = Setup::new(&t);
    s.seed = 7;
    s = s.with(0, Occupant::Ai(d), "P0");
    for (k, o) in extra.iter().enumerate() {
        s = s.with(k + 1, *o, &format!("P{}", k + 1));
    }
    let mut m = Match::new(lake(), &t, s).unwrap();
    m.fishing.roster = Roster::parse(FISH).unwrap();
    // no buoy comes on the clock during a test
    m.items.next_buoy = u32::MAX / 2;
    let crew = Crew::new(&m);
    (m, crew)
}

/// Puts one fish of roster kind `id` in the lake, still.
fn put_fish(m: &mut Match, id: &str, at: P, depth_mm: i32) -> u32 {
    let kind = m.fishing.roster.0.iter().position(|k| k.id == id).unwrap();
    let k = &m.fishing.roster.0[kind];
    let (tier, length_mm) = (k.tier, k.base_length_mm);
    m.fishing.schools.push(School {
        at,
        home: at,
        target: at,
        rest_until: 0,
    });
    let n = m.fishing.fish.len() as u32;
    m.fishing.fish.push(LakeFish {
        id: n,
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
    n
}

/// Steps with the crew for up to `ticks`, until `stop`; the events seen.
fn run(
    m: &mut Match,
    crew: &mut Crew,
    ticks: u32,
    stop: impl Fn(&MatchEvent) -> bool,
) -> Vec<(u32, MatchEvent)> {
    let mut seen = Vec::new();
    for _ in 0..ticks {
        let i = crew.inputs(m);
        let tick = m.step(&i).to_vec();
        let done = tick.iter().any(&stop);
        seen.extend(tick.into_iter().map(|e| (m.tick, e)));
        if done || m.ended {
            break;
        }
    }
    seen
}

fn has(seen: &[(u32, MatchEvent)], f: impl Fn(&MatchEvent) -> bool) -> bool {
    seen.iter().any(|(_, e)| f(e))
}

/// Plays a minigame with a difficulty's hand alone; the play's ms.
fn hand_plays(kind: Kind, seed: u64, d: Difficulty) -> u32 {
    let t = sample();
    let skill = t.ai.items.hand.get(d).clone();
    let mut mg = Minigame::new(kind, seed, &t.minigames);
    let mut hand = hand::Hand::new(seed, 0);
    hand.reset(0, &skill);
    for tick in 0..50 * 120 {
        let i = hand.play(&mg.sight(), tick, &skill);
        mg.step(&Input::from_seat(&i));
        if let State::Done { ms_taken } = mg.state() {
            return ms_taken;
        }
    }
    panic!(
        "{kind:?} with the {d:?} hand: not done in 2 minutes ({:?})",
        mg.progress()
    );
}

#[test]
fn the_hand_finishes_every_minigame_and_hard_is_quicker_than_easy() {
    for kind in Kind::ALL {
        let (mut easy, mut hard) = (0, 0);
        for seed in 1..=6 {
            easy += hand_plays(kind, seed, Difficulty::Easy);
            hard += hand_plays(kind, seed, Difficulty::Hard);
        }
        assert!(
            hard < easy,
            "{kind:?}: Hard {hard} ms against Easy {easy} ms over 6 games"
        );
    }
    // the reel by hand: out to the reel 220 m from the middle (Hard 76
    // units a tick: 3 ticks; Easy 28: 8 ticks), then 5 turns at 2.6 (Hard)
    // or 0.9 (Easy) turns a second: 97 ticks or 278. About 2.0 s and 5.7 s.
    let hard = hand_plays(Kind::PowerReel, 1, Difficulty::Hard);
    let easy = hand_plays(Kind::PowerReel, 1, Difficulty::Easy);
    assert!((1800..=2300).contains(&hard), "Hard reel {hard} ms");
    assert!((5400..=6000).contains(&easy), "Easy reel {easy} ms");
}

#[test]
fn the_ai_drives_through_a_near_buoy_but_not_a_far_one_holding_an_item() {
    // 30 m off: 1.3 x 30 / 12.8 + 1.5 = 4.5 s, under 25 s
    let (mut m, mut crew) = one_ai(Difficulty::Normal, &[]);
    let at = m.boat(0).unwrap().at();
    m.items.force_next = Some(Item::Camera);
    m.place_buoy(P::new(at.x + 30_000, at.y + 10_000));
    let seen = run(&mut m, &mut crew, 50 * 15, |e| {
        matches!(e, MatchEvent::ItemPicked { .. })
    });
    assert!(
        has(&seen, |e| matches!(e, MatchEvent::ItemPicked { seat: 0, .. })),
        "no pickup in 15 s"
    );
    assert_eq!(m.items.seats[0].held, Some(Item::Camera));
    // holding the camera, 60 m off: 1.3 x 60 / 12.8 + 1.5 = 7.6 s, over 6 s
    let at = m.boat(0).unwrap().at();
    let far = P::new((at.x + 60_000).min(220_000), at.y);
    let far = if far.dist(at) < 59_000 {
        P::new(at.x - 60_000, at.y)
    } else {
        far
    };
    m.place_buoy(far);
    run(&mut m, &mut crew, 50 * 3, |_| false);
    assert!(
        !matches!(crew.brains[0].goal, Goal::Buoy { .. }),
        "it went for a buoy 60 m off holding an item: {:?}",
        crew.brains[0].goal
    );
    // 40 m off with the item held: 5.6 s, under 6 s
    let at = m.boat(0).unwrap().at();
    let near = if at.x < 150_000 {
        P::new(at.x + 40_000, at.y)
    } else {
        P::new(at.x - 40_000, at.y)
    };
    m.place_buoy(near);
    run(&mut m, &mut crew, 2, |_| false);
    assert!(
        matches!(crew.brains[0].goal, Goal::Buoy { .. }),
        "{:?}",
        crew.brains[0].goal
    );
}

#[test]
fn a_seat_with_a_fish_on_does_not_leave_for_the_buoy() {
    let (mut m, mut crew) = one_ai(Difficulty::Normal, &[]);
    let at = m.boat(0).unwrap().at();
    put_fish(&mut m, "bluegill", P::new(at.x + 20_000, at.y), 1000);
    run(&mut m, &mut crew, 50 * 40, |e| {
        matches!(e, MatchEvent::Hooked { .. })
    });
    assert_eq!(m.fishing.lines[0].stage, Stage::Reeling);
    // a buoy 15 m off while it reels
    m.place_buoy(P::new(at.x, at.y + 15_000));
    let seen = run(&mut m, &mut crew, 50 * 30, |e| {
        matches!(e, MatchEvent::Caught { .. })
    });
    assert!(
        has(&seen, |e| matches!(e, MatchEvent::Caught { seat: 0, .. })),
        "the fish was not landed"
    );
    assert!(!has(&seen, |e| matches!(e, MatchEvent::FishDropped { .. })));
    // then it goes
    let seen = run(&mut m, &mut crew, 50 * 20, |e| {
        matches!(e, MatchEvent::ItemPicked { .. })
    });
    assert!(has(&seen, |e| matches!(
        e,
        MatchEvent::ItemPicked { seat: 0, .. }
    )));
}

/// The AI with the camera held and the offer up after a catch of `kind`,
/// `left` ticks before the end.
fn offer_after(kind: &str, left: Option<u32>) -> (Match, Crew, i32) {
    let (mut m, crew) = one_ai(Difficulty::Hard, &[]);
    if let Some(l) = left {
        m.tick = m.rules.length_ticks - l;
    }
    m.items.seats[0].held = Some(Item::Camera);
    m.scores[0].catches.push(Catch {
        kind: kind.into(),
        length_mm: 1000,
        value: 300,
    });
    m.items.seats[0].offer_until = Some(m.tick + 500);
    (m, crew, 300)
}

#[test]
fn the_camera_photographs_a_rare_catch_and_saves_itself_for_a_common_one() {
    // a bluegill: Esc, kept for later
    let (mut m, mut crew, _) = offer_after("bluegill", None);
    let seen = run(&mut m, &mut crew, 50 * 2, |e| {
        matches!(e, MatchEvent::CameraSaved { .. })
    });
    assert!(
        has(&seen, |e| matches!(e, MatchEvent::CameraSaved { seat: 0 })),
        "{seen:?}"
    );
    assert_eq!(m.items.seats[0].held, Some(Item::Camera));
    assert_eq!(m.scores[0].catches[0].value, 300);
    // a northern pike (rare): Shift, the minigame, and 300 x 1.5 = 450
    let (mut m, mut crew, _) = offer_after("northern-pike", None);
    let seen = run(&mut m, &mut crew, 50 * 40, |e| {
        matches!(e, MatchEvent::ItemUsed { .. })
    });
    assert!(
        has(&seen, |e| matches!(
            e,
            MatchEvent::ItemUsed {
                seat: 0,
                item: Item::Camera
            }
        )),
        "{seen:?}"
    );
    assert_eq!(m.scores[0].catches[0].value, 450);
    // a bluegill in the last 45 s: any catch is worth the photo
    let (mut m, mut crew, _) = offer_after("bluegill", Some(50 * 40));
    let seen = run(&mut m, &mut crew, 50 * 2, |e| {
        matches!(e, MatchEvent::MinigameStarted { .. })
    });
    assert!(has(&seen, |e| matches!(
        e,
        MatchEvent::MinigameStarted {
            seat: 0,
            item: Item::Camera
        }
    )));
}

/// The AI holding the Power Reel, a fish of `kind` 20 m ahead; the
/// events to the catch.
fn power_reel_on(kind: &str) -> (Match, Vec<(u32, MatchEvent)>) {
    let (mut m, mut crew) = one_ai(Difficulty::Hard, &[]);
    m.items.seats[0].held = Some(Item::PowerReel);
    let at = m.boat(0).unwrap().at();
    put_fish(&mut m, kind, P::new(at.x + 20_000, at.y), 1000);
    // the reel tamed (the marker holds near the top): the test is the item
    m.fishing.tame = true;
    let seen = run(&mut m, &mut crew, 50 * 90, |e| {
        matches!(e, MatchEvent::Caught { .. })
    });
    (m, seen)
}

#[test]
fn the_power_reel_goes_on_at_the_hook_of_a_rare_fish_and_not_a_common_one() {
    let (m, seen) = power_reel_on("northern-pike");
    let hooked = seen
        .iter()
        .find(|(_, e)| matches!(e, MatchEvent::Hooked { .. }))
        .map(|x| x.0);
    let used = seen
        .iter()
        .find(|(_, e)| {
            matches!(
                e,
                MatchEvent::MinigameStarted {
                    item: Item::PowerReel,
                    ..
                }
            )
        })
        .map(|x| x.0);
    let caught = seen
        .iter()
        .find(|(_, e)| matches!(e, MatchEvent::Caught { .. }))
        .map(|x| x.0);
    let (Some(h), Some(u), Some(c)) = (hooked, used, caught) else {
        panic!(
            "hook {hooked:?}, Power Reel {used:?}, catch {caught:?}: {:?}",
            seen.iter()
                .filter(|(_, e)| !matches!(e, MatchEvent::ReelTick { .. } | MatchEvent::MinigameTick { .. }))
                .collect::<Vec<_>>()
        );
    };
    assert!(h <= u && u < c, "hooked {h}, the Power Reel at {u}, landed {c}");
    assert!(has(&seen, |e| matches!(
        e,
        MatchEvent::ItemUsed {
            item: Item::PowerReel,
            ..
        }
    )));
    assert_eq!(m.items.seats[0].held, None);
    // a bluegill: kept
    let (m, seen) = power_reel_on("bluegill");
    assert!(has(&seen, |e| matches!(e, MatchEvent::Caught { .. })));
    assert_eq!(m.items.seats[0].held, Some(Item::PowerReel));
}

/// The brain's wish to use `item` now, on Burt Lake (layout 0) from the
/// spawn, with `goal`.
fn wants(item: Item, goal: Goal, rival: Option<P>) -> bool {
    let t = sample();
    let mut s = Setup::new(&t);
    s.seed = 3;
    s = s
        .with(0, Occupant::Ai(Difficulty::Normal), "P0")
        .with(1, Occupant::Human, "P1");
    let mut m = Match::new(generate(Setting::MichiganLake, 0), &t, s).unwrap();
    m.tick = 1000;
    if let Some(p) = rival {
        let b = m.boats[1].as_mut().unwrap();
        b.set_at(p);
        b.prev_um = b.pos_um;
    }
    let mut brain = Brain::new(0, Difficulty::Normal, &m);
    brain.goal = goal;
    if let Some(p) = rival {
        brain.known[1] = Some((p, [0, 0], m.tick));
    }
    let senses = sense(&m, 0);
    let b = m.boat(0).unwrap().clone();
    items::want_use(&mut brain, &senses, &b, item, m.tick)
}

#[test]
fn nitrous_and_the_teleporter_are_for_far_trips_and_hunts() {
    let t = sample();
    let m = Match::new(generate(Setting::MichiganLake, 0), &t, Setup::new(&t)).unwrap();
    let at = m.boat(0).map_or(m.map.spawn.at, |b| b.at());
    let off = |d: i32| P::new(at.x + d, at.y);
    let until = 1000 + 50 * 20;
    // a drive 250 m off: yes; 150 m: no (200 m)
    assert!(wants(Item::Nitrous, Goal::Explore { to: off(250_000) }, None));
    assert!(!wants(Item::Nitrous, Goal::Explore { to: off(150_000) }, None));
    // a hunt 150 m off: yes (120 m); 100 m: no
    let hunt = Goal::Hunt { seat: 1, until };
    assert!(wants(Item::Nitrous, hunt, Some(off(150_000))));
    assert!(!wants(Item::Nitrous, hunt, Some(off(100_000))));
    // the Teleporter: a hunt 200 m off with 20 s left: yes; 150 m: no
    assert!(wants(Item::Teleporter, hunt, Some(off(200_000))));
    assert!(!wants(Item::Teleporter, hunt, Some(off(150_000))));
    // 200 m off with 10 s left: no (14 s)
    let late = Goal::Hunt {
        seat: 1,
        until: 1000 + 50 * 10,
    };
    assert!(!wants(Item::Teleporter, late, Some(off(200_000))));
    // and not to a fish in sight
    assert!(!wants(Item::Teleporter, Goal::Fish { fish: 0 }, None));
}

#[test]
fn cloaking_tech_goes_on_for_a_hunt_and_not_for_exploring() {
    let t = sample();
    let m = Match::new(generate(Setting::MichiganLake, 0), &t, Setup::new(&t)).unwrap();
    let at = m.boat(0).map_or(m.map.spawn.at, |b| b.at());
    let hunt = Goal::Hunt { seat: 1, until: 3000 };
    assert!(wants(Item::Cloak, hunt, Some(P::new(at.x + 50_000, at.y))));
    assert!(!wants(
        Item::Cloak,
        Goal::Explore {
            to: P::new(at.x + 50_000, at.y)
        },
        None
    ));
}

#[test]
fn cloaking_tech_goes_on_at_the_hook_of_a_rare_fish() {
    let (mut m, mut crew) = one_ai(Difficulty::Hard, &[]);
    m.items.seats[0].held = Some(Item::Cloak);
    let at = m.boat(0).unwrap().at();
    put_fish(&mut m, "lake-trout", P::new(at.x + 20_000, at.y), 1000);
    m.fishing.tame = true;
    let seen = run(&mut m, &mut crew, 50 * 90, |e| {
        matches!(e, MatchEvent::Caught { .. })
    });
    let hooked = seen
        .iter()
        .find(|(_, e)| matches!(e, MatchEvent::Hooked { .. }))
        .map(|x| x.0);
    let cloaked = seen
        .iter()
        .find(|(_, e)| {
            matches!(
                e,
                MatchEvent::ItemUsed {
                    item: Item::Cloak,
                    ..
                }
            )
        })
        .map(|x| x.0);
    assert!(
        hooked.is_some() && cloaked.is_some() && hooked <= cloaked,
        "hook {hooked:?}, cloak {cloaked:?}"
    );
    assert!(
        has(&seen, |e| matches!(e, MatchEvent::Caught { seat: 0, .. })),
        "the trout got away"
    );
}

#[test]
fn enhanced_radar_goes_on_with_no_fish_in_sight_and_the_ai_heads_for_a_marked_fish() {
    let (mut m, mut crew) = one_ai(Difficulty::Normal, &[]);
    m.items.seats[0].held = Some(Item::EnhancedRadar);
    // a pike 150 m off, out of sight (fish show from 25 m to 40 m)
    let at = m.boat(0).unwrap().at();
    let pike = P::new(at.x + 150_000, at.y);
    put_fish(&mut m, "northern-pike", pike, 4000);
    // 8 s blind, the minigame, the ping
    let seen = run(&mut m, &mut crew, 50 * 60, |e| {
        matches!(e, MatchEvent::Revealed { .. })
    });
    let used = seen
        .iter()
        .find(|(_, e)| {
            matches!(
                e,
                MatchEvent::MinigameStarted {
                    item: Item::EnhancedRadar,
                    ..
                }
            )
        })
        .map(|x| x.0);
    assert!(
        used.is_some_and(|u| u >= 8 * 50),
        "the Enhanced Radar at {used:?}"
    );
    assert!(
        has(&seen, |e| matches!(e, MatchEvent::Revealed { seat: 0, count: 1 })),
        "{seen:?}"
    );
    // it makes for the pike
    run(&mut m, &mut crew, 50 * 3, |_| false);
    let to = match crew.brains[0].goal {
        Goal::Explore { to } => to,
        Goal::Fish { .. } => pike,
        g => panic!("{g:?}"),
    };
    assert!(
        to.dist(pike) < 2000,
        "it heads for {to:?}, not the pike at {pike:?}"
    );
}

#[test]
fn the_ai_teleports_toward_a_far_hunted_rival() {
    let t = sample();
    let mut s = Setup::new(&t);
    s.seed = 3;
    s = s
        .with(0, Occupant::Ai(Difficulty::Hard), "P0")
        .with(1, Occupant::Human, "P1");
    let mut m = Match::new(generate(Setting::MichiganLake, 0), &t, s).unwrap();
    m.items.next_buoy = u32::MAX / 2;
    m.items.seats[0].held = Some(Item::Teleporter);
    let mut crew = Crew::new(&m);
    // the rival in the farthest open water from the spawn
    let spawn = m.map.spawn.at;
    let nav = &crew.nav;
    let far = (0..nav.n * nav.n)
        .filter(|&c| nav.open[c])
        .map(|c| nav.centre(c))
        .max_by_key(|p| p.dist2(spawn))
        .unwrap();
    assert!(far.dist(spawn) > 250_000);
    let b = m.boats[1].as_mut().unwrap();
    b.set_at(far);
    b.prev_um = b.pos_um;
    crew.brains[0].known[1] = Some((far, [0, 0], 0));
    crew.brains[0].goal = Goal::Hunt {
        seat: 1,
        until: 50 * 40,
    };
    // the minigame (10 circles, about 9 s), the click, the 5 s charge
    let mut landed = None;
    for _ in 0..50 * 30 {
        let i = crew.inputs(&m);
        let e = m.step(&i).to_vec();
        // the hunt holds while the rival "reels"
        if let Goal::Explore { .. } = crew.brains[0].goal {
            crew.brains[0].goal = Goal::Hunt {
                seat: 1,
                until: 50 * 40,
            };
            crew.brains[0].known[1] = Some((far, [0, 0], m.tick));
        }
        if e.iter().any(|e| matches!(e, MatchEvent::Teleported { seat: 0 })) {
            landed = Some(m.boat(0).unwrap().at());
            break;
        }
    }
    let at = landed.expect("no teleport in 30 s");
    // 35 m short of the rival, give or take the radar's square to the
    // screen's 760 units (about a metre a unit) and water's snap
    let d = at.dist(far);
    assert!(
        (20_000..=50_000).contains(&d),
        "landed {} m from the rival",
        d / 1000
    );
}
