//! Integer angles and roots: no floats in the core.
//!
//! An angle is in binary units: a full turn is [`TURN`] (65536), so 0 points
//! along +x (east) and `TURN / 4` along +y (north). [`sin`] and [`cos`]
//! return values scaled by [`ONE`] (16384). Bhaskara's approximation keeps
//! the sine within 0.2% of the true value, and [`atan2`] is within about
//! 40 units (a quarter of a degree): close enough to shape a shoreline and
//! aim a boat, not for anything that must add up over many steps.

pub const TURN: i32 = 65536;
pub const HALF: i32 = TURN / 2;
pub const QUARTER: i32 = TURN / 4;
pub const ONE: i32 = 16384;

/// The angle brought into `0..TURN`.
pub fn wrap(a: i32) -> i32 {
    a.rem_euclid(TURN)
}

/// Sine scaled by [`ONE`].
pub fn sin(a: i32) -> i32 {
    let a = wrap(a);
    let (x, sign) = if a < HALF {
        (a as i64, 1)
    } else {
        ((a - HALF) as i64, -1)
    };
    let h = HALF as i64;
    let p = x * (h - x);
    let v = 16 * p * ONE as i64 / (5 * h * h - 4 * p);
    sign * v as i32
}

/// Cosine scaled by [`ONE`].
pub fn cos(a: i32) -> i32 {
    sin(a + QUARTER)
}

/// The angle of the vector `(x, y)`, in `0..TURN`; 0 for the zero vector.
pub fn atan2(y: i64, x: i64) -> i32 {
    if x == 0 && y == 0 {
        return 0;
    }
    let (ax, ay) = (x.abs(), y.abs());
    // t = min / max in 0..=ONE; atan(t) ~ (pi/4) t + 0.273 t (1 - t) radians
    let (lo, hi) = if ax >= ay { (ay, ax) } else { (ax, ay) };
    let t = lo * ONE as i64 / hi;
    let eighth = (TURN / 8) as i64;
    let a = (eighth * t + 2847 * t * (ONE as i64 - t) / ONE as i64) / ONE as i64;
    let a = a as i32;
    let a = if ax >= ay { a } else { QUARTER - a };
    let a = if x < 0 { HALF - a } else { a };
    wrap(if y < 0 { -a } else { a })
}

/// The whole square root of a non-negative number.
pub fn isqrt(v: i64) -> i64 {
    if v <= 0 {
        0
    } else {
        (v as u64).isqrt() as i64
    }
}

/// The length of `(x, y)`.
pub fn hypot(x: i64, y: i64) -> i64 {
    isqrt(x * x + y * y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sines_and_angles_match_known_values() {
        assert_eq!(sin(0), 0);
        assert_eq!(sin(QUARTER), ONE);
        assert_eq!(sin(HALF), 0);
        assert_eq!(sin(3 * QUARTER), -ONE);
        assert_eq!(cos(0), ONE);
        // sin 30 degrees = 0.5; TURN / 12 is 30 degrees
        assert!((sin(TURN / 12) - ONE / 2).abs() <= 40, "{}", sin(TURN / 12));
        // sin 45 degrees = 0.7071 -> 11585
        assert!((sin(TURN / 8) - 11585).abs() <= 40, "{}", sin(TURN / 8));
        assert_eq!(atan2(0, 5), 0);
        assert_eq!(atan2(5, 0), QUARTER);
        assert_eq!(atan2(0, -5), HALF);
        assert_eq!(atan2(-5, 0), 3 * QUARTER);
        assert!((atan2(5, 5) - TURN / 8).abs() <= 2);
        assert!((atan2(-5, -5) - 5 * TURN / 8).abs() <= 2);
        // 30 degrees: (cos, sin) = (0.866, 0.5)
        assert!((atan2(500, 866) - TURN / 12).abs() <= 45, "{}", atan2(500, 866));
        assert_eq!(isqrt(1_000_000), 1000);
        assert_eq!(isqrt(999_999), 999);
        assert_eq!(hypot(3000, 4000), 5000);
    }
}
