//! A wreck's burst: a short explosion drawn by the core in the world (a
//! glowing fireball that swells and dies, and shards thrown out and
//! falling back to the water), and the camera shake the wrecked player
//! feels. Every number is `data/flow.json`'s `burst`; the shards' scatter
//! comes from the wreck's own seat and tick, so a replay draws it the same.

use crate::gfx::fx::{self, mul, ONE, TURN};
use crate::gfx::geom::{v3, V3};
use crate::gfx::render::{push_tri_glow, rgb};
use crate::map::rng::mix;
use crate::scene::mm;

use super::look::BurstLook;

/// One wreck to draw: where (in the world) and what seeded its scatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Burst {
    pub seat: usize,
    pub at: V3,
    /// page µs when it went off
    pub born_us: i64,
    pub seed: u64,
}

/// A colour from the list, `t` (0 to ONE) of the way from hot to cool.
fn shade(colours: &[[i32; 3]], t: i32) -> [i32; 3] {
    let n = colours.len() as i32;
    if n == 1 {
        return rgb(colours[0]);
    }
    let pos = (t.clamp(0, ONE) as i64 * (n - 1) as i64) as i32;
    let k = (pos >> 16).min(n - 2) as usize;
    let f = pos - ((k as i32) << 16);
    let (a, b) = (rgb(colours[k]), rgb(colours[k + 1]));
    [0, 1, 2].map(|i| fx::lerp(a[i], b[i], f.min(ONE)))
}

/// A number from the seed, 0 to ONE.
fn pick(seed: u64, k: u64) -> i32 {
    (mix(seed.wrapping_add(k.wrapping_mul(0x9e37_79b9))) % (ONE as u64 + 1)) as i32
}

/// The fireball's radius (mm) `age` ms in: swelling fast to its widest at
/// `swell_pct` of the burst, then shrinking to nothing at the end.
pub fn fireball_mm(look: &BurstLook, age_ms: i32) -> i32 {
    let peak = look.ms * look.swell_pct / 100;
    if age_ms < 0 || age_ms >= look.ms {
        return 0;
    }
    if age_ms < peak {
        (look.fireball_mm as i64 * age_ms as i64 / peak.max(1) as i64) as i32
    } else {
        (look.fireball_mm as i64 * (look.ms - age_ms) as i64 / (look.ms - peak).max(1) as i64) as i32
    }
}

/// An octahedron's eight faces round `c`, `r` (world units) across.
fn ball(out: &mut Vec<i32>, c: V3, r: i32, colour: [i32; 3]) {
    let px = [v3(r, 0, 0), v3(-r, 0, 0)];
    let py = [v3(0, r, 0), v3(0, -r, 0)];
    let pz = [v3(0, 0, r), v3(0, 0, -r)];
    for x in px {
        for y in py {
            for z in pz {
                let (a, b, d) = (c.add(x), c.add(y), c.add(z));
                let n = x.add(y).add(z).norm();
                push_tri_glow(out, [a, b, d], n, colour, ONE);
            }
        }
    }
}

/// The burst's triangles `age` ms after it went off (none once it is
/// over), glowing so they read through the fog.
pub fn tris(b: &Burst, look: &BurstLook, age_ms: i32) -> Vec<i32> {
    let mut out = Vec::new();
    if age_ms < 0 || age_ms >= look.ms {
        return out;
    }
    let life = fx::ratio(age_ms, look.ms);
    let r = fireball_mm(look, age_ms);
    if r > 0 {
        ball(&mut out, b.at.add(v3(0, mm(r / 2), 0)), mm(r), shade(&look.colours, life / 2));
    }
    // seconds in, Q16
    let s = fx::ratio(age_ms, 1000);
    for k in 0..look.shards.max(0) as u64 {
        let turn = (k as i64 * TURN as i64 / look.shards.max(1) as i64) as i32
            + mul(pick(b.seed, k), TURN / look.shards.max(1));
        let speed = mul(mm(look.speed_mm_per_s), ONE / 2 + pick(b.seed, k + 100) / 2);
        let up = mul(mm(look.up_mm_per_s), ONE / 2 + pick(b.seed, k + 200) / 2);
        let fall = mm(look.fall_mm_per_s2);
        let (c, sn) = (fx::cos(turn), fx::sin(turn));
        let out_d = mul(speed, s);
        let height = mul(up, s) - mul(fall, mul(s, s)) / 2;
        if height < -mm(look.shard_mm) {
            continue;
        }
        let p = b.at.add(v3(mul(c, out_d), height.max(0), mul(sn, out_d)));
        let half = mm(look.shard_mm) / 2;
        // each shard tumbles as it flies
        let spin = (pick(b.seed, k + 300) as i64 * TURN as i64 / ONE as i64) as i32 + mul(TURN, s);
        let (sc, ss) = (fx::cos(spin), fx::sin(spin));
        let a = p.add(v3(mul(sc, half), mul(ss, half), 0));
        let bb = p.add(v3(-mul(ss, half), mul(sc, half), mul(sc, half)));
        let d = p.add(v3(0, -mul(sc, half), mul(ss, half)));
        let tint = shade(&look.colours, (life + pick(b.seed, k + 400) / 3).min(ONE));
        push_tri_glow(&mut out, [a, bb, d], V3::UP, tint, ONE);
    }
    out
}

/// The wrecked player's camera shake `age` ms in, as (across, up) offsets
/// in world units: a wobble at `shake_hz` dying away over `shake_ms`.
pub fn shake(look: &BurstLook, age_ms: i32) -> (i32, i32) {
    if age_ms < 0 || age_ms >= look.shake_ms {
        return (0, 0);
    }
    let left = mm(look.shake_mm) as i64 * (look.shake_ms - age_ms) as i64 / look.shake_ms as i64;
    let phase = (age_ms as i64 * look.shake_hz as i64 * TURN as i64 / 1000) as i32;
    let x = (left * fx::sin(phase) as i64 / ONE as i64) as i32;
    // the up wobble runs at a different rate, so the shake is not a line
    let y = (left * fx::cos(phase + phase / 2) as i64 / ONE as i64 / 2) as i32;
    (x, y)
}

impl crate::app::App {
    /// The match's camera with the flow's touches: shaken for a player
    /// whose boat was just wrecked, and on the postgame turning slowly
    /// round the boat.
    pub(crate) fn flow_camera(
        &self,
        mut cam: crate::gfx::render::Camera,
        lake: &crate::scene::Lake,
        at: crate::map::P,
    ) -> crate::gfx::render::Camera {
        let (Some(l), Some(p)) = (self.flow.look.as_ref(), self.play.as_ref()) else {
            return cam;
        };
        if let Some(pg) = self.flow.postgame.as_ref() {
            let ms = pg.ms(self.clock);
            let a = ((ms % l.postgame.orbit_ms as i64) * TURN as i64 / l.postgame.orbit_ms as i64) as i32;
            let (c, s) = (fx::cos(a), fx::sin(a));
            let turn = |d: V3| v3(mul(d.x, c) - mul(d.z, s), d.y, mul(d.x, s) + mul(d.z, c));
            let target = lake.world(at, 0);
            cam.eye = target.add(turn(cam.eye.sub(target)));
            cam.fwd = turn(cam.fwd);
        }
        if let Some(b) = self.flow.bursts.iter().rev().find(|b| b.seat == p.seat) {
            let (dx, dy) = shake(&l.burst, ((self.clock - b.born_us) / 1000) as i32);
            let right = crate::gfx::geom::right_of(cam.fwd.norm());
            cam.eye = cam.eye.add(right.scale(dx)).add(V3::UP.scale(dy));
        }
        cam
    }

    /// The flow's part of the world: each wreck's burst, and the ghosted
    /// boats drawn see-through.
    pub(crate) fn flow_world(&self, frame: &mut crate::gfx::render::Frame, ghosts: &[crate::scene::Instance]) {
        let Some(l) = self.flow.look.as_ref() else {
            frame.tris.extend(crate::scene::pose(ghosts));
            return;
        };
        for b in &self.flow.bursts {
            frame.tris.extend(tris(b, &l.burst, ((self.clock - b.born_us) / 1000) as i32));
        }
        if !ghosts.is_empty() {
            frame.set_ghost(crate::scene::pose(ghosts), fx::ratio(l.ghost_pct, 100));
        }
    }
}
