//! The 3D world of a map: the lake, its shores and islands, its props, and
//! each frame's draw data.
//!
//! # Use
//!
//! ```text
//! let lake = Lake::build(&map, &models, &look)?;     once per match
//! // upload lake.vertices once (the page does it when `stamp` changes)
//! let cam = camera::boat(&lake, &look.camera, boat_at, boat_yaw, mouse_yaw, mouse_pitch);
//! let frame = scene::frame(&lake, cam, aspect, Some(lake.sight_mm), clock_ms, &instances);
//! ```
//!
//! - [`Lake::build`] makes everything static from a [`Map`]: the terrain
//!   and lake bed, the water surface, and every prop the map places (trees,
//!   reeds, lily pads, rocks, logs, the dock, cabins). It is uploaded once.
//! - [`frame`] gives one frame's [`Frame`]: the view values, the props near
//!   enough to see as instanced groups, and the frame's [`Instance`]s (boats,
//!   anglers, fish, the buoy: anything that moves) posed into a triangle list.
//! - [`camera`] holds the third-person boat camera and the fly-over camera.
//!
//! # Units and axes
//!
//! The map is in millimetres, `x` east and `y` north from its south-west
//! corner, heights above the lake's surface. The world the renderer draws
//! is in metres (Q16) on glTF axes with the map's middle at the origin:
//! `+x` east, `+y` up, `+z` south (so north is `-z`), and the water's
//! surface at `y = 0`. [`Lake::world`] turns a map point into the world,
//! [`yaw_of_heading`] a map heading (from east toward north) into the yaw
//! that turns a model's `+z` that way, and [`Lake::place`] gives the
//! transform that stands a model at a map point.

pub mod camera;
pub mod look;
mod props;
mod terrain;
#[cfg(test)]
mod tests;
pub mod tour;

pub use look::LakeLook;

use crate::gfx::fx::{self, ONE, QUARTER};
use crate::gfx::geom::{v3, Affine, V3};
use crate::gfx::render::{self, Air, Camera, Frame, Rgb, GROUP_VALUES, STRIDE};
use crate::map::{Map, P};
use crate::model::{Anim, Model};

/// Millimetres to the world's metres (Q16).
pub fn mm(v: i32) -> i32 {
    (v as i64 * ONE as i64 / 1000) as i32
}

/// The world's metres (Q16) to millimetres.
pub fn to_mm(v: i32) -> i32 {
    (v as i64 * 1000 / ONE as i64) as i32
}

/// The yaw that turns a model's `+z` toward a map heading (angle units,
/// 0 east, a quarter turn north).
pub fn yaw_of_heading(heading: i32) -> i32 {
    fx::wrap(QUARTER + heading)
}

/// A map heading from a world yaw: the inverse of [`yaw_of_heading`].
pub fn heading_of_yaw(yaw: i32) -> i32 {
    fx::wrap(yaw - QUARTER)
}

/// One prop mesh in the static buffer, from vertex `first`: its pose of
/// `verts` vertices, and for a moving (swaying or bobbing) model two more
/// blocks after it, the motion along its loop's cosine and sine, which
/// loops in `ms` (see `props.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropMesh {
    pub model: String,
    pub first: i32,
    pub verts: i32,
    pub moving: bool,
    pub ms: u32,
    /// the model's ball, for culling
    pub radius: i32,
}

/// One prop standing on the map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prop {
    pub mesh: usize,
    pub place: Affine,
    /// where it stands in the world, and its ball's radius at its scale
    pub at: V3,
    pub radius: i32,
    /// its own start in its loop, so copies of one model do not sway in
    /// step (SECOND-ORDER R1.104)
    pub phase_ms: u32,
}

/// The static world of one map, built once per match.
#[derive(Debug, Clone)]
pub struct Lake {
    pub map: Map,
    /// the fog's whole distance on this map (`Params::sight_mm`)
    pub sight_mm: i32,
    /// the terrain's height at each grid corner, `(n + 1)²`, row by row from
    /// the south, in mm
    pub heights: Vec<i32>,
    /// every static vertex, [`render::VERTEX_VALUES`] each: the terrain, the
    /// water, then each prop mesh's frames
    pub vertices: Vec<i32>,
    /// first vertex and count of the terrain and of the water
    pub terrain: [i32; 2],
    pub water: [i32; 2],
    pub meshes: Vec<PropMesh>,
    pub props: Vec<Prop>,
    /// the tour's path over the water, in mm, and the length to each point
    pub tour: Vec<P>,
    pub tour_at: Vec<i64>,
    pub look: LakeLook,
}

/// Something drawn this frame that is not part of the static world: a
/// boat, an angler, a fish, the buoy. It is posed on the CPU into the
/// frame's triangle list.
#[derive(Debug, Clone, Copy)]
pub struct Instance<'a> {
    pub model: &'a Model,
    /// model space to the world (see [`Lake::place`])
    pub place: Affine,
    /// the animation playing and how far into it, or none (rest)
    pub anim: Option<Anim>,
    pub ms: u32,
    /// the colour that fills the model's `player` slot
    pub colour: Rgb,
}

impl Lake {
    /// The static world for a map, its props from `models` as `look` names
    /// them. Refuses a look that names a model that is not loaded.
    pub fn build(map: &Map, models: &[Model], look: &LakeLook) -> Result<Lake, String> {
        look.check()?;
        for id in look.model_ids() {
            if !models.iter().any(|m| m.id == id) {
                return Err(format!(
                    "The lake's look names the model \"{id}\", which is not loaded."
                ));
            }
        }
        let mut lake = Lake {
            map: map.clone(),
            sight_mm: map.setting.params().sight_mm,
            heights: terrain::corner_heights(map),
            vertices: Vec::new(),
            terrain: [0; 2],
            water: [0; 2],
            meshes: Vec::new(),
            props: Vec::new(),
            tour: Vec::new(),
            tour_at: Vec::new(),
            look: look.clone(),
        };
        let placed = props::place(&lake, look);
        let forest = props::forest_cells(map, &placed, look);
        let mut verts = Vec::new();
        lake.terrain = terrain::ground(&lake, &forest, &mut verts);
        lake.water = terrain::water(&lake, &mut verts);
        let (meshes, props) = props::bake(&lake, models, &placed, look, &mut verts);
        lake.vertices = verts;
        lake.meshes = meshes;
        lake.props = props;
        let (path, at) = tour::path(&lake.map);
        lake.tour = path;
        lake.tour_at = at;
        Ok(lake)
    }

    fn half_mm(&self) -> i32 {
        self.map.size_mm() / 2
    }

    /// A map point at a height (mm over the water) in the world.
    pub fn world(&self, p: P, height_mm: i32) -> V3 {
        let h = self.half_mm();
        v3(mm(p.x - h), mm(height_mm), mm(h - p.y))
    }

    /// The map point under a world point.
    pub fn map_point(&self, w: V3) -> P {
        let h = self.half_mm();
        P::new(to_mm(w.x) + h, h - to_mm(w.z))
    }

    /// The transform that stands a model's origin at a map point and
    /// height, its `+z` toward `heading`, at `scale` thousandths.
    pub fn place(&self, p: P, height_mm: i32, heading: i32, scale: i32) -> Affine {
        let s = scale * ONE / 1000;
        Affine::translate(self.world(p, height_mm))
            .then(&Affine::rot_y(yaw_of_heading(heading)))
            .then(&Affine::scale(v3(s, s, s)))
    }

    /// The terrain's height at a map point, in mm, on the very triangles
    /// drawn (off the map: the edge's).
    pub fn ground_mm(&self, p: P) -> i32 {
        terrain::height_at(self, p)
    }

    /// The draw data's stamp for this world: the page uploads
    /// [`Lake::vertices`] again when it changes.
    pub fn stamp(&self) -> u32 {
        // the seed and the size of the buffer: a new map or a new look
        (self.map.seed.wrapping_mul(2_654_435_761)) ^ self.vertices.len() as u32
    }

    /// The fog's start and end (world metres) for a sight distance in mm,
    /// by the look's shares; `None` is no fog.
    pub fn fog(&self, sight_mm: Option<i32>) -> (i32, i32) {
        match sight_mm {
            Some(s) => (
                mm((s as i64 * self.look.fog_start_pct as i64 / 100) as i32),
                mm((s as i64 * self.look.fog_end_pct as i64 / 100) as i32),
            ),
            None => (mm(self.look.camera.far_mm * 10), mm(self.look.camera.far_mm * 11)),
        }
    }

    /// The sky, the water and its tint at `clock_ms`.
    pub fn air(&self, clock_ms: u64) -> Air {
        let l = &self.look;
        let w = &l.water;
        let wave = |k: usize| {
            let v = &w.waves[k];
            // radians per metre, along the way it travels
            let kk = (fx::TWO_PI * 1000 / v.length_mm as i64) as i32;
            let a = fx::deg(v.toward_deg);
            // a map heading toward north is the world's -z
            let (kx, kz) = (fx::mul(kk, fx::cos(a)), -fx::mul(kk, fx::sin(a)));
            let t = (clock_ms % v.period_ms as u64) as i64;
            let phase = -(fx::TWO_PI * t / v.period_ms as i64) as i32;
            [kx, kz, mm(v.height_mm), phase]
        };
        let pct = |v: i32| v * ONE / 100;
        Air {
            zenith: render::rgb(l.zenith),
            sky_band: [pct(l.sky_band[0]), pct(l.sky_band[1])],
            under: [
                (ONE as i64 * 1000 / l.under.clear_mm as i64) as i32,
                pct(l.under.strength_pct),
            ],
            under_colour: render::rgb(l.under.colour),
            waves: [wave(0), wave(1)],
            water: [
                pct(w.alpha_down_pct),
                pct(w.alpha_grazing_pct),
                pct(w.fresnel_pow_pct),
                0,
            ],
            spec: {
                let g = render::rgb(w.glint);
                [g[0], g[1], g[2], w.shine * ONE]
            },
        }
    }
}

/// One frame of the world seen by `cam`. `sight_mm` is the fog's distance
/// (`Some(lake.sight_mm)` in a match; `None` draws no fog, for the
/// overview). Props past the fog are left out. `cam.far` is set here: just
/// past the fog, or the look's far plane with no fog.
pub fn frame(
    lake: &Lake,
    mut cam: Camera,
    aspect: i32,
    sight_mm: Option<i32>,
    clock_ms: u64,
    instances: &[Instance],
) -> Frame {
    let fog = lake.fog(sight_mm);
    cam.far = match sight_mm {
        Some(_) => fog.1 + fog.1 / 8,
        None => mm(lake.look.camera.far_mm),
    };
    let view = render::view_values(&cam, aspect, fog, &lake.look.light, &lake.air(clock_ms));

    // the props near enough to see, by mesh, and where each is in its loop
    let reach = sight_mm.map(|_| fog.1);
    let fwd = cam.fwd.norm();
    let mut members: Vec<Vec<(usize, i32, i32)>> = vec![Vec::new(); lake.meshes.len()];
    for (i, p) in lake.props.iter().enumerate() {
        let off = p.at.sub(cam.eye);
        if let Some(r) = reach {
            if off.flat().longer_than(r + p.radius) {
                continue;
            }
        }
        // behind the camera
        if off.dot(fwd) < -p.radius {
            continue;
        }
        let m = &lake.meshes[p.mesh];
        let (c, s) = if m.moving {
            let t = (clock_ms + p.phase_ms as u64) % m.ms as u64;
            let a = (t * fx::TURN as u64 / m.ms as u64) as i32;
            (fx::cos(a), fx::sin(a))
        } else {
            (0, 0)
        };
        members[p.mesh].push((i, c, s));
    }
    let mut groups = Vec::new();
    let mut inst = Vec::new();
    for (m, list) in lake.meshes.iter().zip(&members) {
        if list.is_empty() {
            continue;
        }
        // a still mesh has one block: its motion is that block times 0
        let block = |k: i32| (m.first + if m.moving { k * m.verts } else { 0 }) * STRIDE as i32;
        groups.extend_from_slice(&[
            block(0),
            block(1),
            block(2),
            m.verts,
            (inst.len() * render::BYTES_PER_VALUE) as i32,
            list.len() as i32,
        ]);
        for &(i, c, s) in list {
            let a = &lake.props[i].place;
            inst.extend_from_slice(&[
                a.c0.x, a.c0.y, a.c0.z, a.c1.x, a.c1.y, a.c1.z, a.c2.x, a.c2.y, a.c2.z, a.t.x, a.t.y, a.t.z,
                c, s,
            ]);
        }
    }
    debug_assert_eq!(groups.len() % GROUP_VALUES, 0);

    Frame {
        view,
        tris: pose(instances),
        statics: [lake.terrain[0], lake.terrain[1], lake.water[0], lake.water[1]],
        groups,
        instances: inst,
    }
}

/// The instances posed into one triangle list.
pub fn pose(instances: &[Instance]) -> Vec<i32> {
    let mut tris = Vec::new();
    for i in instances {
        let t = i.model.transforms(&i.model.poses(i.anim, i.ms));
        i.model.draw(&t, &i.place, &i.model.colours(i.colour), &mut tris);
    }
    tris
}

/// Where a figure rides on a model: the frame of the anchor `name` (a
/// boat's `drive` or `fish`) of `on`, posed as it is. `None` if it has no
/// such anchor.
pub fn seat(on: &Instance, name: &str) -> Option<Affine> {
    let anchor = on.model.anchor(name)?;
    let t = on.model.transforms(&on.model.poses(on.anim, on.ms));
    Some(on.place.then(&on.model.anchor_frame(anchor, &t)))
}
