//! The browser checks' page flags (SECOND-ORDER R6.90 to R6.99), never set
//! for players:
//!
//! - **`?fast`** (`flag.fast`): skips the waits that only make a person
//!   wait, and leaves every rule's timing alone. The 3-2-1 countdown runs
//!   through in one frame, tick by tick with the same inputs, so the match
//!   and its events are unchanged (`app/play.rs`, `run_ticks`); "Lines up"
//!   is not held, and the postgame opens on every card and the ranking
//!   (`flow/mod.rs`); the Fish-Boy is up and down at once (`fishboy/`); a
//!   minigame on the Minigames screen opens with no instruction banner
//!   (`app/minigames.rs`). A match's own minigames keep their banner: its
//!   ticks are the match's, and input waits on it. The hook window, the
//!   respawn, the buoy interval and the reel stay real; a check that needs
//!   them short uses the `?debug=` flags.
//! - **`?mute`** (`flag.mute`): the master volume is 0 and the sound frame
//!   says `muted`, so the page builds every sound's graph (the Sounds
//!   check reads it) but connects none to the speakers. The reader's
//!   volume setting is not changed or stored.

use super::App;

impl App {
    /// `flag.fast` and `flag.mute`: on, unless `value` is "off".
    pub(super) fn test_flag(&mut self, name: &str, value: &str) {
        let on = value != "off";
        match name {
            "flag.fast" => {
                self.fast = on;
                self.fishboy.fast = on;
            }
            _ => self.mute = on,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::play::tests::{lobby, tick};
    use crate::app::{App, Screen};

    const FLOW: &str = include_str!("../../../../data/flow.json");
    const FISH: &str = include_str!("../../../../data/fish.json");
    const TUNING: &str = include_str!("../../../../data/tuning.json");

    /// A match of four Hard AIs (`?debug=watch` hands this seat to one too)
    /// on the shipped tuning, fast or not.
    fn watched(fast: bool) -> App {
        let mut a = lobby();
        // the shipped tuning, with its 3 s countdown and its fish
        a.act("data.tuning", TUNING).unwrap();
        a.act("data.fish", FISH).unwrap();
        a.act("data.flow", FLOW).unwrap();
        a.act("debug.flags", "watch").unwrap();
        if fast {
            a.act("flag.fast", "").unwrap();
        }
        a.act("lobby.ai", "3").unwrap();
        for k in 1..=3 {
            a.act(&format!("lobby.ai.{k}"), "hard").unwrap();
        }
        // the first frame (`frame`) starts the match's clock
        a.act("lobby.start", "").unwrap();
        a
    }

    /// One frame of 20 ms, and the events the match logged in it, each
    /// with its tick.
    fn frame(a: &mut App, heard: &mut Vec<String>) {
        a.play.as_mut().unwrap().log.clear();
        tick(a);
        let log = &a.play.as_ref().unwrap().log;
        // the log keeps 64; a frame here must log fewer to be read whole
        assert!(log.len() < 64, "{}", log.len());
        for (k, e) in log {
            heard.push(format!("{k} {}", serde_json::to_string(e).unwrap()));
        }
    }

    /// Frames until the match's clock reaches `to` ticks.
    fn run_to(a: &mut App, to: u32, heard: &mut Vec<String>) {
        while a.play.as_ref().unwrap().game.tick < to {
            frame(a, heard);
        }
    }

    #[test]
    fn fast_mode_is_off_unless_the_page_asks() {
        let mut a = watched(false);
        assert!(!a.fast && !a.mute && !a.fishboy.fast);
        tick(&mut a);
        tick(&mut a);
        // 3 s of 20 ms ticks, one of them run (the first frame starts the clock)
        assert_eq!(a.play.as_ref().unwrap().game.countdown, 149);
    }

    #[test]
    fn fast_mode_leaves_a_seeded_match_the_same_tick_for_tick() {
        // 3 min of play after the 3 s countdown
        let to = 9000;
        let (mut slow, mut fast) = (watched(false), watched(true));
        let (mut a, mut b) = (Vec::new(), Vec::new());
        frame(&mut fast, &mut b);
        // one frame in, the countdown's 150 ticks have run, and no clock yet
        let g = &fast.play.as_ref().unwrap().game;
        assert_eq!((g.countdown, g.tick), (0, 0));
        run_to(&mut slow, to, &mut a);
        run_to(&mut fast, to, &mut b);
        // the countdown's three beats and its Go, then the match's own
        let beats = a.iter().filter(|e| e.contains("\"Countdown\"")).count();
        assert_eq!(beats, 3, "{a:?}");
        assert_eq!(a.iter().filter(|e| e.contains("MatchStarted")).count(), 1);
        let caught = a.iter().filter(|e| e.contains("\"Caught\"")).count();
        assert!(caught >= 2, "the AIs landed fish to compare: {a:?}");
        assert_eq!(a, b);
        let (s, f) = (
            &slow.play.as_ref().unwrap().game,
            &fast.play.as_ref().unwrap().game,
        );
        assert_eq!((s.tick, f.tick), (to, to));
        assert_eq!(s.scores, f.scores);
        for seat in 0..4 {
            let (x, y) = (s.boat(seat).unwrap(), f.boat(seat).unwrap());
            assert_eq!((x.at(), x.heading), (y.at(), y.heading), "seat {seat}");
        }
    }

    #[test]
    fn fast_mode_skips_lines_up_the_reveal_and_the_fish_boys_raise() {
        let mut a = watched(true);
        tick(&mut a);
        // the Fish-Boy is all the way up the moment it is raised
        let clock = a.clock;
        a.fishboy.toggle(clock);
        assert_eq!(a.fishboy.amount(clock), crate::gfx::fx::ONE);
        a.fishboy.toggle(clock);
        assert_eq!(a.fishboy.amount(clock), 0);
        a.act("debug.time_left", "40").unwrap();
        for _ in 0..4 {
            tick(&mut a);
        }
        assert!(a.play.as_ref().unwrap().game.ended);
        // no 3 s of "Lines up": the postgame is up, on its ranking
        assert_eq!(a.screen, Screen::Postgame);
        assert!(a.postgame_ranked());
    }

    #[test]
    fn a_minigame_on_its_screen_opens_without_its_banner_in_fast_mode() {
        for (fast, banner) in [(false, true), (true, false)] {
            let mut a = lobby();
            if fast {
                a.act("flag.fast", "").unwrap();
            }
            a.act("menu.back", "").unwrap();
            a.act("menu.minigames", "").unwrap();
            a.act("minigames.play.power_reel", "").unwrap();
            let m = a.minigames.game.as_ref().unwrap();
            assert_eq!(m.in_banner(), banner, "fast {fast}");
        }
    }

    #[test]
    fn mute_sends_the_sounds_nowhere_and_keeps_the_volume_setting() {
        let mut a = lobby();
        a.load_sounds(
            include_str!("../../../../data/sounds.json"),
            include_str!("../../../../data/tuning.json"),
        )
        .unwrap();
        let f = a.sound_frame();
        assert_eq!(
            (f["master"].to_string(), f["muted"].clone()),
            ("0.8".into(), false.into())
        );
        a.act("flag.mute", "").unwrap();
        let f = a.sound_frame();
        assert_eq!(
            (f["master"].to_string(), f["muted"].clone()),
            ("0.0".into(), true.into())
        );
        assert_eq!(a.settings.volume, 80);
    }
}
