//! The AI players. Expected values are worked by hand from the shipped
//! `ai` tuning (`tuning::tests::ai_sample`) and the sample match tuning:
//! top speed 16 m/s, a ram at 90% of it (14.4 m/s), a perfect common reel
//! 3 s, Hard's reaction 150 to 300 ms (8 to 15 ticks).

use super::senses::sense;
use super::*;
use crate::map::fish::Tier;
use crate::map::{generate, Map, Setting, Spawn};
use crate::roster::Roster;
use crate::sim::fishing::lake::{FishState, LakeFish, School};
use crate::sim::{MatchEvent, Occupant, Setup, WreckCause};
use crate::tuning::{self, Tuning};

const FISH: &str = include_str!("../../../../data/fish.json");

fn sample() -> Tuning {
    tuning::tests::sample()
}

fn roster() -> Roster {
    Roster::parse(FISH).unwrap()
}

/// A square lake 40 cells (160 m) a side, land round the edge, the spawn
/// at (60 m, 80 m) facing east.
fn small_lake() -> Map {
    let n = 40;
    let water: Vec<bool> = (0..n * n)
        .map(|i| {
            let (x, y) = (i % n, i / n);
            !(x == 0 || y == 0 || x == n - 1 || y == n - 1)
        })
        .collect();
    let spawn = Spawn {
        at: P::new(60_000, 80_000),
        heading: 0,
        dock: P::new(0, 80_000),
    };
    Map::from_water(Setting::MichiganLake, 0, n, 4000, &water, spawn)
}

fn setup(t: &Tuning, seats: &[Occupant]) -> Setup {
    let mut s = Setup::new(t);
    s.seed = 7;
    for (k, o) in seats.iter().enumerate() {
        s = s.with(k, *o, &format!("P{k}"));
    }
    s
}

/// Puts one fish of roster kind `id` in the lake, still, as a school of one.
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

/// Steps the match with the crew's inputs (and none for people) for up to
/// `ticks`, until `stop` holds of an event; the events seen.
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

#[test]
fn the_ai_drives_from_the_spawn_to_the_far_end_without_grounding() {
    let t = sample();
    // the real Burt Lake, layout 0, with no fish to stop for
    let map = generate(Setting::MichiganLake, 0);
    let mut m = Match::new(map, &t, setup(&t, &[Occupant::Ai(Difficulty::Normal)])).unwrap();
    let mut crew = Crew::new(&m);
    // the open water farthest from the spawn with land in the straight way
    let nav = &crew.nav;
    let spawn = m.map.spawn.at;
    let far = (0..nav.n * nav.n)
        .filter(|&c| nav.open[c])
        .map(|c| nav.centre(c))
        .filter(|&p| super::nav::land_between(&m.map, spawn, p))
        .max_by_key(|p| p.dist2(spawn))
        .unwrap();
    assert!(
        far.dist(spawn) > 300_000,
        "the far point is {} m off",
        far.dist(spawn) / 1000
    );
    crew.brains[0].goal = Goal::Explore { to: far };
    let mut arrived = false;
    for _ in 0..crate::sim::TICKS_PER_S as u32 * 90 {
        let i = crew.inputs(&m);
        let e = m.step(&i).to_vec();
        assert!(
            !e.iter().any(|e| matches!(e, MatchEvent::Wrecked { .. })),
            "wrecked at tick {}",
            m.tick
        );
        if m.boat(0).unwrap().at().dist(far) < 16_000 {
            arrived = true;
            break;
        }
        // it goes nowhere else on the way
        if !matches!(crew.brains[0].goal, Goal::Explore { to } if to == far) {
            crew.brains[0].goal = Goal::Explore { to: far };
        }
    }
    assert!(
        arrived,
        "it did not get there in 90 s; it is at {:?}",
        m.boat(0).unwrap().at()
    );
}

/// One AI in the small lake with one bluegill 20 m ahead, 1 m down.
fn bluegill_ahead(d: Difficulty) -> (Match, Crew, u32) {
    let t = sample();
    let mut m = Match::new(small_lake(), &t, setup(&t, &[Occupant::Ai(d)])).unwrap();
    m.fishing.roster = roster();
    let f = put_fish(&mut m, "bluegill", P::new(80_000, 80_000), 1000);
    let crew = Crew::new(&m);
    (m, crew, f)
}

#[test]
fn the_ai_catches_a_fish_it_can_see() {
    let (mut m, mut crew, f) = bluegill_ahead(Difficulty::Normal);
    // the drive (about 2 s), the cast (2 s), the fish's swim to the lure,
    // the wait (up to 6 s), up to 3 nibbles (up to 4.2 s) and a 3 s reel:
    // well inside 40 s
    let seen = run(&mut m, &mut crew, 40 * 50, |e| {
        matches!(e, MatchEvent::Caught { .. })
    });
    assert!(
        seen.iter()
            .any(|(_, e)| matches!(e, MatchEvent::Caught { seat: 0, fish, .. } if *fish == f)),
        "no catch in 40 s: {:?}",
        seen.iter().map(|(t, e)| format!("{t} {e:?}")).collect::<Vec<_>>()
    );
    assert_eq!(m.scores[0].catches.len(), 1);
    // it never scared the fish with Space before the bite
    assert!(!seen.iter().any(|(_, e)| matches!(e, MatchEvent::Scared { .. })));
}

#[test]
fn the_ai_hooks_within_its_reaction_time() {
    for d in Difficulty::ALL {
        let (mut m, mut crew, _) = bluegill_ahead(d);
        let skill = m.tuning.ai.skill.get(d).clone();
        let seen = run(&mut m, &mut crew, 40 * 50, |e| {
            matches!(e, MatchEvent::Hooked { .. } | MatchEvent::Missed { .. })
        });
        let bite = seen
            .iter()
            .find(|(_, e)| matches!(e, MatchEvent::Bite { .. }))
            .map(|x| x.0);
        let hook = seen
            .iter()
            .find(|(_, e)| matches!(e, MatchEvent::Hooked { .. }))
            .map(|x| x.0);
        let (Some(bite), Some(hook)) = (bite, hook) else {
            panic!("{d:?}: no bite and hook: {seen:?}");
        };
        // it sees the bite after that step and presses on the step after
        // its reaction: reaction ticks plus one
        let gap = hook - bite;
        let lo = skill.reaction_min_ms as u32 / 20 + 1;
        let hi = skill.reaction_max_ms as u32 / 20 + 1;
        assert!(
            (lo..=hi).contains(&gap),
            "{d:?}: hooked {gap} ticks after the bite, not {lo} to {hi}"
        );
    }
}

/// Points per seat after a 4-minute match of four AIs on a seed, with the
/// AI going for the item buoys or not (`ai.items.use_items`).
fn match_points(seed: u64, seats: [Difficulty; 4], items: bool) -> Vec<i32> {
    let mut t = sample();
    t.ai.items.use_items = items;
    let mut s = Setup::new(&t);
    s.seed = seed;
    s.length_ms = 240_000;
    let r = roster();
    s.fish = r.0.iter().map(|k| (k.id.clone(), k.count)).collect();
    for (k, d) in seats.iter().enumerate() {
        s = s.with(k, Occupant::Ai(*d), &format!("P{k}"));
    }
    let map = generate(Setting::MichiganLake, [0, 3, 8, 11][seed as usize % 4]);
    let mut m = Match::new(map, &t, s).unwrap();
    m.stock(&r).unwrap();
    let mut crew = Crew::new(&m);
    while !m.ended {
        let i = crew.inputs(&m);
        m.step(&i);
    }
    m.scores
        .iter()
        .map(|s| s.catches.iter().map(|c| c.value).sum())
        .collect()
}

#[test]
fn hard_beats_easy_on_average() {
    use Difficulty::{Easy, Hard};
    // with items, and without: there the skill alone decides (with items,
    // Hard's quicker hand at the minigames wins even on Easy's skill)
    for items in [true, false] {
        let (mut hard, mut easy) = (0, 0);
        for seed in 1..=4 {
            // the seats change round so neither sits nearer the fish
            let seats = if seed % 2 == 0 {
                [Hard, Easy, Hard, Easy]
            } else {
                [Easy, Hard, Easy, Hard]
            };
            let p = match_points(seed, seats, items);
            for (k, d) in seats.iter().enumerate() {
                if *d == Hard {
                    hard += p[k];
                } else {
                    easy += p[k];
                }
            }
        }
        assert!(
            hard > easy,
            "items {items}: Hard {hard} points against Easy {easy}"
        );
    }
}

#[test]
fn a_near_legendary_hook_is_hunted_and_a_far_common_one_is_not() {
    let t = sample();
    let mut m = Match::new(
        small_lake(),
        &t,
        setup(&t, &[Occupant::Ai(Difficulty::Normal), Occupant::Human]),
    )
    .unwrap();
    m.fishing.roster = roster();
    let s = sense(&m, 0);
    let skill = t.ai.skill.get(Difficulty::Normal);
    let at = P::new(0, 0);
    // a muskellunge 100 m off, just hooked: there in 100 x 1.3 / 14.4 + 2.5
    // = 11.5 s, inside a legendary's 33 s reel; worth 55 x 5 x 1.25 = 344,
    // times 45% aggression and 1.5 = 232, against 40 points a minute for
    // 23 s = 15
    assert!(super::hunt::worth_hunting(
        &s,
        skill,
        at,
        P::new(100_000, 0),
        "muskellunge",
        Tier::Legendary,
        0
    ));
    // a bluegill 300 m off: 29.6 s away, and its reel is 3 s
    assert!(!super::hunt::worth_hunting(
        &s,
        skill,
        at,
        P::new(300_000, 0),
        "bluegill",
        Tier::Common,
        0
    ));
    // the same muskellunge 40 s after its hook: the reel will be over
    assert!(!super::hunt::worth_hunting(
        &s,
        skill,
        at,
        P::new(100_000, 0),
        "muskellunge",
        Tier::Legendary,
        40_000
    ));

    // and in the match: seat 1, 60 m east in sight, hooks a muskellunge
    let b = m.boats[1].as_mut().unwrap();
    b.set_at(P::new(120_000, 80_000));
    b.prev_um = b.pos_um;
    let mut crew = Crew::new(&m);
    m.boats[0].as_mut().unwrap().set_at(P::new(60_000, 80_000));
    crew.inputs(&m);
    m.events.push(MatchEvent::Announced {
        seat: 1,
        fish: 0,
        kind: "muskellunge".into(),
        tier: Tier::Legendary,
        delay_ms: 0,
    });
    crew.inputs(&m);
    assert!(
        matches!(crew.brains[0].goal, Goal::Hunt { seat: 1, .. }),
        "{:?}",
        crew.brains[0].goal
    );
}

#[test]
fn a_full_match_of_four_ais_runs_the_same_twice_and_ends_with_catches() {
    let t = sample();
    let r = roster();
    let play = || {
        let mut s = Setup::new(&t);
        s.seed = 11;
        s.fish = r.0.iter().map(|k| (k.id.clone(), k.count)).collect();
        for (k, d) in [
            Difficulty::Normal,
            Difficulty::Hard,
            Difficulty::Easy,
            Difficulty::Normal,
        ]
        .iter()
        .enumerate()
        {
            s = s.with(k, Occupant::Ai(*d), &format!("P{k}"));
        }
        let mut m = Match::new(generate(Setting::MichiganLake, 3), &t, s).unwrap();
        m.stock(&r).unwrap();
        let mut crew = Crew::new(&m);
        let mut log = Vec::new();
        let mut grounded = 0;
        while !m.ended {
            let i = crew.inputs(&m);
            let tick = m.tick + 1;
            for e in m.step(&i) {
                if matches!(
                    e,
                    MatchEvent::Wrecked {
                        cause: WreckCause::Grounded,
                        ..
                    }
                ) {
                    grounded += 1;
                }
                log.push(format!("{tick} {e:?}"));
            }
        }
        let at: Vec<P> = m.boats.iter().flatten().map(|b| b.at()).collect();
        (m.tick, m.scores.clone(), at, log, grounded)
    };
    let a = play();
    let b = play();
    // 8 minutes at 50 ticks a second
    assert_eq!(a.0, 24_000);
    assert_eq!(a.1, b.1);
    assert_eq!(a.2, b.2);
    assert_eq!(a.3, b.3);
    let catches: usize = a.1.iter().map(|s| s.catches.len()).sum();
    assert!(catches >= 4, "only {catches} catches");
    assert_eq!(a.4, 0, "an AI grounded");
}

#[test]
fn the_ai_reads_only_what_its_seat_could_know() {
    let t = sample();
    let mut m = Match::new(
        small_lake(),
        &t,
        setup(&t, &[Occupant::Ai(Difficulty::Normal), Occupant::Human]),
    )
    .unwrap();
    m.fishing.roster = roster();
    // a fish 100 m off and 12 m down is out of sight (25 m for deep fish)
    put_fish(&mut m, "muskellunge", P::new(150_000 - 10_000, 150_000), 12_000);
    // a rival 130 m off across the water: past the 120 m fog and the
    // 80 m motor; then 50 m off: in sight
    let b = m.boats[1].as_mut().unwrap();
    b.set_at(P::new(60_000, 80_000 + 130_000 / 2));
    let me = m.boats[0].as_mut().unwrap();
    me.set_at(P::new(60_000, 80_000 - 130_000 / 2));
    let s = sense(&m, 0);
    assert!(s.fishing.fish.is_empty());
    assert!(s.rivals.is_empty());
    m.boats[1]
        .as_mut()
        .unwrap()
        .set_at(P::new(60_000, 80_000 - 130_000 / 2 + 50_000));
    let s = sense(&m, 0);
    assert_eq!(s.rivals.len(), 1);
    assert_eq!(s.rivals[0].seat, 1);
}

#[test]
fn the_ai_slows_under_the_grounding_speed_near_land() {
    // a goal past the small lake's east shore, straight ahead from 90 m
    // back: at top speed the boat would hit the shore at 16 m/s and wreck
    let t = sample();
    let mut m = Match::new(small_lake(), &t, setup(&t, &[Occupant::Ai(Difficulty::Normal)])).unwrap();
    let mut crew = Crew::new(&m);
    let to = P::new(170_000, 80_000);
    let mut fastest_near = 0;
    for _ in 0..50 * 15 {
        crew.brains[0].goal = Goal::Explore { to };
        let i = crew.inputs(&m);
        let e = m.step(&i).to_vec();
        assert!(
            !e.iter().any(|e| matches!(e, MatchEvent::Wrecked { .. })),
            "wrecked at tick {}",
            m.tick
        );
        let b = m.boat(0).unwrap();
        if b.at().x > 148_000 {
            fastest_near = fastest_near.max(b.speed_mm_per_s());
        }
    }
    // within 8 m of the shore it keeps to its careful 40% (6.4 m/s), with
    // the throttle's 0.3 m/s of slack: under half of top speed (8 m/s)
    assert!(fastest_near > 0, "it never got near the shore");
    assert!(fastest_near < 8000, "{fastest_near} mm/s by the shore");
}

#[test]
fn the_ai_hunts_on_the_hook_popup_not_the_hook_and_knows_its_delay() {
    let t = sample();
    let new = || {
        let mut m = Match::new(
            small_lake(),
            &t,
            setup(&t, &[Occupant::Ai(Difficulty::Normal), Occupant::Human]),
        )
        .unwrap();
        m.fishing.roster = roster();
        // a minute in, seat 1 60 m east in sight
        m.tick = 3000;
        let b = m.boats[1].as_mut().unwrap();
        b.set_at(P::new(120_000, 80_000));
        b.prev_um = b.pos_um;
        m.boats[0].as_mut().unwrap().set_at(P::new(60_000, 80_000));
        let mut crew = Crew::new(&m);
        crew.inputs(&m);
        (m, crew)
    };
    let hunting = |crew: &Crew| matches!(crew.brains[0].goal, Goal::Hunt { seat: 1, .. });
    let announced = |delay_ms: i32| MatchEvent::Announced {
        seat: 1,
        fish: 0,
        kind: "muskellunge".into(),
        tier: Tier::Legendary,
        delay_ms,
    };
    // the hook alone is not heard
    let (mut m, mut crew) = new();
    m.events.push(MatchEvent::Hooked {
        seat: 1,
        fish: 0,
        kind: "muskellunge".into(),
        tier: Tier::Legendary,
    });
    crew.inputs(&m);
    assert!(!hunting(&crew), "{:?}", crew.brains[0].goal);
    // a popup 30 s late: 3 s of a legendary's 33 s reel are left, and it is
    // 60 x 1.3 / 14.4 + 2.5 = 7.9 s away
    let (mut m, mut crew) = new();
    m.events.push(announced(30_000));
    crew.inputs(&m);
    assert!(!hunting(&crew), "{:?}", crew.brains[0].goal);
    // the same popup at once is hunted
    let (mut m, mut crew) = new();
    m.events.push(announced(0));
    crew.inputs(&m);
    assert!(hunting(&crew), "{:?}", crew.brains[0].goal);
}

/// The worst tick of a seeded 8-minute match of four Hard AI players on
/// Burt Lake, layout `layout`: (A* searches in it, their steps), and the
/// match's searches in all.
fn worst_tick_of_four_hard_ais(seed: u64, layout: u32) -> ((u64, u64), u64) {
    let t = sample();
    let r = roster();
    let mut s = Setup::new(&t);
    s.seed = seed;
    s.fish = r.0.iter().map(|k| (k.id.clone(), k.count)).collect();
    for k in 0..4 {
        s = s.with(k, Occupant::Ai(Difficulty::Hard), &format!("P{k}"));
    }
    let mut m = Match::new(generate(Setting::MichiganLake, layout), &t, s).unwrap();
    m.stock(&r).unwrap();
    let mut crew = Crew::new(&m);
    let mut worst = (0, 0);
    let mut was = crew.nav.work();
    while !m.ended {
        let i = crew.inputs(&m);
        let now = crew.nav.work();
        worst = (worst.0.max(now.0 - was.0), worst.1.max(now.1 - was.1));
        was = now;
        m.step(&i);
    }
    (worst, was.0)
}

#[test]
fn four_hard_ais_run_at_most_one_path_search_a_tick() {
    // A cost guard, counted in steps rather than timed, so a busy machine
    // cannot fail it. The AI's slow ticks were path searches, several
    // players starting one in the same tick (all four leave the spawn on
    // the first). One search on this 96-cell grid takes at most
    // 96 x 96 = 9216 cells off the open list, plus its 2 m line samples;
    // the steps allowed are about 3 times the worst measured (SECOND-ORDER
    // R6.4).
    for (seed, layout) in [(9, 3), (5, 0)] {
        let ((searches, steps), all) = worst_tick_of_four_hard_ais(seed, layout);
        assert!(all > 10, "seed {seed}: only {all} searches in a match");
        assert_eq!(searches, 1, "seed {seed}: {searches} path searches in one tick");
        assert!(steps <= 10_000, "seed {seed}: a tick took {steps} search steps");
    }
}
