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
//! - **The HUD**: [`App::fishing_hud`] says what the line's stage shows (a
//!   prompt, the depth, the charge, the reel bar and the catch meter);
//!   `flow/hud.rs` draws it on the 2D layer. The popups (every player's
//!   hooks, this seat's misses and breaks) are rows of `data/flow.json`.

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
use crate::sim::Mode;

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
    /// the murk's colour looking level just under the surface, and with
    /// the camera `deep_mm` down (blended between by the camera's depth)
    pub colour: [i32; 3],
    pub colour_deep: [i32; 3],
    pub deep_mm: i32,
    /// looking straight down, and the view's depth (hundredths) where the
    /// murk starts to turn to it and is all of it
    pub down: [i32; 3],
    pub down_band: [i32; 2],
    /// looking up toward the light, and where the murk turns to it
    pub zenith: [i32; 3],
    pub zenith_band: [i32; 2],
    /// the light's colour (shafts, caustics, the surface's sheen)
    pub sheen: [i32; 3],
    /// the light under the water: brighter and greener than over it, so
    /// the fish read against the murk
    pub sun: [i32; 3],
    pub sky: [i32; 3],
    pub ground: [i32; 3],
    /// the fish are drawn this much (percent) unlit, in their own colours,
    /// so they stand out from the bed and the murk
    pub fish_lift_pct: i32,
    pub shafts: ShaftLook,
    pub caustics: CausticLook,
    pub surface: SurfaceLook,
    pub motes: MoteLook,
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

/// Soft light shafts fanning down from the light (sky.frag's `murk`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShaftLook {
    /// the light's direction, hundredths (toward it)
    pub dir: [i32; 3],
    /// two sets of stripes round it, so many to a turn each, drifting
    /// round once in so many milliseconds
    pub counts: [i32; 2],
    pub drift_ms: [i32; 2],
    /// strength and sharpness (a power, hundredths), and the cosine
    /// (hundredths) from the light where they fade in and are whole
    pub strength_pct: i32,
    pub power_pct: i32,
    pub fade: [i32; 2],
}

/// The caustic net on what looks up, and the sheen on the surface.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CausticLook {
    /// exactly three ripples
    pub ripples: Vec<RippleLook>,
    /// the lines' sharpness (a power), and their strength on the bed and
    /// on the surface's underside
    pub power: i32,
    pub bed_pct: i32,
    pub surface_pct: i32,
}

/// One ripple: its wavelength, heading (degrees from east toward north)
/// and period.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RippleLook {
    pub length_mm: i32,
    pub toward_deg: i32,
    pub period_ms: i32,
}

/// The surface seen from below.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceLook {
    /// its colour outside the window of light overhead
    pub underside: [i32; 3],
    /// the window: from how straight up (hundredths) it starts and is
    /// whole, and the opacity in it and outside
    pub window: [i32; 2],
    pub window_alpha_pct: i32,
    pub alpha_pct: i32,
    /// what floats higher than `float_mm` (lily pads, a boat's hull) is
    /// drawn `shadow_pct` toward `shadow`, a dark shape against the light
    pub float_mm: i32,
    pub shadow: [i32; 3],
    pub shadow_pct: i32,
}

/// Specks drifting in the water round the camera, and bubbles rising.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoteLook {
    pub count: i32,
    /// they fill a box this wide round the camera, which they wrap in
    pub box_mm: i32,
    pub size_mm: i32,
    pub colour: [i32; 3],
    /// drifting this fast at most each way
    pub drift_mm_per_s: i32,
    /// so many in a hundred are bubbles, this big, rising this fast
    pub bubble_pct: i32,
    pub bubble_mm: i32,
    pub bubble: [i32; 3],
    pub rise_mm_per_s: i32,
    /// how unlit they are (percent), so they read against the murk
    pub glow_pct: i32,
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

/// The fishing HUD's state for this seat (`App::fishing_hud`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FishingHud {
    /// the stage's key: ready, too_fast, charging, flying, sinking,
    /// waiting, nibble, bite, reeling or landed
    pub key: &'static str,
    pub prompt: String,
    /// the lure's depth, while it sinks or waits
    pub depth: Option<String>,
    /// the cast's charge, percent, while Space is held
    pub charge: Option<i32>,
    pub reel: Option<ReelBars>,
}

/// The reel bar and the catch meter, in percent of the bar from the bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReelBars {
    pub zone: [i32; 2],
    pub fish: i32,
    pub meter: i32,
    pub in_zone: bool,
    pub tier: crate::map::fish::Tier,
}
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

/// The light and air under the water for an eye at `eye`: the murk by the
/// eye's depth, darker looking down and bright looking up, the shafts and
/// the caustics moving with the clock, the surface's underside and what
/// floats on it as dark shapes (see the shaders' notes).
pub fn under_air(lake: &Lake, u: &UnderLook, eye: V3, clock_ms: u64) -> (Light, Air) {
    let rgb = render::rgb;
    let depth = -scene::to_mm(eye.y);
    let t = (depth.max(0) as i64 * ONE as i64 / u.deep_mm.max(1) as i64).min(ONE as i64) as i32;
    let murk: [i32; 3] = [0, 1, 2].map(|k| u.colour[k] + (u.colour_deep[k] - u.colour[k]) * t / ONE);
    let light = Light {
        fog: murk,
        sun: u.sun,
        sky: u.sky,
        ground: u.ground,
        ..lake.look.light.clone()
    };
    let hundredth = |v: i32| v * ONE / 100;
    // a phase going round once in `period` ms
    let phase =
        |period: i32| (fx::TWO_PI * (clock_ms % period.max(1) as u64) as i64 / period.max(1) as i64) as i32;
    let s = &u.shafts;
    let dir = v3(hundredth(s.dir[0]), hundredth(s.dir[1]), hundredth(s.dir[2])).norm();
    let c = &u.caustics;
    let ripple = |k: usize| {
        let r = &c.ripples[k.min(c.ripples.len().saturating_sub(1))];
        let kk = (fx::TWO_PI * 1000 / r.length_mm.max(1) as i64) as i32;
        let a = deg(r.toward_deg);
        [
            fx::mul(kk, fx::cos(a)),
            -fx::mul(kk, fx::sin(a)),
            phase(r.period_ms),
            0,
        ]
    };
    let f = &u.surface;
    let air = Air {
        zenith: rgb(u.zenith),
        sky_band: [hundredth(u.zenith_band[0]), hundredth(u.zenith_band[1])],
        under: [
            (ONE as i64 * 1000 / u.clear_mm.max(1) as i64) as i32,
            u.tint_pct * ONE / 100,
        ],
        under_colour: rgb(murk),
        deep: render::Deep {
            colour: rgb(u.down),
            band: [hundredth(u.down_band[0]), hundredth(u.down_band[1])],
            shaft: [
                s.counts[0] * ONE,
                s.counts[1] * ONE,
                phase(s.drift_ms[0]),
                -phase(s.drift_ms[1]),
            ],
            shaft_look: [
                hundredth(s.strength_pct),
                hundredth(s.power_pct),
                hundredth(s.fade[0]),
                hundredth(s.fade[1]),
            ],
            shaft_dir: [dir.x, dir.y, dir.z],
            sheen: rgb(u.sheen),
            ripples: [ripple(0), ripple(1), ripple(2)],
            caustic: [
                ONE / 3,
                c.power * ONE,
                hundredth(c.bed_pct),
                hundredth(c.surface_pct),
            ],
            underside: rgb(f.underside),
            window: [
                hundredth(f.window[0]),
                hundredth(f.window[1]),
                hundredth(f.window_alpha_pct),
                hundredth(f.alpha_pct),
            ],
            silhouette: [-mm(f.float_mm), hundredth(f.shadow_pct)],
            shadow: rgb(f.shadow),
        },
        ..lake.air(clock_ms)
    };
    (light, air)
}

/// The drifting specks and rising bubbles round the underwater camera, as
/// small diamonds turned to it, appended to `out`. Each has its own place
/// in a box that wraps round the eye, so they stay put as it moves and
/// never run out; none is drawn over the surface or under the bed.
pub fn motes(lake: &Lake, m: &MoteLook, cam: &Camera, clock_ms: u64, out: &mut Vec<i32>) {
    let b = m.box_mm.max(1) as i64;
    let eye = [
        scene::to_mm(cam.eye.x),
        scene::to_mm(cam.eye.y),
        scene::to_mm(cam.eye.z),
    ]
    .map(|v| v as i64);
    let fwd = cam.fwd.norm();
    let right = crate::gfx::geom::right_of(fwd);
    let up = right.cross(fwd);
    let t = (clock_ms % 3_600_000) as i64;
    let seed = lake.map.seed as u64 ^ 0x6d6f_7465;
    let glow = m.glow_pct * ONE / 100;
    for i in 0..m.count {
        let h = crate::map::rng::hash2(seed, i, 0);
        let h2 = crate::map::rng::hash2(seed, i, 1);
        let bubble = ((h % 100) as i32) < m.bubble_pct;
        let unit = |k: u64, span: i64| (k % (2 * span as u64 + 1)) as i64 - span;
        let d = m.drift_mm_per_s as i64;
        let v = if bubble {
            [
                unit(h2 >> 8, d / 4),
                m.rise_mm_per_s as i64,
                unit(h2 >> 24, d / 4),
            ]
        } else {
            [unit(h2 >> 8, d), unit(h2 >> 40, d / 3), unit(h2 >> 24, d)]
        };
        let base = [(h >> 8) as i64 % b, (h >> 28) as i64 % b, (h >> 46) as i64 % b];
        let at: [i64; 3] = [0, 1, 2].map(|k| {
            let p = base[k] + v[k] * t / 1000 - eye[k] + b / 2;
            eye[k] + p.rem_euclid(b) - b / 2
        });
        let p = v3(mm(at[0] as i32), mm(at[1] as i32), mm(at[2] as i32));
        let ground = lake.ground_mm(lake.map_point(p));
        let size = if bubble { m.bubble_mm } else { m.size_mm };
        if at[1] > -(size as i64) || (at[1] as i32) < ground + size {
            continue;
        }
        let (r, u) = (right.scale(mm(size) / 2), up.scale(mm(size) / 2));
        let colour = render::rgb(if bubble { m.bubble } else { m.colour });
        let n = fwd.neg();
        render::push_tri_glow(out, [p.sub(r), p.sub(u), p.add(r)], n, colour, glow);
        render::push_tri_glow(out, [p.sub(r), p.add(r), p.add(u)], n, colour, glow);
    }
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
            let Some(fish) = self.fish_instance(lake, f, under, look) else {
                continue;
            };
            if under {
                // lifted toward their own colours so they read against the murk
                let lift = look.under.fish_lift_pct * ONE / 100;
                let mut posed = scene::pose(&[fish]);
                for v in posed.chunks_mut(render::VERTEX_VALUES) {
                    v[9] = v[9].max(lift);
                }
                tris.extend(posed);
            } else {
                instances.push(fish);
            }
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
        // the mouse tilts it from where its pitch starts
        let start = self.controls.as_ref().map_or(0, |c| deg(c.mouse.pitch_start_deg));
        let pitch = deg(look.pitch_deg) + p.cam_pitch - start;
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
            let (light, air) = under_air(lake, u, cam.eye, clock_ms);
            let fog = (mm(pct(u.fog_mm, u.fog_start_pct)), mm(u.fog_mm));
            let cam = Camera {
                far: fog.1 + fog.1 / 8,
                ..cam
            };
            frame.view = render::view_values(&cam, aspect, fog, &light, &air);
            motes(lake, &u.motes, &cam, clock_ms, &mut frame.tris);
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

    fn fish_name(&self, kind: &str) -> String {
        self.fishboy
            .roster
            .get(kind)
            .map_or(kind.to_string(), |k| k.name.clone())
    }

    /// What the fishing HUD shows for this seat's line now (drawn on the 2D
    /// layer by `flow/hud.rs`): a prompt for the line's stage, the lure's
    /// depth, the charge while Space is held, and the reel bar and catch
    /// meter while reeling. `None` with nothing to say (driving, no line).
    pub fn fishing_hud(&self) -> Option<FishingHud> {
        let (p, fv) = (self.play.as_ref()?, self.my_fishing()?);
        let fishing = p
            .game
            .boat(p.seat)
            .is_some_and(|b| b.mode == Mode::Fishing && !b.wrecked());
        let mine = fv.lines.iter().find(|l| l.seat == fv.seat);
        let key = match fv.stage {
            Stage::Idle if fishing && fv.too_fast => "too_fast",
            Stage::Idle if fishing => "ready",
            Stage::Idle => return None,
            Stage::Charging => "charging",
            Stage::Flying => "flying",
            Stage::Sinking => "sinking",
            Stage::Set if fv.biting => "bite",
            Stage::Set if mine.is_some_and(|l| l.tug) => "nibble",
            Stage::Set => "waiting",
            Stage::Reeling => "reeling",
            Stage::Landed => "landed",
        };
        let prompt = match (fv.stage, fv.on_line.as_ref()) {
            (Stage::Landed, Some(f)) => self
                .t("fishing.landed")
                .replace("{feet}", &gallery::feet(f.length_mm))
                .replace("{fish}", &self.fish_name(&f.kind)),
            (Stage::Landed, None) => return None,
            _ => self.t(&format!("fishing.{key}")),
        };
        let depth = mine
            .filter(|_| matches!(fv.stage, Stage::Sinking | Stage::Set))
            .map(|l| {
                self.t("fishing.depth").replace(
                    "{metres}",
                    &format!("{}.{}", l.lure_depth_mm / 1000, l.lure_depth_mm % 1000 / 100),
                )
            });
        let to = |v: i32| (v / 10).clamp(0, 100);
        Some(FishingHud {
            key,
            prompt,
            depth,
            charge: (fv.stage == Stage::Charging).then_some(fv.charge / 10),
            reel: fv
                .reel
                .as_ref()
                .filter(|_| fv.stage == Stage::Reeling)
                .map(|r| ReelBars {
                    zone: [to(r.zone), to(r.zone + r.zone_h)],
                    fish: to(r.fish),
                    meter: to(r.meter),
                    in_zone: r.in_zone,
                    tier: r.tier,
                }),
        })
    }

    /// The `debug` flags the page passes on from its address (`?debug=`):
    /// `fishing` puts a fish by this seat's bobber each time it lands, tames
    /// the reel and widens the hook window to 5 s, so a browser check can
    /// hook and land one by script.
    pub(super) fn debug_act(&mut self, value: &str) {
        self.debug_fishing = value.split(',').any(|f| f.trim() == "fishing");
        self.debug_place = value
            .split(',')
            .map(str::trim)
            .find(|f| ["reeds", "island"].contains(f))
            .map(String::from);
        self.debug_flags = value.split(',').map(|f| f.trim().to_string()).collect();
        self.flow.debug = value.split(',').any(|f| f.trim() == "flow");
    }

    /// Under `?debug=reeds` or `?debug=island`, once the match has run a
    /// tick: this seat's boat stopped in 2-3.5 m of water off the biggest
    /// reed bed (facing it), or just off the biggest island's shore
    /// (facing away, so the camera behind has the island at its back).
    pub(super) fn debug_move(&mut self) {
        let Some(place) = self.debug_place.clone() else {
            return;
        };
        let Some(p) = self.play.as_mut() else {
            return;
        };
        if p.game.tick == 0 {
            return;
        }
        self.debug_place = None;
        let map = &p.game.map;
        let ground = |q: P| map.index(q).map_or(0, |i| map.ground[i]);
        let toward = |from: P, a: i32, d: i32| {
            use crate::map::trig;
            P::new(
                from.x + (trig::cos(a) as i64 * d as i64 / trig::ONE as i64) as i32,
                from.y + (trig::sin(a) as i64 * d as i64 / trig::ONE as i64) as i32,
            )
        };
        let turn = crate::map::trig::TURN;
        let spot = if place == "reeds" {
            map.reeds.iter().max_by_key(|r| r.clumps.len()).and_then(|r| {
                (3..30)
                    .flat_map(|d| (0..16).map(move |k| (d * 1000, k * turn / 16)))
                    .find_map(|(d, a)| {
                        let q = toward(r.at, a, d);
                        let deep = (-3500..=-2000).contains(&ground(q));
                        let clear = r.clumps.iter().all(|c| c.dist(q) > 4000);
                        (deep && clear).then_some((q, a + turn / 2))
                    })
            })
        } else {
            map.islands.iter().max_by_key(|i| i.area_m2).and_then(|i| {
                (0..16).find_map(|k| {
                    let a = k * turn / 16;
                    let shore = (0..80)
                        .map(|d| toward(i.at, a, d * 1000))
                        .find(|q| ground(*q) < -1200)?;
                    let q = toward(shore, a, 2500);
                    (ground(q) < -1200).then_some((q, a))
                })
            })
        };
        let Some((at, heading)) = spot else {
            return;
        };
        let seat = p.seat;
        if let Some(Some(b)) = p.game.boats.get_mut(seat) {
            b.pos_um = [at.x as i64 * 1000, at.y as i64 * 1000];
            b.heading = crate::map::trig::wrap(heading);
            b.vel_um = [0, 0];
        }
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
