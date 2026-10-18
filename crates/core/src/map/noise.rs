//! Value noise on an integer lattice: smooth bumps that make a shoreline
//! ragged, a forest clumpy and a lake bed uneven.

use super::rng::hash2;

/// Noise at a point in millimetres, for bumps about `wavelength` mm apart,
/// in `-1024..=1024`.
pub fn value(seed: u64, x: i32, y: i32, wavelength: i32) -> i32 {
    let gx = x.div_euclid(wavelength);
    let gy = y.div_euclid(wavelength);
    // position inside the lattice cell, 0..4096, eased with 3t^2 - 2t^3
    let fx = ease((x.rem_euclid(wavelength) as i64 * 4096 / wavelength as i64) as i32);
    let fy = ease((y.rem_euclid(wavelength) as i64 * 4096 / wavelength as i64) as i32);
    let at = |i: i32, j: i32| ((hash2(seed, gx + i, gy + j) & 2047) as i32) - 1024;
    let top = lerp(at(0, 0), at(1, 0), fx);
    let bottom = lerp(at(0, 1), at(1, 1), fx);
    lerp(top, bottom, fy)
}

/// Several octaves of [`value`], each half the wavelength and half the
/// weight of the last; about `-1024..=1024`.
pub fn fractal(seed: u64, x: i32, y: i32, wavelength: i32, octaves: u32) -> i32 {
    let mut sum = 0;
    let mut weight = 1024;
    let mut total = 0;
    let mut w = wavelength;
    for o in 0..octaves {
        sum += value(seed.wrapping_add(o as u64 * 7919), x, y, w.max(1)) * weight;
        total += weight;
        weight /= 2;
        w /= 2;
    }
    sum / total.max(1)
}

fn ease(t: i32) -> i32 {
    let t = t as i64;
    ((3 * t * t * 4096 - 2 * t * t * t) / (4096 * 4096)) as i32
}

fn lerp(a: i32, b: i32, t: i32) -> i32 {
    a + ((b - a) as i64 * t as i64 / 4096) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_stays_in_range_and_is_smooth() {
        let mut prev = value(3, 0, 1234, 10_000);
        for i in 1..2000 {
            let v = value(3, i * 50, 1234, 10_000);
            assert!((-1024..=1024).contains(&v));
            // 50 mm steps on a 10 m wavelength never jump far
            assert!((v - prev).abs() <= 40, "jump {prev} -> {v} at {i}");
            prev = v;
        }
        assert_ne!(value(3, 5000, 5000, 10_000), value(4, 5000, 5000, 10_000));
    }
}
