//! The Lake preview: a debug screen, opened from the Title, that draws one
//! of the setting's checked maps with a camera tour or a fixed view, for
//! screenshots and for choosing the fog distance by eye. The fog's
//! distance and whether it is on are set live here (`lake.fog`,
//! `lake.fog_on`); [`App::sight_mm`] gives the distance the world is drawn
//! with, which starts at the map's own `sight_mm`.
//!
//! A boat (with its angler at the wheel) stands where the view puts it, so
//! the boat camera has something to follow; the Under view adds a few fish
//! below the surface and the Spawn view the item buoy. They are drawn
//! through the same [`scene::Instance`] list a match's boats will use.

use super::*;
use crate::gfx::fx::{self, deg, HALF, ONE};
use crate::gfx::geom::{v3, Affine, V3};
use crate::gfx::render::{self, Frame};
use crate::map::list::{self, MapList};
use crate::map::{trig, Setting, P};
use crate::model::{Anim, Kind};
use crate::scene::{self, camera, tour, yaw_of_heading, Instance, Lake};
use crate::view::Choice;

/// The fog slider's range and step, in metres.
pub const FOG_MIN_M: u32 = 30;
pub const FOG_MAX_M: u32 = 400;
pub const FOG_STEP_M: u32 = 10;

/// What the preview's camera does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// round the lake on the boat camera
    Tour,
    /// the driver's eye at the spawn point, facing across the lake
    Spawn,
    /// the boat camera at the spawn point, turned back to the dock
    Dock,
    /// the boat camera by the largest island
    Island,
    /// low over the largest reed bed
    Reeds,
    /// down through the surface at fish under it
    Under,
    /// high over the whole lake
    Above,
}

impl View {
    pub const ALL: [View; 7] = [
        View::Tour,
        View::Spawn,
        View::Dock,
        View::Island,
        View::Reeds,
        View::Under,
        View::Above,
    ];

    pub fn key(self) -> &'static str {
        match self {
            View::Tour => "tour",
            View::Spawn => "spawn",
            View::Dock => "dock",
            View::Island => "island",
            View::Reeds => "reeds",
            View::Under => "under",
            View::Above => "above",
        }
    }
}

/// The preview's choices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    pub list: Option<MapList>,
    pub seed: Option<u32>,
    pub view: View,
    pub fog_on: bool,
    /// the fog distance set here, or the map's own
    pub fog_m: Option<u32>,
    /// when the tour started (µs)
    pub from: i64,
    /// why the lake cannot be drawn, if it cannot
    pub problem: Option<String>,
}

impl Default for Preview {
    fn default() -> Preview {
        Preview {
            list: None,
            seed: None,
            view: View::Tour,
            fog_on: true,
            fog_m: None,
            from: 0,
            problem: None,
        }
    }
}

/// Fish under the surface for the Under view: (model, decimetres ahead of
/// the boat, decimetres to its left, depth in mm, heading turn in
/// degrees), a pike over a school of perch, and a bass.
const UNDER_FISH: &[(&str, i32, i32, i32, i32)] = &[
    ("northern-pike", 36, 3, 350, 20),
    ("yellow-perch", 30, -8, 600, -40),
    ("yellow-perch", 34, -11, 650, -35),
    ("yellow-perch", 27, -12, 700, -45),
    ("smallmouth-bass", 44, 10, 900, 120),
];

impl App {
    /// Takes the setting's checked map list (`data/maps/<setting>.json`).
    pub fn load_maps(&mut self, json: &str) -> Result<(), String> {
        let list = list::parse(json)?;
        self.preview.seed = list.maps.first().map(|m| m.seed);
        self.preview.list = Some(list);
        Ok(())
    }

    /// The fog distance the world is drawn with, in mm: `None` when the
    /// fog is off.
    pub fn sight_mm(&self) -> Option<i32> {
        if !self.preview.fog_on {
            return None;
        }
        let own = self
            .world
            .as_ref()
            .map_or(Setting::MichiganLake.params().sight_mm, |l| l.sight_mm);
        Some(self.preview.fog_m.map_or(own, |m| m as i32 * 1000))
    }

    /// Builds the chosen map's world, or records why it cannot.
    fn build_preview(&mut self) {
        let lake_look = self.look.as_ref().and_then(|l| l.lake.as_ref());
        let made = match (self.preview.seed, lake_look) {
            (Some(seed), Some(look)) => {
                if self.world.as_ref().is_some_and(|w| w.map.seed == seed) {
                    return;
                }
                let map = crate::map::generate(Setting::MichiganLake, seed);
                Lake::build(&map, &self.models, look)
            }
            _ => Err(self.t("lake.none")),
        };
        match made {
            Ok(lake) => {
                self.world = Some(lake);
                self.preview.problem = None;
            }
            Err(e) => {
                self.world = None;
                self.preview.problem = Some(e);
            }
        }
    }

    pub(super) fn open_lake(&mut self) {
        self.go(Screen::Lake);
        self.preview.from = self.clock;
        self.build_preview();
    }

    /// The preview's controls (`lake.*`).
    pub(super) fn lake_act(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            "lake.seed" => {
                let seed: u32 = value
                    .parse()
                    .ok()
                    .filter(|s| {
                        self.preview
                            .list
                            .as_ref()
                            .is_some_and(|l| l.maps.iter().any(|m| m.seed == *s))
                    })
                    .ok_or_else(|| format!("There is no checked map seed \"{value}\"."))?;
                self.preview.seed = Some(seed);
                self.build_preview();
            }
            "lake.view" => {
                self.preview.view = View::ALL
                    .into_iter()
                    .find(|v| v.key() == value)
                    .ok_or_else(|| format!("There is no lake view \"{value}\"."))?;
                self.preview.from = self.clock;
            }
            "lake.fog" => {
                let m: u32 = value
                    .parse()
                    .map_err(|_| format!("The fog distance \"{value}\" is not a whole number."))?;
                let m = m.clamp(FOG_MIN_M, FOG_MAX_M);
                self.preview.fog_m = Some((m - FOG_MIN_M) / FOG_STEP_M * FOG_STEP_M + FOG_MIN_M);
            }
            "lake.fog_on" => self.preview.fog_on = value == "on",
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    pub(super) fn lake_panels(&self) -> Vec<Panel> {
        let p = &self.preview;
        let mut items = Vec::new();
        if let Some(list) = &p.list {
            items.push(Item::Select {
                id: "lake.seed".into(),
                label: self.t("lake.seed"),
                value: p.seed.map_or(String::new(), |s| s.to_string()),
                options: list
                    .maps
                    .iter()
                    .map(|m| Choice {
                        value: m.seed.to_string(),
                        text: m.seed.to_string(),
                        disabled: false,
                    })
                    .collect(),
            });
        }
        items.push(Item::Select {
            id: "lake.view".into(),
            label: self.t("lake.view"),
            value: p.view.key().into(),
            options: View::ALL
                .iter()
                .map(|v| Choice {
                    value: v.key().into(),
                    text: self.t(&format!("lake.view.{}", v.key())),
                    disabled: false,
                })
                .collect(),
        });
        items.push(Item::Toggle {
            id: "lake.fog_on".into(),
            label: self.t("lake.fog_on"),
            value: p.fog_on,
        });
        let own = self
            .world
            .as_ref()
            .map_or(Setting::MichiganLake.params().sight_mm, |l| l.sight_mm);
        let metres = p.fog_m.unwrap_or((own / 1000) as u32);
        items.push(Item::Slider {
            id: "lake.fog".into(),
            label: self.t("lake.fog"),
            min: FOG_MIN_M,
            max: FOG_MAX_M,
            step: FOG_STEP_M,
            value: metres,
            text: self.t("lake.metres").replace("{m}", &metres.to_string()),
        });
        items.push(self.button("menu.back"));
        let mut panel = Panel::new("lake", self.t("lake.heading"), items);
        panel.sub = match &p.problem {
            Some(e) => e.clone(),
            None => self.t(Setting::MichiganLake.name_key()),
        };
        let mut panels = vec![panel];
        if self.scene_unavailable {
            panels.push(Panel::new("notice", self.t("gallery.no_scene"), Vec::new()));
        }
        panels
    }

    fn model(&self, id: &str) -> Option<&Model> {
        self.models.iter().find(|m| m.id == id)
    }

    /// The preview's frame: the world, its camera and its boat.
    pub fn lake_frame(&self) -> Frame {
        let (Some(lake), Some(look)) = (self.world.as_ref(), self.look.as_ref()) else {
            return Frame::default();
        };
        let ms = ((self.clock - self.preview.from).max(0) / 1000) as u64;
        let clock_ms = (self.clock.max(0) / 1000) as u64;
        let cam_look = &lake.look.camera;
        let map = &lake.map;
        let spawn = map.spawn;
        let near_tour = |t: P| {
            let k = (0..lake.tour.len())
                .min_by_key(|&k| lake.tour[k].dist2(t))
                .unwrap_or(0);
            lake.tour.get(k).copied().unwrap_or(spawn.at)
        };
        let heading_to = |a: P, b: P| trig::atan2((b.y - a.y) as i64, (b.x - a.x) as i64);

        // where the boat is, which way it faces, and the camera's turn from
        // straight behind it
        let (boat_at, heading, turn) = match self.preview.view {
            View::Tour => {
                let (p, h) = tour::pose(lake, ms);
                (p, h, 0)
            }
            View::Spawn | View::Above => (spawn.at, spawn.heading, 0),
            View::Dock => (spawn.at, spawn.heading, HALF),
            View::Island => {
                let island = map
                    .islands
                    .iter()
                    .max_by_key(|i| i.area_m2)
                    .map_or(spawn.at, |i| i.at);
                let at = near_tour(island);
                (at, heading_to(at, island) - deg(35), deg(20))
            }
            View::Reeds | View::Under => {
                let reeds = map
                    .reeds
                    .iter()
                    .max_by_key(|r| r.clumps.len())
                    .map_or(spawn.at, |r| r.at);
                let at = near_tour(reeds);
                (at, heading_to(at, reeds), 0)
            }
        };
        let boat_yaw = yaw_of_heading(heading);
        let target = lake.world(boat_at, 0);
        // a point decimetres ahead of the boat and to its left
        let ahead = |dm: i32, left: i32| {
            let f = crate::gfx::geom::facing(boat_yaw, 0);
            let l = v3(f.z, 0, -f.x);
            target.add(f.scale(dm * ONE / 10)).add(l.scale(left * ONE / 10))
        };
        let cam = match self.preview.view {
            View::Reeds => camera::boat(lake, cam_look, target, boat_yaw, turn, -deg(6)),
            // the driver's eye: over the stern seat, looking ahead
            View::Spawn => {
                let pitch = deg(4);
                let eye = ahead(-14, 0).add(v3(0, scene::mm(1900), 0));
                let reach = fx::int(20);
                let centre = eye.add(crate::gfx::geom::facing(boat_yaw, -pitch).scale(reach));
                camera::fly_over(cam_look, centre, boat_yaw, pitch, reach)
            }
            View::Under => camera::fly_over(cam_look, ahead(32, 0), boat_yaw, deg(56), fx::ratio(7, 2)),
            View::Above => {
                let pitch = deg(cam_look.above_pitch_deg);
                let dist = fx::div(scene::mm(cam_look.above_mm), fx::sin(pitch));
                camera::fly_over(cam_look, V3::ZERO, boat_yaw + HALF / 4, pitch, dist)
            }
            _ => camera::boat(lake, cam_look, target, boat_yaw, turn, 0),
        };

        // the boat and its angler, the buoy, and fish under the water
        let colour = look.player(0);
        let mut instances = Vec::new();
        if let Some(boat) = self.models.iter().find(|m| m.kind == Kind::Boat) {
            let b = Instance {
                model: boat,
                place: Affine::translate(target).then(&Affine::rot_y(boat_yaw)),
                anim: None,
                ms: 0,
                colour,
            };
            instances.push(b);
            if let (Some(angler), Some(seat)) = (
                self.models.iter().find(|m| m.kind == Kind::Figure),
                scene::seat(&b, "drive"),
            ) {
                instances.push(Instance {
                    model: angler,
                    place: seat,
                    anim: Some(Anim::Drive),
                    ms: (ms % 60_000) as u32,
                    colour,
                });
            }
        }
        if let (View::Spawn | View::Tour, Some(buoy)) = (self.preview.view, self.model("item-buoy")) {
            let at = lake.world(scene_ahead(spawn.at, spawn.heading, 30_000), 0);
            instances.push(Instance {
                model: buoy,
                place: Affine::translate(at),
                anim: Some(Anim::Bob),
                ms: (clock_ms % 60_000) as u32,
                colour,
            });
        }
        if self.preview.view == View::Under {
            for &(id, fwd, left, depth, turn) in UNDER_FISH {
                if let Some(fish) = self.model(id) {
                    let at = ahead(fwd, left).add(v3(0, -scene::mm(depth), 0));
                    instances.push(Instance {
                        model: fish,
                        place: Affine::translate(at).then(&Affine::rot_y(boat_yaw + deg(turn))),
                        anim: Some(Anim::Idle),
                        ms: (clock_ms % 60_000) as u32,
                        colour,
                    });
                }
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
mod tests;

/// A map point `d` mm from `p` toward a map heading.
fn scene_ahead(p: P, heading: i32, d: i32) -> P {
    P::new(
        p.x + (trig::cos(heading) as i64 * d as i64 / trig::ONE as i64) as i32,
        p.y + (trig::sin(heading) as i64 * d as i64 / trig::ONE as i64) as i32,
    )
}
