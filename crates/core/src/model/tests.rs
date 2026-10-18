use super::*;
use crate::gfx::fx::int;
use serde_json::{json, Value};

/// A one-triangle prop: three corners a metre out on x, y and z.
fn tiny() -> Value {
    json!({
        "name": "Tiny",
        "kind": "prop",
        "unit": "mm",
        "colours": { "red": [255, 0, 0] },
        "parts": [{ "name": "a", "vertices": [[1000, 0, 0], [0, 1000, 0], [0, 0, 1000]] }],
        "faces": [{ "colour": "red", "v": [0, 1, 2] }]
    })
}

/// Two parts: `arm` (one triangle at x 1 m) hangs from `base` (one at the
/// origin), its pivot at x 1 m.
fn jointed() -> Value {
    json!({
        "name": "Jointed",
        "kind": "prop",
        "unit": "mm",
        "colours": { "grey": [128, 128, 128], "hull": { "slot": "player", "shade": 50 } },
        "parts": [
            { "name": "base", "vertices": [[0, 0, 0], [0, 100, 0], [0, 0, 100]] },
            { "name": "arm", "parent": "base", "pivot": [1000, 0, 0],
              "vertices": [[2000, 0, 0], [2000, 100, 0], [2000, 0, 100]] }
        ],
        "faces": [{ "colour": "grey", "v": [0, 1, 2] }, { "colour": "hull", "v": [3, 4, 5] }],
        "anchors": { "drive": { "part": "arm", "at": [2000, 0, 0] } },
        "animations": {
            "sway": { "ms": 1000, "keys": [
                { "at": 0 },
                { "at": 500, "parts": { "arm": { "turn": [0, 90, 0] } } }
            ] },
            "bob": { "ms": 100, "keys": [
                { "at": 0, "parts": { "base": { "turn": [0, 90, 0] }, "arm": { "scale": 0 } } }
            ] }
        }
    })
}

fn load(v: &Value) -> Result<Model, String> {
    parse("m", &v.to_string())
}

fn refused(v: Value, msg: &str) {
    assert_eq!(load(&v), Err(format!("m: {msg}")));
}

fn near(a: V3, b: V3) -> bool {
    a.sub(b).len() < ONE / 500
}

#[test]
fn a_model_file_becomes_flat_triangles_in_metres() {
    let m = load(&tiny()).unwrap();
    assert_eq!((m.name.as_str(), m.kind), ("Tiny", Kind::Prop));
    let mut out = Vec::new();
    let t = m.transforms(&m.poses(None, 0));
    m.draw(&t, &Affine::IDENTITY, &m.colours([0; 3]), &mut out);
    assert_eq!(out.len(), 3 * render::VERTEX_VALUES);
    // 1000 mm is one metre
    assert_eq!(&out[0..3], &[ONE, 0, 0]);
    // the face (1,0,0) (0,1,0) (0,0,1) faces out along (1,1,1)/√3: 37837 each
    for k in 3..6 {
        assert!((out[k] - 37_837).abs() < 8, "{}", out[k]);
    }
    assert_eq!(&out[6..9], &[ONE, 0, 0]);
    assert_eq!((m.length_mm(), m.height_mm()), (1000, 1000));
}

#[test]
fn a_four_cornered_face_is_two_triangles() {
    let mut v = tiny();
    v["parts"][0]["vertices"] = json!([[0, 0, 0], [1000, 0, 0], [1000, 0, 1000], [0, 0, 1000]]);
    v["faces"][0]["v"] = json!([0, 1, 2, 3]);
    let m = load(&v).unwrap();
    let mut out = Vec::new();
    m.draw(
        &m.transforms(&m.poses(None, 0)),
        &Affine::IDENTITY,
        &m.colours([0; 3]),
        &mut out,
    );
    assert_eq!(out.len(), 6 * render::VERTEX_VALUES);
    // wound from +x toward +z about the origin: its normal is -y
    assert_eq!(&out[3..6], &[0, -ONE, 0]);
}

#[test]
fn a_file_in_another_unit_or_kind_or_with_no_name_is_refused() {
    let mut v = tiny();
    v["unit"] = json!("cm");
    refused(v, "the unit is \"cm\"; model files use \"mm\".");
    let mut v = tiny();
    v["kind"] = json!("ship");
    refused(v, "there is no kind \"ship\".");
    let mut v = tiny();
    v["name"] = json!(" ");
    refused(v, "the model has no name.");
    let mut v = tiny();
    v["colour"] = json!("x");
    assert!(load(&v).unwrap_err().contains("unknown field `colour`"));
}

#[test]
fn a_bad_colour_is_refused_by_name() {
    let mut v = tiny();
    v["colours"]["red"] = json!([256, 0, 0]);
    refused(v, "the colour \"red\" is outside 0 to 255.");
    let mut v = tiny();
    v["colours"]["red"] = json!({ "slot": "rival" });
    refused(v, "the colour \"red\" names no slot \"rival\".");
    let mut v = tiny();
    v["colours"]["red"] = json!({ "slot": "player", "shade": 201 });
    refused(v, "the colour \"red\" has a shade outside 0 to 200.");
    let mut v = tiny();
    v["colours"]["blue"] = json!([0, 0, 255]);
    refused(v, "the colour \"blue\" is on no face.");
}

#[test]
fn a_bad_face_is_refused_by_number() {
    let mut v = tiny();
    v["faces"][0]["colour"] = json!("pink");
    refused(v, "face 0 names the colour \"pink\", which is not listed.");
    let mut v = tiny();
    v["faces"][0]["v"] = json!([0, 1]);
    refused(v, "face 0 has fewer than 3 corners.");
    let mut v = tiny();
    v["faces"][0]["v"] = json!([0, 1, 3]);
    refused(v, "face 0 names vertex 3; there are 3.");
    let mut v = tiny();
    v["faces"][0]["v"] = json!([0, 1, 1]);
    refused(v, "face 0 names vertex 1 twice.");
    let mut v = tiny();
    v["parts"][0]["vertices"]
        .as_array_mut()
        .unwrap()
        .push(json!([5, 5, 5]));
    refused(v, "vertex 3 is in no face.");
    let mut v = tiny();
    v["faces"] = json!([]);
    refused(v, "the model has no faces.");
}

#[test]
fn a_bad_part_is_refused_by_name() {
    let mut v = jointed();
    v["parts"][1]["name"] = json!("base");
    refused(v, "there are two parts \"base\".");
    let mut v = jointed();
    v["parts"][1]["parent"] = json!("arm");
    refused(
        v,
        "the part \"arm\" has a parent \"arm\" that is not listed before it.",
    );
    let mut v = jointed();
    v["parts"][1]["pivot"] = json!([30_001, 0, 0]);
    refused(v, "the part \"arm\" reaches past 30000 mm.");
    let mut v = jointed();
    v["anchors"]["drive"]["part"] = json!("mast");
    refused(v, "the anchor \"drive\" names no part \"mast\".");
}

#[test]
fn a_bad_animation_is_refused_by_name() {
    let mut v = jointed();
    v["animations"]["dance"] = json!({ "ms": 10, "keys": [{ "at": 0 }] });
    refused(v, "there is no animation \"dance\".");
    let mut v = jointed();
    v["animations"]["sway"]["keys"][1]["at"] = json!(0);
    refused(
        v,
        "the animation \"sway\" has a key at 0 out of order or past its end.",
    );
    let mut v = jointed();
    v["animations"]["sway"]["keys"][1]["at"] = json!(1000);
    refused(
        v,
        "the animation \"sway\" has a key at 1000 out of order or past its end.",
    );
    let mut v = jointed();
    v["animations"]["sway"]["keys"][0]["at"] = json!(10);
    refused(v, "the animation \"sway\" must start with a key at 0.");
    let mut v = jointed();
    v["animations"]["sway"]["ms"] = json!(0);
    refused(v, "the animation \"sway\" lasts 0 ms; it must be 1 to 60000.");
    let mut v = jointed();
    v["animations"]["sway"]["keys"][1]["parts"]["mast"] = json!({});
    refused(v, "the animation \"sway\" names no part \"mast\".");
    let mut v = jointed();
    v["animations"]["sway"]["keys"][1]["parts"]["arm"]["scale"] = json!(401);
    refused(v, "the animation \"sway\" poses \"arm\" out of range.");
}

#[test]
fn a_fish_needs_its_swims_and_a_figure_its_poses() {
    let mut v = jointed();
    v["kind"] = json!("fish");
    refused(v, "a fish model needs the animation \"idle\".");
    let mut v = jointed();
    v["kind"] = json!("figure");
    refused(v, "a figure model needs the animation \"drive\".");
    let mut v = jointed();
    v["kind"] = json!("fish");
    v["animations"]["idle"] = json!({ "ms": 10, "keys": [{ "at": 0 }] });
    refused(v, "a fish model needs the animation \"panic\".");
}

#[test]
fn a_part_turns_about_its_pivot_easing_between_keys() {
    let m = load(&jointed()).unwrap();
    let tip = |ms: u32| {
        let t = m.transforms(&m.poses(Some(Anim::Sway), ms));
        t[1].apply(m.verts[3].0)
    };
    // at rest the arm's tip is 2 m out on x
    assert!(near(tip(0), v3(int(2), 0, 0)));
    // at 500 ms it has turned 90° about its pivot (x 1 m): +x turns to -z
    assert!(near(tip(500), v3(ONE, 0, -ONE)), "{:?}", tip(500));
    // halfway there it is at 45° (easing is even at the middle)
    let h = ONE * 7071 / 10_000;
    assert!(near(tip(250), v3(ONE + h, 0, -h)), "{:?}", tip(250));
    // a quarter of the way there, eased: 3/16 − 2/64 of 90° = 14.0625°
    // (cos 0.97003, sin 0.24298), not 22.5°
    let q = tip(125);
    assert!(
        near(q, v3(ONE + ONE * 9700 / 10_000, 0, -ONE * 2430 / 10_000)),
        "{q:?}"
    );
    // straight, not eased, when the animation says linear: a quarter of 90°
    let mut v = jointed();
    v["animations"]["sway"]["ease"] = json!("linear");
    let lin = load(&v).unwrap();
    let t = lin.transforms(&lin.poses(Some(Anim::Sway), 125));
    let (c, s) = (ONE * 9239 / 10_000, ONE * 3827 / 10_000);
    assert!(near(t[1].apply(lin.verts[3].0), v3(ONE + c, 0, -s)));
    v["animations"]["sway"]["ease"] = json!("bouncy");
    refused(v, "the animation \"sway\" has no ease \"bouncy\".");
    // it eases back after the last key, and loops
    assert!(near(tip(750), tip(250)));
    assert!(near(tip(1000), tip(0)));
}

#[test]
fn a_child_moves_with_its_parent_and_scale_zero_hides_it() {
    let m = load(&jointed()).unwrap();
    let t = m.transforms(&m.poses(Some(Anim::Bob), 0));
    // the base turns 90°: the arm's tip (2, 0, 0) goes to (0, 0, -2)
    // with it, but the arm is scaled to nothing at its pivot (1, 0, 0),
    // which the base's turn carries to (0, 0, -1)
    assert!(near(t[1].apply(m.verts[3].0), v3(0, 0, -ONE)));
    assert!(near(t[1].apply(m.verts[4].0), v3(0, 0, -ONE)));
    // with no such animation, everything is at rest
    let rest = m.transforms(&m.poses(Some(Anim::Panic), 0));
    assert!(near(rest[1].apply(m.verts[3].0), v3(int(2), 0, 0)));
}

#[test]
fn an_anchor_follows_its_part() {
    let m = load(&jointed()).unwrap();
    let a = m.anchor("drive").unwrap();
    let t = m.transforms(&m.poses(Some(Anim::Sway), 500));
    let f = m.anchor_frame(a, &t);
    assert!(near(f.t, v3(ONE, 0, -ONE)));
    // and turns with it: the frame's +z now points along +x
    assert!(near(f.apply_dir(v3(0, 0, ONE)), v3(ONE, 0, 0)));
}

#[test]
fn the_player_slot_takes_the_players_colour_at_its_shade() {
    let m = load(&jointed()).unwrap();
    assert!(m.has_slot());
    assert!(!load(&tiny()).unwrap().has_slot());
    let c = m.colours([ONE, ONE / 2, 0]);
    // colours are listed by name: grey, then hull
    assert_eq!(c[0], [128 * ONE / 255; 3]);
    assert_eq!(c[1], [ONE / 2, ONE / 4, 0]);
}

#[test]
fn models_come_in_gallery_order() {
    let mut fish = jointed();
    fish["kind"] = json!("fish");
    fish["animations"]["idle"] = json!({ "ms": 10, "keys": [{ "at": 0 }] });
    fish["animations"]["panic"] = json!({ "ms": 10, "keys": [{ "at": 0 }] });
    let mut long = tiny();
    long["parts"][0]["vertices"][2] = json!([0, 0, 3000]);
    let all = json!({ "zz": tiny(), "aa": long, "fish": fish });
    let ids: Vec<String> = parse_all(&all.to_string())
        .unwrap()
        .into_iter()
        .map(|m| m.id)
        .collect();
    // the fish first (by kind), then the props shortest first
    assert_eq!(ids, ["fish", "zz", "aa"]);
    assert_eq!(parse_all("{}"), Err("There are no models.".into()));
    let mut bad = tiny();
    bad["unit"] = json!("in");
    let e = parse_all(&json!({ "reed": bad }).to_string()).unwrap_err();
    assert!(e.starts_with("reed: the unit"), "{e}");
}
