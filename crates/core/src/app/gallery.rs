//! The Gallery: each model on a plain floor, turning slowly, in any of its
//! animations, in any player colour or all four side by side. A boat can
//! carry the angler in the poses its anchors name. The page sends the
//! clock (`advance`) and the canvas size (`resize`); the core decides
//! everything drawn.

use serde::Deserialize;

use super::*;
use crate::gfx::fx::{self, deg, ONE, TURN};
use crate::gfx::geom::{facing, v3, Affine, V3};
use crate::gfx::render::{self, Camera, Light, Rgb};
use crate::model::{self, Anim, Kind, Model};
use crate::view::Choice;

/// The players' colours by lobby slot.
pub const PLAYER_COLOURS: [&str; 4] = ["blue", "red", "green", "yellow"];

/// The camera's vertical field of view.
const FOV: i32 = deg(35);
/// How much room the camera leaves round a model, in percent.
const MARGIN: i32 = 112;
/// How far below a fish its floor lies, as a share of its ball's radius.
const FISH_FLOOR: i32 = ONE * 3 / 5;
/// The floor's sides (a flat many-sided disc).
const FLOOR_SIDES: i32 = 32;
/// The view from above looks this far down (straight down would leave the
/// camera no up).
const ABOVE: i32 = deg(88);
/// The four copies stand this far apart, in percent of a copy's width.
const ROW_GAP: i32 = 108;

/// `data/look.json`: the players' colours and the gallery's light.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Look {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// 0–255, by `PLAYER_COLOURS` name
    pub players: std::collections::BTreeMap<String, [i32; 3]>,
    pub gallery: GalleryLook,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GalleryLook {
    pub light: Light,
    pub floor: [i32; 3],
    /// one turn of the turntable, in seconds
    pub turn_seconds: i32,
}

impl Look {
    pub fn parse(json: &str) -> Result<Look, String> {
        let look: Look =
            serde_json::from_str(json).map_err(|e| format!("The look file could not be read: {e}"))?;
        for name in PLAYER_COLOURS {
            match look.players.get(name) {
                None => return Err(format!("The look file has no player colour \"{name}\".")),
                Some(c) if c.iter().any(|v| !(0..=255).contains(v)) => {
                    return Err(format!("The player colour \"{name}\" is outside 0 to 255."))
                }
                Some(_) => {}
            }
        }
        if let Some(extra) = look
            .players
            .keys()
            .find(|k| !PLAYER_COLOURS.contains(&k.as_str()))
        {
            return Err(format!(
                "The look file has a player colour \"{extra}\" no slot uses."
            ));
        }
        look.gallery.light.check()?;
        if look.gallery.turn_seconds <= 0 {
            return Err("The gallery's turn must take some seconds.".into());
        }
        Ok(look)
    }

    fn player(&self, slot: usize) -> Rgb {
        render::rgb(self.players[PLAYER_COLOURS[slot]])
    }
}

/// What the gallery shows the model doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Show {
    Still,
    Anim(Anim),
    /// the angler in this pose at the anchor of the same name
    Rider(Anim),
}

impl Show {
    fn key(self) -> String {
        match self {
            Show::Still => "still".into(),
            Show::Anim(a) => format!("anim.{}", a.key()),
            Show::Rider(a) => format!("rider.{}", a.key()),
        }
    }
}

/// The gallery's choices and its turntable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gallery {
    pub model: usize,
    pub show: Show,
    /// a slot of `PLAYER_COLOURS`, or `None` for all four side by side
    pub colour: Option<usize>,
    pub turning: bool,
    /// the turntable's angle at `turn_from`
    pub yaw: i32,
    pub turn_from: i64,
    /// when the animation started
    pub anim_from: i64,
    /// looking straight down, as from a boat, not from the model's angle
    pub above: bool,
}

impl Default for Gallery {
    fn default() -> Gallery {
        Gallery {
            model: 0,
            show: Show::Still,
            colour: Some(0),
            turning: true,
            yaw: 0,
            turn_from: 0,
            anim_from: 0,
            above: false,
        }
    }
}

/// Millimetres as feet to two places: 304.8 mm to the foot, rounded.
pub fn feet(mm: i32) -> String {
    let hundredths = (mm as i64 * 1000 + 1524) / 3048;
    format!("{}.{:02}", hundredths / 100, hundredths % 100)
}

impl App {
    /// Takes the models and the look, checked, or says why not.
    pub fn load_models(&mut self, models_json: &str, look_json: &str) -> Result<(), String> {
        let models = model::parse_all(models_json)?;
        let look = Look::parse(look_json)?;
        self.models = models;
        self.look = Some(look);
        self.open_model(0);
        Ok(())
    }

    /// The angler: the first figure.
    fn rider(&self) -> Option<&Model> {
        self.models.iter().find(|m| m.kind == Kind::Figure)
    }

    /// What the current model can show: still, each of its animations,
    /// and the rider in each pose this model has an anchor for.
    pub fn shows(&self) -> Vec<Show> {
        let Some(m) = self.models.get(self.gallery.model) else {
            return vec![Show::Still];
        };
        let mut out = vec![Show::Still];
        out.extend(m.animations.iter().map(|a| Show::Anim(a.anim)));
        if let Some(r) = self.rider() {
            for a in &r.animations {
                if m.kind != Kind::Figure && m.anchor(a.anim.key()).is_some() {
                    out.push(Show::Rider(a.anim));
                }
            }
        }
        out
    }

    /// Opens a model at its own angle, still.
    fn open_model(&mut self, i: usize) {
        let g = &mut self.gallery;
        g.model = i;
        g.show = Show::Still;
        g.anim_from = self.clock;
        g.turn_from = self.clock;
        g.yaw = self.models.get(i).map_or(0, |m| m.yaw);
    }

    /// The turntable's angle now.
    pub fn gallery_yaw(&self) -> i32 {
        let g = &self.gallery;
        if !g.turning {
            return g.yaw;
        }
        let secs = self.look.as_ref().map_or(1, |l| l.gallery.turn_seconds) as i64;
        let turned = (self.clock - g.turn_from) * TURN as i64 / (secs * 1_000_000);
        fx::wrap(g.yaw + (turned % TURN as i64) as i32)
    }

    /// Whether the current choices put the player's colour on screen.
    fn shows_colour(&self) -> bool {
        let Some(m) = self.models.get(self.gallery.model) else {
            return false;
        };
        m.has_slot()
            || matches!(self.gallery.show, Show::Rider(_)) && self.rider().is_some_and(|r| r.has_slot())
    }

    /// The Gallery's controls (`gallery.*`).
    pub(super) fn gallery_act(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            "gallery.model" => {
                let i = self
                    .models
                    .iter()
                    .position(|m| m.id == value)
                    .ok_or_else(|| format!("There is no model \"{value}\"."))?;
                self.open_model(i);
            }
            "gallery.show" => {
                let show = self
                    .shows()
                    .into_iter()
                    .find(|s| s.key() == value)
                    .ok_or_else(|| format!("This model cannot show \"{value}\"."))?;
                self.gallery.show = show;
                self.gallery.anim_from = self.clock;
            }
            "gallery.colour" => {
                self.gallery.colour = match value {
                    "all" => None,
                    _ => Some(
                        PLAYER_COLOURS
                            .iter()
                            .position(|c| *c == value)
                            .ok_or_else(|| format!("There is no player colour \"{value}\"."))?,
                    ),
                };
            }
            "gallery.view" => {
                self.gallery.above = match value {
                    "model" => false,
                    "above" => true,
                    _ => return Err(format!("There is no view \"{value}\".")),
                };
            }
            "gallery.turn" => {
                self.gallery.yaw = self.gallery_yaw();
                self.gallery.turn_from = self.clock;
                self.gallery.turning = value == "on";
            }
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    fn show_label(&self, s: Show) -> String {
        match s {
            Show::Still => self.t("gallery.still"),
            Show::Anim(a) => self.t(&format!("anim.{}", a.key())),
            Show::Rider(a) => self
                .t("gallery.rider")
                .replace(
                    "{figure}",
                    &self.rider().map_or(String::new(), |r| r.name.clone()),
                )
                .replace("{pose}", &self.t(&format!("anim.{}", a.key()))),
        }
    }

    pub(super) fn gallery_panels(&self) -> Vec<Panel> {
        let g = &self.gallery;
        let Some(m) = self.models.get(g.model) else {
            let mut p = Panel::new(
                "gallery",
                self.t("gallery.heading"),
                vec![self.button("menu.back")],
            );
            p.sub = self.t("gallery.none");
            return vec![p];
        };
        let choice = |value: String, text: String| Choice {
            value,
            text,
            disabled: false,
        };
        let mut items = vec![
            Item::Select {
                id: "gallery.model".into(),
                label: self.t("gallery.model"),
                value: m.id.clone(),
                options: self
                    .models
                    .iter()
                    .map(|m| choice(m.id.clone(), m.name.clone()))
                    .collect(),
            },
            Item::Select {
                id: "gallery.show".into(),
                label: self.t("gallery.show"),
                value: g.show.key(),
                options: self
                    .shows()
                    .into_iter()
                    .map(|s| choice(s.key(), self.show_label(s)))
                    .collect(),
            },
        ];
        if self.shows_colour() {
            let mut options: Vec<Choice> = PLAYER_COLOURS
                .iter()
                .map(|c| choice(c.to_string(), self.t(&format!("colour.{c}"))))
                .collect();
            options.push(choice("all".into(), self.t("gallery.colour.all")));
            items.push(Item::Select {
                id: "gallery.colour".into(),
                label: self.t("gallery.colour"),
                value: g.colour.map_or("all".into(), |c| PLAYER_COLOURS[c].to_string()),
                options,
            });
        }
        items.push(Item::Select {
            id: "gallery.view".into(),
            label: self.t("gallery.view"),
            value: if g.above { "above" } else { "model" }.into(),
            options: vec![
                choice("model".into(), self.t("gallery.view.model")),
                choice("above".into(), self.t("gallery.view.above")),
            ],
        });
        items.push(Item::Toggle {
            id: "gallery.turn".into(),
            label: self.t("gallery.turn"),
            value: g.turning,
        });
        items.push(self.button("menu.back"));
        let mut p = Panel::new("gallery", m.name.clone(), items);
        let (key, size) = match m.kind {
            Kind::Figure | Kind::Prop => ("gallery.tall", m.height_mm()),
            _ => ("gallery.long", m.length_mm()),
        };
        p.sub = self.t(key).replace("{feet}", &feet(size));
        let mut panels = vec![p];
        if self.scene_unavailable {
            panels.push(Panel::new("notice", self.t("gallery.no_scene"), Vec::new()));
        }
        panels
    }

    /// This frame's view values and triangles (empty off the Gallery).
    pub fn frame(&self) -> (Vec<i32>, Vec<i32>) {
        let (Some(look), Some(m)) = (self.look.as_ref(), self.models.get(self.gallery.model)) else {
            return (Vec::new(), Vec::new());
        };
        if self.screen != Screen::Gallery {
            return (Vec::new(), Vec::new());
        }
        let g = &self.gallery;
        let ms = ((self.clock - g.anim_from).max(0) / 1000 % model::MAX_MS as i64) as u32;
        let (anim, rider) = match g.show {
            Show::Still => (None, None),
            Show::Anim(a) => (Some(a), None),
            Show::Rider(a) => (
                None,
                self.rider().zip(m.anchor(a.key())).map(|(r, at)| (r, at, a)),
            ),
        };
        let transforms = m.transforms(&m.poses(anim, ms));
        let (centre, radius) = m.ball();
        let slots: Vec<usize> = match g.colour {
            Some(c) => vec![c],
            None => (0..PLAYER_COLOURS.len()).collect(),
        };
        // the copies stand in a row across the screen, each turning about
        // its own middle
        let width = (radius * 2) / 100 * ROW_GAP;
        let count = slots.len() as i32;
        let yaw = self.gallery_yaw();
        let mut tris = Vec::new();
        for (k, slot) in slots.iter().enumerate() {
            let across = width * (2 * k as i32 - (count - 1)) / 2;
            let middle = v3(centre.x, 0, centre.z);
            let place = Affine::translate(v3(across, 0, 0))
                .then(&Affine::translate(middle))
                .then(&Affine::rot_y(yaw))
                .then(&Affine::translate(middle.neg()));
            let colours = m.colours(look.player(*slot));
            m.draw(&transforms, &place, &colours, &mut tris);
            if let Some((r, at, a)) = rider {
                let seat = place.then(&m.anchor_frame(at, &transforms));
                let rt = r.transforms(&r.poses(Some(a), ms));
                r.draw(&rt, &seat, &r.colours(look.player(*slot)), &mut tris);
            }
        }

        // the camera, fitted to the row
        let aspect = render::aspect(self.size);
        let row_half = width * (count - 1) / 2 + radius;
        let fit = radius.max(fx::div(row_half, aspect.max(ONE)));
        let dist = Camera::fit(FOV, fit, MARGIN);
        let fwd = facing(fx::HALF, -if g.above { ABOVE } else { m.pitch });
        let cam = Camera {
            eye: centre.sub(fwd.scale(dist)),
            fwd,
            fov: FOV,
            near: dist / 50,
            far: dist * 8,
        };

        // the floor: under a fish, a little below it
        let mut floor_y = mm_y(m.min_mm[1]);
        if m.kind == Kind::Fish {
            floor_y -= fx::mul(radius, FISH_FLOOR);
        }
        push_floor(
            &mut tris,
            v3(centre.x, floor_y, centre.z),
            dist * 4,
            render::rgb(look.gallery.floor),
        );

        let fog = (dist + radius, dist * 4);
        (render::view_values(&cam, aspect, fog, &look.gallery.light), tris)
    }
}

fn mm_y(y: i32) -> i32 {
    model::mm([0, y, 0]).y
}

/// A flat disc of `radius` round `at`, facing up.
fn push_floor(out: &mut Vec<i32>, at: V3, radius: i32, c: Rgb) {
    let up = V3::UP;
    let rim = |k: i32| {
        let a = TURN * k / FLOOR_SIDES;
        at.add(v3(fx::mul(fx::cos(a), radius), 0, fx::mul(fx::sin(a), radius)))
    };
    for k in 0..FLOOR_SIDES {
        render::push_tri(out, [at, rim(k + 1), rim(k)], up, c);
    }
}

#[cfg(test)]
mod tests;
