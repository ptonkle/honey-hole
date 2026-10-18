//! The world's tests on a small hand-drawn lake with two made-up models (a
//! swaying "tree" and a still "rock"). Expected values are worked out by
//! hand from the units and axes in `scene/mod.rs`, not read back.

use serde_json::{json, Value};

use super::*;
use crate::gfx::fx::{deg, int, HALF};
use crate::gfx::geom::facing;
use crate::map::{Setting, Spawn, Tree, TreeKind};
use crate::model;

const CELL: i32 = 4000;
const N: usize = 30;

/// A tree: a trunk triangle and a crown triangle that sways about y,
/// 4° · sin of the loop, sampled every 100 ms of a 1200 ms sway in whole
/// degrees.
fn tree() -> Model {
    let v = json!({
        "name": "Tree", "kind": "prop", "unit": "mm",
        "colours": { "bark": [100, 60, 30], "leaf": [30, 90, 40] },
        "parts": [
            { "name": "trunk", "vertices": [[0, 0, 0], [0, 3000, 0], [300, 0, 0]] },
            { "name": "crown", "parent": "trunk", "vertices": [[0, 3000, 0], [1000, 3000, 0], [0, 4000, 0]] }
        ],
        "faces": [{ "colour": "bark", "v": [0, 1, 2] }, { "colour": "leaf", "v": [3, 4, 5] }],
        "animations": { "sway": { "ms": 1200, "ease": "linear", "keys": [
            { "at": 0 },
            { "at": 100, "parts": { "crown": { "turn": [0, 2, 0] } } },
            { "at": 200, "parts": { "crown": { "turn": [0, 3, 0] } } },
            { "at": 300, "parts": { "crown": { "turn": [0, 4, 0] } } },
            { "at": 400, "parts": { "crown": { "turn": [0, 3, 0] } } },
            { "at": 500, "parts": { "crown": { "turn": [0, 2, 0] } } },
            { "at": 600 },
            { "at": 700, "parts": { "crown": { "turn": [0, -2, 0] } } },
            { "at": 800, "parts": { "crown": { "turn": [0, -3, 0] } } },
            { "at": 900, "parts": { "crown": { "turn": [0, -4, 0] } } },
            { "at": 1000, "parts": { "crown": { "turn": [0, -3, 0] } } },
            { "at": 1100, "parts": { "crown": { "turn": [0, -2, 0] } } }
        ] } }
    });
    model::parse("tree", &v.to_string()).unwrap()
}

fn rock() -> Model {
    let v = json!({
        "name": "Rock", "kind": "prop", "unit": "mm",
        "colours": { "grey": [120, 120, 120] },
        "parts": [{ "name": "a", "vertices": [[-500, 0, 0], [500, 0, 0], [0, 600, 0]] }],
        "faces": [{ "colour": "grey", "v": [0, 1, 2] }]
    });
    model::parse("rock", &v.to_string()).unwrap()
}

/// The game's lake look with every prop the tree or the rock, and no
/// scattered extras unless a test asks.
fn look_json() -> Value {
    let file: Value = serde_json::from_str(include_str!("../../../../data/look.json")).unwrap();
    let mut lake = file["lake"].clone();
    for k in ["pine", "birch", "maple"] {
        lake["props"][k] = json!(["tree"]);
    }
    for k in ["weeds", "bed_rocks"] {
        lake["props"][k] = json!(["rock"]);
    }
    for k in [
        "reed_stems",
        "lily_stems",
        "bed_log",
        "reeds",
        "lily_pads",
        "shore_rocks",
        "boulder",
        "log",
        "dock",
        "cabin",
    ] {
        lake["props"][k] = json!("rock");
    }
    for k in [
        "shore_rocks_permille",
        "boulder_permille",
        "log_permille",
        "lily_pct",
        "weeds_pct",
        "weed_permille",
        "bed_rock_permille",
        "bed_log_permille",
    ] {
        lake["props"][k] = json!(0);
    }
    lake["props"]["cabins"] = json!(0);
    lake["props"]["frames"] = json!(12);
    lake
}

fn look() -> LakeLook {
    serde_json::from_value(look_json()).unwrap()
}

/// A 30-cell map (120 m) with a square of water 14 cells across in the
/// middle, the spawn at its middle facing east, and three trees on land.
fn map() -> Map {
    let water: Vec<bool> = (0..N * N)
        .map(|i| {
            let (x, y) = (i % N, i / N);
            (8..22).contains(&x) && (8..22).contains(&y)
        })
        .collect();
    let mid = P::new(15 * CELL, 15 * CELL);
    let spawn = Spawn {
        at: mid,
        heading: 0,
        dock: P::new(8 * CELL, 15 * CELL),
    };
    let mut m = Map::from_water(Setting::MichiganLake, 7, N, CELL, &water, spawn);
    m.trees = vec![
        Tree {
            at: P::new(3 * CELL, 3 * CELL),
            kind: TreeKind::Pine,
            scale: 1000,
        },
        Tree {
            at: P::new(26 * CELL, 4 * CELL),
            kind: TreeKind::Birch,
            scale: 1000,
        },
        Tree {
            at: P::new(4 * CELL, 26 * CELL),
            kind: TreeKind::Maple,
            scale: 2000,
        },
    ];
    m
}

fn lake() -> Lake {
    Lake::build(&map(), &[tree(), rock()], &look()).unwrap()
}

fn near(a: V3, b: V3) -> bool {
    a.sub(b).len() < ONE / 100
}

fn cam_at(eye: V3, fwd: V3) -> Camera {
    Camera {
        eye,
        fwd,
        fov: deg(60),
        near: ONE / 4,
        far: int(1000),
    }
}

#[test]
fn a_map_point_lands_where_the_axes_say() {
    let l = lake();
    // the map is 120 m a side: its middle is the world's origin
    assert_eq!(l.world(P::new(60_000, 60_000), 0), V3::ZERO);
    // 10 m east is +x, 10 m north is -z, 2 m up is +y
    assert_eq!(l.world(P::new(70_000, 60_000), 0), v3(int(10), 0, 0));
    assert_eq!(l.world(P::new(60_000, 70_000), 2000), v3(0, int(2), -int(10)));
    assert_eq!(l.map_point(v3(int(10), 0, -int(10))), P::new(70_000, 70_000));
    // a model's +z faces the heading: east is +x, north is -z
    let z = v3(0, 0, ONE);
    assert!(near(
        l.place(P::new(0, 0), 0, 0, 1000).apply_dir(z),
        v3(ONE, 0, 0)
    ));
    assert!(near(
        l.place(P::new(0, 0), 0, QUARTER, 1000).apply_dir(z),
        v3(0, 0, -ONE)
    ));
    // and the yaw for it turns back into the heading
    assert_eq!(heading_of_yaw(yaw_of_heading(QUARTER)), QUARTER);
    // twice the size at 2000 thousandths
    let up = l.place(P::new(0, 0), 0, 0, 2000).apply_dir(v3(0, ONE, 0));
    assert!(near(up, v3(0, 2 * ONE, 0)));
}

#[test]
fn the_ground_under_a_point_is_on_the_drawn_triangles() {
    let mut l = lake();
    // cell (0, 0) rises along its south-west to north-east diagonal
    // (0 + 0 is even): its corners 0 (sw), 1000 (se), 0 (nw), 1000 (ne)
    let w = N + 1;
    l.heights[0] = 0;
    l.heights[1] = 1000;
    l.heights[w] = 0;
    l.heights[w + 1] = 1000;
    assert_eq!(l.ground_mm(P::new(0, 0)), 0);
    assert_eq!(l.ground_mm(P::new(CELL - 1, 0)), 999);
    // halfway east, a quarter north: the south-east triangle, 500
    assert_eq!(l.ground_mm(P::new(2000, 1000)), 500);
    // cell (1, 0) has the other diagonal: corners 1000, 0 (se), 1000, 0
    l.heights[2] = 0;
    l.heights[w + 2] = 0;
    // a quarter in each way: the south-west triangle, from 1000 at its
    // corner toward 0 east: 750
    assert_eq!(l.ground_mm(P::new(CELL + 1000, 1000)), 750);
    // off the map reads the edge
    assert_eq!(l.ground_mm(P::new(-5000, -5000)), 0);
}

#[test]
fn the_terrain_and_water_fill_the_grid_and_the_water_only_where_it_is_low() {
    let l = lake();
    let edge = 4 * N as i32 * 2 * 3;
    // two triangles a cell, and the skirt's two per edge segment
    assert_eq!(l.terrain, [0, (N * N * 2 * 3) as i32 + edge]);
    assert_eq!(l.water[0], l.terrain[1]);
    let overlap = l.look.water.shore_overlap_mm;
    let mut low = 0;
    for j in 0..N {
        for i in 0..N {
            let h = |a: usize, b: usize| l.heights[b * (N + 1) + a];
            let tris = if (i + j) % 2 == 0 {
                [
                    [h(i, j), h(i + 1, j), h(i + 1, j + 1)],
                    [h(i, j), h(i + 1, j + 1), h(i, j + 1)],
                ]
            } else {
                [
                    [h(i, j), h(i + 1, j), h(i, j + 1)],
                    [h(i + 1, j), h(i + 1, j + 1), h(i, j + 1)],
                ]
            };
            low += tris.iter().filter(|t| t.iter().any(|&v| v < overlap)).count() as i32;
        }
    }
    assert!(low > 0);
    assert_eq!(l.water[1], low * 3);
    // the water's triangles are flat at the surface and face up
    let v = &l.vertices[l.water[0] as usize * render::VERTEX_VALUES..];
    for k in 0..l.water[1] as usize {
        let at = k * render::VERTEX_VALUES;
        assert_eq!((v[at + 1], v[at + 4]), (0, ONE), "vertex {k}");
    }
}

#[test]
fn each_tree_kind_takes_its_model_and_every_prop_stands_on_the_ground() {
    let l = lake();
    // the dock (rock) and the three trees
    assert_eq!(l.props.len(), 4);
    let names: Vec<&str> = l.props.iter().map(|p| l.meshes[p.mesh].model.as_str()).collect();
    assert_eq!(names, ["rock", "tree", "tree", "tree"]);
    for (p, t) in l.props[1..].iter().zip(&l.map.trees) {
        assert_eq!(p.at, l.world(t.at, l.ground_mm(t.at)));
    }
    // the maple is twice the size
    assert!(near(l.props[3].place.apply_dir(v3(0, ONE, 0)), v3(0, 2 * ONE, 0)));
}

#[test]
fn a_swaying_model_is_baked_as_its_pose_and_its_motion_and_a_still_one_once() {
    let l = lake();
    let t = l.meshes.iter().find(|m| m.model == "tree").unwrap();
    // two triangles of three vertices, in three blocks, over 1200 ms
    assert_eq!((t.verts, t.moving, t.ms), (6, true, 1200));
    let r = l.meshes.iter().find(|m| m.model == "rock").unwrap();
    assert_eq!((r.verts, r.moving), (3, false));
    // the crown's corner at x 1 m swings about y by 4° · sin(loop):
    // a quarter of the way round (cos 0, sin 1) it has turned 4°, so it
    // sits at x cos 4°, z −sin 4° (the sway as one harmonic, within 1 cm)
    let at = |block: i32| {
        let i = ((t.first + block * t.verts + 4) as usize) * render::VERTEX_VALUES;
        v3(l.vertices[i], l.vertices[i + 1], l.vertices[i + 2])
    };
    let quarter = at(0).add(at(2));
    let four = deg(4);
    assert!(
        near(quarter, v3(fx::cos(four), int(3), -fx::sin(four))),
        "{quarter:?}"
    );
    // and half way round, back at rest (cos −1, sin 0)
    let half = at(0).sub(at(1));
    assert!(near(half, v3(ONE, int(3), 0)), "{half:?}");
    // the trunk does not sway: no motion in its blocks
    assert_eq!(at(1).y, 0);
}

#[test]
fn copies_of_one_model_sway_from_their_own_phases() {
    let l = lake();
    let trees: Vec<&Prop> = l
        .props
        .iter()
        .filter(|p| l.meshes[p.mesh].model == "tree")
        .collect();
    let phases: Vec<u32> = trees.iter().map(|p| p.phase_ms).collect();
    assert!(phases.iter().all(|&p| p < 1200));
    assert!(phases[0] != phases[1] || phases[1] != phases[2], "{phases:?}");
    // no fog, from high above: all four drawn; each tree's place in its
    // loop is the clock plus its phase, as a cosine and a sine
    let cam = cam_at(v3(0, int(300), 0), v3(0, -ONE, 0));
    let clock = 5000;
    let f = frame(&l, cam, ONE, None, clock, &[]);
    assert_eq!(f.instances.len(), 4 * render::INSTANCE_VALUES);
    assert_eq!(f.groups.len(), 2 * render::GROUP_VALUES, "one draw per model");
    let mut got: Vec<(i32, i32)> = Vec::new();
    for g in f.groups.chunks(render::GROUP_VALUES) {
        if g[3] != 6 {
            continue;
        }
        let first = g[4] as usize / render::BYTES_PER_VALUE;
        for k in 0..g[5] as usize {
            let at = first + k * render::INSTANCE_VALUES + 12;
            got.push((f.instances[at], f.instances[at + 1]));
        }
    }
    let mut want: Vec<(i32, i32)> = phases
        .iter()
        .map(|&p| {
            let a = ((clock + p as u64) % 1200 * fx::TURN as u64 / 1200) as i32;
            (fx::cos(a), fx::sin(a))
        })
        .collect();
    want.sort();
    got.sort();
    assert_eq!(got, want);
}

#[test]
fn groups_point_inside_the_static_buffer_at_a_pose_and_its_motion() {
    let l = lake();
    let cam = cam_at(v3(0, int(300), 0), v3(0, -ONE, 0));
    let f = frame(&l, cam, ONE, None, 777, &[]);
    let bytes = l.vertices.len() * render::BYTES_PER_VALUE;
    let mut drawn = 0;
    for g in f.groups.chunks(render::GROUP_VALUES) {
        let m = l.meshes.iter().find(|m| m.verts == g[3]).unwrap();
        let block = m.verts * render::STRIDE as i32;
        let start = m.first * render::STRIDE as i32;
        assert_eq!(g[0], start);
        if m.moving {
            // the cosine's block, then the sine's, after the pose
            assert_eq!((g[1], g[2]), (start + block, start + 2 * block));
            assert!((start + 3 * block) as usize <= bytes);
        } else {
            // a still model's motion is its pose times 0
            assert_eq!((g[1], g[2]), (start, start));
        }
        drawn += g[5];
    }
    assert_eq!(drawn as usize, l.props.len());
    assert_eq!(f.statics, [l.terrain[0], l.terrain[1], l.water[0], l.water[1]]);
}

#[test]
fn a_model_past_the_far_plane_or_behind_the_camera_is_not_posed() {
    let mut l = lake();
    l.look.fog_end_pct = 100;
    let r = rock();
    // the camera at the origin looking east; fog at 20 m puts the far plane
    // at 22.5 m. The rock's rest bounds are 1000 x 600 mm round (0, 300, 0)
    // mm: a ball of 583 mm, doubled for a swing to 1166 mm.
    let cam = cam_at(V3::ZERO, v3(ONE, 0, 0));
    let posed = |x_m: i32| {
        let rock = Instance {
            model: &r,
            place: Affine::translate(v3(int(x_m), 0, 0)),
            anim: None,
            ms: 0,
            colour: [0; 3],
        };
        !frame(&l, cam, ONE, Some(20_000), 0, &[rock]).tris.is_empty()
    };
    // 23 m off: its ball reaches 21.8 m, inside the far plane
    assert!(posed(23), "a rock 23 m ahead is posed");
    // 24 m off: its nearest is 22.8 m, past it
    assert!(!posed(24), "a rock 24 m ahead is not");
    // 1 m behind the eye its ball still reaches in front; 2 m behind it does not
    assert!(posed(-1), "a rock 1 m behind is posed");
    assert!(!posed(-2), "a rock 2 m behind is not");
}

#[test]
fn props_past_the_fog_or_behind_the_camera_are_left_out() {
    let mut l = lake();
    l.look.fog_end_pct = 100;
    // the camera at the world's middle looking east; the fog whole at 50 m
    let cam = cam_at(v3(0, int(2), 0), v3(ONE, 0, 0));
    let count = |l: &Lake, sight: Option<i32>| {
        frame(l, cam, ONE, sight, 0, &[]).instances.len() / render::INSTANCE_VALUES
    };
    // from the middle (60 m, 60 m) the dock lies about 27 m west and two
    // trees 62 m to the west: behind the eye. One tree, 62 m south-east
    // (44 m east), is ahead.
    assert_eq!(count(&l, None), 1, "no fog: only the tree ahead");
    assert_eq!(count(&l, Some(70_000)), 1, "fog at 70 m: the tree at 62 m shows");
    assert_eq!(count(&l, Some(20_000)), 0, "fog at 20 m: it does not");
    // fog draws the far plane just past it; no fog, the look's far plane
    let f = frame(&l, cam, ONE, Some(20_000), 0, &[]);
    let near_far = |v: &[i32]| v[..16].to_vec();
    let mut far = cam;
    far.far = mm(20_000) + mm(20_000) / 8;
    assert_eq!(near_far(&f.view), far.matrix(ONE).0.to_vec());
}

#[test]
fn the_fog_is_the_sight_distance_by_the_looks_shares_and_none_is_far_away() {
    let mut l = lake();
    l.look.fog_start_pct = 25;
    l.look.fog_end_pct = 100;
    assert_eq!(l.fog(Some(120_000)), (mm(30_000), mm(120_000)));
    let (start, _) = l.fog(None);
    assert!(
        start > mm(l.look.camera.far_mm),
        "no fog starts past the far plane"
    );
    assert_eq!(l.sight_mm, 120_000, "the map's own sight distance");
}

#[test]
fn the_waves_travel_with_the_clock() {
    let l = lake();
    let w = &l.look.water.waves[0];
    let a = l.air(0).waves[0];
    let b = l.air(w.period_ms as u64 / 4).waves[0];
    // a quarter period on, the phase is a quarter turn less (−π/2)
    let quarter = (fx::TWO_PI / 4) as i32;
    assert!((a[3] - b[3] - quarter).abs() < 8, "{a:?} {b:?}");
    // a whole period on, back where it was
    assert_eq!(l.air(w.period_ms as u64).waves[0], a);
    // k is 2π over the length, along the wave's heading
    let k = (fx::TWO_PI * 1000 / w.length_mm as i64) as i32;
    assert!((v3(a[0], 0, a[1]).len() - k).abs() < 16);
    assert_eq!(a[2], mm(w.height_mm));
}

#[test]
fn the_boat_camera_stands_behind_and_above_and_keeps_clear_of_the_ground() {
    let l = lake();
    let c = &l.look.camera;
    let target = v3(0, 0, 0);
    // the boat faces +x (yaw a quarter turn): the eye is behind it, on -x,
    // back_mm from the pivot at pitch_deg down
    let cam = camera::boat(&l, c, target, QUARTER, 0, 0);
    let down = deg(c.pitch_deg);
    let want_x = -fx::mul(mm(c.back_mm), fx::cos(down));
    let want_y = mm(c.pivot_mm) + fx::mul(mm(c.back_mm), fx::sin(down));
    assert!((cam.eye.x - want_x).abs() < ONE / 50, "{:?}", cam.eye);
    assert!((cam.eye.y - want_y).abs() < ONE / 50, "{:?}", cam.eye);
    assert!(cam.eye.z.abs() < ONE / 50);
    // it looks at the aim point ahead of the boat and low over the water
    let aim = v3(mm(c.ahead_mm), mm(c.aim_mm), 0);
    assert!(near(cam.fwd, aim.sub(cam.eye).norm()));
    // so it looks down, but less than it would at the boat itself
    assert!(cam.fwd.y < 0 && cam.fwd.y > target.sub(cam.eye).norm().y);
    // the mouse turns it half round: now in front, on +x
    let round = camera::boat(&l, c, target, QUARTER, HALF, 0);
    assert!(round.eye.x > mm(c.back_mm) / 2);
    // pitched far down into a hill, the eye rises to the clearance
    let mut hill = l.clone();
    hill.heights.iter_mut().for_each(|h| *h = 30_000);
    let low = camera::boat(&hill, c, target, QUARTER, 0, -deg(60));
    assert_eq!(low.eye.y, mm(30_000 + c.clear_mm));
}

#[test]
fn land_behind_the_boat_pulls_the_camera_in_and_it_stays_over_the_land() {
    let mut l = lake();
    let c = l.look.camera.clone();
    // a 6 m bank west of the boat: every corner from 4 m behind it (the map's
    // middle is 60 m, so corners at x 56 m and less)
    let n = l.map.n;
    for j in 0..=n {
        for i in 0..=n {
            if (i as i32) * CELL <= 56_000 {
                l.heights[j * (n + 1) + i] = 6000;
            }
        }
    }
    let cam = camera::boat(&l, &c, v3(0, 0, 0), QUARTER, 0, 0);
    // pulled in from about 6.9 m to no nearer than min_back_mm along the
    // orbit (its flat share is at least 0.9 of it at an 11° pitch)
    let back = -cam.eye.x;
    assert!(back < mm(4000), "still behind the bank: {:?}", cam.eye);
    assert!(back >= mm(c.min_back_mm) * 9 / 10, "{:?}", cam.eye);
    // and never inside the land under it
    let ground = l.ground_mm(l.map_point(cam.eye));
    assert!(
        cam.eye.y >= mm(ground + c.clear_mm) - 2,
        "{:?} over {ground}",
        cam.eye
    );
    // open water behind: no pull
    let open = camera::boat(&lake(), &c, v3(0, 0, 0), QUARTER, 0, 0);
    assert!(-open.eye.x > mm(6000));
}

#[test]
fn the_fly_over_camera_looks_at_its_centre_from_its_distance() {
    let c = &look().camera;
    let cam = camera::fly_over(c, V3::ZERO, 0, deg(90), int(100));
    // straight down from 100 m
    assert!(near(cam.eye, v3(0, int(100), 0)));
    assert!(near(cam.fwd, facing(0, -deg(90))));
}

#[test]
fn a_look_naming_a_model_that_is_not_loaded_is_refused_by_name() {
    let mut v = look_json();
    v["props"]["cabin"] = json!("shed");
    let look: LakeLook = serde_json::from_value(v).unwrap();
    assert_eq!(
        Lake::build(&map(), &[tree(), rock()], &look).err(),
        Some("The lake's look names the model \"shed\", which is not loaded.".into())
    );
    let mut v = look_json();
    v["fog_start_pct"] = json!(100);
    let look: LakeLook = serde_json::from_value(v).unwrap();
    assert_eq!(
        look.check(),
        Err("The lake's fog must start before it ends.".into())
    );
    let mut v = look_json();
    v["water"]["waves"] = json!([]);
    let look: LakeLook = serde_json::from_value(v).unwrap();
    assert_eq!(
        look.check(),
        Err("The lake's water needs exactly two waves.".into())
    );
}

#[test]
fn instances_are_posed_into_the_frames_triangles_in_their_colour() {
    let l = lake();
    let r = rock();
    let place = l.place(P::new(60_000, 60_000), -500, 0, 1000);
    let i = Instance {
        model: &r,
        place,
        anim: None,
        ms: 0,
        colour: [0; 3],
    };
    let cam = cam_at(v3(0, int(10), int(10)), v3(0, -ONE, -ONE));
    let f = frame(&l, cam, ONE, Some(120_000), 0, &[i]);
    assert_eq!(f.tris.len(), 3 * render::VERTEX_VALUES);
    // the rock's top corner, 0.6 m over its origin half a metre down
    let top = &f.tris[2 * render::VERTEX_VALUES..2 * render::VERTEX_VALUES + 3];
    assert!(near(v3(top[0], top[1], top[2]), v3(0, mm(100), 0)));
    // a seat on a model with no such anchor is none
    assert!(seat(&i, "drive").is_none());
}

#[test]
fn the_stamp_changes_with_the_map() {
    let a = lake();
    let mut m = map();
    m.seed = 8;
    let b = Lake::build(&m, &[tree(), rock()], &look()).unwrap();
    assert_ne!(a.stamp(), b.stamp());
    assert_ne!(a.stamp(), 0);
}
