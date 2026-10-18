//! What the page's renderer draws, and every number it needs to read it.
//!
//! A frame ([`Frame`]) is:
//! - the view values the shaders take as uniforms (the camera, the fog, the
//!   sun, the sky, the water's waves and tint), laid out by [`UNIFORMS`];
//! - a triangle list posed this frame (the Gallery's model, the match's
//!   boats, anglers and fish), each vertex its position, flat normal, colour
//!   and glow (Q16; the shim turns them into floats);
//! - the ranges of the static world to draw (`statics`: the terrain and the
//!   water, uploaded once per world, `crate::scene`);
//! - the instanced groups (`groups`) and their per-instance values
//!   (`instances`): each group draws one prop mesh from the static buffer
//!   once per instance, its sway added back at the instance's own phase.
//!
//! Every number the page needs to read these arrays (strides, offsets,
//! where each uniform sits in the view values) is in [`numbers_json`], so
//! the page holds none of its own. (This replaces the one-list-a-frame
//! renderer of SECOND-ORDER R1.45.)

use serde::Deserialize;
use serde_json::json;

use super::fx::{self, div, sin, ONE};
use super::geom::{right_of, v3, Mat4, V3};

/// A colour, each channel 0..=ONE.
pub type Rgb = [i32; 3];

/// From 0–255 channels.
pub fn rgb(c: [i32; 3]) -> Rgb {
    c.map(|v| v * ONE / 255)
}

/// Values per vertex: position xyz, normal xyz, colour rgb, glow (0 lit,
/// ONE unlit).
pub const VERTEX_VALUES: usize = 10;
pub const BYTES_PER_VALUE: usize = 4;
/// Bytes per vertex.
pub const STRIDE: usize = VERTEX_VALUES * BYTES_PER_VALUE;
/// Values per prop instance: the placing transform's three columns and its
/// translation (12), and the cosine and sine of where it is in its loop (2).
pub const INSTANCE_VALUES: usize = 14;
/// Values per instanced group: the byte offsets in the static buffer of its
/// mesh's pose and of the motion along its loop's cosine and sine, the
/// vertices the mesh holds, the byte offset of its first instance and how
/// many instances it draws.
pub const GROUP_VALUES: usize = 6;

/// How many vertices a triangle list holds.
pub fn vertex_count(values: &[i32]) -> u32 {
    (values.len() / VERTEX_VALUES) as u32
}

/// Pushes one flat, lit triangle.
pub fn push_tri(out: &mut Vec<i32>, p: [V3; 3], n: V3, c: Rgb) {
    push_tri_glow(out, p, n, c, 0);
}

/// Pushes one flat triangle with a glow: 0 is lit by the sun and shade,
/// ONE is drawn at full brightness.
pub fn push_tri_glow(out: &mut Vec<i32>, p: [V3; 3], n: V3, c: Rgb, glow: i32) {
    for q in p {
        out.extend_from_slice(&[q.x, q.y, q.z, n.x, n.y, n.z, c[0], c[1], c[2], glow]);
    }
}

/// A camera: where it is, where it looks, its vertical field of view
/// (angle units) and its near and far planes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Camera {
    pub eye: V3,
    pub fwd: V3,
    pub fov: i32,
    pub near: i32,
    pub far: i32,
}

impl Camera {
    /// The view-projection matrix for a screen `aspect` (width / height, Q16).
    pub fn matrix(&self, aspect: i32) -> Mat4 {
        Mat4::perspective(self.fov, aspect, self.near, self.far).mul(&Mat4::look(self.eye, self.fwd, V3::UP))
    }

    /// How far back a camera with this field of view stands for a ball of
    /// `radius` to fill the height of the screen, with `margin` percent to
    /// spare.
    pub fn fit(fov: i32, radius: i32, margin: i32) -> i32 {
        (div(radius, sin(fov / 2)) as i64 * margin as i64 / 100) as i32
    }
}

fn half() -> i32 {
    50
}

/// The light and the air, from `data/look.json`: 0–255 colours, and the
/// sun's direction (toward the sun) in hundredths.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Light {
    /// the fog's colour, which is the background's (the horizon's) too
    pub fog: [i32; 3],
    pub sun_dir: [i32; 3],
    pub sun: [i32; 3],
    /// the light from above and from below, blended by how a face points
    pub sky: [i32; 3],
    pub ground: [i32; 3],
    /// how much of the fog an unlit (glowing) colour takes, in percent
    #[serde(default = "half")]
    pub glow_fog_pct: i32,
}

impl Light {
    pub fn check(&self) -> Result<(), String> {
        for c in [self.fog, self.sun, self.sky, self.ground] {
            if c.iter().any(|v| !(0..=255).contains(v)) {
                return Err(format!("A light colour {c:?} is outside 0 to 255."));
            }
        }
        if self.sun_dir == [0, 0, 0] {
            return Err("The sun has no direction.".into());
        }
        if !(0..=100).contains(&self.glow_fog_pct) {
            return Err("The glow's share of the fog is outside 0 to 100.".into());
        }
        Ok(())
    }
}

/// The sky, the water and what lies under it, as the shaders take them
/// (Q16). [`Air::plain`] is a sky of the fog's colour and no water, for a
/// screen with no lake (the Gallery).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Air {
    /// the sky's colour overhead; the fog's colour is the horizon's
    pub zenith: Rgb,
    /// the height of the view direction (-ONE..ONE) where the sky starts
    /// to turn from the horizon's colour, and where it is the zenith's
    pub sky_band: [i32; 2],
    /// the tint of things under the water: how fast it grows per metre of
    /// depth, and its strength at most (ONE: all tint)
    pub under: [i32; 2],
    pub under_colour: Rgb,
    /// two waves on the water: (k along x, k along z, height, phase), k in
    /// radians per metre and the phase in radians
    pub waves: [[i32; 4]; 2],
    /// the water's opacity looking straight down, at a grazing angle, the
    /// power that blends the two, and 0
    pub water: [i32; 4],
    /// the sun's glint on the water: its colour and its sharpness
    pub spec: [i32; 4],
}

impl Air {
    pub fn plain(light: &Light) -> Air {
        Air {
            zenith: rgb(light.fog),
            sky_band: [0, ONE],
            under: [0, 0],
            under_colour: rgb(light.fog),
            waves: [[0; 4]; 2],
            water: [ONE, ONE, ONE, 0],
            spec: [0, 0, 0, ONE],
        }
    }
}

/// The uniforms in the view values, in order: name and size. The first 16
/// values are the view-projection matrix (`uVP`).
pub const UNIFORMS: &[(&str, usize)] = &[
    ("uEye", 3),
    ("uFog", 2),
    ("uFogColour", 3),
    ("uSunDir", 3),
    ("uSun", 3),
    ("uSky", 3),
    ("uGround", 3),
    ("uHemi", 2),
    ("uZenith", 3),
    ("uSkyBand", 2),
    ("uUnder", 2),
    ("uUnderColour", 3),
    ("uCamRight", 3),
    ("uCamUp", 3),
    ("uCamFwd", 3),
    ("uCamTan", 2),
    ("uWave1", 4),
    ("uWave2", 4),
    ("uWater", 4),
    ("uSpec", 4),
    ("uGlow", 1),
];

/// The matrix's 16 values come first.
const MATRIX_VALUES: usize = 16;

/// How many view values there are.
pub const VIEW_VALUES: usize = {
    let mut n = MATRIX_VALUES;
    let mut i = 0;
    while i < UNIFORMS.len() {
        n += UNIFORMS[i].1;
        i += 1;
    }
    n
};

/// Where a uniform starts in the view values, and its size.
pub fn uniform_at(name: &str) -> (usize, usize) {
    let mut at = MATRIX_VALUES;
    for (n, size) in UNIFORMS {
        if *n == name {
            return (at, *size);
        }
        at += size;
    }
    panic!("no uniform {name}")
}

/// The view values the page uploads each frame, in [`UNIFORMS`] order
/// after the matrix. `fog` is the fog's start and end distances.
pub fn view_values(cam: &Camera, aspect: i32, fog: (i32, i32), light: &Light, air: &Air) -> Vec<i32> {
    let mut v = cam.matrix(aspect).0.to_vec();
    let mut put = |name: &str, values: &[i32]| {
        debug_assert_eq!((v.len(), values.len()), uniform_at(name), "{name}");
        v.extend_from_slice(values);
    };
    put("uEye", &[cam.eye.x, cam.eye.y, cam.eye.z]);
    put("uFog", &[fog.0, fog.1]);
    put("uFogColour", &rgb(light.fog));
    let [x, y, z] = light.sun_dir.map(|c| c * ONE / 100);
    let d = v3(x, y, z).norm();
    put("uSunDir", &[d.x, d.y, d.z]);
    put("uSun", &rgb(light.sun));
    put("uSky", &rgb(light.sky));
    put("uGround", &rgb(light.ground));
    put("uHemi", &[ONE / 2, ONE / 2]);
    put("uZenith", &air.zenith);
    put("uSkyBand", &air.sky_band);
    put("uUnder", &air.under);
    put("uUnderColour", &air.under_colour);
    let fwd = cam.fwd.norm();
    let right = right_of(fwd);
    let up = right.cross(fwd);
    let tan_y = div(sin(cam.fov / 2), fx::cos(cam.fov / 2));
    put("uCamRight", &[right.x, right.y, right.z]);
    put("uCamUp", &[up.x, up.y, up.z]);
    put("uCamFwd", &[fwd.x, fwd.y, fwd.z]);
    put("uCamTan", &[fx::mul(tan_y, aspect), tan_y]);
    put("uWave1", &air.waves[0]);
    put("uWave2", &air.waves[1]);
    put("uWater", &air.water);
    put("uSpec", &air.spec);
    put("uGlow", &[light.glow_fog_pct * ONE / 100]);
    v
}

/// One frame for the page: see the module's doc.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Frame {
    pub view: Vec<i32>,
    pub tris: Vec<i32>,
    /// the static world's terrain and water: first vertex and count of each
    /// (all 0 when no world is drawn)
    pub statics: [i32; 4],
    /// [`GROUP_VALUES`] per instanced draw
    pub groups: Vec<i32>,
    /// [`INSTANCE_VALUES`] per instance, in group order
    pub instances: Vec<i32>,
}

impl Frame {
    /// The triangles this frame draws, and its draw calls (the sky, the
    /// terrain, the posed list, each group and the water).
    pub fn stats(&self) -> (u64, u32) {
        let mut tris = (self.tris.len() / VERTEX_VALUES / 3) as u64;
        tris += (self.statics[1] / 3 + self.statics[3] / 3) as u64;
        let mut draws = 1;
        draws += (self.statics[1] > 0) as u32 + (self.statics[3] > 0) as u32 + (!self.tris.is_empty()) as u32;
        for g in self.groups.chunks(GROUP_VALUES) {
            tris += (g[3] / 3) as u64 * g[5] as u64;
            draws += 1;
        }
        (tris, draws)
    }
}

/// The drawing buffer's size for a canvas of `css` pixels at a device
/// pixel ratio of `dpr_pct` hundredths, at least one pixel each way.
pub fn buffer_size(css_w: i32, css_h: i32, dpr_pct: i32) -> [i32; 2] {
    [(css_w * dpr_pct / 100).max(1), (css_h * dpr_pct / 100).max(1)]
}

/// Width over height as Q16.
pub fn aspect(size: [i32; 2]) -> i32 {
    fx::ratio(size[0], size[1].max(1))
}

/// Every number the page's renderer needs, as JSON.
pub fn numbers_json() -> String {
    let b = BYTES_PER_VALUE as u32;
    let calls = ["", "uniform1fv", "uniform2fv", "uniform3fv", "uniform4fv"];
    let uniforms: Vec<serde_json::Value> = UNIFORMS
        .iter()
        .map(|(name, size)| json!([name, size, uniform_at(name).0, calls[*size]]))
        .collect();
    let (fog_at, _) = uniform_at("uFogColour");
    json!({
        "vertex": {
            "values": VERTEX_VALUES,
            "stride": STRIDE,
            "attributes": [["aPos", 3, 0], ["aNormal", 3, 3 * b], ["aColour", 3, 6 * b], ["aGlow", 1, 9 * b]],
        },
        // a prop's motion along its loop's cosine and sine, each at its
        // block's own offset
        "motion_cos": [["aPosCos", 3, 0], ["aNormalCos", 3, 3 * b]],
        "motion_sin": [["aPosSin", 3, 0], ["aNormalSin", 3, 3 * b]],
        "instance": {
            "values": INSTANCE_VALUES,
            "stride": INSTANCE_VALUES as u32 * b,
            "attributes": [["aC0", 3, 0], ["aC1", 3, 3 * b], ["aC2", 3, 6 * b], ["aT", 3, 9 * b], ["aLoop", 2, 12 * b]],
        },
        "group": GROUP_VALUES,
        // the sky's one triangle, which covers the screen, and its size
        "sky": [-1, -1, 3, -1, -1, 3],
        "sky_size": 2,
        "matrix": [0, MATRIX_VALUES],
        "uniforms": uniforms,
        "fog_colour": [fog_at, fog_at + 3],
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::fx::{deg, int};

    const LIGHT: Light = Light {
        fog: [255, 0, 0],
        sun_dir: [0, 100, 0],
        sun: [255, 255, 255],
        sky: [0, 0, 255],
        ground: [0, 255, 0],
        glow_fog_pct: 50,
    };

    fn uniform_in(n: &serde_json::Value, name: &str) -> (usize, usize) {
        let u = n["uniforms"]
            .as_array()
            .unwrap()
            .iter()
            .find(|u| u[0] == name)
            .unwrap();
        (u[2].as_u64().unwrap() as usize, u[1].as_u64().unwrap() as usize)
    }

    #[test]
    fn the_view_values_sit_where_the_numbers_say() {
        let cam = Camera {
            eye: v3(int(1), int(2), int(3)),
            fwd: v3(0, 0, -ONE),
            fov: deg(40),
            near: ONE / 10,
            far: int(50),
        };
        let v = view_values(&cam, ONE, (int(4), int(9)), &LIGHT, &Air::plain(&LIGHT));
        assert_eq!(v.len(), VIEW_VALUES);
        let n: serde_json::Value = serde_json::from_str(&numbers_json()).unwrap();
        let slice = |name: &str| {
            let (at, len) = uniform_in(&n, name);
            v[at..at + len].to_vec()
        };
        assert_eq!(slice("uEye"), vec![int(1), int(2), int(3)]);
        assert_eq!(slice("uFog"), vec![int(4), int(9)]);
        // the fog is pure red: 255 of 255 is ONE
        assert_eq!(slice("uFogColour"), vec![ONE, 0, 0]);
        assert_eq!(
            &v[21..24],
            &[ONE, 0, 0],
            "the fog colour follows the eye and the fog"
        );
        assert_eq!(n["fog_colour"], json!([21, 24]), "the clear colour is the fog's");
        // straight up, already one long
        assert_eq!(slice("uSunDir"), vec![0, ONE, 0]);
        assert_eq!(slice("uHemi"), vec![ONE / 2, ONE / 2]);
        // looking along -z: right is +x, up is +y
        assert_eq!(slice("uCamRight"), vec![ONE, 0, 0]);
        assert_eq!(slice("uCamUp"), vec![0, ONE, 0]);
        assert_eq!(slice("uGlow"), vec![ONE / 2]);
        // a plain sky has no tint under water
        assert_eq!(slice("uUnder"), vec![0, 0]);
        assert_eq!(n["vertex"]["stride"], 40);
        assert_eq!(n["instance"]["stride"], 56);
        // the uniforms end where the values do
        let (at, len) = uniform_in(&n, "uGlow");
        assert_eq!(at + len, VIEW_VALUES);
    }

    #[test]
    fn a_fitted_camera_stands_back_by_the_radius_over_half_the_view() {
        // sin 30° is a half, so a ball of radius 1 needs 2 back, and 10% more
        let d = Camera::fit(deg(60), ONE, 110);
        assert!((d - ONE * 22 / 10).abs() < 64, "{d}");
    }

    #[test]
    fn the_buffer_is_the_css_size_at_the_pixel_ratio_and_never_empty() {
        assert_eq!(buffer_size(800, 600, 150), [1200, 900]);
        assert_eq!(buffer_size(0, 0, 100), [1, 1]);
        assert_eq!(aspect([1600, 900]), fx::ratio(16, 9));
    }

    #[test]
    fn a_light_outside_its_range_or_with_no_sun_is_refused() {
        assert!(LIGHT.check().is_ok());
        let bad = Light {
            sky: [0, 0, 256],
            ..LIGHT
        };
        assert!(bad.check().is_err());
        let dark = Light {
            sun_dir: [0, 0, 0],
            ..LIGHT
        };
        assert_eq!(dark.check(), Err("The sun has no direction.".into()));
        let glare = Light {
            glow_fog_pct: 101,
            ..LIGHT
        };
        assert!(glare.check().is_err());
    }

    #[test]
    fn a_triangle_is_three_vertices_of_ten_values() {
        let mut out = Vec::new();
        push_tri(&mut out, [V3::ZERO; 3], v3(0, ONE, 0), [1, 2, 3]);
        assert_eq!(out.len(), 3 * VERTEX_VALUES);
        assert_eq!(&out[..10], &[0, 0, 0, 0, ONE, 0, 1, 2, 3, 0]);
        let mut lamp = Vec::new();
        push_tri_glow(&mut lamp, [V3::ZERO; 3], v3(0, ONE, 0), [1, 2, 3], ONE);
        assert_eq!(lamp[9], ONE, "a glowing triangle carries its glow");
    }

    #[test]
    fn a_frame_counts_its_triangles_and_draw_calls() {
        let f = Frame {
            tris: vec![0; 2 * 3 * VERTEX_VALUES],
            statics: [0, 30, 30, 6],
            // two groups: 12 vertices (4 triangles) 5 times, and 3 once
            groups: vec![0, 0, 0, 12, 0, 5, 0, 0, 0, 3, 0, 1],
            ..Frame::default()
        };
        // 2 posed + 10 terrain + 2 water + 20 + 1
        assert_eq!(f.stats(), (35, 6));
    }
}
