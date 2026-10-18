// The page's per-frame core calls, natively, over a watched 4-AI match (Hard).
// Copy into crates/core/examples/ and run from the repository root after
// scripts/build.sh: cargo run --release --example frameprof -- 150
use std::fs;
use std::time::Instant;
use tool_core::api::Session;

fn main() {
    let read = |p: &str| fs::read_to_string(format!("dist/data/{p}")).unwrap();
    if std::env::args().nth(1).as_deref() == Some("sizes") {
        let models = tool_core::model::parse_all(&read("models.json")).unwrap();
        for m in &models {
            let t = m.transforms(&m.poses(None, 0));
            let mut out = Vec::new();
            m.draw(&t, &tool_core::gfx::geom::Affine::IDENTITY, &m.colours([0; 3]), &mut out);
            println!("{:20} {:?} tris {}", m.id, m.kind, out.len() / 30);
        }
        return;
    }
    let secs: i64 = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(120);
    let mut s = Session::new(&read("strings.json"), &read("patch-notes.json"));
    s.load_models(&read("models.json"), &read("look.json"));
    s.act("data.ui2d", &read("ui2d.json"));
    s.act("data.fish", &read("fish.json"));
    s.act("data.flow", &read("flow.json"));
    for (n, f) in [
        ("data.tuning", "tuning.json"),
        ("data.controls", "controls.json"),
        ("data.names", "names.json"),
        ("data.maps", "maps/michigan-lake.json"),
        ("data.fish", "fish.json"),
    ] {
        let r = s.act(n, &read(f));
        assert!(!r.contains("\"error\""), "{n}: {r}");
    }
    s.load_sounds(&read("sounds.json"), &read("tuning.json"));
    s.act("debug.flags", "watch");
    s.resize(1920, 1080, 100);
    s.act("menu.play", "");
    s.act("menu.offline", "");
    s.act("lobby.ai", "3");
    for k in 1..=3 {
        s.act(&format!("lobby.ai.{k}"), "hard");
    }
    let r = s.act("lobby.start", "");
    assert!(!r.contains("\"error\""), "{r}");
    let mut now: i64 = 1_000_000;
    let names = ["advance", "frame", "inset", "ui2d", "ui", "sound"];
    let mut tot = [0u128; 6];
    let mut worst = [0u128; 6];
    let mut frames = 0u64;
    let mut big = (0usize, 0usize, 0usize);
    let mut sum_tris = 0usize;
    let mut worst_frame = (0u128, 0i64);
    while now < secs * 1_000_000 {
        now += 16_667;
        let mut t = [0u128; 6];
        let a = Instant::now();
        s.advance(now);
        t[0] = a.elapsed().as_nanos();
        let a = Instant::now();
        let f = s.frame();
        t[1] = a.elapsed().as_nanos();
        let a = Instant::now();
        let i = s.inset_frame();
        t[2] = a.elapsed().as_nanos();
        let a = Instant::now();
        let u = s.build_ui2d();
        t[3] = a.elapsed().as_nanos();
        let a = Instant::now();
        let ui = s.ui();
        t[4] = a.elapsed().as_nanos();
        let a = Instant::now();
        let snd = s.sound();
        t[5] = a.elapsed().as_nanos();
        if frames % 600 == 0 {
            let n = &u.list.nums;
            let mut i = 0;
            let mut hist = std::collections::BTreeMap::new();
            let q = |x: i32| x >> 16;
            while i < n.len() {
                let code = q(n[i]);
                i += 1;
                let args = match code {
                    0 => 4,
                    1 => 5,
                    2 => 6,
                    3 => 8,
                    4 => 6,
                    5 => 6,
                    6 => 7,
                    7 => 4 + 2 * q(n[i + 3]) as usize,
                    8 => 6,
                    9 => 6,
                    10 => 4,
                    11 => 0,
                    12 => 5,
                    13 => 4,
                    14 => 1 + q(n[i]) as usize,
                    15 => 1,
                    _ => panic!("op {code}"),
                };
                *hist.entry(code).or_insert(0) += 1;
                i += args;
            }
            let status: String = u.list.status.split(' ').filter(|w| w.starts_with("fishing=") || w.starts_with("fishboy=")).collect::<Vec<_>>().join(" ");
            println!("t {} s {status}: ops {:?} strings {}", now / 1_000_000, hist, u.list.strings.len());
        }
        let all: u128 = t.iter().sum();
        if all > worst_frame.0 {
            worst_frame = (all, now);
        }
        for k in 0..6 {
            tot[k] += t[k];
            worst[k] = worst[k].max(t[k]);
        }
        sum_tris += f.tris.len() + i.2.tris.len();
        big.0 = big.0.max(f.tris.len());
        big.1 = big.1.max(i.2.tris.len());
        big.2 = big.2.max(ui.len() + snd.len() + u.list.strings_json().len());
        frames += 1;
    }
    for k in 0..6 {
        println!(
            "{:8} mean {:7.1} us  worst {:7.1} us",
            names[k],
            tot[k] as f64 / frames as f64 / 1e3,
            worst[k] as f64 / 1e3
        );
    }
    println!(
        "frames {frames}; main tris values max {} (= {} KB f32), inset {}; mean both {} KB; text max {}; worst frame {} us at {} s",
        big.0,
        big.0 * 4 / 1024,
        big.1,
        sum_tris / frames as usize * 4 / 1024,
        big.2,
        worst_frame.0 / 1000,
        worst_frame.1 / 1_000_000
    );
}
