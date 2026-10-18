//! Fishing on screen: the fish under the water, the bobbers, lures and
//! lines, the underwater camera with its picture-in-picture of the boat,
//! and the fishing HUD (the charge meter, the reel bar, the popups).
//!
//! Everything here reads the seat's [`FishingView`] (`sim::fishing::view`)
//! and draws it; no rule is decided here. The drawing's numbers are the
//! `fishing` section of `data/look.json` ([`FishingLook`]).
//!
//! - **Fish** from the boat camera are drawn `fish_scale_pct` of life size
//!   and at `fish_depth_pct` of their depth, so a fish a few tens of
//!   centimetres long reads from the chase camera nine metres back
//!   (SECOND-ORDER R2.38); under the water they are life size at their true
//!   depth. A fish on a line plays its panic swim.
//! - **The underwater camera**: with this seat's line in the water, the
//!   main view looks at the lure from `under.back_mm` behind it, turned by
//!   the mouse, in a short green-blue fog; the boat camera's view goes into
//!   a picture-in-picture in the top-left corner ([`App::inset_frame`]).
//! - **The HUD** is a stand-in made of panels (`Item::Meter`) until the 2D
//!   layer draws it: the charge meter while Space is held, the reel bar and
//!   the catch meter while reeling, and a popup for 2 s for each hook (every
//!   player's) and for this seat's misses, breaks and catches.

use serde::Deserialize;

use super::play::PlayView;
use super::*;
use crate::gfx::fx::{self, deg, ONE};
use crate::gfx::geom::{facing, v3, Affine, V3};
use crate::gfx::render::{self, Air, Camera, Frame, Light, Rgb};
use crate::map::P;
use crate::model::Anim;
use crate::scene::{self, mm, yaw_of_heading, Instance, Lake};
use crate::sim::fishing::view::{FishSeen, FishingView};
use crate::sim::fishing::Stage;
use crate::sim::{MatchEvent, Mode, TICK_MS};

/// `data/look.json`'s `fishing` section.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishingLook {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// fish seen from the boat camera: their size and depth drawn, percent
    pub fish_scale_pct: i32,
    pub fish_depth_pct: i32,
    pub bobber_scale_pct: i32,
    pub lure_scale_pct: i32,
    /// the line's width drawn from the boat camera and under the water
    pub line_mm: i32,
    pub line_under_mm: i32,
    pub line: [i32; 3],
    /// a landed fish hangs this far under the rod's tip
    pub landed_drop_mm: i32,
    pub under: UnderLook,
    pub pip: PipLook,
}

/// The underwater camera.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnderLook {
    /// the murk's colour, and the colour looking up toward the light
    pub colour: [i32; 3],
    pub zenith: [i32; 3],
    /// the murk's end, and where it starts as a share of that
    pub fog_mm: i32,
    pub fog_start_pct: i32,
    /// the water's own tint: how deep it takes to reach full, and its most
    pub clear_mm: i32,
    pub tint_pct: i32,
    /// behind the lure and above it, and how far it looks down
    pub back_mm: i32,
    pub up_mm: i32,
    pub pitch_deg: i32,
    pub fov_deg: i32,
    /// the camera stays this far under the surface and over the bed
    pub clear_surface_mm: i32,
    pub clear_bed_mm: i32,
}

/// The picture-in-picture of the boat.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PipLook {
    /// its width as a share of the screen's, its height over its width,
    /// and its gap from the corner, in percent of the screen's width, and
    /// its border in thousandths of it
    pub width_pct: i32,
    pub tall_pct: i32,
    pub margin_pct: i32,
    pub border_permille: i32,
}

/// Popups last this long (the person: about 2 s).
const POPUP_MS: u32 = 2000;
/// The hook window under `?debug=fishing`.
const DEBUG_HOOK_WINDOW_MS: i32 = 5000;

fn pct(v: i32, p: i32) -> i32 {
    (v as i64 * p as i64 / 100) as i32
}

/// A flat strip from `a` to `b`, `width` wide, turned to face `eye`.
fn ribbon(out: &mut Vec<i32>, a: V3, b: V3, eye: V3, width: i32, c: Rgb) {
    let along = b.sub(a);
    let side = along.cross(eye.sub(a)).norm().scale(width / 2);
    if side == V3::ZERO {
        return;
    }
    let (a0, a1, b0, b1) = (a.sub(side), a.add(side), b.sub(side), b.add(side));
    let n = along.cross(side).norm();
    render::push_tri_glow(out, [a0, a1, b1], n, c, ONE / 2);
    render::push_tri_glow(out, [a0, b1, b0], n, c, ONE / 2);
}

impl App {
    fn fishing_look(&self) -> Option<&FishingLook> {
        self.look.as_ref()?.fishing.as_ref()
    }

    /// This browser's seat's fishing view.
    pub fn my_fishing(&self) -> Option<FishingView> {
        let p = self.play.as_ref()?;
        Some(p.game.fishing_view(p.seat))
    }

    /// The main view is the underwater camera: this seat's line is in.
    pub fn underwater(&self) -> bool {
        self.my_fishing()
            .is_some_and(|v| matches!(v.stage, Stage::Sinking | Stage::Set | Stage::Reeling))
            && self.fishing_look().is_some()
    }

    /// Where the angler's rod tip is, for each boat in fishing mode (the
    /// angler's `line` anchor, placed as `match_frame` places it).
    fn rod_tips(&self, lake: &Lake, view: &PlayView) -> Vec<(usize, V3)> {
        let ms = ((self.clock.max(0) / 1000) % 60_000) as u32;
        let boat = self.models.iter().find(|m| m.kind == crate::model::Kind::Boat);
        let angler = self.models.iter().find(|m| m.kind == crate::model::Kind::Figure);
        let (Some(boat), Some(angler)) = (boat, angler) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for b in view
            .world
            .boats
            .iter()
            .filter(|b| !b.wrecked && b.mode == Mode::Fishing)
        {
            let on = Instance {
                model: boat,
                place: lake.place(P::new(b.x_mm, b.y_mm), 0, b.heading, 1000),
                anim: None,
                ms,
                colour: [0; 3],
            };
            let Some(seat) = scene::seat(&on, Anim::Fish.key()) else {
                continue;
            };
            let figure = Instance {
                model: angler,
                place: seat.then(&Affine::rot_y(b.cast_yaw)),
                anim: Some(Anim::Fish),
                ms,
                colour: [0; 3],
            };
            if let Some(tip) = scene::seat(&figure, "line") {
                out.push((b.seat, tip.t));
            }
        }
        out
    }

    /// A fish model where the view says it is: life size at its depth
    /// under the water, or larger and shallower from the boat camera.
    fn fish_instance<'a>(
        &'a self,
        lake: &Lake,
        f: &FishSeen,
        under: bool,
        look: &FishingLook,
    ) -> Option<Instance<'a>> {
        let model = self.models.iter().find(|m| m.id == f.kind)?;
        let mut scale = (f.length_mm as i64 * 1000 / f.base_length_mm.max(1) as i64) as i32;
        let mut depth = f.depth_mm;
        if !under {
            scale = pct(scale, look.fish_scale_pct);
            depth = pct(depth, look.fish_depth_pct);
        }
        Some(Instance {
            model,
            place: lake.place(P::new(f.x_mm, f.y_mm), -depth, f.heading, scale),
            anim: Some(if f.panic { Anim::Panic } else { Anim::Idle }),
            ms: f.swim_ms % 60_000,
            colour: [0; 3],
        })
    }

    /// The fish, bobbers and lures to draw this frame, and the lines as
    /// triangles seen from `eye`.
    pub(super) fn fishing_instances<'a>(
        &'a self,
        lake: &Lake,
        view: &PlayView,
        under: bool,
        eye: V3,
        instances: &mut Vec<Instance<'a>>,
    ) -> Vec<i32> {
        let mut tris = Vec::new();
        let (Some(look), Some(fv)) = (self.fishing_look(), self.my_fishing()) else {
            return tris;
        };
        let ms = ((self.clock.max(0) / 1000) % 60_000) as u32;
        for f in &fv.fish {
            instances.extend(self.fish_instance(lake, f, under, look));
        }
        let tips = self.rod_tips(lake, view);
        let bobber = self.models.iter().find(|m| m.id == "bobber");
        let lure = self.models.iter().find(|m| m.id == "lure");
        let width = mm(if under { look.line_under_mm } else { look.line_mm });
        let colour = render::rgb(look.line);
        for l in &fv.lines {
            let tip = tips.iter().find(|(s, _)| *s == l.seat).map(|(_, t)| *t);
            if l.stage == Stage::Landed {
                // the catch, hanging from the rod's tip
                if let (Some(tip), Some(f)) = (tip, fv.on_line.as_ref().filter(|_| l.seat == fv.seat)) {
                    if let Some(model) = self.models.iter().find(|m| m.id == f.kind) {
                        let s = (f.length_mm as i64 * ONE as i64 * look.fish_scale_pct as i64
                            / (100 * f.base_length_mm.max(1) as i64)) as i32;
                        let hang = tip.sub(v3(0, mm(look.landed_drop_mm), 0));
                        let place = Affine::translate(hang)
                            .then(&Affine::rot_x(-fx::QUARTER))
                            .then(&Affine::scale(v3(s, s, s)));
                        instances.push(Instance {
                            model,
                            place,
                            anim: Some(Anim::Panic),
                            ms,
                            colour: [0; 3],
                        });
                        ribbon(&mut tris, tip, hang, eye, width, colour);
                    }
                }
                continue;
            }
            let float = lake.world(P::new(l.bobber_x_mm, l.bobber_y_mm), l.bobber_up_mm);
            if let Some(b) = bobber {
                let dip = if l.tug { -mm(40) } else { 0 };
                let s = look.bobber_scale_pct * 10;
                instances.push(Instance {
                    model: b,
                    place: Affine::translate(float.add(v3(0, dip, 0))).then(&Affine::scale(v3(
                        s * ONE / 1000,
                        s * ONE / 1000,
                        s * ONE / 1000,
                    ))),
                    anim: Some(Anim::Bob),
                    ms: (ms + l.seat as u32 * 700) % 60_000,
                    colour: [0; 3],
                });
            }
            if let Some(tip) = tip {
                // a little sag in the middle while the line is slack
                let sag = if l.stage == Stage::Flying { 0 } else { mm(400) };
                let mid = tip.lerp(float, ONE / 2).sub(v3(0, sag, 0));
                ribbon(&mut tris, tip, mid, eye, width, colour);
                ribbon(&mut tris, mid, float, eye, width, colour);
            }
            if l.stage != Stage::Flying {
                let hook = lake.world(P::new(l.bobber_x_mm, l.bobber_y_mm), -l.lure_depth_mm);
                ribbon(&mut tris, float, hook, eye, width, colour);
                if let Some(m) = lure {
                    let s = look.lure_scale_pct * ONE / 100;
                    instances.push(Instance {
                        model: m,
                        place: Affine::translate(hook).then(&Affine::scale(v3(s, s, s))),
                        anim: Some(Anim::Bob),
                        ms,
                        colour: [0; 3],
                    });
                }
            }
        }
        tris
    }

    /// The underwater camera on this seat's lure, if its line is in.
    fn under_camera(&self, lake: &Lake, view: &PlayView, look: &UnderLook) -> Option<Camera> {
        let fv = self.my_fishing()?;
        let l = fv.lines.iter().find(|l| l.seat == fv.seat)?;
        let at = P::new(l.bobber_x_mm, l.bobber_y_mm);
        let target = lake.world(at, -l.lure_depth_mm);
        let p = self.play.as_ref()?;
        let pitch = deg(look.pitch_deg) + (p.cam_pitch - deg(15)) / 3;
        let back = facing(yaw_of_heading(view.cam_yaw), -pitch);
        let mut eye = target
            .sub(back.scale(mm(look.back_mm)))
            .add(v3(0, mm(look.up_mm), 0));
        let bed = lake.ground_mm(lake.map_point(eye));
        eye.y = eye
            .y
            .max(mm(bed + look.clear_bed_mm))
            .min(-mm(look.clear_surface_mm));
        Some(Camera {
            eye,
            fwd: target.sub(eye).norm(),
            fov: deg(look.fov_deg),
            near: mm(50),
            far: mm(look.fog_mm),
        })
    }

    /// A match frame with the fishing drawn in: from the boat camera
    /// `cam`, or, when `under`, from the underwater camera with its murk.
    pub(super) fn fishing_frame<'a>(
        &'a self,
        lake: &Lake,
        view: &PlayView,
        cam: Camera,
        aspect: i32,
        under: bool,
        mut instances: Vec<Instance<'a>>,
    ) -> Frame {
        let clock_ms = (self.clock.max(0) / 1000) as u64;
        let look = self.fishing_look();
        let under_cam = match (under, look) {
            (true, Some(l)) => self.under_camera(lake, view, &l.under),
            _ => None,
        };
        let (cam, under) = match under_cam {
            Some(c) => (c, true),
            None => (cam, false),
        };
        let lines = self.fishing_instances(lake, view, under, cam.eye, &mut instances);
        let sight = if under {
            look.map_or(self.sight_mm(), |l| Some(l.under.fog_mm))
        } else {
            self.sight_mm()
        };
        let mut frame = scene::frame(lake, cam, aspect, sight, clock_ms, &instances);
        frame.tris.extend(lines);
        if let (true, Some(l)) = (under, look) {
            let u = &l.under;
            let light = Light {
                fog: u.colour,
                ..lake.look.light.clone()
            };
            let air = Air {
                zenith: render::rgb(u.zenith),
                under: [
                    (ONE as i64 * 1000 / u.clear_mm.max(1) as i64) as i32,
                    u.tint_pct * ONE / 100,
                ],
                under_colour: render::rgb(u.colour),
                ..lake.air(clock_ms)
            };
            let fog = (mm(pct(u.fog_mm, u.fog_start_pct)), mm(u.fog_mm));
            let cam = Camera {
                far: fog.1 + fog.1 / 8,
                ..cam
            };
            frame.view = render::view_values(&cam, aspect, fog, &light, &air);
        }
        frame
    }

    /// The picture-in-picture while the underwater camera is the main
    /// view: its rectangle and its border's, in drawing-buffer pixels from
    /// the bottom left (x, y, width, height; empty when there is none), and
    /// the boat camera's frame drawn into it.
    pub fn inset_frame(&self) -> (Vec<i32>, Vec<i32>, Frame) {
        let none = (Vec::new(), Vec::new(), Frame::default());
        let Some(look) = self.fishing_look().filter(|_| self.underwater()) else {
            return none;
        };
        let pip = &look.pip;
        let [w, h] = self.size;
        let iw = pct(w, pip.width_pct);
        let ih = pct(iw, pip.tall_pct);
        let gap = pct(w, pip.margin_pct);
        let edge = (w * pip.border_permille / 1000).max(1);
        let rect = vec![gap, h - gap - ih, iw, ih];
        let border = vec![gap - edge, h - gap - ih - edge, iw + 2 * edge, ih + 2 * edge];
        match self.match_frame_as(false, render::aspect([iw, ih])) {
            Some(f) => (rect, border, f),
            None => none,
        }
    }

    // ---- the HUD ----

    fn tier_name(&self, t: crate::map::fish::Tier) -> String {
        self.t(&format!("tier.{}", crate::sim::fishing::tier_key(t)))
    }

    fn fish_name(&self, kind: &str) -> String {
        self.fishboy
            .roster
            .get(kind)
            .map_or(kind.to_string(), |k| k.name.clone())
    }

    /// The fishing panels over Play: the line's state with its meters, and
    /// the popups of the last 2 s.
    pub(super) fn fishing_panels(&self) -> Vec<Panel> {
        let (Some(p), Some(fv)) = (self.play.as_ref(), self.my_fishing()) else {
            return Vec::new();
        };
        let mut panels = Vec::new();
        let fishing = p
            .game
            .boat(p.seat)
            .is_some_and(|b| b.mode == Mode::Fishing && !b.wrecked());
        let mine = fv.lines.iter().find(|l| l.seat == fv.seat);
        let mut items = Vec::new();
        let heading = match fv.stage {
            Stage::Idle if fishing && fv.too_fast => Some(self.t("fishing.too_fast")),
            Stage::Idle if fishing => Some(self.t("fishing.ready")),
            Stage::Idle => None,
            Stage::Charging => {
                items.push(Item::Meter {
                    id: "fishing.charge".into(),
                    label: self.t("fishing.charge"),
                    fill: (fv.charge / 10) as u32,
                    zone: None,
                    marker: None,
                    vertical: false,
                    text: format!("{}%", fv.charge / 10),
                });
                Some(self.t("fishing.charging"))
            }
            Stage::Flying => Some(self.t("fishing.flying")),
            Stage::Sinking => Some(self.t("fishing.sinking")),
            Stage::Set if fv.biting => Some(self.t("fishing.bite")),
            Stage::Set if mine.is_some_and(|l| l.tug) => Some(self.t("fishing.nibble")),
            Stage::Set => Some(self.t("fishing.waiting")),
            Stage::Reeling => {
                if let Some(r) = &fv.reel {
                    let to = |v: i32| (v / 10).clamp(0, 100) as u32;
                    items.push(Item::Meter {
                        id: "fishing.reel".into(),
                        label: self.t("fishing.reel"),
                        fill: 0,
                        zone: Some([to(r.zone), to(r.zone + r.zone_h)]),
                        marker: Some(to(r.fish)),
                        vertical: true,
                        text: self.t(if r.in_zone {
                            "fishing.in_zone"
                        } else {
                            "fishing.out_of_zone"
                        }),
                    });
                    items.push(Item::Meter {
                        id: "fishing.meter".into(),
                        label: self.t("fishing.meter"),
                        fill: to(r.meter),
                        zone: None,
                        marker: None,
                        vertical: true,
                        text: format!("{}%", r.meter / 10),
                    });
                }
                Some(self.t("fishing.reeling"))
            }
            Stage::Landed => fv.on_line.as_ref().map(|f| {
                self.t("fishing.landed")
                    .replace("{feet}", &gallery::feet(f.length_mm))
                    .replace("{fish}", &self.fish_name(&f.kind))
            }),
        };
        if let Some(h) = heading {
            let mut panel = Panel::new("fishing", h, items);
            if matches!(fv.stage, Stage::Sinking | Stage::Set) {
                if let Some(l) = mine {
                    panel.sub = self.t("fishing.depth").replace(
                        "{metres}",
                        &format!("{}.{}", l.lure_depth_mm / 1000, l.lure_depth_mm % 1000 / 100),
                    );
                }
            }
            panels.push(panel);
        }
        panels.extend(self.popups(p, &fv));
        panels
    }

    /// Popups for the events of the last 2 s, newest first.
    fn popups(&self, p: &play::Play, fv: &FishingView) -> Vec<Panel> {
        let now = p.game.tick;
        let keep = POPUP_MS / TICK_MS as u32;
        let name = |seat: usize| {
            p.game
                .setup
                .seats
                .get(seat)
                .map_or(String::new(), |s| s.name.clone())
        };
        let mut out = Vec::new();
        for (tick, e) in p.log.iter().rev() {
            if now.saturating_sub(*tick) >= keep {
                break;
            }
            let own = |seat: &usize| *seat == fv.seat;
            let say = |key: &str| Some(Panel::new("popup", self.t(key), Vec::new()));
            let popup = match e {
                MatchEvent::Hooked { seat, kind, tier, .. } => {
                    let mut panel = Panel::new(
                        &format!("popup tier-{}", crate::sim::fishing::tier_key(*tier)),
                        self.t("popup.hooked")
                            .replace("{player}", &name(*seat))
                            .replace("{fish}", &self.fish_name(kind)),
                        Vec::new(),
                    );
                    panel.sub = self.tier_name(*tier);
                    Some(panel)
                }
                MatchEvent::CastRefused { seat, .. } if own(seat) => say("popup.refused"),
                MatchEvent::Scared { seat } if own(seat) => say("popup.scared"),
                MatchEvent::Missed { seat } if own(seat) => say("popup.missed"),
                MatchEvent::LineBroke { seat, .. } if own(seat) => say("popup.broke"),
                MatchEvent::FishDropped { seat, .. } if own(seat) => say("popup.dropped"),
                _ => None,
            };
            out.extend(popup);
        }
        out
    }

    /// The `debug` flags the page passes on from its address (`?debug=`):
    /// `fishing` puts a fish by this seat's bobber each time it lands, tames
    /// the reel and widens the hook window to 5 s, so a browser check can
    /// hook and land one by script.
    pub(super) fn debug_act(&mut self, value: &str) {
        self.debug_fishing = value.split(',').any(|f| f.trim() == "fishing");
    }

    /// After the ticks: the debug fish by a bobber that just landed.
    pub(super) fn debug_fish(&mut self, landed: bool) {
        if !(self.debug_fishing && landed) {
            return;
        }
        let Some(p) = self.play.as_mut() else {
            return;
        };
        let seat = p.seat;
        let fi = &mut p.game.fishing;
        let Some(kind) = fi
            .roster
            .0
            .iter()
            .position(|k| k.tier == crate::map::fish::Tier::Common)
        else {
            return;
        };
        let at = fi.lines[seat].bobber;
        let tier = fi.roster.0[kind].tier;
        let length = fi.roster.0[kind].base_length_mm * 6 / 5;
        let near = P::new(at.x + 600, at.y);
        fi.schools.push(crate::sim::fishing::lake::School {
            at: near,
            home: near,
            target: near,
            rest_until: u32::MAX,
        });
        let id = fi.fish.len();
        fi.fish.push(crate::sim::fishing::lake::LakeFish {
            id: id as u32,
            kind,
            tier,
            length_mm: length,
            at: near,
            depth_mm: 1200,
            home_depth_mm: 1200,
            heading: 0,
            school: fi.schools.len() - 1,
            offset: P::new(0, 0),
            state: crate::sim::fishing::lake::FishState::Roaming,
            phase_ms: 0,
        });
        fi.tame = true;
        // a page drawing slowly (a software renderer) still strikes in time
        fi.tuning.hook_window_ms = fi.tuning.hook_window_ms.max(DEBUG_HOOK_WINDOW_MS);
    }

    /// The line's whine while this seat reels, for the sound engine.
    pub(super) fn line_tension(&self) -> Option<i64> {
        let r = self.my_fishing()?.reel?;
        Some(r.tension as i64)
    }
}

#[cfg(test)]
mod tests;
