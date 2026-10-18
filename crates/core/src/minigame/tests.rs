//! The six minigames. Expected values are worked by hand from the sample
//! tuning (`tuning::tests::sample_json`): a 1600 x 900 canvas, a 1 s
//! banner (50 ticks of 20 ms), and each game's numbers there.

use super::atoms::Where;
use super::*;
use crate::map::trig::{self, TURN};

const BANNER_TICKS: u32 = 50;

fn tuning() -> MinigameTuning {
    serde_json::from_value(tuning::tests::sample_json()).unwrap()
}

fn new(kind: Kind) -> Minigame {
    Minigame::new(kind, 7, &tuning())
}

fn at(p: (i32, i32)) -> Input {
    Input {
        pointer: Some(p),
        ..Input::default()
    }
}

fn press(p: (i32, i32)) -> Input {
    Input {
        pointer: Some(p),
        held: true,
        pressed: true,
        ..Input::default()
    }
}

fn drag(p: (i32, i32)) -> Input {
    Input {
        pointer: Some(p),
        held: true,
        ..Input::default()
    }
}

fn release(p: (i32, i32)) -> Input {
    Input {
        pointer: Some(p),
        released: true,
        ..Input::default()
    }
}

fn quit() -> Input {
    Input {
        quit: true,
        ..Input::default()
    }
}

fn past_banner(mg: &mut Minigame) {
    for _ in 0..BANNER_TICKS {
        mg.step(&Input::default());
    }
    assert!(!mg.in_banner());
}

/// Steps with `script` (given the minigame and the play tick about to
/// run, from 1) until it is no longer playing or `max` play ticks pass.
fn run(mg: &mut Minigame, max: u32, mut script: impl FnMut(&Minigame, u32) -> Input) -> State {
    for k in 1..=max {
        let input = script(mg, k);
        mg.step(&input);
        if mg.state() != State::Playing {
            break;
        }
    }
    mg.state()
}

fn ms(state: State) -> u32 {
    match state {
        State::Done { ms_taken } => ms_taken,
        other => panic!("not done: {other:?}"),
    }
}

// ---- every minigame ----

#[test]
fn every_minigame_opens_with_its_banner_and_the_start_cue() {
    for kind in Kind::ALL {
        let mut mg = new(kind);
        assert!(mg.in_banner(), "{kind:?}");
        assert_eq!(mg.take_cues(), vec![cue::START], "{kind:?}");
        for _ in 0..BANNER_TICKS - 1 {
            mg.step(&Input::default());
        }
        assert!(mg.in_banner(), "{kind:?} is still on its banner after 49 ticks");
        mg.step(&Input::default());
        assert!(!mg.in_banner(), "{kind:?} plays after 50 ticks (1 s)");
        assert_eq!(mg.state(), State::Playing);
    }
}

#[test]
fn input_during_the_banner_does_nothing() {
    // a whole clockwise turn round the reel, all inside the banner
    let mut mg = new(Kind::PowerReel);
    let o = (800, 470);
    for k in 0..BANNER_TICKS as i32 - 1 {
        let a = k * TURN / 24;
        mg.step(&at((
            o.0 + 150 * trig::cos(a) / trig::ONE,
            o.1 + 150 * trig::sin(a) / trig::ONE,
        )));
    }
    let Game::Reel(g) = &mg.game else { unreachable!() };
    assert_eq!(g.swept, 0);
}

#[test]
fn esc_quits_every_minigame_in_its_banner_and_in_play() {
    for kind in Kind::ALL {
        let mut mg = new(kind);
        mg.step(&quit());
        assert_eq!(mg.state(), State::Quit, "{kind:?} in the banner");
        let mut mg = new(kind);
        past_banner(&mut mg);
        for _ in 0..10 {
            mg.step(&Input::default());
        }
        mg.step(&quit());
        assert_eq!(mg.state(), State::Quit, "{kind:?} in play");
        // nothing moves it on afterwards
        mg.step(&press((800, 450)));
        assert_eq!(mg.state(), State::Quit);
    }
}

#[test]
fn no_minigame_can_be_failed_by_doing_nothing() {
    // ten minutes untouched: still playing, waiting
    for kind in Kind::ALL {
        let mut mg = new(kind);
        let state = run(&mut mg, 30_000, |_, _| Input::default());
        assert_eq!(state, State::Playing, "{kind:?}");
    }
}

#[test]
fn no_minigame_can_be_failed_by_clicking_wildly() {
    // a click somewhere new every few ticks: finished or still going,
    // never ended any other way
    for kind in Kind::ALL {
        let mut mg = new(kind);
        let mut r = crate::map::rng::Rng::new(3);
        let state = run(&mut mg, 5_000, |_, k| {
            let p = (r.range(0, 1600), r.range(0, 900));
            match k % 3 {
                0 => press(p),
                1 => drag(p),
                _ => release(p),
            }
        });
        assert_ne!(state, State::Quit, "{kind:?}");
    }
}

#[test]
fn the_same_seed_plays_the_same_and_another_seed_differently() {
    let t = tuning();
    let atoms = |seed| {
        let mg = Minigame::new(Kind::Nitrous, seed, &t);
        let Game::Atoms(g) = &mg.game else { unreachable!() };
        g.atoms.iter().map(|a| a.pos()).collect::<Vec<_>>()
    };
    assert_eq!(atoms(1), atoms(1));
    assert_ne!(atoms(1), atoms(2));
}

#[test]
fn the_fish_boy_minigames_say_so_and_the_camera_hides_the_cursor() {
    let layers: Vec<Layer> = Kind::ALL.iter().map(|k| k.layer()).collect();
    assert_eq!(
        layers,
        [
            Layer::FishBoy,
            Layer::Hud,
            Layer::Hud,
            Layer::Hud,
            Layer::Hud,
            Layer::FishBoy
        ]
    );
    for kind in Kind::ALL {
        assert_eq!(new(kind).hides_cursor(), kind == Kind::Camera, "{kind:?}");
    }
}

// ---- Power Reel ----

/// The point at `angle` (clockwise on screen from the right), `r` from
/// the reel's middle, (800, 470): the canvas's middle, 20 down.
fn round_reel(angle: i32, r: i32) -> (i32, i32) {
    (
        800 + r * trig::cos(angle) / trig::ONE,
        470 + r * trig::sin(angle) / trig::ONE,
    )
}

#[test]
fn a_perfect_reel_run_finishes_after_five_clockwise_turns() {
    let mut mg = new(Kind::PowerReel);
    past_banner(&mut mg);
    // 15 degrees a tick from the right, clockwise: the first tick only
    // takes the starting angle, so tick 1 + 5 * 24 completes five turns
    let state = run(&mut mg, 1000, |_, k| {
        at(round_reel((k as i32 - 1) * TURN / 24, 150))
    });
    assert_eq!(ms(state), 121 * 20);
    assert_eq!(mg.progress(), (5, 5));
}

#[test]
fn anticlockwise_circles_count_for_nothing() {
    let mut mg = new(Kind::PowerReel);
    past_banner(&mut mg);
    let state = run(&mut mg, 240, |_, k| at(round_reel(-(k as i32) * TURN / 24, 150)));
    assert_eq!(state, State::Playing);
    let Game::Reel(g) = &mg.game else { unreachable!() };
    assert_eq!((g.swept, g.turns_done()), (0, 0));
}

#[test]
fn winding_back_loses_only_the_turn_in_progress() {
    let mut mg = new(Kind::PowerReel);
    past_banner(&mut mg);
    // one and a half turns clockwise (37 samples, 36 steps of 15 degrees)
    for k in 0..37 {
        mg.step(&at(round_reel(k * TURN / 24, 150)));
    }
    // then a whole turn back
    for k in 1..=24 {
        mg.step(&at(round_reel((36 - k) * TURN / 24, 150)));
    }
    let Game::Reel(g) = &mg.game else { unreachable!() };
    assert_eq!(g.turns_done(), 1);
    assert_eq!(g.swept, TURN as i64, "the half turn is gone, the whole one kept");
}

#[test]
fn circling_inside_the_dead_zone_counts_for_nothing() {
    let mut mg = new(Kind::PowerReel);
    past_banner(&mut mg);
    run(&mut mg, 240, |_, k| at(round_reel(k as i32 * TURN / 24, 30)));
    let Game::Reel(g) = &mg.game else { unreachable!() };
    assert_eq!(g.swept, 0);
}

#[test]
fn a_sloppy_reel_run_finishes_later() {
    let mut mg = new(Kind::PowerReel);
    past_banner(&mut mg);
    // three steps forward, one back: half the pace
    let mut angle = 0;
    let state = run(&mut mg, 2000, |_, k| {
        if k > 1 {
            angle += if k % 4 == 0 { -TURN / 24 } else { TURN / 24 };
        }
        at(round_reel(angle, 150))
    });
    assert!(ms(state) > 121 * 20, "{state:?}");
}

#[test]
fn moves_between_ticks_are_followed() {
    // a whole turn in 24 pointer moves, all between two ticks
    let mut mg = new(Kind::PowerReel);
    past_banner(&mut mg);
    for k in 0..=24 {
        mg.point(round_reel(k * TURN / 24, 150), false);
    }
    let Game::Reel(g) = &mg.game else { unreachable!() };
    assert_eq!(g.turns_done(), 1);
}

// ---- Cloaking Tech ----

fn cloak_with_eraser(eraser: i32) -> Minigame {
    let mut t = tuning();
    t.cloak.eraser = eraser;
    Minigame::new(Kind::Cloak, 7, &t)
}

/// A press and release inside one tick: one dab of the eraser.
fn dab(p: (i32, i32)) -> Input {
    Input {
        pointer: Some(p),
        pressed: true,
        released: true,
        ..Input::default()
    }
}

#[test]
fn erasing_98_percent_of_the_boat_finishes_the_cloak() {
    // an eraser one unit wide takes one cell a dab: the cell whose middle
    // it is on
    let mut mg = cloak_with_eraser(1);
    past_banner(&mut mg);
    let Game::Cloak(g) = &mg.game else { unreachable!() };
    let (cols, total) = (g.t.cols, g.total);
    let ink: Vec<(i32, i32)> = (0..g.cells.len() as i32)
        .filter(|k| g.cells[*k as usize] != 0)
        .map(|k| g.cell_mid(k % cols, k / cols))
        .collect();
    assert_eq!(ink.len() as i32, total);
    // done once at most 2% is left: (total - left) * 100 >= 98 * total
    let may_leave = total * 2 / 100;
    for p in &ink[..(total - may_leave - 1) as usize] {
        mg.step(&dab(*p));
    }
    assert_eq!(mg.state(), State::Playing, "one cell more than 2% is left");
    mg.step(&dab(ink[(total - may_leave - 1) as usize]));
    assert!(matches!(mg.state(), State::Done { .. }), "2% left: done");
}

/// A back-and-forth sweep over the picture (x 200..1400, y 150..750),
/// `rows_apart` between passes, `speed` units a tick, the button held.
fn sweep(rows_apart: i32, speed: i32) -> impl FnMut(&Minigame, u32) -> Input {
    let (mut x, mut y, mut dir) = (180, 150, 1);
    move |_, _| {
        let p = (x, y);
        x += dir * speed;
        if !(180..=1420).contains(&x) {
            dir = -dir;
            x = x.clamp(180, 1420);
            y += rows_apart;
            if y > 760 {
                y = 150 + rows_apart / 2;
            }
        }
        drag(p)
    }
}

#[test]
fn a_tidy_sweep_erases_the_boat_quickly_and_a_sloppy_one_later() {
    let mut mg = cloak_with_eraser(80);
    past_banner(&mut mg);
    let tidy = ms(run(&mut mg, 20_000, sweep(60, 40)));
    let mut mg = cloak_with_eraser(80);
    past_banner(&mut mg);
    // passes too far apart, so a second round is needed, and slower
    let sloppy = ms(run(&mut mg, 20_000, sweep(150, 25)));
    assert!(tidy < sloppy, "{tidy} < {sloppy}");
    // 11 passes of 1240 units at 40 a tick, about 340 ticks
    assert!(tidy < 8_000, "{tidy}");
}

#[test]
fn a_stroke_erases_the_whole_way_between_two_ticks() {
    // the pointer jumps 600 units in one tick with the button held: the
    // cells between are gone too
    let mut mg = cloak_with_eraser(40);
    past_banner(&mut mg);
    mg.step(&press((500, 450)));
    mg.step(&drag((1100, 450)));
    let Game::Cloak(g) = &mg.game else { unreachable!() };
    // the middle row (row 15 of 30) from x 500 to 1100: cells 15..45
    let row = 15 * g.t.cols;
    let erased = (15..45).filter(|c| g.cells[(row + c) as usize] == 0).count();
    assert_eq!(erased, 30);
}

#[test]
fn moving_without_the_button_erases_nothing() {
    let mut mg = cloak_with_eraser(80);
    past_banner(&mut mg);
    for k in 0..300 {
        mg.step(&at((180 + k * 4, 450)));
    }
    let Game::Cloak(g) = &mg.game else { unreachable!() };
    assert_eq!(g.left, g.total);
}

// ---- Nitrous ----

fn atoms_of(mg: &Minigame) -> &atoms::Atoms {
    let Game::Atoms(g) = &mg.game else { unreachable!() };
    g
}

/// The middle of the bin for nitrogen or oxygen.
fn bin_mid(mg: &Minigame, nitrogen: bool) -> (i32, i32) {
    let b = atoms_of(mg).bins[if nitrogen { 0 } else { 1 }];
    (b.x + b.w / 2, b.y + b.h / 2)
}

/// A free atom a press on its own middle picks up (none drawn over it).
fn pickable(mg: &Minigame) -> usize {
    let g = atoms_of(mg);
    (0..g.atoms.len())
        .find(|&k| g.atoms[k].at == Where::Free && g.under(g.atoms[k].pos()) == Some(k))
        .unwrap()
}

/// Picks up atom `k` and drops it in its own bin (or the other, `wrong`):
/// three ticks.
fn carry_atom(mg: &mut Minigame, k: usize, wrong: bool) {
    let g = atoms_of(mg);
    let (p, nitrogen) = (g.atoms[k].pos(), g.atoms[k].nitrogen);
    let to = bin_mid(mg, nitrogen != wrong);
    mg.step(&press(p));
    mg.step(&drag(to));
    mg.step(&release(to));
}

fn carry(mg: &mut Minigame, wrong: bool) -> usize {
    let k = pickable(mg);
    carry_atom(mg, k, wrong);
    k
}

#[test]
fn dropping_each_atom_in_its_bin_finishes_nitrous() {
    let mut mg = new(Kind::Nitrous);
    past_banner(&mut mg);
    for _ in 0..12 {
        carry(&mut mg, false);
    }
    // 12 atoms (8 N, 4 O) at three ticks each
    assert_eq!(ms(mg.state()), 36 * 20);
    assert_eq!(mg.progress(), (12, 12));
}

#[test]
fn a_wrong_bin_throws_the_atom_back_out() {
    let mut mg = new(Kind::Nitrous);
    past_banner(&mut mg);
    mg.take_cues();
    let k = carry(&mut mg, true);
    let g = atoms_of(&mg);
    let a = g.atoms[k];
    assert_eq!(a.at, Where::Free, "back in play, not binned");
    assert!(a.pos().1 < g.arena.bottom(), "back inside the box");
    assert!(a.vy < 0, "flying up, away from the bins");
    assert_eq!((g.bounces, g.binned()), (1, 0));
    assert!(mg.take_cues().contains(&cue::MISS));
}

#[test]
fn a_drop_outside_the_bins_lets_the_atom_go_where_it_is() {
    let mut mg = new(Kind::Nitrous);
    past_banner(&mut mg);
    let g = atoms_of(&mg);
    let p = g.atoms[0].pos();
    let mid = (g.arena.x + g.arena.w / 2, g.arena.y + g.arena.h / 2);
    mg.step(&press(p));
    mg.step(&drag(mid));
    mg.step(&release(mid));
    let g = atoms_of(&mg);
    assert_eq!(g.atoms[0].at, Where::Free);
    assert_eq!(g.binned(), 0);
    // it went on from the middle with its own speed: a tick's move away
    let q = g.atoms[0].pos();
    assert!((q.0 - mid.0).abs() <= 10 && (q.1 - mid.1).abs() <= 10, "{q:?}");
}

#[test]
fn a_sloppy_nitrous_run_finishes_later() {
    let mut mg = new(Kind::Nitrous);
    past_banner(&mut mg);
    // every atom tried in the wrong bin first
    while mg.state() == State::Playing {
        let k = carry(&mut mg, true);
        if atoms_of(&mg).under(atoms_of(&mg).atoms[k].pos()) == Some(k) {
            carry_atom(&mut mg, k, false);
        }
    }
    // at least six ticks an atom instead of three
    assert!(ms(mg.state()) >= 72 * 20, "{:?}", mg.state());
}

// ---- Teleporter ----

fn beats_of(mg: &Minigame) -> &beats::Beats {
    let Game::Beats(g) = &mg.game else { unreachable!() };
    g
}

/// A fresh Teleporter with one press on its first circle `at_now` play
/// ticks in (its ring meets its edge at 50: 1000 ms).
fn press_first_at(at_now: u32) -> (i32, i32) {
    let mut mg = new(Kind::Teleporter);
    past_banner(&mut mg);
    mg.step(&Input::default());
    let target = beats_of(&mg).circles[0].at;
    for _ in 1..at_now {
        mg.step(&at(target));
    }
    assert_eq!(beats_of(&mg).now, at_now);
    mg.step(&press(target));
    let g = beats_of(&mg);
    (g.hits, g.misses)
}

#[test]
fn a_click_as_the_ring_meets_the_edge_is_a_hit() {
    assert_eq!(press_first_at(50), (1, 0));
}

#[test]
fn the_timing_window_is_160_ms_either_side() {
    // 160 ms is 8 ticks
    assert_eq!(press_first_at(42), (1, 0), "8 ticks early");
    assert_eq!(press_first_at(58), (1, 0), "8 ticks late");
    assert_eq!(press_first_at(41), (0, 1), "9 ticks early");
    assert_eq!(press_first_at(20), (0, 1), "far too early");
}

#[test]
fn an_unclicked_circle_is_missed_after_its_window() {
    let mut mg = new(Kind::Teleporter);
    past_banner(&mut mg);
    // the first circle's window ends at 58; at 59 it is gone
    for _ in 0..59 {
        mg.step(&Input::default());
    }
    assert_eq!(beats_of(&mg).misses, 0);
    mg.step(&Input::default());
    assert_eq!(beats_of(&mg).misses, 1);
}

/// Presses the oldest circle when its ring meets its edge, except where
/// `skip` says to press it early.
fn rhythm(mut early: impl FnMut(i32) -> bool) -> impl FnMut(&Minigame, u32) -> Input {
    move |mg, _| {
        let g = beats_of(mg);
        // the press is judged at this tick's `now`
        match g.circles.first() {
            Some(c) if early(c.number) && g.now + 20 == g.meets(c) => press(c.at),
            Some(c) if g.now == g.meets(c) => press(c.at),
            _ => Input::default(),
        }
    }
}

#[test]
fn a_perfect_teleporter_run_takes_ten_circles() {
    let mut mg = new(Kind::Teleporter);
    past_banner(&mut mg);
    // circle n appears at 35 (n - 1) and meets its edge 50 later; the
    // tenth meets at 365, pressed on play tick 366
    let state = run(&mut mg, 5000, rhythm(|_| false));
    assert_eq!(ms(state), 366 * 20);
    assert_eq!(beats_of(&mg).misses, 0);
}

#[test]
fn a_sloppy_teleporter_run_takes_more_circles() {
    let mut mg = new(Kind::Teleporter);
    past_banner(&mut mg);
    // every odd circle pressed too early: 19 circles for 10 hits, the
    // nineteenth meeting at 35 * 18 + 50 = 680
    let state = run(&mut mg, 5000, rhythm(|n| n % 2 == 1 && n < 19));
    assert_eq!(ms(state), 681 * 20);
    assert_eq!(beats_of(&mg).misses, 9);
}

// ---- High-Quality Camera ----

fn shoot_of(mg: &Minigame) -> &camera::Shoot {
    let Game::Camera(g) = &mg.game else { unreachable!() };
    g
}

fn on_canvas(p: (i32, i32)) -> bool {
    (0..1600).contains(&p.0) && (0..900).contains(&p.1)
}

#[test]
fn a_perfect_camera_run_photographs_each_fish_as_it_shows() {
    let mut mg = new(Kind::Camera);
    past_banner(&mut mg);
    let state = run(&mut mg, 20_000, |mg, _| {
        match shoot_of(mg).fish.iter().map(|f| f.pos()).find(|p| on_canvas(*p)) {
            Some(p) => press(p),
            None => Input::default(),
        }
    });
    let perfect = ms(state);
    assert_eq!(shoot_of(&mg).photos, 10);
    assert_eq!(shoot_of(&mg).missed, 0);
    // a fish every 0.5 to 1 s: ten in well under fifteen seconds
    assert!(perfect < 15_000, "{perfect}");

    // missing by a wide margin every other try
    let mut mg = new(Kind::Camera);
    past_banner(&mut mg);
    let mut tries = 0;
    let state = run(&mut mg, 20_000, |mg, _| {
        match shoot_of(mg).fish.iter().map(|f| f.pos()).find(|p| on_canvas(*p)) {
            Some(p) => {
                tries += 1;
                if tries % 4 == 0 {
                    press(p)
                } else {
                    press((p.0, p.1 + 400))
                }
            }
            None => Input::default(),
        }
    });
    let sloppy = ms(state);
    assert!(perfect < sloppy, "{perfect} < {sloppy}");
}

#[test]
fn a_click_away_from_every_fish_takes_no_photo() {
    let mut mg = new(Kind::Camera);
    past_banner(&mut mg);
    mg.take_cues();
    // the reach is the fish's half length (80 units at 100%): 81 misses
    while shoot_of(&mg).fish.iter().all(|f| !on_canvas(f.pos())) {
        mg.step(&Input::default());
    }
    let p = shoot_of(&mg)
        .fish
        .iter()
        .map(|f| f.pos())
        .find(|p| on_canvas(*p))
        .unwrap();
    mg.step(&press((p.0 + 81, p.1)));
    assert_eq!((shoot_of(&mg).photos, shoot_of(&mg).missed), (0, 1));
    assert_eq!(mg.take_cues(), vec![cue::SHUTTER], "the shutter clicks anyway");
}

// ---- Enhanced Radar ----

fn focus_of(mg: &mut Minigame) -> &mut focus::Focus {
    let Game::Focus(g) = &mut mg.game else {
        unreachable!()
    };
    g
}

#[test]
fn every_slider_starts_away_from_its_sweet_spot() {
    for seed in 0..40 {
        let mut mg = Minigame::new(Kind::Radar, seed, &tuning());
        let g = focus_of(&mut mg);
        for k in 0..focus::SLIDERS {
            assert!(g.off(k) >= 250, "seed {seed} slider {k}");
            assert!((100..=900).contains(&g.sweet[k]), "seed {seed} slider {k}");
        }
    }
}

#[test]
fn within_ten_percent_of_every_spot_it_snaps_sharp_then_finishes() {
    let mut mg = new(Kind::Radar);
    past_banner(&mut mg);
    mg.take_cues();
    let g = focus_of(&mut mg);
    // 10% of the track is 100 thousandths: two right on the edge, one
    // a thousandth past it
    g.value = [g.sweet[0] + 100, g.sweet[1] - 100, g.sweet[2] + 101];
    mg.step(&Input::default());
    assert!(
        focus_of(&mut mg).snapped.is_none(),
        "101 thousandths off: not yet"
    );
    assert_eq!(mg.progress(), (2, 3));
    let g = focus_of(&mut mg);
    g.value[2] = g.sweet[2] + 100;
    mg.step(&Input::default());
    assert!(focus_of(&mut mg).snapped.is_some(), "all within 100: sharp");
    assert_eq!(mg.take_cues(), vec![cue::SUCCESS]);
    assert_eq!(focus_of(&mut mg).fault(0), 0, "sharp at once");
    // it stays sharp for a second (50 ticks), then it is done
    for _ in 0..49 {
        mg.step(&press((1250, 300)));
        assert_eq!(mg.state(), State::Playing);
    }
    mg.step(&Input::default());
    assert!(matches!(mg.state(), State::Done { .. }));
}

#[test]
fn a_slider_follows_the_pointer_from_its_knob() {
    let mut mg = new(Kind::Radar);
    past_banner(&mut mg);
    let v = focus_of(&mut mg).value[1];
    // the middle slider's knob, then up the track to 455: half way
    // between its top (150) and its bottom (760)
    mg.step(&press(focus::Focus::knob_at(1, v)));
    mg.step(&drag((1380, 455)));
    assert_eq!(focus_of(&mut mg).value[1], 500);
    mg.step(&release((1380, 455)));
    // released, a move no longer drags it
    mg.step(&at((1380, 200)));
    assert_eq!(focus_of(&mut mg).value[1], 500);
}

/// Drags each slider in turn from its knob to `aim(slider, sweet)`, then
/// to the spot, two ticks each leg.
fn tune(mg: &mut Minigame, aim: impl Fn(i32) -> i32) -> State {
    for k in 0..focus::SLIDERS {
        let g = focus_of(mg);
        let (v, s) = (g.value[k], g.sweet[k]);
        let knob = focus::Focus::knob_at(k, v);
        mg.step(&press(knob));
        mg.step(&drag(focus::Focus::knob_at(k, aim(s))));
        mg.step(&drag(focus::Focus::knob_at(k, s)));
        mg.step(&release(focus::Focus::knob_at(k, s)));
    }
    run(mg, 200, |_, _| Input::default())
}

#[test]
fn a_tidy_radar_run_is_faster_than_a_sloppy_one() {
    let mut mg = new(Kind::Radar);
    past_banner(&mut mg);
    let tidy = ms(tune(&mut mg, |s| s));
    // four ticks a slider; the third reaches its spot on the tenth tick,
    // which snaps the picture sharp; done 50 ticks later
    assert_eq!(tidy, (10 + 50) * 20);
    let mut mg = new(Kind::Radar);
    past_banner(&mut mg);
    // overshooting each, then waiting a second before each correction
    let sloppy = {
        for k in 0..focus::SLIDERS {
            let g = focus_of(&mut mg);
            let (v, s) = (g.value[k], g.sweet[k]);
            mg.step(&press(focus::Focus::knob_at(k, v)));
            let over = if s < 500 { s + 300 } else { s - 300 };
            mg.step(&drag(focus::Focus::knob_at(k, over)));
            for _ in 0..50 {
                mg.step(&drag(focus::Focus::knob_at(k, over)));
            }
            mg.step(&drag(focus::Focus::knob_at(k, s)));
            mg.step(&release(focus::Focus::knob_at(k, s)));
        }
        ms(run(&mut mg, 200, |_, _| Input::default()))
    };
    assert!(tidy < sloppy, "{tidy} < {sloppy}");
}
