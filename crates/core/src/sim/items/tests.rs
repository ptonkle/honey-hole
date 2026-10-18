//! The items: the buoy's clock and fair point, the pickup, Shift and Esc,
//! and each effect. Expected values are worked from the sample tuning
//! (`tuning::tests::sample`, `items_sample`): a buoy every 60 s is 3000
//! ticks, its warning 5 s (250 ticks) before, beats a second (50 ticks)
//! apart; a 60 s cloak is 3000 ticks, a 20 s Power Reel 1000, 15 s of
//! nitrous 750, the teleport's 5 s charge 250.

use super::*;
use crate::map::fish::Tier;
use crate::map::{generate, grid, Map, Setting, Spawn};
use crate::sim::fishing::lake::{FishState, LakeFish};
use crate::sim::{Catch, Occupant, Setup};
use crate::tuning::{self, Tuning};

fn sample() -> Tuning {
    tuning::tests::sample()
}

fn setup(t: &Tuning, humans: usize) -> Setup {
    let mut s = Setup::new(t);
    for k in 0..humans {
        s = s.with(k, Occupant::Human, &format!("P{k}"));
    }
    s.seed = 7;
    s
}

/// A square lake `n` cells of 4 m a side, land round the edge and in the
/// columns from `wall` east; the spawn at `at`, facing east.
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
    Map::from_water(Setting::MichiganLake, 0, n, 4000, &water, spawn)
}

fn small(humans: usize) -> Match {
    let t = sample();
    Match::new(lake(40, None, P::new(60_000, 80_000)), &t, setup(&t, humans)).unwrap()
}

fn press(name: &str) -> SeatInput {
    let mut i = SeatInput::default();
    i.pressed.set(name, true);
    i
}

fn idle(m: &mut Match, ticks: u32) -> Vec<(u32, MatchEvent)> {
    let mut out = Vec::new();
    for _ in 0..ticks {
        let tick = m.tick + 1;
        for e in m.step(&[]) {
            out.push((tick, e.clone()));
        }
    }
    out
}

/// Seat `seat` presses Shift for one tick; the others do nothing.
fn shift(m: &mut Match, seat: usize) -> Vec<MatchEvent> {
    let mut inputs = vec![SeatInput::default(); m.seats.len()];
    inputs[seat] = press("item");
    m.step(&inputs).to_vec()
}

/// Gives a seat an item through the buoy, by driving onto it.
fn give(m: &mut Match, seat: usize, item: Item) {
    m.items.force_next = Some(item);
    let at = m.boat(seat).unwrap().at();
    m.place_buoy(at);
    m.step(&[]);
    assert_eq!(m.items.seats[seat].held, Some(item));
}

#[test]
fn the_buoy_spawns_at_the_interval_after_its_warning_and_five_beats() {
    let t = sample();
    let map = generate(Setting::MichiganLake, 1);
    let mut m = Match::new(map, &t, setup(&t, 2)).unwrap();
    let ev = idle(&mut m, 3000);
    let at = |want: &dyn Fn(&MatchEvent) -> bool| -> Vec<u32> {
        ev.iter().filter(|(_, e)| want(e)).map(|(k, _)| *k).collect()
    };
    assert_eq!(at(&|e| *e == MatchEvent::BuoyWarning), [2750]);
    let beats: Vec<(u32, u32)> = ev
        .iter()
        .filter_map(|(k, e)| match e {
            MatchEvent::BuoyBeat { left } => Some((*k, *left)),
            _ => None,
        })
        .collect();
    assert_eq!(beats, [(2800, 5), (2850, 4), (2900, 3), (2950, 2), (3000, 1)]);
    assert_eq!(at(&|e| matches!(e, MatchEvent::BuoySpawned { .. })), [3000]);
    let b = m.items.buoy.expect("a buoy in the lake");
    assert!(m.map.wet(b));
    assert_eq!(m.radar_view(1).buoy, Some(b), "it shows on every radar");
    // the next a minute on
    assert_eq!(m.items.next_buoy, 6000);
}

#[test]
fn the_buoy_lies_a_fair_trip_from_every_player() {
    let t = sample();
    let map = generate(Setting::MichiganLake, 1);
    let mut m = Match::new(map, &t, setup(&t, 3)).unwrap();
    // one boat 40 m on across the lake from the others
    let spawn = m.map.spawn;
    let away = crate::sim::fishing::lake::ahead(spawn.at, spawn.heading, 40_000);
    assert!(m.map.wet(away));
    m.boats[2].as_mut().unwrap().set_at(away);
    m.items.next_buoy = m.tick + 1;
    m.step(&[]);
    let b = m.items.buoy.unwrap();
    let water: Vec<bool> = m.map.cells.iter().map(|c| c.water()).collect();
    let cell = m.map.index(b).unwrap();
    let trips: Vec<i32> = (0..3)
        .map(|s| {
            let from = m.map.index(m.boat(s).unwrap().at()).unwrap();
            grid::distances(m.map.n, m.map.cell_mm, &[from], &water)[cell]
        })
        .collect();
    let (lo, hi) = (*trips.iter().min().unwrap(), *trips.iter().max().unwrap());
    let mean = trips.iter().sum::<i32>() / 3;
    // the stated tolerance: 8 m, or 5% of the mean trip when that is more
    assert!(hi - lo <= (mean / 20).max(8000), "trips {trips:?}");
    assert!(lo >= 60_000, "not handed to anyone: {trips:?}");
}

#[test]
fn an_unclaimed_buoy_moves_when_the_next_is_due() {
    let mut m = small(1);
    m.place_buoy(P::new(100_000, 100_000));
    m.items.next_buoy = m.tick + 1;
    // in the 160 m lake no point is 60 m from the boat and open: no fair
    // point, so the old buoy is taken away rather than left
    let ev = m.step(&[]).to_vec();
    assert_ne!(m.items.buoy, Some(P::new(100_000, 100_000)), "{ev:?}");
    // and kept when the tuning says it stays
    let mut m = small(1);
    m.items.tuning.unclaimed_moves = false;
    m.place_buoy(P::new(100_000, 100_000));
    m.items.next_buoy = m.tick + 1;
    m.step(&[]);
    assert_eq!(m.items.buoy, Some(P::new(100_000, 100_000)));
}

#[test]
fn driving_through_the_buoy_gives_an_item_and_a_new_one_replaces_it() {
    let mut m = small(2);
    m.items.force_next = Some(Item::Nitrous);
    // 2.9 m from the boat's middle: inside the 3 m pickup
    let at = m.boat(1).unwrap().at();
    m.place_buoy(P::new(at.x + 2900, at.y));
    let ev = m.step(&[]).to_vec();
    assert!(ev.contains(&MatchEvent::ItemPicked {
        seat: 1,
        item: Item::Nitrous,
        dropped: None
    }));
    assert_eq!(m.items.buoy, None, "one buoy, taken");
    m.items.force_next = Some(Item::Cloak);
    m.place_buoy(at);
    let ev = m.step(&[]).to_vec();
    assert!(ev.contains(&MatchEvent::ItemPicked {
        seat: 1,
        item: Item::Cloak,
        dropped: Some(Item::Nitrous)
    }));
    assert_eq!(m.items.seats[1].held, Some(Item::Cloak));
    // 3.1 m off: no one takes it
    m.place_buoy(P::new(at.x + 3100, at.y));
    m.step(&[]);
    assert!(m.items.buoy.is_some());
}

#[test]
fn the_pickup_claim_goes_to_the_nearest_boat_and_the_lower_seat_on_a_tie() {
    let b = Some(P::new(0, 0));
    let boats = [(2, P::new(1000, 0)), (1, P::new(0, 900)), (0, P::new(0, 900))];
    assert_eq!(claims::pickup(b, &boats, 3000), Some(0));
    assert_eq!(claims::pickup(b, &boats, 800), None);
    assert_eq!(claims::pickup(None, &boats, 3000), None);
}

#[test]
fn items_are_drawn_by_their_weights() {
    // weights 1, 0, 2, 0, 0, 1: rolls 0 / 1-2 / 3 of 4
    let w = |i: Item| match i {
        Item::Cloak => 1,
        Item::Nitrous => 2,
        Item::EnhancedRadar => 1,
        _ => 0,
    };
    let got: Vec<Item> = (0..5).map(|r| claims::draw(w, r)).collect();
    assert_eq!(
        got,
        [
            Item::Cloak,
            Item::Nitrous,
            Item::Nitrous,
            Item::EnhancedRadar,
            Item::Cloak
        ]
    );
}

#[test]
fn shift_starts_the_minigame_the_boat_coasts_and_esc_wastes_the_item() {
    let mut m = small(1);
    give(&mut m, 0, Item::PowerReel);
    let ev = shift(&mut m, 0);
    assert!(ev.contains(&MatchEvent::MinigameStarted {
        seat: 0,
        item: Item::PowerReel
    }));
    assert_eq!(m.items.seats[0].held, None, "spent when used");
    assert!(m.boat(0).unwrap().captured, "the boat coasts");
    let ev = m.step(&[press("cancel")]).to_vec();
    assert!(ev.contains(&MatchEvent::ItemWasted {
        seat: 0,
        item: Item::PowerReel
    }));
    assert!(!ev.iter().any(|e| matches!(e, MatchEvent::MenuOpened { .. })));
    assert!(m.items.seats[0].playing.is_none());
    assert!(!m.boat(0).unwrap().captured);
    assert_eq!(m.fishing.reel_power_pct[0], 100, "no effect");
    // Shift with nothing held
    let ev = shift(&mut m, 0);
    assert!(!ev.iter().any(|e| matches!(e, MatchEvent::MinigameStarted { .. })));
}

#[test]
fn a_fishboy_minigame_raises_the_fishboy_and_tab_cannot_lower_it() {
    let mut m = small(1);
    give(&mut m, 0, Item::Cloak);
    shift(&mut m, 0);
    assert!(m.seats[0].fishboy);
    m.step(&[press("fishboy")]);
    assert!(m.seats[0].fishboy, "Tab is ignored");
    m.step(&[press("cancel")]);
    assert!(!m.seats[0].fishboy, "Esc wastes it and puts it down");
}

/// Uses an item with its minigame done at once.
fn use_now(m: &mut Match, seat: usize, item: Item) -> Vec<MatchEvent> {
    give(m, seat, item);
    m.items.quick = true;
    shift(m, seat)
}

#[test]
fn the_cloak_lasts_a_minute_and_wipes_its_blip() {
    let mut m = small(2);
    // seat 0 pings and sees seat 1
    m.set_fishboy(0, true);
    let mut i = vec![SeatInput::default(); 2];
    i[0] = press("ping");
    m.step(&i);
    idle(&mut m, 160);
    assert!(m.seats[0].ping.sightings[1].is_some());
    let ev = use_now(&mut m, 1, Item::Cloak);
    let from = m.tick;
    assert!(ev.contains(&MatchEvent::ItemUsed {
        seat: 1,
        item: Item::Cloak
    }));
    assert!(m.seats[0].ping.sightings[1].is_none(), "the old blip is wiped");
    assert!(m.radar_view(0).boats[1].cloaked);
    idle(&mut m, 2999);
    assert!(m.items.cloaked(1, m.tick));
    let ev = idle(&mut m, 1);
    assert_eq!(m.tick, from + 3000);
    assert!(!m.items.cloaked(1, m.tick));
    assert!(ev.iter().any(|(_, e)| *e == MatchEvent::CloakEnded { seat: 1 }));
}

#[test]
fn a_cloaked_seat_does_not_show_on_another_seats_ping() {
    let mut m = small(3);
    use_now(&mut m, 1, Item::Cloak);
    m.set_fishboy(0, true);
    let mut i = vec![SeatInput::default(); 3];
    i[0] = press("ping");
    m.step(&i);
    idle(&mut m, 160);
    let s = &m.seats[0].ping.sightings;
    assert!(s[1].is_none(), "cloaked: unseen");
    assert!(s[2].is_some(), "the uncloaked one is seen");
}

#[test]
fn the_power_reel_doubles_the_reel_for_twenty_seconds() {
    let mut m = small(1);
    use_now(&mut m, 0, Item::PowerReel);
    assert_eq!(m.fishing.reel_power_pct[0], 200);
    idle(&mut m, 999);
    assert_eq!(m.fishing.reel_power_pct[0], 200);
    let ev = idle(&mut m, 1);
    assert_eq!(m.fishing.reel_power_pct[0], 100);
    assert!(ev.iter().any(|(_, e)| *e
        == MatchEvent::EffectEnded {
            seat: 0,
            item: Item::PowerReel
        }));
}

#[test]
fn nitrous_lasts_fifteen_seconds_and_raises_top_speed_to_150_percent() {
    let mut m = small(1);
    use_now(&mut m, 0, Item::Nitrous);
    assert!(m.boat(0).unwrap().nitrous);
    assert_eq!(m.boat(0).unwrap().top(&m.rules), 24_000);
    idle(&mut m, 749);
    assert!(m.boat(0).unwrap().nitrous);
    idle(&mut m, 1);
    assert!(!m.boat(0).unwrap().nitrous);
    assert_eq!(m.boat(0).unwrap().top(&m.rules), 16_000);
}

/// Seat 0 on nitrous heading east at `speed` into seat 1, broadside, as
/// the match core's own ram test sets it up.
fn nitrous_ram(speed: i32) -> Match {
    let mut m = small(2);
    use_now(&mut m, 0, Item::Nitrous);
    let a = m.boats[0].as_mut().unwrap();
    a.set_at(P::new(60_000, 80_000));
    a.heading = 0;
    a.set_vel([speed, 0]);
    let b = m.boats[1].as_mut().unwrap();
    b.set_at(P::new(60_000 + 1000 + 1800, 80_000));
    b.heading = crate::map::trig::TURN / 4;
    m
}

#[test]
fn nitrous_does_not_change_the_ram_threshold() {
    // 80% of the plain 16 m/s is 12.8 m/s; with nitrous on it would be
    // 19.2 m/s, and a ram at 12.8 still wrecks
    let mut m = nitrous_ram(12_800);
    m.step(&[]);
    assert!(m.boat(1).unwrap().wrecked());
    let mut m = nitrous_ram(12_640);
    m.step(&[]);
    assert!(!m.boat(1).unwrap().wrecked());
}

/// The radar screen's point (x, y units) over a map point.
fn on_radar(m: &Match, p: P) -> [i32; 2] {
    let (lo, side) = radar::region(&m.map, crate::fishboy::RADAR_MARGIN_MM);
    let r = radar::MAP_RECT;
    let x = r.x as i64 + (p.x - lo.x) as i64 * r.w as i64 / side as i64;
    let y = r.y as i64 + (lo.y as i64 + side as i64 - p.y as i64) * r.h as i64 / side as i64;
    [x as i32, y as i32]
}

#[test]
fn a_teleport_lands_in_open_water_after_its_five_second_charge() {
    let mut m = small(1);
    let ev = use_now(&mut m, 0, Item::Teleporter);
    assert!(ev.contains(&MatchEvent::TeleportChoosing { seat: 0 }));
    assert!(m.seats[0].fishboy, "the radar comes up to choose");
    // a click on the land in the lake's corner snaps to open water
    let mut click = press("click");
    click.pointer = Some(on_radar(&m, P::new(2000, 2000)));
    let start = m.boat(0).unwrap().at();
    m.boats[0].as_mut().unwrap().set_vel([2000, 0]);
    let ev = m.step(&[click]).to_vec();
    assert!(ev.contains(&MatchEvent::ItemUsed {
        seat: 0,
        item: Item::Teleporter
    }));
    let to = m.items.seats[0].charge.unwrap().to;
    let i = m.map.index(to).unwrap();
    assert!(m.map.cells[i].water() && m.map.shore[i] >= 6000, "{to:?}");
    idle(&mut m, 249);
    assert_ne!(m.boat(0).unwrap().at(), to);
    assert!(
        m.boat(0).unwrap().at().dist(start) > 0,
        "it can still move while charging"
    );
    let ev = idle(&mut m, 1);
    assert!(ev.iter().any(|(_, e)| *e == MatchEvent::Teleported { seat: 0 }));
    let b = m.boat(0).unwrap();
    assert_eq!(b.at(), to);
    assert_eq!(b.speed_mm_per_s(), 0, "at rest");
}

#[test]
fn a_click_on_land_snaps_to_the_nearest_open_water() {
    let map = lake(40, Some(20), P::new(40_000, 80_000));
    // land from x = 80 m east: the cell by it (middle 78 m) is 4 m from
    // land between middles, the next (74 m) 8 m; of the two rows level
    // with 80 m north, the lower cell
    let p = claims::teleport_target(&map, P::new(120_000, 80_000), 6000).unwrap();
    assert_eq!(p, P::new(74_000, 78_000));
    let open = P::new(40_000, 80_000);
    assert_eq!(claims::teleport_target(&map, open, 6000), Some(open));
}

fn caught(m: &mut Match, seat: usize, value: i32) {
    m.scores[seat].catches.push(Catch {
        kind: "bluegill".into(),
        length_mm: 200,
        value,
    });
    m.events.push(MatchEvent::Caught {
        seat,
        fish: 0,
        kind: "bluegill".into(),
        tier: Tier::Common,
        length_mm: 200,
    });
    m.items_after_fishing();
}

#[test]
fn the_camera_is_offered_after_a_catch_only_with_one_held() {
    let mut m = small(1);
    caught(&mut m, 0, 100);
    assert_eq!(m.items.seats[0].offer_until, None, "nothing held");
    give(&mut m, 0, Item::Nitrous);
    caught(&mut m, 0, 100);
    assert_eq!(m.items.seats[0].offer_until, None, "another item held");
    give(&mut m, 0, Item::Camera);
    let ev = shift(&mut m, 0);
    assert!(
        ev.iter().any(|e| matches!(e, MatchEvent::ItemRefused { .. })),
        "not with Shift alone"
    );
    caught(&mut m, 0, 100);
    assert!(m.events.contains(&MatchEvent::CameraOffered { seat: 0 }));
    // Esc: save for later
    m.step(&[press("cancel")]);
    assert_eq!(m.items.seats[0].offer_until, None);
    assert_eq!(m.items.seats[0].held, Some(Item::Camera));
    // the next catch: Shift uses it, and the catch is worth 1.5 times
    caught(&mut m, 0, 100);
    m.items.quick = true;
    shift(&mut m, 0);
    assert_eq!(m.items.seats[0].held, None);
    assert_eq!(m.scores[0].catches.last().unwrap().value, 150);
}

#[test]
fn the_camera_offer_times_out_and_keeps_the_camera() {
    let mut m = small(1);
    give(&mut m, 0, Item::Camera);
    caught(&mut m, 0, 100);
    // 8 s is 400 ticks
    idle(&mut m, 399);
    assert!(m.items.seats[0].offer_until.is_some());
    let ev = idle(&mut m, 1);
    assert!(ev.iter().any(|(_, e)| *e == MatchEvent::CameraSaved { seat: 0 }));
    assert_eq!(m.items.seats[0].held, Some(Item::Camera));
}

fn fish(id: usize, tier: Tier, at: P) -> LakeFish {
    LakeFish {
        id: id as u32,
        kind: 0,
        tier,
        length_mm: 300,
        at,
        depth_mm: 1000,
        home_depth_mm: 1000,
        heading: 0,
        school: 0,
        offset: P::new(0, 0),
        state: FishState::Roaming,
        phase_ms: 0,
    }
}

#[test]
fn enhanced_radar_pings_and_marks_the_nearest_fish_of_each_tier() {
    let mut m = small(1);
    let me = m.boat(0).unwrap().at();
    let off = |dx: i32| P::new(me.x + dx, me.y);
    let mut caught = fish(6, Tier::Legendary, off(1000));
    caught.state = FishState::Caught { seat: 0 };
    m.fishing.fish = vec![
        fish(0, Tier::Common, off(9000)),
        fish(1, Tier::Common, off(5000)),
        fish(2, Tier::Uncommon, off(-7000)),
        fish(3, Tier::Rare, off(20_000)),
        fish(4, Tier::Legendary, off(30_000)),
        fish(5, Tier::Legendary, off(40_000)),
        caught,
    ];
    m.fishing.schools = vec![crate::sim::fishing::lake::School {
        at: me,
        home: me,
        target: me,
        rest_until: u32::MAX,
    }];
    let ev = use_now(&mut m, 0, Item::EnhancedRadar);
    assert!(ev.contains(&MatchEvent::Pinged { seat: 0 }));
    assert!(ev.contains(&MatchEvent::Revealed { seat: 0, count: 4 }));
    let got: Vec<(usize, Tier)> = m.items.seats[0].marks.iter().map(|k| (k.fish, k.tier)).collect();
    assert_eq!(
        got,
        [
            (4, Tier::Legendary),
            (3, Tier::Rare),
            (2, Tier::Uncommon),
            (1, Tier::Common)
        ]
    );
    // marked until the next ping, after the 5 s cooldown
    idle(&mut m, 260);
    assert_eq!(m.items.seats[0].marks.len(), 4);
    m.step(&[press("ping")]);
    assert!(m.items.seats[0].marks.is_empty());
}

#[test]
fn a_wreck_loses_the_teleports_charge() {
    let mut m = small(1);
    use_now(&mut m, 0, Item::Teleporter);
    let mut click = press("click");
    click.pointer = Some(on_radar(&m, P::new(100_000, 100_000)));
    m.step(&[click]);
    assert!(m.items.seats[0].charge.is_some());
    let tick = m.tick;
    m.wreck(0, crate::sim::WreckCause::Grounded, tick);
    assert!(m.items.seats[0].charge.is_none());
    assert!(m.events.contains(&MatchEvent::ItemWasted {
        seat: 0,
        item: Item::Teleporter
    }));
}
