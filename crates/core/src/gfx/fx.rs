//! Q16.16 fixed point and integer angles, ported from Ape Race's `fx.rs`
//! (its seeded dice left out: nothing here draws a chance).
//!
//! The core may not use floats (BUILDER-SPEC D2), so every length, speed,
//! time and matrix entry is an `i32` with 16 fractional bits: `ONE` is 1.0,
//! one metre or one second. Products go through `i64` so they cannot
//! overflow on the way. Angles are integers with `TURN` units to a full
//! circle, so wrapping is a mask.

/// 1.0 in Q16.16.
pub const ONE: i32 = 1 << 16;
pub const FRAC_BITS: u32 = 16;
/// Angle units in a full turn.
pub const TURN: i32 = 1 << 16;
pub const QUARTER: i32 = TURN / 4;
pub const HALF: i32 = TURN / 2;

/// 2π in Q16.16 (6.283185 × 65 536).
pub const TWO_PI: i64 = 411_775;

pub const fn int(n: i32) -> i32 {
    n * ONE
}

/// `num / den` as Q16.16, for constants written as fractions.
pub const fn ratio(num: i32, den: i32) -> i32 {
    ((num as i64 * ONE as i64) / den as i64) as i32
}

/// Hundredths: `cm(150)` is 1.5.
pub const fn cm(n: i32) -> i32 {
    ratio(n, 100)
}

pub const fn mul(a: i32, b: i32) -> i32 {
    ((a as i64 * b as i64) >> FRAC_BITS) as i32
}

/// `a / b`. A zero divisor gives zero: every caller treats "no length" as
/// "no direction".
pub fn div(a: i32, b: i32) -> i32 {
    if b == 0 {
        return 0;
    }
    (((a as i64) << FRAC_BITS) / b as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// Linear blend: `t = 0` gives `a`, `t = ONE` gives `b`.
pub fn lerp(a: i32, b: i32, t: i32) -> i32 {
    a + (((b as i64 - a as i64) * t as i64) >> FRAC_BITS) as i32
}

/// Moves `from` toward `to` by at most `step`.
pub fn approach(from: i32, to: i32, step: i32) -> i32 {
    if from < to {
        (from + step).min(to)
    } else {
        (from - step).max(to)
    }
}

/// Degrees to angle units.
pub const fn deg(d: i32) -> i32 {
    ((d as i64 * TURN as i64) / 360) as i32
}

/// Wraps an angle into `[-HALF, HALF)`, for differences between headings.
pub fn wrap(a: i32) -> i32 {
    ((a + HALF) & (TURN - 1)) - HALF
}

/// Sine of an angle in `TURN` units, as Q16.16: the angle folded into the
/// quarter wave around zero, in radians, summed as a Taylor series to the
/// 11th power, exact to the last bit over `[-π/2, π/2]`.
pub fn sin(angle: i32) -> i32 {
    let a = angle & (TURN - 1);
    let x = if a < QUARTER {
        a
    } else if a < 3 * QUARTER {
        HALF - a
    } else {
        a - TURN
    };
    let r = x as i64 * TWO_PI / TURN as i64;
    let r2 = (r * r) >> FRAC_BITS;
    let mut term = r;
    let mut sum = r;
    for k in 1..=5i64 {
        term = -((term * r2) >> FRAC_BITS) / ((2 * k) * (2 * k + 1));
        sum += term;
    }
    sum.clamp(-(ONE as i64), ONE as i64) as i32
}

pub fn cos(angle: i32) -> i32 {
    sin(angle + QUARTER)
}

/// The angle of the point (x, y) from the +x axis, in `TURN` units, within
/// about 0.1°. Polynomial arctangent on the first octant, then folded.
pub fn atan2(y: i32, x: i32) -> i32 {
    if x == 0 && y == 0 {
        return 0;
    }
    let ax = (x as i64).abs();
    let ay = (y as i64).abs();
    let (num, den, swap) = if ay <= ax { (ay, ax, false) } else { (ax, ay, true) };
    // t in [0, 1] as Q16
    let t = (num << FRAC_BITS) / den;
    // atan(t) ≈ π/4·t − t(t − 1)(0.2447 + 0.0663t), radians; in turn units:
    // π/4 rad = TURN/8; 0.2447 rad = 2552 units; 0.0663 rad = 691 units.
    let one = ONE as i64;
    let base = t * (TURN as i64 / 8) / one;
    let bend = (t * (t - one) / one) * (2552 + 691 * t / one) / one;
    let mut a = base - bend;
    if swap {
        a = QUARTER as i64 - a;
    }
    if x < 0 {
        a = HALF as i64 - a;
    }
    if y < 0 {
        a = -a;
    }
    wrap(a as i32)
}

/// Integer square root (floor): the standard library's, all integers, so
/// the same answer as the Newton's iteration it replaced (`isqrt_newton` in
/// the tests) at a fraction of the cost, through 64 bits when the value
/// fits. (The AI's copies of the ape take millions of these a second.)
pub fn isqrt(n: u128) -> u128 {
    if n <= u64::MAX as u128 {
        (n as u64).isqrt() as u128
    } else {
        n.isqrt()
    }
}

/// `isqrt(s << 32)`, the same answer, cheaper (`V3::norm` takes one for
/// every direction): the root of `s` in 64 bits, then its next 16 bits from
/// the remainder (a first guess never too small, and at most a few over).
pub fn isqrt_shl32(s: u64) -> u64 {
    if s < 1 << 32 {
        return (s << 32).isqrt();
    }
    // s = r² + rem; s·2³² = (r·2¹⁶ + q)² + ... with q·(r·2¹⁷ + q) <= rem·2³²
    let r = s.isqrt();
    let rem = s - r * r;
    let n = (s as u128) << 32;
    let mut root = (r << 16) + (rem << 15) / r;
    while root as u128 * root as u128 > n {
        root -= 1;
    }
    root
}

/// Square root of a non-negative Q16.16 value.
pub fn sqrt(v: i32) -> i32 {
    if v <= 0 {
        return 0;
    }
    isqrt((v as u128) << FRAC_BITS) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_hits_its_landmarks() {
        assert_eq!(sin(0), 0);
        assert!((sin(QUARTER) - ONE).abs() <= 1);
        assert!((sin(deg(30)) - ONE / 2).abs() <= 8, "{}", sin(deg(30)));
        assert!((sin(HALF)).abs() <= 1);
        assert!((sin(-QUARTER) + ONE).abs() <= 1);
        assert!((cos(0) - ONE).abs() <= 1);
        assert!((cos(deg(60)) - ONE / 2).abs() <= 8);
    }

    #[test]
    fn atan2_inverts_sine_and_cosine() {
        for d in (-179..180).step_by(7) {
            let a = deg(d);
            let back = atan2(sin(a), cos(a));
            assert!(wrap(back - a).abs() <= deg(1) / 5, "{d}: {back} vs {a}");
        }
        assert_eq!(atan2(0, 0), 0);
    }

    #[test]
    fn products_and_quotients_round_trip() {
        assert_eq!(mul(int(3), ONE / 2), ONE * 3 / 2);
        assert_eq!(div(int(3), int(2)), ONE * 3 / 2);
        assert_eq!(div(int(3), 0), 0);
        assert!((mul(div(int(7), int(3)), int(3)) - int(7)).abs() <= 2);
        assert_eq!(cm(150), ONE * 3 / 2);
    }

    #[test]
    fn square_roots() {
        assert_eq!(sqrt(int(4)), int(2));
        assert_eq!(sqrt(int(9)), int(3));
        assert!((sqrt(int(2)) - 92_682).abs() <= 1);
        assert_eq!(sqrt(-5), 0);
        assert_eq!(isqrt(1 << 100), 1 << 50);
    }

    #[test]
    fn lerp_and_approach() {
        assert_eq!(lerp(0, int(10), ONE / 2), int(5));
        assert_eq!(approach(0, 10, 3), 3);
        assert_eq!(approach(9, 10, 3), 10);
        assert_eq!(approach(10, 0, 4), 6);
    }
}
