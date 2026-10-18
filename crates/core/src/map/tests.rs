//! Map tests: generation, the validator on hand-drawn lakes, fish and the
//! buoy point. Expected values are worked out by hand from the setting's
//! numbers (4 m cells, channels 12 m, island gaps 20 m, the lane 16 m
//! either side), not read back from the code under test.

use super::fish::{self, Tier};
use super::validate::{check, Problem};
use super::{buoy, generate, list, Cell, Map, Setting, Spawn, P};

const CELL: i32 = 4000;

/// A hand-drawn map of `n` cells a side: `water(x, y)` per cell, the spawn
/// at the middle of cell (sx, sy) facing `heading`.
fn drawn(n: usize, water: impl Fn(i32, i32) -> bool, sx: i32, sy: i32, heading: i32) -> Map {
    let cells: Vec<bool> = (0..n * n)
        .map(|i| water((i % n) as i32, (i / n) as i32))
        .collect();
    let at = P::new(sx * CELL + CELL / 2, sy * CELL + CELL / 2);
    let spawn = Spawn {
        at,
        heading,
        dock: at,
    };
    Map::from_water(Setting::MichiganLake, 0, n, CELL, &cells, spawn)
}

/// An 80-cell map with a rectangle of water from (x0, y0) to (x1, y1)
/// inclusive, kept 12 cells (48 m) from the edge so the edge rule holds.
fn rect(x0: i32, y0: i32, x1: i32, y1: i32) -> impl Fn(i32, i32) -> bool {
    move |x, y| x >= x0 && x <= x1 && y >= y0 && y <= y1
}

fn has(map: &Map, f: impl Fn(&Problem) -> bool) -> bool {
    check(map).problems.iter().any(f)
}

#[test]
fn a_seed_always_makes_the_same_map_and_other_seeds_other_maps() {
    let a = generate(Setting::MichiganLake, 3);
    let b = generate(Setting::MichiganLake, 3);
    assert_eq!(a, b);
    let c = generate(Setting::MichiganLake, 4);
    assert_ne!(a.cells, c.cells, "seeds 3 and 4 made the same lake");
    assert_ne!(a.trees, c.trees);
}

#[test]
fn a_made_map_has_its_parts_where_they_belong() {
    let map = generate(Setting::MichiganLake, 0);
    let p = Setting::MichiganLake.params();
    assert_eq!(map.n, 192);
    assert_eq!(map.size_mm(), 768_000);
    // the spawn is on water, the dock on land
    assert!(map.wet(map.spawn.at));
    assert!(!map.wet(map.spawn.dock));
    // reeds stand in water within 14 m of the shore
    assert!(!map.reeds.is_empty());
    for patch in &map.reeds {
        for &c in &patch.clumps {
            let i = map.index(c).unwrap();
            assert!(map.cells[i].water(), "a reed clump on land at {c:?}");
            assert!(map.shore[i] <= 14_000, "a reed clump {} mm out", map.shore[i]);
        }
    }
    // trees grow on land, not right at the water's edge, and within reach
    assert!(map.trees.len() > 500);
    for t in &map.trees {
        let i = map.index(t.at).unwrap();
        assert!(!map.cells[i].water(), "a tree in the lake at {:?}", t.at);
        assert!((2500..=p.tree_reach_mm).contains(&map.shore[i]));
    }
    // island cells are the islands' and lie off the map's edge
    for i in 0..map.n * map.n {
        assert_eq!(map.cells[i] == Cell::Island, map.island[i] != 0);
        if map.cells[i].water() {
            assert!(map.ground[i] < 0);
            assert_eq!(map.cells[i] == Cell::Shallow, map.ground[i] >= -p.shallow_mm);
        }
    }
    assert!(!map.islands.is_empty());
}

#[test]
fn a_roomy_lake_passes_the_water_rules() {
    // 56 x 56 cells of water: one region, no narrow channel, spawn in the
    // middle of the west side facing east with 120 m of open water ahead
    let map = drawn(80, rect(12, 12, 67, 67), 20, 40, 0);
    let v = check(&map);
    assert_eq!(v.report.pockets, 0);
    assert_eq!(v.report.cut_off_cells, 0);
    let water_rules = |p: &Problem| {
        matches!(
            p,
            Problem::Pockets(_)
                | Problem::NarrowChannel { .. }
                | Problem::SpawnNotRoomy
                | Problem::SpawnCrowded
                | Problem::LaneBlocked { .. }
                | Problem::EdgeCut
        )
    };
    assert!(!v.problems.iter().any(water_rules), "{:?}", v.problems);
    // 56 * 56 cells of 16 m²
    assert_eq!(v.report.water_m2, 56 * 56 * 16);
    // a rectangle hides nothing
    assert_eq!(v.report.cover, 0);
    assert!(v.problems.contains(&Problem::TooOpen(0)));
}

#[test]
fn a_cut_off_pocket_is_rejected() {
    let lake = rect(12, 12, 50, 67);
    let map = drawn(80, |x, y| lake(x, y) || rect(55, 30, 62, 37)(x, y), 20, 40, 0);
    assert!(check(&map).problems.contains(&Problem::Pockets(1)));
}

#[test]
fn a_channel_too_narrow_for_a_boat_is_rejected() {
    // two basins, x 12..=35 and x 44..=67, joined at y 40.. by a channel
    let with_channel = |width: i32| {
        move |x: i32, y: i32| {
            (x <= 35 || x >= 44) && rect(12, 12, 67, 67)(x, y)
                || (x > 35 && x < 44 && y >= 40 && y < 40 + width)
        }
    };
    // 2 cells (8 m): every channel cell is 4 m from land, under the 8 m a
    // channel's middle needs, so the east basin (24 x 56 cells, less its
    // rim) is cut off
    let narrow = drawn(80, with_channel(2), 20, 40, 0);
    assert!(has(&narrow, |p| matches!(p, Problem::NarrowChannel { .. })));
    // 3 cells (12 m): the middle row is 8 m from land on both sides
    let wide = drawn(80, with_channel(3), 20, 40, 0);
    assert!(!has(&wide, |p| matches!(p, Problem::NarrowChannel { .. })));
}

#[test]
fn the_spawn_needs_room_and_an_open_lane() {
    // land 2 cells (8 m) north of the spawn: within the 12 m kept clear
    let crowded = drawn(80, rect(12, 12, 67, 67), 30, 66, 0);
    assert!(has(&crowded, |p| *p == Problem::SpawnCrowded));
    // facing east from x = 20 on a lake ending at x = 50: land starts at
    // x = 51; x = 48 is 3 cells (12 m) from it, under the lane's 16 m, so
    // the lane fails 28 cells ahead: 112 000 mm
    let short = drawn(80, rect(12, 12, 50, 67), 20, 40, 0);
    assert!(check(&short)
        .problems
        .contains(&Problem::LaneBlocked { at_mm: 112_000 }));
    let long = drawn(80, rect(12, 12, 67, 67), 20, 40, 0);
    assert!(!has(&long, |p| matches!(p, Problem::LaneBlocked { .. })));
}

#[test]
fn islands_keep_clear_of_the_shore_and_each_other() {
    // island A: x 15..=22, y 50..=57; the shore is x <= 11, so x 12..=14 is
    // 3 cells (12 m) of water between them
    // island B: x 40..=47, y 30..=37, 6 cells (24 m) or more from all land
    let lake = rect(12, 12, 67, 67);
    let a = rect(15, 50, 22, 57);
    let b = rect(40, 30, 47, 37);
    let map = drawn(80, |x, y| lake(x, y) && !a(x, y) && !b(x, y), 30, 20, 0);
    assert_eq!(map.islands.len(), 2);
    let v = check(&map);
    let touching: Vec<&Problem> = v
        .problems
        .iter()
        .filter(|p| matches!(p, Problem::IslandTouches { .. }))
        .collect();
    let a_id = map.island[(52 * 80 + 18) as usize];
    assert_eq!(
        touching,
        vec![&Problem::IslandTouches {
            island: a_id,
            gap_mm: 12_000
        }]
    );
    // both are 64 cells: 1024 m², big enough to hide behind
    assert_eq!(v.report.hiding_islands, 2);
}

#[test]
fn a_lake_full_of_land_is_too_cramped() {
    // pillars of land 2 cells wide every 6 cells cut most views
    let lake = rect(12, 12, 67, 67);
    let map = drawn(80, |x, y| lake(x, y) && !((x % 6 < 2) && (y % 6 < 2)), 20, 40, 0);
    assert!(
        has(&map, |p| matches!(p, Problem::TooCramped(_))),
        "{:?}",
        check(&map).report
    );
}

#[test]
fn every_accepted_seed_of_a_scan_passes_and_rejected_ones_say_why() {
    let (maps, rejected) = list::scan(Setting::MichiganLake, 0, 12);
    assert_eq!(maps.maps.len() + rejected.len(), 12);
    assert!(!maps.maps.is_empty() && !rejected.is_empty());
    for e in &maps.maps {
        assert!(check(&generate(Setting::MichiganLake, e.seed)).ok());
    }
    for (_, problems) in &rejected {
        assert!(!problems.is_empty());
    }
}

#[test]
fn a_match_seed_picks_a_listed_map() {
    let list = list::parse(
        r#"{"setting": "michigan-lake", "from": 0, "scanned": 20,
        "maps": [{"seed": 5, "stamp": "x"}, {"seed": 9, "stamp": "y"}, {"seed": 12, "stamp": "z"}]}"#,
    )
    .unwrap();
    let mut seen = [0; 3];
    for m in 0..300u64 {
        let s = list::pick(&list, m);
        let k = [5, 9, 12]
            .iter()
            .position(|&x| x == s)
            .expect("a seed not in the list");
        seen[k] += 1;
    }
    // each about 100 times in 300
    assert!(seen.iter().all(|&c| c > 60), "{seen:?}");
    assert!(list::parse(
        r#"{"setting": "mars", "from": 0, "scanned": 1, "maps": [{"seed": 1, "stamp": "x"}]}"#
    )
    .is_err());
    assert!(list::parse(r#"{"setting": "michigan-lake", "from": 0, "scanned": 1, "maps": []}"#).is_err());
}

fn placed(seed: u32) -> (Map, Vec<fish::Fish>) {
    let map = generate(Setting::MichiganLake, seed);
    let f = fish::place(&map, &fish::EXAMPLE_COUNTS, 77).expect("the example counts fit");
    (map, f)
}

#[test]
fn the_fish_counts_are_exact_per_kind() {
    let (_, f) = placed(0);
    assert_eq!(f.len(), 65);
    for &(id, count) in &fish::EXAMPLE_COUNTS {
        assert_eq!(f.iter().filter(|x| x.kind == id).count() as u32, count, "{id}");
    }
    let tiers = |t: Tier| f.iter().filter(|x| x.tier == t).count();
    assert_eq!((tiers(Tier::Common), tiers(Tier::Uncommon)), (30, 20));
    assert_eq!((tiers(Tier::Rare), tiers(Tier::Legendary)), (10, 5));
    // other counts are inputs too
    let (map, _) = placed(0);
    let few = fish::place(&map, &[("walleye", 2), ("muskellunge", 1)], 5).unwrap();
    assert_eq!(few.len(), 3);
    assert!(fish::place(&map, &[("bluegill", 1)], 5).is_err());
    assert!(fish::place(&map, &[("goldfish", 4)], 5).is_err());
}

#[test]
fn schools_hold_two_to_four_of_one_kind_close_together() {
    for seed in [0, 3, 8] {
        let (_, f) = placed(seed);
        let mut schools: Vec<u32> = f.iter().filter(|x| x.tier.schools()).map(|x| x.school).collect();
        schools.dedup();
        assert!(schools.len() >= 50 / 4);
        for s in schools {
            let members: Vec<&fish::Fish> = f.iter().filter(|x| x.school == s).collect();
            assert!((2..=4).contains(&members.len()), "a school of {}", members.len());
            assert!(members.iter().all(|m| m.kind == members[0].kind));
            // all within 6 m of the school's middle: at most 12 m apart
            for a in &members {
                for b in &members {
                    assert!(a.at.dist2(b.at) <= 12_000i64 * 12_000, "{:?} {:?}", a.at, b.at);
                }
            }
        }
    }
}

#[test]
fn rare_and_legendary_fish_are_single_and_far_apart() {
    for seed in [0, 3, 8] {
        let (map, f) = placed(seed);
        let singles: Vec<&fish::Fish> = f.iter().filter(|x| !x.tier.schools()).collect();
        assert_eq!(singles.len(), 15);
        for (k, a) in singles.iter().enumerate() {
            assert_eq!(f.iter().filter(|x| x.school == a.school).count(), 1);
            // 100 m from the spawn
            assert!(a.at.dist2(map.spawn.at) >= 100_000i64 * 100_000);
            for b in &singles[k + 1..] {
                assert!(a.at.dist2(b.at) >= 60_000i64 * 60_000, "{:?} {:?}", a.at, b.at);
            }
        }
    }
}

#[test]
fn fish_swim_at_their_tiers_depths_over_deep_enough_water() {
    let (map, f) = placed(0);
    for x in &f {
        // common 0.5-3 m over a bed of at most 6 m; uncommon 2-6 m over at
        // most 11 m; rare 3-9 m; legendary 6-14 m; always 0.3 m off the bed
        let (lo, hi, deepest_bed) = match x.tier {
            Tier::Common => (500, 3000, 6000),
            Tier::Uncommon => (2000, 6000, 11_000),
            Tier::Rare => (3000, 9000, i32::MAX),
            Tier::Legendary => (6000, 14_000, i32::MAX),
        };
        assert!((lo..=hi).contains(&x.depth_mm), "{} at {} mm", x.kind, x.depth_mm);
        let bed = -map.ground[map.index(x.at).unwrap()];
        assert!(
            x.depth_mm <= bed - 300,
            "{} at {} mm over {} mm",
            x.kind,
            x.depth_mm,
            bed
        );
        assert!(bed <= deepest_bed, "{} over {} mm", x.kind, bed);
    }
}

#[test]
fn the_buoy_is_about_as_far_by_boat_from_every_player() {
    // open water: trips are straight lines, so check with plain distance.
    // The core's tolerance is 8 m or 5% of the mean trip; its grid trips
    // differ from straight lines by up to 2% and half a cell, so allow
    // 12 m plus 9% of the mean here.
    let map = drawn(120, rect(12, 12, 107, 107), 60, 60, 0);
    // two players; three (they always have a point equally far from all);
    // four on a circle of 150 m round (240 m, 240 m), at 0, 70, 160 and 250
    // degrees (cos and sin worked by hand). Four players anywhere else may
    // have no fair point at all, and then the fairest point wins.
    let cases: Vec<Vec<P>> = vec![
        vec![P::new(100_000, 100_000), P::new(380_000, 300_000)],
        vec![
            P::new(100_000, 100_000),
            P::new(380_000, 120_000),
            P::new(200_000, 380_000),
        ],
        vec![
            P::new(390_000, 240_000),
            P::new(291_303, 380_954),
            P::new(99_046, 291_303),
            P::new(188_697, 99_046),
        ],
    ];
    for players in &cases {
        for seed in 0..4 {
            let b = buoy::fair_point(&map, players, seed).unwrap();
            assert!(map.wet(b));
            let d: Vec<i64> = players.iter().map(|p| p.dist(b)).collect();
            let (lo, hi) = (*d.iter().min().unwrap(), *d.iter().max().unwrap());
            let mean = d.iter().sum::<i64>() / d.len() as i64;
            assert!(hi - lo <= 12_000 + mean * 9 / 100, "trips {d:?}");
            assert!(lo >= 60_000);
        }
    }
    // everyone at the spawn: a trip of about 150 m
    let s = P::new(100_000, 240_000);
    let b = buoy::fair_point(&map, &[s, s, s], 1).unwrap();
    let d = s.dist(b);
    assert!((130_000..=165_000).contains(&d), "{d}");
}
