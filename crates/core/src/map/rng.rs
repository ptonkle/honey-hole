//! A small integer random generator (SplitMix64), so a seed gives the same
//! numbers on every machine. The core reads no random device: every seed
//! comes in from outside (the match, the map list).
//!
//! Each stage of generation draws from its own stream (`Rng::stream`), so a
//! change to how trees are placed does not move the shoreline of every seed.

/// SplitMix64 (Steele, Lea and Flood, 2014).
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

/// Mixes one 64-bit value into another well-spread one.
pub fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A hash of a seed and two lattice coordinates, for value noise.
pub fn hash2(seed: u64, a: i32, b: i32) -> u64 {
    let k = mix(seed ^ mix((a as u32 as u64) | ((b as u32 as u64) << 32)));
    mix(k.wrapping_add(GOLDEN))
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng {
            state: mix(seed.wrapping_add(GOLDEN)),
        }
    }

    /// An independent stream for one stage, named by `salt`.
    pub fn stream(seed: u64, salt: u64) -> Rng {
        Rng::new(mix(seed) ^ mix(salt.wrapping_mul(GOLDEN)))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GOLDEN);
        mix(self.state)
    }

    /// A number in `0..n` (0 when `n` is 0).
    pub fn below(&mut self, n: u32) -> u32 {
        (((self.next_u64() >> 32) * n as u64) >> 32) as u32
    }

    /// A number in `lo..=hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + self.below((hi - lo + 1) as u32) as i32
    }

    /// True `permille` times in a thousand.
    pub fn chance(&mut self, permille: u32) -> bool {
        self.below(1000) < permille
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_gives_the_same_numbers_and_ranges_hold() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        let mut c = Rng::new(8);
        let xs: Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
        let ys: Vec<u64> = (0..8).map(|_| b.next_u64()).collect();
        let zs: Vec<u64> = (0..8).map(|_| c.next_u64()).collect();
        assert_eq!(xs, ys);
        assert_ne!(xs, zs);
        let mut seen = [0u32; 5];
        for _ in 0..5000 {
            let v = a.range(2, 6);
            assert!((2..=6).contains(&v));
            seen[(v - 2) as usize] += 1;
        }
        // each of five values about 1000 times
        assert!(seen.iter().all(|&s| (850..1150).contains(&s)), "{seen:?}");
    }
}
