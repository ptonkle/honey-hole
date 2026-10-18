//! What the page's renderer draws: one triangle list a frame, each vertex
//! its position, flat normal and colour (Q16; the shim turns them into
//! floats), and the view values the shaders take as uniforms (the camera,
//! the fog, the sun). Every number the page needs to read these arrays
//! (strides, offsets, where each uniform sits in the view values) is in
//! [`numbers_json`], so the page holds none of its own.
//!
//! One list a frame is enough for the gallery. The game's world will want
//! its scenery uploaded once and drawn by instance, as Ape Race does
//! (SECOND-ORDER R1.45).

use serde::Deserialize;
use serde_json::json;

use super::fx::{self, div, sin, ONE};
use super::geom::{v3, Mat4, V3};

/// A colour, each channel 0..=ONE.
pub type Rgb = [i32; 3];

/// From 0–255 channels.
pub fn rgb(c: [i32; 3]) -> Rgb {
    c.map(|v| v * ONE / 255)
}

/// Values per vertex: position xyz, normal xyz, colour rgb.
pub const VERTEX_VALUES: usize = 9;
pub const BYTES_PER_VALUE: usize = 4;

/// How many vertices a triangle list holds.
pub fn vertex_count(values: &[i32]) -> u32 {
    (values.len() / VERTEX_VALUES) as u32
}

/// Pushes one flat triangle.
pub fn push_tri(out: &mut Vec<i32>, p: [V3; 3], n: V3, c: Rgb) {
    for q in p {
        out.extend_from_slice(&[q.x, q.y, q.z, n.x, n.y, n.z, c[0], c[1], c[2]]);
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

/// The light and the air, from `data/look.json`: 0–255 colours, and the
/// sun's direction (toward the sun) in hundredths.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Light {
    /// the fog's colour, which is the background's too
    pub fog: [i32; 3],
    pub sun_dir: [i32; 3],
    pub sun: [i32; 3],
    /// the light from above and from below, blended by how a face points
    pub sky: [i32; 3],
    pub ground: [i32; 3],
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
        Ok(())
    }
}

/// The view values the page uploads each frame, in the order
/// `numbers_json` names: the matrix (16), the eye (3), the fog's start and
/// end distances (2), the fog's colour (3), the sun's direction (3) and
/// colour (3), the sky's and the ground's light (6), and the blend of the
/// two by a face's height (2: n.y times the first, plus the second).
pub fn view_values(cam: &Camera, aspect: i32, fog: (i32, i32), light: &Light) -> Vec<i32> {
    let mut v = cam.matrix(aspect).0.to_vec();
    v.extend([cam.eye.x, cam.eye.y, cam.eye.z, fog.0, fog.1]);
    v.extend(rgb(light.fog));
    let [x, y, z] = light.sun_dir.map(|c| c * ONE / 100);
    let d = v3(x, y, z).norm();
    v.extend([d.x, d.y, d.z]);
    v.extend(rgb(light.sun));
    v.extend(rgb(light.sky));
    v.extend(rgb(light.ground));
    v.extend([ONE / 2, ONE / 2]);
    v
}

/// How many view values there are.
pub const VIEW_VALUES: usize = 38;

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
    json!({
        "vertex": {
            "values": VERTEX_VALUES,
            "stride": VERTEX_VALUES as u32 * b,
            "attributes": [["aPos", 3, 0], ["aNormal", 3, 3 * b], ["aColour", 3, 6 * b]],
        },
        "matrix": [0, 16],
        "uniforms": [
            // name, size, where it starts in the view values, the GL call
            ["uEye", 3, 16, "uniform3fv"],
            ["uFog", 2, 19, "uniform2fv"],
            ["uFogColour", 3, 21, "uniform3fv"],
            ["uSunDir", 3, 24, "uniform3fv"],
            ["uSun", 3, 27, "uniform3fv"],
            ["uSky", 3, 30, "uniform3fv"],
            ["uGround", 3, 33, "uniform3fv"],
            ["uHemi", 2, 36, "uniform2fv"],
        ],
        "fog_colour": [21, 24],
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
    };

    fn uniform_at(n: &serde_json::Value, name: &str) -> (usize, usize) {
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
        let v = view_values(&cam, ONE, (int(4), int(9)), &LIGHT);
        assert_eq!(v.len(), VIEW_VALUES);
        let n: serde_json::Value = serde_json::from_str(&numbers_json()).unwrap();
        let slice = |name: &str| {
            let (at, len) = uniform_at(&n, name);
            v[at..at + len].to_vec()
        };
        assert_eq!(slice("uEye"), vec![int(1), int(2), int(3)]);
        assert_eq!(slice("uFog"), vec![int(4), int(9)]);
        // the fog is pure red: 255 of 255 is ONE
        assert_eq!(slice("uFogColour"), vec![ONE, 0, 0]);
        assert_eq!(&v[21..24], &[ONE, 0, 0], "the clear colour is the fog's");
        // straight up, already one long
        assert_eq!(slice("uSunDir"), vec![0, ONE, 0]);
        assert_eq!(slice("uHemi"), vec![ONE / 2, ONE / 2]);
        assert_eq!(n["vertex"]["stride"], 36);
        // the uniforms end where the values do
        let (at, len) = uniform_at(&n, "uHemi");
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
    }

    #[test]
    fn a_triangle_is_three_vertices_of_nine_values() {
        let mut out = Vec::new();
        push_tri(&mut out, [V3::ZERO; 3], v3(0, ONE, 0), [1, 2, 3]);
        assert_eq!(out.len(), 3 * VERTEX_VALUES);
        assert_eq!(&out[..9], &[0, 0, 0, 0, ONE, 0, 1, 2, 3]);
    }
}
