//! Online play over the simulated transport (`loopback.rs`): latency,
//! jitter, resends and drops, from a seed. Expected values are worked by
//! hand from the sample tuning: a tick is 20 ms, a ram needs 12.8 m/s
//! toward the other boat, a wreck lasts 2 s (100 ticks).

use super::loopback::{LinkSpec, Loop};
use super::session::Status;
use super::*;
use crate::sim::{MatchEvent, Occupant, SeatInput, WreckCause};

fn hold(names: &[&str]) -> SeatInput {
    let mut i = SeatInput::default();
    for n in names {
        i.held.set(n, true);
    }
    i
}

#[test]
fn two_to_four_browsers_agree_on_the_seats_and_the_timer() {
    for joiners in 1..=3 {
        for seed in [1, 2, 3] {
            // two people and room for two AI players; 4 people leave none
            let mut l = Loop::new(joiners, 2, LinkSpec::ROUGH, seed);
            l.lobby_and_start();
            l.run(5000);
            let host = l.nodes[0].game();
            let humans = joiners + 1;
            let ai = 2.min(4 - humans);
            let occupants: Vec<Occupant> = host.setup.seats.iter().map(|s| s.occupant).collect();
            assert_eq!(
                occupants.iter().filter(|o| **o == Occupant::Human).count(),
                humans,
                "{joiners} joiners: {occupants:?}"
            );
            assert_eq!(
                occupants.iter().filter(|o| matches!(o, Occupant::Ai(_))).count(),
                ai,
                "{joiners} joiners: {occupants:?}"
            );
            for (k, node) in l.nodes.iter().enumerate() {
                let g = node.game();
                assert_eq!(
                    g.setup, host.setup,
                    "seed {seed}: browser {k} has the host's seats"
                );
                // seated in the order their Hello came (one at a time here)
                assert_eq!(node.session.seat(), Some(k), "seed {seed}");
                // within two ticks (40 ms) of the host's clock, on a link of
                // 60 ± 40 ms
                let off = g.tick as i64 - host.tick as i64;
                assert!(
                    off.abs() <= 2,
                    "seed {seed}: browser {k} is {off} ticks off the host"
                );
                assert_eq!(g.boats.iter().flatten().count(), humans + ai);
            }
        }
    }
}

#[test]
fn the_lobby_survives_dropped_messages() {
    // half the messages lost outright: the Hello, the Ready and the start
    // are repeated until seen
    let spec = LinkSpec {
        drop_pct: 50,
        ..LinkSpec::CLEAN
    };
    for seed in 1..=8 {
        let mut l = Loop::new(3, 0, spec, seed);
        l.lobby_and_start();
        let r = l.nodes[0].session.roster();
        assert!(r.seats[1..].iter().all(|s| s.ready), "seed {seed}");
    }
}

#[test]
fn a_boat_driven_in_one_browser_moves_in_the_others_100_ms_behind() {
    let mut l = Loop::new(2, 0, LinkSpec::CLEAN, 4);
    l.lobby_and_start();
    l.nodes[1].input = hold(&["forward"]);
    l.run(3000);
    let own = l.nodes[1].game().boat(1).unwrap().at().x;
    let spawn = l.nodes[0].game().spawn_slot(1).0.x;
    assert!(own > spawn + 10_000, "the joiner's boat moved on its own browser");
    for k in [0, 2] {
        let seen = l.nodes[k].game().boat(1).unwrap().at().x;
        // about 13 m/s after 3 s: 100 ms behind is about 1.3 m, and the
        // ticks may differ by one or two (up to 40 ms, 0.6 m)
        let behind = own - seen;
        assert!(
            (600..=2200).contains(&behind),
            "browser {k} draws the boat {behind} mm behind its owner"
        );
    }
}

#[test]
fn joiners_with_no_link_between_them_see_each_other_through_the_host() {
    let mut l = Loop::new(2, 0, LinkSpec::CLEAN, 5);
    l.cut.insert((1, 2));
    l.lobby_and_start();
    l.nodes[2].input = hold(&["forward"]);
    l.run(2000);
    let own = l.nodes[2].game().boat(2).unwrap().at().x;
    let seen = l.nodes[1].game().boat(2).unwrap().at().x;
    let spawn = l.nodes[0].game().spawn_slot(2).0.x;
    assert!(
        seen > spawn + 5000,
        "joiner 1 sees joiner 2's boat move ({seen} from {spawn})"
    );
    assert!(
        own - seen < 3000,
        "and close behind its owner ({own} against {seen})"
    );
}

/// Joiner 1's boat 15 m west of joiner 2's, at top speed toward it.
fn set_up_a_ram(l: &mut Loop) -> i32 {
    let target = l.nodes[0].game().spawn_slot(2).0;
    let victim_at = crate::map::P::new(target.x + 40_000, target.y);
    {
        let m = l.nodes[2].game_mut();
        let b = m.boats[2].as_mut().unwrap();
        b.set_at(victim_at);
        b.prev_um = b.pos_um;
        b.ghost_until = 0;
    }
    {
        let m = l.nodes[1].game_mut();
        let b = m.boats[1].as_mut().unwrap();
        b.set_at(crate::map::P::new(victim_at.x - 15_000, victim_at.y));
        b.prev_um = b.pos_um;
        b.heading = 0;
        b.set_vel([16_000, 0]);
        b.ghost_until = 0;
    }
    l.nodes[1].input = hold(&["forward"]);
    victim_at.x
}

#[test]
fn a_ram_judged_by_the_host_wrecks_the_victim_in_every_browser() {
    for seed in [1, 2, 3, 4] {
        let mut l = Loop::new(2, 0, LinkSpec::ROUGH, seed);
        l.lobby_and_start();
        // past the respawn ghosting (3 s)
        l.run(3500);
        set_up_a_ram(&mut l);
        let wrecked_everywhere = l.run_until(3000, |l| {
            l.nodes
                .iter()
                .all(|n| n.game().boat(2).is_some_and(|b| b.wrecked()))
        });
        assert!(
            wrecked_everywhere,
            "seed {seed}: the victim is wrecked in every browser"
        );
        // the victim's own state can beat the host's verdict to a browser
        l.run(1000);
        for (k, node) in l.nodes.iter().enumerate() {
            let wrecks: Vec<_> = node
                .log
                .iter()
                .filter(|(_, e)| matches!(e, MatchEvent::Wrecked { .. }))
                .map(|(_, e)| e.clone())
                .collect();
            assert_eq!(
                wrecks,
                vec![MatchEvent::Wrecked {
                    seat: 2,
                    cause: WreckCause::Rammed { by: 1 }
                }],
                "seed {seed}: browser {k} hears one wreck, by the rammer"
            );
            assert!(
                node.log
                    .iter()
                    .any(|(_, e)| *e == MatchEvent::Stalled { seat: 1 }),
                "seed {seed}: browser {k} hears the rammer stall"
            );
        }
        let rammer = l.nodes[1].game();
        assert!(
            rammer.boat(1).unwrap().stalled(rammer.tick),
            "seed {seed}: the rammer's own browser stalls its motor"
        );
    }
}

#[test]
fn a_slow_bump_wrecks_nobody() {
    let mut l = Loop::new(2, 0, LinkSpec::CLEAN, 6);
    l.lobby_and_start();
    l.run(3500);
    set_up_a_ram(&mut l);
    // 10 m/s and no throttle: under the 12.8 m/s a ram needs
    l.nodes[1].game_mut().boats[1]
        .as_mut()
        .unwrap()
        .set_vel([10_000, 0]);
    l.nodes[1].input = SeatInput::default();
    l.run(3000);
    for node in &l.nodes {
        assert!(!node
            .log
            .iter()
            .any(|(_, e)| matches!(e, MatchEvent::Wrecked { .. })));
    }
}

#[test]
fn a_grounding_is_judged_by_its_owner_and_heard_by_everyone() {
    let mut l = Loop::new(2, 0, LinkSpec::CLEAN, 7);
    l.lobby_and_start();
    l.run(200);
    {
        // 3 m from the north shore (land from y = 156 m), at top speed
        let m = l.nodes[2].game_mut();
        let b = m.boats[2].as_mut().unwrap();
        b.set_at(crate::map::P::new(80_000, 152_000));
        b.heading = crate::map::trig::QUARTER;
        b.set_vel([0, 16_000]);
    }
    l.run(1000);
    for (k, node) in l.nodes.iter().enumerate() {
        let n = node
            .log
            .iter()
            .filter(|(_, e)| {
                *e == MatchEvent::Wrecked {
                    seat: 2,
                    cause: WreckCause::Grounded,
                }
            })
            .count();
        assert_eq!(n, 1, "browser {k} hears the grounding once");
    }
}

#[test]
fn a_joiner_who_leaves_the_lobby_frees_its_seat() {
    let mut l = Loop::new(2, 1, LinkSpec::CLEAN, 8);
    l.connect();
    l.run(1000);
    assert_eq!(l.nodes[0].session.roster().seats[2].who, Who::Player);
    l.close(2, true);
    l.run(1000);
    for k in [0, 1] {
        let r = l.nodes[k].session.roster();
        // the AI player moves into the seat left open
        assert!(
            matches!(r.seats[2].who, Who::Ai { .. }),
            "browser {k} sees seat 3 open for the AI: {:?}",
            r.seats
        );
        assert_eq!(r.seats[3].who, Who::Open);
    }
}

#[test]
fn a_joiner_who_leaves_a_match_takes_its_boat_out_and_the_match_goes_on() {
    for say_bye in [true, false] {
        let mut l = Loop::new(2, 0, LinkSpec::CLEAN, 10);
        l.lobby_and_start();
        l.run(1000);
        l.close(2, say_bye);
        l.run(2000);
        for k in [0, 1] {
            let g = l.nodes[k].game();
            assert!(g.boat(2).is_none(), "browser {k}: the leaver's boat is gone");
            assert!(g.boat(1).is_some() && !g.ended);
            assert_eq!(l.nodes[k].session.gone(), &[2]);
            assert_eq!(*l.nodes[k].session.status(), Status::Playing);
        }
    }
}

#[test]
fn the_host_leaving_ends_the_match_for_everyone() {
    for say_bye in [true, false] {
        let mut l = Loop::new(2, 0, LinkSpec::CLEAN, 11);
        l.lobby_and_start();
        l.run(1000);
        l.close(0, say_bye);
        l.run(500);
        for k in [1, 2] {
            assert_eq!(
                *l.nodes[k].session.status(),
                Status::HostLeft,
                "say_bye {say_bye}"
            );
        }
    }
    // a host that goes silent, with no word from the transport either
    let mut l = Loop::new(1, 0, LinkSpec::CLEAN, 12);
    l.lobby_and_start();
    l.nodes[0].gone = true;
    l.run(SILENT_MS + 1000);
    assert_eq!(*l.nodes[1].session.status(), Status::HostLeft);
}

#[test]
fn the_host_ends_the_match_and_its_end_reaches_every_browser() {
    let mut l = Loop::new(1, 0, LinkSpec::ROUGH, 13);
    l.lobby_and_start();
    // the sample match is 8 minutes; end it from the host's side
    let length = l.nodes[0].game().rules.length_ticks;
    l.nodes[0].game_mut().tick = length - 5;
    l.run(1000);
    assert!(l.nodes[0].game().ended);
    assert!(l.nodes[1].game().ended, "the joiner's match ends with the host's");
    // and a joiner's own clock running out does not end it alone
    let mut l = Loop::new(1, 0, LinkSpec::CLEAN, 14);
    l.lobby_and_start();
    let length = l.nodes[1].game().rules.length_ticks;
    l.nodes[1].game_mut().tick = length + 10;
    l.nodes[1].game_mut().online.end_elsewhere = true;
    l.run(100);
    assert!(!l.nodes[1].game().ended);
}

#[test]
fn two_claims_for_one_thing_are_settled_the_same_way_everywhere() {
    for seed in 1..=6 {
        // 200 ms apart, the earlier wins whatever the links did; at one
        // tick, one wins, and every browser agrees which
        for (t1, t2, winner) in [(300, 310, Some(1)), (310, 300, Some(2)), (300, 300, None)] {
            let mut l = Loop::new(2, 0, LinkSpec::ROUGH, seed);
            l.lobby_and_start();
            l.nodes[1].plans.push((t1, Claim::Stub { thing: 7 }));
            l.nodes[2].plans.push((t2, Claim::Stub { thing: 7 }));
            l.run(8000);
            let host: Vec<(usize, bool)> = l.nodes[0].verdicts.iter().map(|v| (v.seat, v.granted)).collect();
            let won: Vec<usize> = host.iter().filter(|(_, g)| *g).map(|(s, _)| *s).collect();
            assert_eq!(host.len(), 2, "seed {seed}: both claims answered: {host:?}");
            assert_eq!(won.len(), 1, "seed {seed}: one claim granted: {host:?}");
            if let Some(w) = winner {
                let seat = l.nodes[w].session.seat().unwrap();
                assert_eq!(won, vec![seat], "seed {seed}, claims at {t1} and {t2}");
            }
            for (k, node) in l.nodes.iter().enumerate() {
                let got: Vec<(usize, bool)> = node.verdicts.iter().map(|v| (v.seat, v.granted)).collect();
                assert_eq!(got, host, "seed {seed}: browser {k} hears the host's verdicts");
            }
        }
    }
}

#[test]
fn an_offline_claim_is_answered_at_once() {
    let l = Loop::new(0, 0, LinkSpec::CLEAN, 1);
    let mut m = crate::sim::Match::new(
        l.map.clone(),
        &l.tuning,
        crate::sim::Setup::new(&l.tuning)
            .with(0, Occupant::Human, "A")
            .with(1, Occupant::Human, "B"),
    )
    .unwrap();
    let a = m.claim(0, Claim::Stub { thing: 1 });
    let b = m.claim(1, Claim::Stub { thing: 1 });
    m.step(&[]);
    let got: Vec<(usize, u32, bool)> = m
        .online
        .verdicts
        .iter()
        .map(|v| (v.seat, v.id, v.granted))
        .collect();
    assert_eq!(got, vec![(0, a, true), (1, b, false)]);
    m.online.release(&Thing {
        kind: "stub".into(),
        id: 1,
    });
    m.claim(1, Claim::Stub { thing: 1 });
    m.step(&[]);
    assert!(
        m.online.verdicts[0].granted,
        "a released thing can be claimed again"
    );
}

#[test]
fn a_joiner_with_other_match_numbers_or_a_full_lobby_is_refused() {
    let mut l = Loop::new(1, 0, LinkSpec::CLEAN, 15);
    l.nodes[1].session = Session::join("Odd", "0000000000000000", 0);
    l.connect();
    l.run(1000);
    assert_eq!(
        *l.nodes[1].session.status(),
        Status::Refused {
            refusal: Refusal::Tuning
        }
    );
    let mut l = Loop::new(4, 0, LinkSpec::CLEAN, 16);
    l.connect();
    l.run(1000);
    let refused: Vec<usize> = (1..5)
        .filter(|k| {
            *l.nodes[*k].session.status()
                == Status::Refused {
                    refusal: Refusal::Full,
                }
        })
        .collect();
    assert_eq!(refused.len(), 1, "four seats: the fifth browser is refused");
}

#[test]
fn a_room_code_reads_back_and_a_bad_one_is_refused() {
    let code = room_code(42);
    assert_eq!(code.len(), CODE_LEN);
    assert_eq!(clean_code(&code.to_lowercase()), Some(code.clone()));
    assert_eq!(clean_code("ABC"), None);
    assert_eq!(clean_code("ABCDE0"), None, "0 is not a code letter");
    assert_eq!(room_name("ABCDEF"), format!("honey-hole-p{PROTOCOL}-ABCDEF"));
}

// ---- fishing, shared (`sim/fishing/shared.rs`) ----

const ROSTER: &str = r#"{"fish": [
    {"id": "bluegill", "name": "Bluegill", "tier": "common", "model": "bluegill", "base_length_mm": 200, "value": 10, "count": 8},
    {"id": "walleye", "name": "Walleye", "tier": "uncommon", "model": "walleye", "base_length_mm": 400, "value": 20, "count": 4}
]}"#;

/// Two joiners in a match on a stocked lake, past the start's ghosting.
fn fishing_loop(seed: u64) -> Loop {
    let mut l = Loop::new(2, 0, LinkSpec::ROUGH, seed);
    l.roster = Some(crate::roster::Roster::parse(ROSTER).unwrap());
    l.lobby_and_start();
    l.run(500);
    l
}

/// Joiner `k`'s lure set 10 m ahead of its bow, with fish `fish` there
/// biting until well after `press_at`, when Space goes down; Space stays
/// held after, and the reel's fish holds still (`tame`), so it lands.
fn bite(l: &mut Loop, k: usize, fish: usize, press_at: u32) {
    use crate::sim::fishing::lake::FishState;
    use crate::sim::fishing::{Interest, Stage};
    let m = l.nodes[k].game_mut();
    let b = m.boats[k].as_mut().unwrap();
    b.mode = crate::sim::Mode::Fishing;
    b.set_vel([0, 0]);
    let rod = m.rod(k);
    let at = crate::map::P::new(rod.x + 10_000, rod.y);
    m.fishing.tame = true;
    let line = &mut m.fishing.lines[k];
    line.stage = Stage::Set;
    line.bobber = at;
    line.lure_depth_mm = 1000;
    line.length_mm = 40_000;
    line.interest = Some(Interest {
        fish,
        arrived: true,
        next: u32::MAX,
        nibbles_left: 0,
        biting_until: Some(press_at + 1000),
    });
    let f = &mut m.fishing.fish[fish];
    f.at = at;
    f.depth_mm = 1000;
    f.state = FishState::Lured { seat: k };
    l.nodes[k].input = hold(&["cast"]);
    let mut press = hold(&["cast"]);
    press.pressed.set("cast", true);
    l.nodes[k].presses.push((press_at, press));
}

fn hooks_of(node: &super::loopback::Node) -> Vec<usize> {
    node.log
        .iter()
        .filter_map(|(_, e)| match e {
            MatchEvent::Hooked { seat, .. } => Some(*seat),
            _ => None,
        })
        .collect()
}

#[test]
fn two_lines_on_one_fish_one_hook_holds_and_its_catch_counts_everywhere() {
    use crate::sim::fishing::lake::FishState;
    for seed in [1, 2, 3] {
        let mut l = fishing_loop(seed);
        let t = l.nodes[1].game().tick;
        bite(&mut l, 1, 0, t + 20);
        bite(&mut l, 2, 0, t + 30);
        l.run(8000);
        for (k, node) in l.nodes.iter().enumerate() {
            let g = node.game();
            assert_eq!(
                g.fishing.fish[0].state,
                FishState::Caught { seat: 1 },
                "seed {seed}: browser {k} has the first hook's catch"
            );
            assert_eq!(g.scores[1].catches.len(), 1, "seed {seed}: browser {k} counts it");
            assert!(
                g.scores[2].catches.is_empty(),
                "seed {seed}: browser {k} gives seat 3 nothing"
            );
            assert_eq!(
                g.scores[1],
                l.nodes[0].game().scores[1],
                "seed {seed}: the same catch"
            );
        }
        // the host and the winner hear one hook, the winner's; the loser
        // hooked it for a moment and let it go
        assert_eq!(hooks_of(&l.nodes[0]), vec![1], "seed {seed}");
        assert_eq!(hooks_of(&l.nodes[1]), vec![1], "seed {seed}");
        assert!(
            l.nodes[2]
                .log
                .iter()
                .any(|(_, e)| *e == MatchEvent::FishDropped { seat: 2, fish: 0 }),
            "seed {seed}: the second line lets the fish go"
        );
    }
}

#[test]
fn a_fish_that_gets_away_is_free_for_another_line() {
    use crate::sim::fishing::lake::FishState;
    let mut l = fishing_loop(4);
    let t = l.nodes[1].game().tick;
    bite(&mut l, 1, 0, t + 20);
    // Esc a second later: the fish gets away
    let mut esc = hold(&[]);
    esc.pressed.set("cancel", true);
    l.nodes[1].presses.push((t + 70, esc));
    l.run(3000);
    l.nodes[1].input = SeatInput::default();
    let thing = Thing {
        kind: "fish".into(),
        id: 0,
    };
    assert_eq!(
        l.nodes[0].game().online.holder(&thing),
        None,
        "the host lets go of it"
    );
    for (k, node) in l.nodes.iter().enumerate() {
        assert!(
            !matches!(node.game().fishing.fish[0].state, FishState::Hooked { .. }),
            "browser {k} shows it free"
        );
    }
    let t = l.nodes[2].game().tick;
    bite(&mut l, 2, 0, t + 20);
    l.run(8000);
    for (k, node) in l.nodes.iter().enumerate() {
        assert_eq!(
            node.game().fishing.fish[0].state,
            FishState::Caught { seat: 2 },
            "browser {k}: the other line lands it"
        );
    }
}

#[test]
fn the_free_fish_are_where_the_host_has_them() {
    let mut l = fishing_loop(5);
    // joiner 1's lake drifts (its own lines draw on its chances alone):
    // every fish 10 m off
    for f in l.nodes[1].game_mut().fishing.fish.iter_mut() {
        f.at = crate::map::P::new(f.at.x + 10_000, f.at.y);
    }
    l.run(2000);
    let host = &l.nodes[0].game().fishing.fish;
    for k in [1, 2] {
        let mine = &l.nodes[k].game().fishing.fish;
        for (f, (a, b)) in host.iter().zip(mine).enumerate().skip(1) {
            // a second of roaming at most since the last word
            let d = a.at.dist(b.at);
            assert!(d <= 3000, "browser {k}: fish {f} is {d} mm from the host's");
        }
    }
}

#[test]
fn the_countdown_runs_out_together_and_the_clock_then_follows_the_host() {
    for seed in [1, 2, 3] {
        let mut l = Loop::new(2, 0, LinkSpec::ROUGH, seed);
        l.countdown_ms = 3000;
        l.lobby_and_start();
        // just after Go, and again later
        l.run_until(5000, |l| l.nodes[0].game().countdown == 0);
        for wait in [300, 3000] {
            l.run(wait);
            let host = l.nodes[0].game().tick;
            for (k, node) in l.nodes.iter().enumerate() {
                // a jittery link's readings can be a tick or so off either
                // way (60 ± 40 ms each way): within 60 ms
                let off = node.game().tick as i64 - host as i64;
                assert!(
                    off.abs() <= 3,
                    "seed {seed}: browser {k} is {off} ticks off the host {wait} ms after Go"
                );
            }
        }
        for (k, node) in l.nodes.iter().enumerate() {
            let beats = node
                .log
                .iter()
                .filter(|(_, e)| matches!(e, MatchEvent::Countdown { .. }))
                .count();
            let gos = node
                .log
                .iter()
                .filter(|(_, e)| *e == MatchEvent::MatchStarted)
                .count();
            assert_eq!(
                (beats, gos),
                (3, 1),
                "seed {seed}: browser {k} hears 3-2-1-Go once"
            );
        }
    }
}

#[test]
fn a_hook_popup_comes_at_the_hosts_delay_once_in_every_browser() {
    for (seed, delay) in [(1, 0), (2, 0), (1, 1000), (2, 1000)] {
        let mut l = Loop::new(2, 0, LinkSpec::ROUGH, seed);
        l.tuning.balance.announce_delay_ms.common = delay;
        l.tuning.balance.announce_delay_ms.uncommon = delay;
        l.roster = Some(crate::roster::Roster::parse(ROSTER).unwrap());
        l.lobby_and_start();
        l.run(500);
        let t = l.nodes[1].game().tick;
        // both lines on one fish: the first hook holds, the second lets go
        bite(&mut l, 1, 0, t + 20);
        bite(&mut l, 2, 0, t + 30);
        l.run(6000);
        for (k, node) in l.nodes.iter().enumerate() {
            let said: Vec<(u32, usize)> = node
                .log
                .iter()
                .filter_map(|(tick, e)| match e {
                    MatchEvent::Announced { seat, .. } => Some((*tick, *seat)),
                    _ => None,
                })
                .collect();
            let seats: Vec<usize> = said.iter().map(|(_, s)| *s).collect();
            assert_eq!(
                seats,
                vec![1],
                "seed {seed}, delay {delay}: browser {k} announces the hook that held, once"
            );
        }
        // on the host, the popup comes the delay after its grant
        let host = &l.nodes[0].log;
        let granted = host
            .iter()
            .find(|(_, e)| matches!(e, MatchEvent::Hooked { seat: 1, .. }))
            .map(|(t, _)| *t)
            .unwrap();
        let said = host
            .iter()
            .find(|(_, e)| matches!(e, MatchEvent::Announced { .. }))
            .map(|(t, _)| *t)
            .unwrap();
        assert_eq!(
            said - granted,
            crate::sim::ticks(delay),
            "seed {seed}, delay {delay}"
        );
    }
}

#[test]
fn another_players_line_and_the_fish_on_it_show_on_every_screen() {
    use crate::sim::fishing::Stage;
    let mut l = fishing_loop(6);
    let t = l.nodes[1].game().tick;
    bite(&mut l, 1, 0, t + 20);
    l.run(1500);
    let own = l.nodes[1].game().fishing.lines[1].clone();
    assert_eq!(own.stage, Stage::Reeling, "joiner 1 is reeling");
    for k in [0, 2] {
        let g = l.nodes[k].game();
        let seen = &g.fishing.lines[1];
        assert_eq!(seen.stage, Stage::Reeling, "browser {k} draws joiner 1's line");
        let d = seen.bobber.dist(own.bobber);
        assert!(
            d < 2000,
            "browser {k}: the bobber {d} mm from where joiner 1 has it"
        );
        let f = g.fishing.fish[0].at.dist(seen.bobber);
        assert!(f < 500, "browser {k}: the fish on it at its lure ({f} mm)");
        let view = g.fishing_view(k);
        assert!(
            view.lines.iter().any(|v| v.seat == 1),
            "browser {k}'s view has the line"
        );
        // and its own fish are left alone by it: no lure of joiner 1's here
        assert!(seen.interest.is_none());
    }
}

// ---- items, shared (`sim/items/online.rs`) ----

use crate::sim::items::Item;

/// Where the host's buoy is once it is up on every browser.
fn buoy_up(l: &mut Loop) -> crate::map::P {
    // the buoy's clock is every browser's; the buoy is the host's
    for n in l.nodes.iter_mut() {
        let t = n.game().tick;
        n.game_mut().items.next_buoy = t + 3;
    }
    assert!(
        l.run_until(3000, |l| l.nodes.iter().all(|n| n.game().items.buoy.is_some())),
        "the host's buoy reaches every browser"
    );
    let at = l.nodes[0].game().items.buoy.unwrap();
    for n in &l.nodes {
        assert_eq!(n.game().items.buoy, Some(at), "one buoy, the host's");
    }
    at
}

fn put_boat(l: &mut Loop, k: usize, at: crate::map::P) {
    let b = l.nodes[k].game_mut().boats[k].as_mut().unwrap();
    b.set_at(at);
    b.prev_um = b.pos_um;
    b.set_vel([0, 0]);
}

#[test]
fn the_host_spawns_the_buoy_and_one_boat_takes_it_on_every_screen() {
    for seed in [1, 2, 3] {
        let mut l = Loop::new(2, 0, LinkSpec::ROUGH, seed);
        l.lobby_and_start();
        l.run(500);
        let at = buoy_up(&mut l);
        // joiner 1 drives through it, joiner 2 a moment later
        put_boat(&mut l, 1, at);
        l.run(60);
        put_boat(&mut l, 2, at);
        l.run(3000);
        for (k, n) in l.nodes.iter().enumerate() {
            let g = n.game();
            assert_eq!(g.items.buoy, None, "seed {seed}: browser {k} has no buoy left");
            let picks: Vec<usize> = n
                .log
                .iter()
                .filter_map(|(_, e)| match e {
                    MatchEvent::ItemPicked { seat, .. } => Some(*seat),
                    _ => None,
                })
                .collect();
            assert_eq!(
                picks,
                vec![1],
                "seed {seed}: browser {k} hears one pickup, joiner 1's"
            );
            let spawned = n
                .log
                .iter()
                .filter(|(_, e)| matches!(e, MatchEvent::BuoySpawned { .. }))
                .count();
            assert_eq!(spawned, 1, "seed {seed}: browser {k} hears the buoy spawn once");
        }
        assert!(
            l.nodes[1].game().items.seats[1].held.is_some(),
            "seed {seed}: joiner 1 holds an item"
        );
        assert!(
            l.nodes[2].game().items.seats[2].held.is_none(),
            "seed {seed}: joiner 2 none"
        );
    }
}

/// Joiner `k` uses `item` now (its minigame done at once).
fn use_item(l: &mut Loop, k: usize, item: Item) {
    let m = l.nodes[k].game_mut();
    m.items.seats[k].held = Some(item);
    m.items.quick = true;
    let t = m.tick;
    let mut shift = SeatInput::default();
    shift.pressed.set("item", true);
    l.nodes[k].presses.push((t, shift));
}

#[test]
fn nitrous_and_the_cloak_show_on_every_screen() {
    let mut l = Loop::new(2, 0, LinkSpec::CLEAN, 4);
    l.lobby_and_start();
    l.run(500);
    use_item(&mut l, 1, Item::Nitrous);
    use_item(&mut l, 2, Item::Cloak);
    l.run(1000);
    for k in [0, 1, 2] {
        let g = l.nodes[k].game();
        assert!(
            g.boat(1).unwrap().nitrous,
            "browser {k}: joiner 1's boat has nitrous"
        );
        assert!(
            g.online.hidden.get(2).copied().unwrap_or(false),
            "browser {k}: joiner 2 is cloaked"
        );
        if k != 1 {
            assert!(
                l.nodes[k].log.iter().any(|(_, e)| *e
                    == MatchEvent::ItemUsed {
                        seat: 1,
                        item: Item::Nitrous
                    }),
                "browser {k} hears the nitrous"
            );
        }
    }
}

#[test]
fn a_teleports_destination_and_charge_reach_every_browser() {
    let mut l = Loop::new(2, 0, LinkSpec::CLEAN, 5);
    l.lobby_and_start();
    l.run(500);
    use_item(&mut l, 1, Item::Teleporter);
    l.run(200);
    assert!(
        l.nodes[1].game().items.seats[1].choosing,
        "joiner 1 picks a destination"
    );
    // a click on the radar, at the lake's middle
    let to_want = crate::map::P::new(80_000, 80_000);
    let (lo, side) = crate::fishboy::radar::region(&l.map, crate::fishboy::RADAR_MARGIN_MM);
    let r = crate::fishboy::radar::MAP_RECT;
    let x = r.x as i64 + (to_want.x - lo.x) as i64 * r.w as i64 / side as i64;
    let y = r.y as i64 + (lo.y as i64 + side as i64 - to_want.y as i64) * r.h as i64 / side as i64;
    let mut click = SeatInput::default();
    click.pressed.set("click", true);
    click.pointer = Some([x as i32, y as i32]);
    let t = l.nodes[1].game().tick;
    l.nodes[1].presses.push((t, click));
    l.run(500);
    let to = l.nodes[1].game().items.seats[1].charge.expect("charging").to;
    for k in [0, 2] {
        let g = l.nodes[k].game();
        let c = g.items.seats[1].charge.expect("the others see the charge");
        assert_eq!(c.to, to, "browser {k} knows where it goes");
        assert!(
            g.items.charge_permille(1, g.tick) > 0,
            "browser {k} draws the shimmer"
        );
    }
    l.run(6000);
    for k in [0, 1, 2] {
        let at = l.nodes[k].game().boat(1).unwrap().at();
        assert!(
            at.dist(to) < 1500,
            "browser {k}: joiner 1 is there ({} mm off)",
            at.dist(to)
        );
        assert!(
            l.nodes[k]
                .log
                .iter()
                .any(|(_, e)| *e == MatchEvent::Teleported { seat: 1 }),
            "browser {k} hears the teleport"
        );
    }
}
