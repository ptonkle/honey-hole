//! The 2D draw list: its encoding, its layers and its holes. Expected
//! values are worked out by hand.

use super::*;
use serde_json::Value;

fn code(name: &str) -> i32 {
    let n: Value = serde_json::from_str(&numbers_json()).unwrap();
    n["ops"][name].as_i64().unwrap() as i32
}

#[test]
fn an_op_is_its_code_then_its_arguments_and_strings_are_shared() {
    let mut l = List::new();
    l.fill_rect(Rgba::solid([255, 0, 0]), Rect::new(1, 2, 3, 4));
    l.fill_rect(Rgba::solid([255, 0, 0]), Rect::new(5, 6, 7, 8));
    // code, colour index 0, then four numbers, all Q16
    assert_eq!(
        l.nums,
        [code("fill_rect"), 0, 1, 2, 3, 4, code("fill_rect"), 0, 5, 6, 7, 8].map(|v| v * ONE)
    );
    assert_eq!(l.strings, vec!["rgba(255,0,0,1.00)".to_string()]);
}

#[test]
fn the_numbers_name_every_op_once_and_a_full_turn() {
    let n: Value = serde_json::from_str(&numbers_json()).unwrap();
    let ops = n["ops"].as_object().unwrap();
    assert_eq!(ops.len(), Op::ALL.len());
    let mut codes: Vec<i64> = ops.values().map(|v| v.as_i64().unwrap()).collect();
    codes.sort();
    assert_eq!(codes, (0..Op::ALL.len() as i64).collect::<Vec<_>>());
    // 2π to six places
    let turn = n["full_turn"].to_string();
    assert!(turn.starts_with("6.2831"), "{turn}");
    // a pass's view values follow its eight leading values
    assert_eq!(n["pass"]["view"][0], 8);
}

#[test]
fn colours_are_css_and_a_clear_colour_draws_nothing() {
    assert_eq!(Rgba([10, 20, 30], 45).css(), "rgba(10,20,30,0.45)");
    assert_eq!(Rgba([300, -4, 30], 100).css(), "rgba(255,0,30,1.00)");
    assert_eq!(Rgba::NONE.css(), "");
    assert_eq!(Rgba::solid([100, 50, 0]).shade(50), Rgba::solid([50, 25, 0]));
}

#[test]
fn a_letterboxed_layer_keeps_its_shape_and_sits_in_the_middle() {
    // 1920 x 1080 into 1000 x 1000: the width binds, at 1000/1920; the
    // canvas is 562 high, so it starts (1000 - 562) / 2 = 219 down
    let t = Transform::letterbox([1920, 1080], Rect::new(0, 0, 1000, 1000));
    assert_eq!(t.sx, t.sy);
    assert_eq!(t.tx, 0);
    assert!((t.ty - 219).abs() <= 1, "{}", t.ty);
    assert_eq!(t.apply(1920, 0).0, 1000);
}

#[test]
fn a_point_on_a_layer_comes_back_from_its_pixel() {
    let t = Transform::onto([1600, 900], Rect::new(160, 90, 1280, 720));
    // 1280/1600 = 0.8: unit (800, 450) is pixel (160 + 640, 90 + 360)
    assert_eq!(t.apply(800, 450), (800, 450));
    assert_eq!(t.apply(0, 0), (160, 90));
    assert_eq!(t.invert(800, 450), (800, 450));
    assert_eq!(t.invert(160 + 8, 90 + 8), (10, 10));
}

#[test]
fn a_model_cuts_a_hole_and_keeps_its_place_in_pixels() {
    let mut l = List::new();
    l.layer(Transform {
        sx: 2 * ONE,
        sy: 2 * ONE,
        tx: 10,
        ty: 20,
    });
    l.model(
        Rect::new(5, 5, 50, 25),
        "perch",
        Some(Anim::Idle),
        300,
        [1, 2, 3],
        100,
    );
    // after the layer's five values: the hole and its rectangle in units
    assert_eq!(&l.nums[5..10], &[code("hole"), 5, 5, 50, 25].map(|v| v * ONE));
    let s = &l.shots[0];
    assert_eq!(s.px, Rect::new(20, 30, 100, 50));
    assert_eq!((s.model.as_str(), s.ms, s.background), ("perch", 300, [1, 2, 3]));
}

#[test]
fn a_pass_turns_its_rectangle_up_the_buffer_as_gl_counts_rows() {
    // a box 30 down and 50 high in a buffer 200 high starts 120 up
    assert_eq!(Pass::gl(Rect::new(10, 30, 40, 50), [300, 200]), [10, 120, 40, 50]);
    let p = Pass {
        gl_rect: [1, 2, 3, 4],
        clear: Some([5, 6, 7]),
        view: vec![9],
        tris: Vec::new(),
    };
    assert_eq!(p.values(), vec![ONE, 2 * ONE, 3 * ONE, 4 * ONE, ONE, 5, 6, 7, 9]);
}

#[test]
fn a_bitmap_is_four_bytes_a_pixel_from_its_palette() {
    let b = Bitmap {
        id: "b".into(),
        w: 2,
        h: 1,
        pixels: vec![1, 0],
        palette: vec![Rgba([1, 2, 3], 100), Rgba([4, 5, 6], 0)],
    };
    assert_eq!(b.rgba(), vec![4, 5, 6, 0, 1, 2, 3, 255]);
}

#[test]
fn text_names_its_colour_font_alignment_and_words() {
    let font = Font {
        family: "mono".into(),
        weight: "bold".into(),
        advance: 60,
    };
    let mut l = List::new();
    l.text(Rgba::solid([0, 0, 0]), &font, 20, Align::Right, (7, 8), "Hi");
    let at = |k: usize| l.strings[(l.nums[k] / ONE) as usize].as_str();
    assert_eq!(at(1), "rgba(0,0,0,1.00)");
    assert_eq!(at(2), "bold 20px mono");
    assert_eq!(at(3), "right");
    assert_eq!(at(6), "Hi");
    // two glyphs at 60% of 20
    assert_eq!(font.width("Hi", 20), 24);
}
