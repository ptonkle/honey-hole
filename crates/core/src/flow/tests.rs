//! The match's flow: the countdown, popups, the postgame's reveal and
//! ranking, Rematch, the map seed's option, a wreck's burst and the
//! ghosting. Expected values are worked by hand from the sample tuning
//! (W adds 16000 × 20 / 5000 = 64 mm/s a tick) and from the shipped
//! `data/flow.json` and `data/strings.json`.

use serde_json::{json, Value};

use super::look::FlowLook;
use super::popups::{of_events, Names, Popup, Popups};
use super::postgame::{columns, places, ranking, Column, Reveal};
use super::*;
use crate::app::play::tests as play;
use crate::map::fish::Tier;
use crate::map::{generate, Setting};
use crate::sim::{MatchEvent, Score, SeatInput, Setup, WreckCause};

const FLOW: &str = include_str!("../../../../data/flow.json");
const UI2D: &str = include_str!("../../../../data/ui2d.json");
const FISH: &str = include_str!("../../../../data/fish.json");
const STRINGS: &str = include_str!("../../../../data/strings.json");

fn look() -> FlowLook {
    FlowLook::parse(FLOW).unwrap()
}

fn catch(kind: &str, value: i32) -> Catch {
    Catch {
        kind: kind.into(),
        length_mm: 300,
        value,
    }
}

// ---- the countdown ----

#[test]
fn the_countdown_holds_the_boats_still_then_lets_them_go() {
    let t = crate::tuning::tests::sample();
    let mut s = Setup::new(&t).with(0, Occupant::Human, "Rosie");
    s.countdown_ms = 3000;
    let mut m = Match::new(generate(Setting::MichiganLake, 0), &t, s).unwrap();
    // 3 s of 20 ms ticks
    assert_eq!(m.countdown, 150);
    let start = m.boat(0).unwrap().at();
    let mut w = SeatInput::default();
    w.held.forward = true;
    let mut beats = Vec::new();
    for k in 1..=150 {
        for e in m.step(&[w]) {
            beats.push((k, e.clone()));
        }
    }
    assert_eq!(
        m.boat(0).unwrap().forward_speed(),
        0,
        "W does nothing in the countdown"
    );
    assert_eq!(m.boat(0).unwrap().at(), start);
    assert_eq!(m.tick, 0, "the clock waits");
    assert_eq!(m.remaining_ms(), 480_000);
    assert_eq!(
        beats,
        vec![
            (1, MatchEvent::Countdown { left: 3 }),
            (51, MatchEvent::Countdown { left: 2 }),
            (101, MatchEvent::Countdown { left: 1 }),
            (150, MatchEvent::MatchStarted),
        ]
    );
    m.step(&[w]);
    assert_eq!(m.tick, 1);
    assert_eq!(m.boat(0).unwrap().forward_speed(), 64, "after Go, W drives");
}

#[test]
fn a_match_set_up_without_a_countdown_starts_at_once() {
    let t = crate::tuning::tests::sample();
    let s = Setup::new(&t).with(0, Occupant::Human, "Rosie");
    let mut m = Match::new(generate(Setting::MichiganLake, 0), &t, s).unwrap();
    assert_eq!(m.countdown, 0);
    assert!(m.step(&[]).is_empty(), "no beat and no Go");
    assert_eq!(m.tick, 1);
}

#[test]
fn the_match_setup_takes_the_countdown_from_the_tuning() {
    let mut a = play::lobby();
    let mut t = a.tuning.clone().unwrap();
    t.flow.countdown_ms = 2000;
    a.tuning = Some(t);
    a.act("lobby.start", "").unwrap();
    assert_eq!(a.play.as_ref().unwrap().game.countdown, 100);
}

// ---- popups ----

fn popup(text: &str, group: &str) -> Popup {
    Popup {
        key: String::new(),
        text: text.into(),
        style: "plain".into(),
        tier: None,
        group: group.into(),
        shown_us: None,
    }
}

fn shown(p: &Popups) -> Vec<String> {
    p.shown().map(|q| q.text.clone()).collect()
}

const S: i64 = 1_000_000;

#[test]
fn popups_queue_up_and_each_goes_after_its_two_seconds() {
    let mut p = Popups::default();
    for t in ["a", "b", "c"] {
        p.push(popup(t, ""), 0);
    }
    // room for two: the third waits
    p.advance(0, 2000, 2);
    assert_eq!(shown(&p), ["a", "b"]);
    p.advance(2 * S - 1, 2000, 2);
    assert_eq!(shown(&p), ["a", "b"], "still up just before 2 s");
    p.advance(2 * S, 2000, 2);
    assert_eq!(shown(&p), ["c"], "a and b go at 2 s; c shows");
    p.advance(4 * S - 1, 2000, 2);
    assert_eq!(shown(&p), ["c"], "c's 2 s started when it showed");
    p.advance(4 * S, 2000, 2);
    assert!(p.queue.is_empty());
}

#[test]
fn a_popup_in_a_group_replaces_the_one_there_and_starts_again() {
    let mut p = Popups::default();
    p.push(popup("first", ""), 0);
    p.push(popup("buoy in 5", "buoy"), 0);
    p.advance(0, 2000, 4);
    p.push(popup("buoy in 4", "buoy"), S);
    p.advance(S, 2000, 4);
    assert_eq!(shown(&p), ["first", "buoy in 4"], "in place, not added");
    p.advance(2 * S, 2000, 4);
    assert_eq!(shown(&p), ["buoy in 4"], "its time ran from 1 s");
}

/// The shipped strings and names for filling popups.
fn fill(events: &[Value], me: usize) -> Vec<Popup> {
    let strings = crate::strings::parse(STRINGS).unwrap();
    let roster = crate::roster::Roster::parse(FISH).unwrap();
    let players: Vec<String> = ["Gil Waters", "Rod Fisher", "Bob Pike", "Wade Rivers"]
        .map(String::from)
        .to_vec();
    let fish = |id: &str| roster.get(id).map(|k| k.name.clone());
    let tier = |t| strings.t(&format!("tier.{}", crate::roster::tier_key(t)));
    let text = |k: &str| strings.t(k);
    let names = Names {
        players: &players,
        fish: &fish,
        tier: &tier,
        text: &text,
        someone: strings.t("popup.someone"),
        a_fish: strings.t("popup.fish"),
    };
    of_events(&look().popups.rows, events, me, &names)
}

#[test]
fn a_hook_names_the_player_and_the_fish_in_its_tier() {
    let hooked = json!({"type": "Announced", "seat": 1, "fish": 7, "kind": "walleye", "tier": "Uncommon", "delay_ms": 0});
    let p = fill(&[hooked], 0);
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].text, "Rod Fisher has hooked a Walleye!");
    assert_eq!(p[0].tier, Some(Tier::Uncommon));
    assert_eq!(p[0].style, "tier");
    // a landing too
    let caught = json!({"type": "Caught", "seat": 0, "fish": 2, "kind": "muskellunge", "tier": "Legendary", "length_mm": 1200});
    let p = fill(&[caught], 0);
    assert_eq!(p[0].text, "Gil Waters landed a Muskellunge");
    assert_eq!(p[0].tier, Some(Tier::Legendary));
}

#[test]
fn a_wreck_says_who_rammed_whom_or_who_ran_aground() {
    let rammed = serde_json::to_value(MatchEvent::Wrecked {
        seat: 2,
        cause: WreckCause::Rammed { by: 0 },
    })
    .unwrap();
    let aground = serde_json::to_value(MatchEvent::Wrecked {
        seat: 3,
        cause: WreckCause::Grounded,
    })
    .unwrap();
    let p = fill(&[rammed, aground], 1);
    let texts: Vec<&str> = p.iter().map(|q| q.text.as_str()).collect();
    assert_eq!(texts, ["Gil Waters wrecked Bob Pike", "Wade Rivers ran aground"]);
    assert!(p.iter().all(|q| q.style == "wreck" && q.tier.is_none()));
}

#[test]
fn an_unknown_fish_or_seat_still_makes_a_popup_and_other_events_make_none() {
    let odd = json!({"type": "Announced", "seat": 9, "kind": "kraken", "tier": "Rare"});
    let p = fill(
        &[odd, json!({"type": "Bumped", "a": 0, "b": 1, "speed": 3000})],
        0,
    );
    assert_eq!(p.len(), 1, "a bump has no popup");
    assert_eq!(p[0].text, "Someone has hooked a fish!");
    // the buoy's countdown is heard, not shown; its spawn is one line
    let beat = json!({"type": "BuoyBeat", "left": 3});
    assert!(fill(&[beat], 0).is_empty());
    let spawned = json!({"type": "BuoySpawned", "at": [0, 0]});
    assert_eq!(fill(&[spawned], 0)[0].text, "An item has spawned");
}

#[test]
fn a_popup_row_with_a_bad_style_or_fill_is_refused_by_name() {
    let mut v: Value = serde_json::from_str(FLOW).unwrap();
    v["popups"]["rows"][0]["style"] = json!("loud");
    assert_eq!(
        FlowLook::parse(&v.to_string()).unwrap_err(),
        "Popup row 1 (Announced) names the style \"loud\", which is not listed."
    );
    let mut v: Value = serde_json::from_str(FLOW).unwrap();
    v["popups"]["rows"][5]["fill"]["player"] = json!("seat");
    assert!(FlowLook::parse(&v.to_string())
        .unwrap_err()
        .contains("Popup row 6 (Wrecked) fills {player}"));
}

// ---- the postgame ----

fn col(seat: usize, values: &[i32]) -> Column {
    Column {
        seat,
        name: format!("P{seat}"),
        catches: values.iter().map(|v| catch("bluegill", *v)).collect(),
    }
}

/// Three players: 10 + 25, 40, and nothing.
fn reveal() -> Reveal {
    Reveal::new(
        vec![col(0, &[10, 25]), col(1, &[40]), col(2, &[])],
        &look().postgame,
    )
}

#[test]
fn the_reveal_shows_each_players_catches_in_turn_and_counts_their_total_up() {
    // shipped pacing: first card at 900, one every 750, 900 between players
    let r = reveal();
    assert_eq!(r.card_ms, vec![vec![900, 1650], vec![3300], vec![]]);
    // P0's two cards take 1500 + 900 = 2400 (to 3300); P1's one, 1650
    // (to 4950); P2's turn, with no catch, as long as one card (to 6600);
    // the ranking 1200 after the last turn: 6600 - 900 + 1200
    assert_eq!(r.rank_ms, 6900);
    let at = r.at(899);
    assert_eq!((at.cards.clone(), at.current), (vec![0, 0, 0], None));
    // 100 ms into the 10's count of 400: 10 - (10 - 10 × 100 / 400) = 2
    assert_eq!(r.at(1000).totals, vec![2, 0, 0]);
    assert_eq!(r.at(1300).totals, vec![10, 0, 0]);
    assert_eq!(r.at(2100).totals, vec![35, 0, 0]);
    let b = r.at(3700);
    assert_eq!(
        (b.cards, b.totals, b.current),
        (vec![2, 1, 0], vec![35, 40, 0], Some(1))
    );
    assert_eq!(r.at(5000).current, Some(2));
    let end = r.at(6900);
    assert!(end.ranking && end.current.is_none());
    assert_eq!(end.totals, vec![35, 40, 0]);
    // one tally tick a card
    assert_eq!(r.cards_between(-1, 6900), 3);
    assert_eq!(r.cards_between(900, 3300), 2);
}

#[test]
fn the_ranking_orders_by_total_and_ties_share_a_place() {
    assert_eq!(places(&[35, 40, 0]), vec![2, 1, 3]);
    assert_eq!(places(&[30, 50, 30, 50]), vec![3, 1, 3, 1]);
    assert_eq!(places(&[0, 0]), vec![1, 1]);
    let cols = vec![col(0, &[30]), col(1, &[20, 30]), col(2, &[10, 20]), col(3, &[50])];
    let order: Vec<(usize, usize)> = ranking(&cols).iter().map(|(p, c)| (*p, c.seat)).collect();
    // 30, 50, 30, 50: the two 50s first, by seat, then the two 30s
    assert_eq!(order, vec![(1, 1), (1, 3), (3, 0), (3, 2)]);
}

#[test]
fn the_postgame_takes_every_taken_seat_with_its_catches() {
    let scores = vec![
        Score {
            catches: vec![catch("bluegill", 12)],
        },
        Score::default(),
        Score {
            catches: vec![catch("walleye", 30), catch("perch", 8)],
        },
    ];
    let c = columns(&[(0, "A".into()), (2, "C".into())], &scores);
    assert_eq!(c.len(), 2);
    assert_eq!((c[1].seat, c[1].total()), (2, 38));
}

/// A match with two AI players, the flow's look and the roster in, and
/// made-up catches.
fn match_app() -> App {
    let mut a = play::lobby();
    a.act("data.flow", FLOW).unwrap();
    a.act("data.ui2d", UI2D).unwrap();
    a.act("data.fish", FISH).unwrap();
    a.resize(1920, 1080, 100);
    a.act("lobby.ai", "2").unwrap();
    a.act("lobby.ai.1", "hard").unwrap();
    a.act("lobby.length", "240000").unwrap();
    a.act("lobby.start", "").unwrap();
    a.advance(0);
    a
}

/// Runs the page's clock on by `ms`, a tick at a time.
fn run(a: &mut App, ms: i64) {
    for _ in 0..ms / 20 {
        play::tick(a);
    }
}

/// To the end of the match and through the hold to the postgame.
fn to_postgame(a: &mut App) {
    a.act("debug.time_left", "100").unwrap();
    run(a, 200);
    assert!(a.play.as_ref().unwrap().game.ended);
    assert_eq!(a.screen, Screen::Play, "Lines up holds the screen first");
    // the sample tuning's hold is 3 s
    run(a, 3000);
    assert_eq!(a.screen, Screen::Postgame);
}

#[test]
fn the_end_holds_then_opens_the_postgame_with_its_buttons() {
    let mut a = match_app();
    a.act("debug.catches", "").unwrap();
    to_postgame(&mut a);
    let ids = |a: &App| -> Vec<String> {
        a.ui().panels[0]
            .items
            .iter()
            .map(|i| {
                serde_json::to_value(i).unwrap()["id"]
                    .as_str()
                    .unwrap_or("")
                    .to_string()
            })
            .collect()
    };
    assert_eq!(ids(&a), ["postgame.skip"]);
    assert!(a.ui().scene && a.ui().live && !a.ui().capture);
    a.act("postgame.skip", "").unwrap();
    assert!(a.postgame_ranked());
    assert_eq!(
        ids(&a),
        ["postgame.catches", "postgame.rematch", "postgame.title"]
    );
    let b = a.build_ui2d();
    assert!(b.list.status.contains("postgame=ranking"), "{}", b.list.status);
    a.act("postgame.title", "").unwrap();
    assert_eq!(a.screen, Screen::Title);
    assert!(a.play.is_none());
}

#[test]
fn the_ranking_closes_to_show_every_catch_and_total_then_comes_back() {
    let mut a = match_app();
    a.act("debug.catches", "").unwrap();
    to_postgame(&mut a);
    let status = |a: &App| a.build_ui2d().list.status;
    // before the ranking, Catches does nothing
    a.act("postgame.catches", "").unwrap();
    assert!(!a.postgame_breakdown());
    a.act("postgame.skip", "").unwrap();
    let ranked = status(&a);
    assert!(
        ranked.contains("postgame=ranking") && ranked.contains("places="),
        "{ranked}"
    );
    let ids = |a: &App| -> Vec<String> {
        a.ui().panels[0]
            .items
            .iter()
            .map(|i| {
                serde_json::to_value(i).unwrap()["id"]
                    .as_str()
                    .unwrap_or("")
                    .to_string()
            })
            .collect()
    };
    // the button puts the ranking card away: every card up, each total
    a.act("postgame.catches", "").unwrap();
    let s = status(&a);
    assert!(s.contains("postgame=catches") && !s.contains("places="), "{s}");
    let pg = a.flow.postgame.as_ref().unwrap();
    let cards: Vec<String> = pg
        .reveal
        .columns
        .iter()
        .map(|c| c.catches.len().to_string())
        .collect();
    let totals: Vec<String> = pg.reveal.columns.iter().map(|c| c.total().to_string()).collect();
    assert!(s.contains(&format!("cards={}", cards.join(","))), "{s}");
    assert!(s.contains(&format!("totals={}", totals.join(","))), "{s}");
    assert_eq!(
        ids(&a),
        ["postgame.ranking", "postgame.rematch", "postgame.title"]
    );
    // Esc and Space swap back and forth; neither leaves
    a.act("key", "Escape").unwrap();
    assert!(status(&a).contains("postgame=ranking"));
    a.act("key", "Space").unwrap();
    assert!(status(&a).contains("postgame=catches"));
    a.act("postgame.ranking", "").unwrap();
    assert!(status(&a).contains("postgame=ranking"));
    assert_eq!(a.screen, Screen::Postgame);
}

#[test]
fn rematch_keeps_the_setup_with_a_new_seed() {
    let mut a = match_app();
    let before = a.play.as_ref().unwrap().game.setup.clone();
    to_postgame(&mut a);
    a.act("postgame.skip", "").unwrap();
    a.act("postgame.rematch", "").unwrap();
    assert_eq!(a.screen, Screen::Play);
    let after = &a.play.as_ref().unwrap().game;
    assert_ne!(after.setup.seed, before.seed, "a new seed");
    assert_eq!(
        after.setup,
        Setup {
            seed: after.setup.seed,
            ..before.clone()
        }
    );
    assert_eq!(
        after.setup.seats[1].occupant,
        Occupant::Ai(crate::sim::Difficulty::Hard)
    );
    assert_eq!(after.setup.length_ms, 240_000);
    assert!(after.scores.iter().all(|s| s.catches.is_empty()), "a fresh match");
    assert!(!after.ended && a.flow.postgame.is_none() && a.flow.ended_us.is_none());
}

#[test]
fn the_reveal_plays_a_tally_tick_for_each_card_and_the_fanfare_once() {
    let mut a = match_app();
    let bank = crate::sound::Bank::parse(
        include_str!("../../../../data/sounds.json"),
        include_str!("../../../../data/tuning.json"),
    )
    .unwrap();
    a.sound = Some(crate::sound::Sound::new(bank));
    a.act("debug.catches", "").unwrap();
    let cards: usize = a
        .play
        .as_ref()
        .unwrap()
        .game
        .scores
        .iter()
        .map(|s| s.catches.len())
        .sum();
    to_postgame(&mut a);
    let mut heard = Vec::new();
    for _ in 0..1000 {
        play::tick(&mut a);
        let s = a.sound.as_mut().unwrap();
        heard.extend(s.pending().iter().map(|v| v.to_string()));
        let _ = s.frame(80, Vec::new());
    }
    let count = |id: &str| heard.iter().filter(|h| h.contains(&format!("\"{id}\""))).count();
    assert_eq!(count("tally_tick"), cards);
    assert_eq!(count("winner_fanfare"), 1);
}

// ---- the HUD ----

fn status(a: &App) -> String {
    a.build_ui2d().list.status
}

#[test]
fn the_map_seed_shows_only_with_the_option_on() {
    let mut a = match_app();
    let seed = a.play.as_ref().unwrap().game.map.seed;
    assert!(!a.settings.show_seed, "off by default");
    assert!(status(&a).split(' ').any(|w| w == "seed="));
    a.act("settings.show_seed", "on").unwrap();
    assert!(status(&a).split(' ').any(|w| w == format!("seed={seed}")));
    let texts = a.build_ui2d().list.strings;
    assert!(texts.contains(&"HUD.SEED".to_string()), "its line is drawn");
    a.act("settings.show_seed", "off").unwrap();
    assert!(!a.build_ui2d().list.strings.contains(&"HUD.SEED".to_string()));
}

#[test]
fn the_hud_shows_the_clock_and_the_ping_and_no_words() {
    let a = match_app();
    let s = status(&a);
    for w in ["hud=1", "time=4:00", "speed=0", "mode=driving", "hud_ping=ready"] {
        assert!(s.split(' ').any(|x| x == w), "{w} in {s}");
    }
    // KISS (the person, 2026-10-18): no speed gauge, no mode badge, no
    // key hints and no labels; at the start the only text is the clock
    assert_eq!(a.build_ui2d().list.words(), vec!["4:00".to_string()]);
}

#[test]
fn the_countdown_is_drawn_then_go() {
    let mut a = play::lobby();
    let mut t = a.tuning.clone().unwrap();
    t.flow.countdown_ms = 3000;
    a.tuning = Some(t);
    a.act("data.flow", FLOW).unwrap();
    a.act("data.ui2d", UI2D).unwrap();
    a.act("lobby.start", "").unwrap();
    a.advance(0);
    assert!(status(&a).contains("countdown=3"));
    run(&mut a, 1000);
    assert!(status(&a).contains("countdown=2"));
    run(&mut a, 2000);
    assert!(status(&a).contains("countdown=go"), "{}", status(&a));
    // "Go" stays 900 ms
    run(&mut a, 1000);
    assert!(status(&a).split(' ').any(|w| w == "countdown="));
}

#[test]
fn a_hook_heard_in_the_match_pops_up_for_this_player() {
    let mut a = match_app();
    let hooked = json!({"type": "Announced", "seat": 1, "fish": 0, "kind": "walleye", "tier": "Uncommon", "delay_ms": 0});
    a.flow_hear(&[hooked]);
    a.advance(a.clock);
    assert_eq!(a.flow.popups.shown().count(), 1);
    assert!(status(&a).contains("popups=1"));
    assert!(status(&a).contains("popup=tier"));
    run(&mut a, 2000);
    assert_eq!(a.flow.popups.shown().count(), 0, "gone after 2 s");
}

// ---- a wreck ----

#[test]
fn a_wreck_goes_off_where_the_boat_was_and_shakes_its_player() {
    let l = look().burst;
    // the fireball's widest at 22% of 1100 ms (242), half that at 121 (2.4 m wide at most)
    assert_eq!(burst::fireball_mm(&l, 242), 2400);
    assert_eq!(burst::fireball_mm(&l, 121), 1200);
    assert_eq!(burst::fireball_mm(&l, 1100), 0);
    let b = Burst {
        seat: 0,
        at: crate::gfx::geom::V3::ZERO,
        born_us: 0,
        seed: 1,
    };
    assert!(!burst::tris(&b, &l, 100).is_empty());
    assert!(burst::tris(&b, &l, 1100).is_empty(), "over after 1.1 s");
    assert_ne!(burst::shake(&l, 30), (0, 0));
    assert_eq!(burst::shake(&l, 650), (0, 0), "still after 650 ms");
}

#[test]
fn a_ghosted_boat_is_drawn_see_through_in_its_own_list() {
    let mut f = crate::gfx::render::Frame {
        view: vec![7; crate::gfx::render::VIEW_VALUES],
        ..Default::default()
    };
    f.set_ghost(vec![1, 2, 3], fx_half());
    let (at, _) = crate::gfx::render::uniform_at("uOpacity");
    assert_eq!(f.ghost, vec![1, 2, 3]);
    assert_eq!(f.ghost_view[at], fx_half());
    let mut rest = f.ghost_view.clone();
    rest[at] = 7;
    assert_eq!(rest, f.view, "the rest of the view is the frame's");
    // the shipped ghost is 45% solid
    assert_eq!(look().ghost_pct, 45);
}

fn fx_half() -> i32 {
    crate::gfx::fx::ONE / 2
}

#[test]
fn the_made_up_catches_fill_every_taken_seat() {
    let a = {
        let mut a = match_app();
        a.act("debug.catches", "").unwrap();
        a
    };
    let g = &a.play.as_ref().unwrap().game;
    for (k, s) in g.scores.iter().enumerate() {
        let taken = g.setup.seats[k].occupant != Occupant::Empty;
        assert_eq!(!s.catches.is_empty(), taken, "seat {k}");
        assert!(s.catches.len() <= 4);
    }
}

#[test]
fn f8_jumps_to_the_last_seconds_only_under_the_debug_flag() {
    let mut a = match_app();
    a.act("key", "F8").unwrap();
    assert_eq!(
        a.play.as_ref().unwrap().game.remaining_ms(),
        240_000,
        "no flag: F8 does nothing"
    );
    a.act("debug.flags", "flow").unwrap();
    a.act("key", "F8").unwrap();
    let g = &a.play.as_ref().unwrap().game;
    assert_eq!(g.remaining_ms(), DEBUG_LEFT_MS);
    assert!(
        g.scores.iter().any(|s| !s.catches.is_empty()),
        "made-up catches for the postgame"
    );
}

#[test]
fn f9_under_the_debug_flag_wrecks_your_boat_and_says_so() {
    let mut a = match_app();
    a.act("key", "F9").unwrap();
    assert!(
        !a.play.as_ref().unwrap().game.boat(0).unwrap().wrecked(),
        "no flag: nothing"
    );
    a.act("debug.flags", "flow").unwrap();
    a.act("key", "F9").unwrap();
    assert!(a.play.as_ref().unwrap().game.boat(0).unwrap().wrecked());
    assert_eq!(a.flow.popups.queue.len(), 1, "the ran-aground popup");
}

// ---- sounds ----

#[test]
fn every_event_the_flow_emits_has_a_sound() {
    let sounds: Value = serde_json::from_str(include_str!("../../../../data/sounds.json")).unwrap();
    for e in [
        "Countdown",
        "MatchStarted",
        "MatchEnded",
        "Wrecked",
        "TallyTick",
        "Winner",
    ] {
        assert!(sounds["events"].get(e).is_some(), "{e}");
    }
    let bank = crate::sound::Bank::parse(
        include_str!("../../../../data/sounds.json"),
        include_str!("../../../../data/tuning.json"),
    )
    .unwrap();
    let mut s = crate::sound::Sound::new(bank);
    for left in [3, 2, 1] {
        s.event(
            &serde_json::to_value(MatchEvent::Countdown { left }).unwrap(),
            Some(0),
        );
    }
    s.event(&serde_json::to_value(MatchEvent::MatchStarted).unwrap(), Some(0));
    let heard: String = s.pending().iter().map(Value::to_string).collect();
    for cue in ["count_3", "count_2", "count_1", "match_start"] {
        assert!(heard.contains(cue), "{cue} in {heard}");
    }
}
