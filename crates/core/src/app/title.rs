//! The menus' backdrop: Burt Lake drawn live behind the Title and the other
//! menu screens, the camera drifting slowly along the lake's tour with a
//! boat or two cruising ahead of it, in the lake's own fog.
//!
//! Every number is in `data/look.json` `"title"` ([`TitleLook`]); without
//! that section, or before the map list and the models are in, the menus
//! have no backdrop and the page draws its plain background.

use serde::Deserialize;

use super::*;
use crate::gfx::fx::{self, deg, TURN};
use crate::gfx::geom::Affine;
use crate::gfx::render::{self, Frame};
use crate::map::{trig, Setting, P};
use crate::model::{Anim, Kind};
use crate::scene::{self, camera, tour, yaw_of_heading, Instance};

/// `data/look.json` `"title"`: how the backdrop's camera drifts and where
/// its boats cruise.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitleLook {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// which checked map the backdrop shows before any other has been
    /// drawn: its place in the map list
    pub map_index: usize,
    /// the camera's point moves along the tour at this share of the tour's
    /// speed (`lake.camera.tour_mm_per_s`)
    pub drift_pct: i32,
    /// where along the tour the drift starts, in ms of the tour
    pub start_ms: i32,
    /// the camera stands `back_mm` behind its point and `pivot_mm` over
    /// it, `pitch_deg` down, and aims `ahead_mm` beyond it and `aim_mm` up
    pub back_mm: i32,
    pub pivot_mm: i32,
    pub pitch_deg: i32,
    pub ahead_mm: i32,
    pub aim_mm: i32,
    /// the camera sways round its point by up to `sway_deg` either side,
    /// once each `sway_ms`
    pub sway_deg: i32,
    pub sway_ms: i32,
    /// the boats: each `lead_ms` of the tour ahead of the camera's point
    /// (negative: behind), `side_mm` to the left of the tour (negative:
    /// right), in a player colour by slot
    pub boats: Vec<TitleBoat>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitleBoat {
    pub lead_ms: i32,
    pub side_mm: i32,
    pub slot: usize,
}

impl TitleLook {
    pub fn check(&self) -> Result<(), String> {
        let bad = |what: &str| Err(format!("The title backdrop's {what} is out of range."));
        if !(0..=100).contains(&self.drift_pct) {
            return bad("drift_pct");
        }
        if self.sway_ms <= 0 {
            return bad("sway_ms");
        }
        if self.back_mm <= 0 || self.pivot_mm < 0 {
            return bad("camera distance");
        }
        if self.boats.iter().any(|b| b.slot >= PLAYER_SLOTS) {
            return bad("boat slot");
        }
        Ok(())
    }
}

/// How many player colours a backdrop boat may wear.
const PLAYER_SLOTS: usize = crate::app::gallery::PLAYER_COLOURS.len();

/// What the backdrop needs that is not in the look file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Backdrop {
    /// the backdrop's world could not be built (no look, no maps); tried
    /// once, not on every action
    pub failed: bool,
}

impl App {
    /// The screens drawn over the lake backdrop: the menus and the
    /// Minigames screen (the Gallery and the Lake preview draw their own).
    pub fn backdrop(&self) -> bool {
        matches!(
            self.screen,
            Screen::Title
                | Screen::Settings
                | Screen::Notes
                | Screen::PlayMenu
                | Screen::Lobby
                | Screen::Online
                | Screen::OnlineLobby
                | Screen::Sounds
                // its list, and a minigame's 2D layer over the lake
                | Screen::Minigames
        )
    }

    fn title_look(&self) -> Option<&TitleLook> {
        self.look.as_ref()?.title.as_ref()
    }

    /// Builds a world for the backdrop when a menu screen is up and none is
    /// built: the look's map from the checked list. Whatever lake was
    /// drawn last (a match's, the Lake preview's) is kept instead.
    pub(super) fn ensure_backdrop(&mut self) {
        if !self.backdrop() || self.world.is_some() || self.title_backdrop.failed || self.models.is_empty() {
            return;
        }
        let (Some(t), Some(list)) = (self.title_look(), self.maps.as_ref()) else {
            return;
        };
        let Some(seed) = list.maps.get(t.map_index).or(list.maps.first()).map(|m| m.seed) else {
            return;
        };
        let map = crate::map::generate(Setting::MichiganLake, seed);
        if self.show_world(&map).is_err() {
            self.title_backdrop.failed = true;
        }
    }

    /// The backdrop's frame: the lake, the drifting camera and the boats.
    pub fn title_frame(&self) -> Frame {
        let (Some(lake), Some(look), Some(t)) = (self.world.as_ref(), self.look.as_ref(), self.title_look())
        else {
            return Frame::default();
        };
        let clock_ms = (self.clock.max(0) / 1000) as u64;
        // the camera's point, on the tour at the drift's pace
        let along = |lead_ms: i32| {
            let ms = t.start_ms as i64 + clock_ms as i64 * t.drift_pct as i64 / 100 + lead_ms as i64;
            tour::pose(lake, ms.max(0) as u64)
        };
        let (at, heading) = along(0);
        let phase = ((clock_ms % t.sway_ms as u64) as i64 * TURN as i64 / t.sway_ms as i64) as i32;
        let sway = fx::mul(deg(t.sway_deg), fx::sin(phase));
        let mut cam_look = lake.look.camera.clone();
        cam_look.back_mm = t.back_mm;
        cam_look.pivot_mm = t.pivot_mm;
        cam_look.pitch_deg = t.pitch_deg;
        cam_look.ahead_mm = t.ahead_mm;
        cam_look.aim_mm = t.aim_mm;
        cam_look.min_back_mm = cam_look.min_back_mm.min(t.back_mm);
        let cam = camera::boat(
            lake,
            &cam_look,
            lake.world(at, 0),
            yaw_of_heading(heading),
            sway,
            0,
        );

        let mut instances = Vec::new();
        let boat = self.models.iter().find(|m| m.kind == Kind::Boat);
        let angler = self.models.iter().find(|m| m.kind == Kind::Figure);
        let anim_ms = (clock_ms % 60_000) as u32;
        for b in &t.boats {
            let Some(boat) = boat else { break };
            let (p, h) = along(b.lead_ms);
            let left = h + trig::QUARTER;
            let p = P::new(
                p.x + (trig::cos(left) as i64 * b.side_mm as i64 / trig::ONE as i64) as i32,
                p.y + (trig::sin(left) as i64 * b.side_mm as i64 / trig::ONE as i64) as i32,
            );
            let colour = look.player(b.slot);
            let on = Instance {
                model: boat,
                place: Affine::translate(lake.world(p, 0)).then(&Affine::rot_y(yaw_of_heading(h))),
                anim: None,
                ms: 0,
                colour,
            };
            instances.push(on);
            if let (Some(a), Some(seat)) = (angler, scene::seat(&on, "drive")) {
                instances.push(Instance {
                    model: a,
                    place: seat,
                    anim: Some(Anim::Drive),
                    ms: anim_ms,
                    colour,
                });
            }
        }
        scene::frame(
            lake,
            cam,
            render::aspect(self.size),
            self.sight_mm(),
            clock_ms,
            &instances,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> TitleLook {
        TitleLook {
            note: String::new(),
            map_index: 0,
            drift_pct: 30,
            start_ms: 0,
            back_mm: 20000,
            pivot_mm: 4000,
            pitch_deg: 8,
            ahead_mm: 10000,
            aim_mm: 1000,
            sway_deg: 40,
            sway_ms: 90000,
            boats: vec![TitleBoat {
                lead_ms: 1500,
                side_mm: 3000,
                slot: 1,
            }],
        }
    }

    #[test]
    fn the_title_look_refuses_a_drift_past_the_tour_or_a_fifth_colour() {
        assert!(look().check().is_ok());
        let mut fast = look();
        fast.drift_pct = 101;
        assert_eq!(
            fast.check().unwrap_err(),
            "The title backdrop's drift_pct is out of range."
        );
        let mut fifth = look();
        fifth.boats[0].slot = 4;
        assert!(fifth.check().unwrap_err().contains("boat slot"));
    }

    #[test]
    fn the_menus_have_the_backdrop_and_the_debug_screens_their_own_scene() {
        let mut a = crate::app::tests::app();
        assert!(
            a.backdrop() && a.ui().scene && a.ui().backdrop,
            "the Title is drawn over the lake"
        );
        a.act("menu.settings", "").unwrap();
        assert!(a.ui().backdrop, "and Settings");
        a.act("menu.back", "").unwrap();
        a.act("menu.gallery", "").unwrap();
        assert!(
            !a.ui().backdrop && a.ui().scene,
            "the Gallery draws its own scene"
        );
    }

    #[test]
    fn no_backdrop_is_drawn_without_its_look_or_a_map() {
        // the test app has no look, maps or models: nothing to draw, and
        // no world is built
        let mut a = crate::app::tests::app();
        a.act("menu.notes", "").unwrap();
        assert!(a.world.is_none());
        assert!(a.title_frame().tris.is_empty());
    }
}
