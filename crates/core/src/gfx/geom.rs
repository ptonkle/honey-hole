//! Vectors and matrices in Q16.16, ported from Ape Race's `geom.rs`. Y is up. A heading (yaw) of 0
//! faces +z, and a positive yaw turns toward +x.

use super::fx::{self, cos, div, isqrt, mul, sin, ONE};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct V3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

pub const fn v3(x: i32, y: i32, z: i32) -> V3 {
    V3 { x, y, z }
}

// Plain methods, not operator traits: the vector maths reads as calls.
#[allow(clippy::should_implement_trait, clippy::len_without_is_empty)]
impl V3 {
    pub const ZERO: V3 = v3(0, 0, 0);
    pub const UP: V3 = v3(0, ONE, 0);

    pub fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    pub fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
    pub fn scale(self, s: i32) -> V3 {
        v3(mul(self.x, s), mul(self.y, s), mul(self.z, s))
    }
    pub fn neg(self) -> V3 {
        v3(-self.x, -self.y, -self.z)
    }
    /// The dot product as Q16, through 64 bits.
    pub fn dot(self, o: V3) -> i32 {
        (self.dot64(o) >> fx::FRAC_BITS).clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }
    /// The dot product as Q32 in an `i64`, for long vectors.
    pub fn dot64(self, o: V3) -> i64 {
        self.x as i64 * o.x as i64 + self.y as i64 * o.y as i64 + self.z as i64 * o.z as i64
    }
    pub fn cross(self, o: V3) -> V3 {
        let c = |a: i32, b: i32, c: i32, d: i32| {
            ((a as i64 * b as i64 - c as i64 * d as i64) >> fx::FRAC_BITS) as i32
        };
        v3(
            c(self.y, o.z, self.z, o.y),
            c(self.z, o.x, self.x, o.z),
            c(self.x, o.y, self.y, o.x),
        )
    }
    pub fn len(self) -> i32 {
        isqrt(self.len2() as u128) as i32
    }
    /// The squared length, raw (Q32): three squares of `i32`s fit a `u64`.
    fn len2(self) -> u64 {
        let sq = |a: i32| (a as i64 * a as i64) as u64;
        sq(self.x) + sq(self.y) + sq(self.z)
    }
    pub fn len_xz(self) -> i32 {
        v3(self.x, 0, self.z).len()
    }
    /// A vector of length `ONE` the same way, or zero for zero.
    pub fn norm(self) -> V3 {
        let s = self.len2();
        if s == 0 {
            return V3::ZERO;
        }
        // the length with 32 more bits, so short vectors keep their direction
        let l = fx::isqrt_shl32(s) as i64;
        // (a component shifted 32 bits fits an `i64`, and so does the
        // length: the same quotient as in 128 bits, far cheaper)
        let c = |a: i32| (((a as i64) << 32) / l) as i32;
        v3(c(self.x), c(self.y), c(self.z))
    }
    /// Whether `len()` is over `k`, without the square root: the floor of
    /// the root is over `k` exactly when the square is at least (k + 1)².
    pub fn longer_than(self, k: i32) -> bool {
        k < 0 || self.len2() >= (k as u64 + 1) * (k as u64 + 1)
    }
    /// The same direction, at most `max` long.
    pub fn clamp_len(self, max: i32) -> V3 {
        // (most are short enough: no square root for them)
        if !self.longer_than(max) {
            return self;
        }
        let l = self.len();
        if l <= max || l == 0 {
            return self;
        }
        self.scale(div(max, l))
    }
    /// The same direction, exactly `len` long.
    pub fn with_len(self, len: i32) -> V3 {
        self.norm().scale(len)
    }
    pub fn lerp(self, o: V3, t: i32) -> V3 {
        v3(
            fx::lerp(self.x, o.x, t),
            fx::lerp(self.y, o.y, t),
            fx::lerp(self.z, o.z, t),
        )
    }
    pub fn dist(self, o: V3) -> i32 {
        self.sub(o).len()
    }
    pub fn flat(self) -> V3 {
        v3(self.x, 0, self.z)
    }
    /// The part of this vector across `n` (a unit vector).
    pub fn reject(self, n: V3) -> V3 {
        self.sub(n.scale(self.dot(n)))
    }
}

/// The unit vector for a heading and a pitch.
pub fn facing(yaw: i32, pitch: i32) -> V3 {
    let cp = cos(pitch);
    v3(mul(sin(yaw), cp), sin(pitch), mul(cos(yaw), cp))
}

/// The heading of a direction, ignoring its height.
pub fn yaw_of(d: V3) -> i32 {
    fx::atan2(d.x, d.z)
}

/// The pitch of a direction.
pub fn pitch_of(d: V3) -> i32 {
    fx::atan2(d.y, d.len_xz())
}

/// Screen-right for a camera facing `fwd` with +y up.
pub fn right_of(fwd: V3) -> V3 {
    fwd.cross(V3::UP).norm()
}

/// An affine transform: three columns (the images of the unit axes) and a
/// translation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Affine {
    pub c0: V3,
    pub c1: V3,
    pub c2: V3,
    pub t: V3,
}

impl Affine {
    pub const IDENTITY: Affine = Affine {
        c0: v3(ONE, 0, 0),
        c1: v3(0, ONE, 0),
        c2: v3(0, 0, ONE),
        t: V3::ZERO,
    };

    pub fn translate(t: V3) -> Affine {
        Affine {
            t,
            ..Affine::IDENTITY
        }
    }
    pub fn scale(s: V3) -> Affine {
        Affine {
            c0: v3(s.x, 0, 0),
            c1: v3(0, s.y, 0),
            c2: v3(0, 0, s.z),
            t: V3::ZERO,
        }
    }
    pub fn rot_y(a: i32) -> Affine {
        let (s, c) = (sin(a), cos(a));
        Affine {
            c0: v3(c, 0, -s),
            c1: v3(0, ONE, 0),
            c2: v3(s, 0, c),
            t: V3::ZERO,
        }
    }
    pub fn rot_x(a: i32) -> Affine {
        let (s, c) = (sin(a), cos(a));
        Affine {
            c0: v3(ONE, 0, 0),
            c1: v3(0, c, s),
            c2: v3(0, -s, c),
            t: V3::ZERO,
        }
    }
    pub fn rot_z(a: i32) -> Affine {
        let (s, c) = (sin(a), cos(a));
        Affine {
            c0: v3(c, s, 0),
            c1: v3(-s, c, 0),
            c2: v3(0, 0, ONE),
            t: V3::ZERO,
        }
    }
    /// A frame whose +z points along `dir` and whose +y is as near to `up`
    /// as it can be.
    pub fn looking(dir: V3, up: V3) -> Affine {
        let z = dir.norm();
        let mut x = up.cross(z).norm();
        if x == V3::ZERO {
            x = v3(ONE, 0, 0).cross(z).norm();
        }
        let y = z.cross(x).norm();
        Affine {
            c0: x,
            c1: y,
            c2: z,
            t: V3::ZERO,
        }
    }
    /// A frame whose +y points along `dir` (a rod from the origin along it).
    pub fn along(dir: V3) -> Affine {
        let y = dir.norm();
        let side = if y.y.abs() > ONE * 9 / 10 {
            v3(ONE, 0, 0)
        } else {
            V3::UP
        };
        let x = y.cross(side).norm();
        let z = x.cross(y).norm();
        Affine {
            c0: x,
            c1: y,
            c2: z,
            t: V3::ZERO,
        }
    }
    pub fn apply(&self, p: V3) -> V3 {
        self.apply_dir(p).add(self.t)
    }
    pub fn apply_dir(&self, d: V3) -> V3 {
        self.c0.scale(d.x).add(self.c1.scale(d.y)).add(self.c2.scale(d.z))
    }
    /// This transform after `o`: `a.then(&b).apply(p) == a.apply(b.apply(p))`.
    pub fn then(&self, o: &Affine) -> Affine {
        Affine {
            c0: self.apply_dir(o.c0),
            c1: self.apply_dir(o.c1),
            c2: self.apply_dir(o.c2),
            t: self.apply(o.t),
        }
    }
    pub fn at(&self, t: V3) -> Affine {
        Affine { t, ..*self }
    }
}

/// A 4×4 matrix, column-major (as WebGL takes it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mat4(pub [i32; 16]);

impl Mat4 {
    pub fn get(&self, row: usize, col: usize) -> i32 {
        self.0[col * 4 + row]
    }

    pub fn mul(&self, o: &Mat4) -> Mat4 {
        let mut m = [0i32; 16];
        for col in 0..4 {
            for row in 0..4 {
                let mut s: i64 = 0;
                for k in 0..4 {
                    s += self.get(row, k) as i64 * o.get(k, col) as i64;
                }
                m[col * 4 + row] = (s >> fx::FRAC_BITS) as i32;
            }
        }
        Mat4(m)
    }

    /// A perspective projection: vertical field of view `fov` (angle units),
    /// `aspect` = width / height (Q16).
    pub fn perspective(fov: i32, aspect: i32, near: i32, far: i32) -> Mat4 {
        let half = fov / 2;
        let f = div(cos(half), sin(half));
        let mut m = [0i32; 16];
        m[0] = div(f, aspect);
        m[5] = f;
        m[10] = div(far + near, near - far);
        m[11] = -ONE;
        m[14] = ((2 * far as i64 * near as i64) / (near - far) as i64) as i32;
        Mat4(m)
    }

    /// The view transform of a camera at `eye` looking along `fwd`.
    pub fn look(eye: V3, fwd: V3, up: V3) -> Mat4 {
        let f = fwd.norm();
        let s = f.cross(up).norm();
        let u = s.cross(f);
        let mut m = [0i32; 16];
        m[0] = s.x;
        m[4] = s.y;
        m[8] = s.z;
        m[1] = u.x;
        m[5] = u.y;
        m[9] = u.z;
        m[2] = -f.x;
        m[6] = -f.y;
        m[10] = -f.z;
        m[12] = -s.dot(eye);
        m[13] = -u.dot(eye);
        m[14] = f.dot(eye);
        m[15] = ONE;
        Mat4(m)
    }

    /// `M · (p, 1)` as `[x, y, z, w]`.
    pub fn project(&self, p: V3) -> [i32; 4] {
        let mut out = [0i32; 4];
        for (row, o) in out.iter_mut().enumerate() {
            let s = self.get(row, 0) as i64 * p.x as i64
                + self.get(row, 1) as i64 * p.y as i64
                + self.get(row, 2) as i64 * p.z as i64;
            *o = ((s >> fx::FRAC_BITS) + self.get(row, 3) as i64) as i32;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::fx::{deg, int};
    use super::*;

    #[test]
    fn norm_gives_unit_length() {
        for v in [
            v3(int(3), int(4), 0),
            v3(-7, 3, 11),
            v3(int(200), int(-50), int(900)),
        ] {
            let n = v.norm();
            assert!((n.len() - ONE).abs() <= 4, "{v:?} -> {n:?}");
        }
        assert_eq!(V3::ZERO.norm(), V3::ZERO);
        assert_eq!(v3(int(3), int(4), 0).len(), int(5));
    }

    #[test]
    fn long_vectors_do_not_overflow() {
        let v = v3(int(3000), int(4000), 0);
        assert_eq!(v.len(), int(5000));
        assert_eq!(v.dot64(v) >> 32, 25_000_000);
    }

    #[test]
    fn rotation_keeps_length_and_turns_the_right_way() {
        let r = Affine::rot_y(deg(90));
        let p = r.apply(v3(0, 0, ONE));
        assert!((p.x - ONE).abs() <= 2 && p.z.abs() <= 2, "{p:?}");
        let q = Affine::rot_y(deg(37)).apply(v3(int(2), int(1), int(-3)));
        assert!((q.len() - v3(int(2), int(1), int(-3)).len()).abs() <= 8);
    }

    #[test]
    fn facing_and_yaw_agree() {
        for d in [-150, -90, -20, 0, 45, 90, 170] {
            let f = facing(deg(d), 0);
            assert!(fx::wrap(yaw_of(f) - deg(d)).abs() <= deg(1) / 5, "{d}");
        }
        assert!((facing(0, 0).z - ONE).abs() <= 1);
    }

    #[test]
    fn then_composes_in_order() {
        let a = Affine::translate(v3(ONE, 0, 0));
        let b = Affine::scale(v3(int(2), int(2), int(2)));
        let p = v3(ONE, 0, 0);
        assert_eq!(a.then(&b).apply(p), a.apply(b.apply(p)));
        assert_eq!(a.then(&b).apply(p), v3(int(3), 0, 0));
    }

    #[test]
    fn looking_points_z_along_the_direction() {
        let a = Affine::looking(v3(ONE, 0, ONE), V3::UP);
        let z = a.apply_dir(v3(0, 0, ONE));
        assert!(z.sub(v3(ONE, 0, ONE).norm()).len() < 16);
        let along = Affine::along(v3(0, -ONE, ONE));
        let y = along.apply_dir(v3(0, ONE, 0));
        assert!(y.sub(v3(0, -ONE, ONE).norm()).len() < 16);
    }

    #[test]
    fn the_look_point_projects_to_the_centre() {
        let eye = v3(int(2), int(5), int(-10));
        let fwd = facing(deg(30), deg(-10));
        let vp =
            Mat4::perspective(deg(70), ONE * 16 / 9, ONE / 5, int(300)).mul(&Mat4::look(eye, fwd, V3::UP));
        let target = eye.add(fwd.scale(int(20)));
        let [x, y, _, w] = vp.project(target);
        assert!(w > 0);
        assert!(div(x, w).abs() < ONE / 200 && div(y, w).abs() < ONE / 200);
        // something to the camera's right lands right of centre
        let r = target.add(right_of(fwd).scale(int(3)));
        let [x, _, _, w] = vp.project(r);
        assert!(div(x, w) > 0);
    }
}
