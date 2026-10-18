//! The Lake preview's cost, measured: builds a map's world and prints its
//! static triangles, the time to build it, and each view's triangles and
//! draw calls at 1920×1080, with the fog on and off.
//!
//!     cargo run --release --example lake            seed 0
//!     cargo run --release --example lake -- 3 8     these seeds
//!
//! Run from the repository root (it reads data/). Record what it prints in
//! MEASUREMENTS.md.

use std::fs;
use std::time::Instant;

use tool_core::app::lake::View;
use tool_core::app::App;

fn models_json() -> String {
    let mut files: Vec<_> = fs::read_dir("data/models").expect("data/models").map(|e| e.unwrap().path()).collect();
    files.sort();
    let parts: Vec<String> = files
        .iter()
        .map(|p| {
            let id = p.file_stem().unwrap().to_str().unwrap();
            format!("\"{id}\":{}", fs::read_to_string(p).unwrap())
        })
        .collect();
    format!("{{{}}}", parts.join(","))
}

fn main() {
    let read = |p: &str| fs::read_to_string(p).unwrap_or_else(|e| panic!("{p}: {e}"));
    let seeds: Vec<String> = std::env::args().skip(1).collect();
    let seeds = if seeds.is_empty() { vec!["0".to_string()] } else { seeds };
    let mut app = App::new(&read("data/strings.json"), &read("data/patch-notes.json")).unwrap();
    app.load_models(&models_json(), &read("data/look.json")).unwrap();
    app.load_maps(&read("data/maps/michigan-lake.json")).unwrap();
    app.resize(1920, 1080, 100);
    for seed in seeds {
        app.act("menu.lake", "").unwrap();
        let t = Instant::now();
        app.act("lake.seed", &seed).unwrap();
        let built = t.elapsed();
        let lake = app.world.as_ref().expect("a world");
        let props: usize = lake.props.len();
        println!(
            "seed {seed}: built in {:.0?}; terrain {} triangles, water {}, {} props of {} meshes, static buffer {:.1} MB",
            built,
            lake.terrain[1] / 3,
            lake.water[1] / 3,
            props,
            lake.meshes.len(),
            lake.vertices.len() as f64 * 4.0 / 1e6
        );
        for fog in ["on", "off"] {
            app.act("lake.fog_on", fog).unwrap();
            for v in View::ALL {
                app.act("lake.view", v.key()).unwrap();
                app.advance(app.clock + 2_000_000);
                let t = Instant::now();
                let f = app.frame();
                let took = t.elapsed();
                let (tris, draws) = f.stats();
                println!(
                    "  fog {fog:3} {:6}: {tris:7} triangles, {draws:3} draw calls, {:4} prop instances, core {:.1?}",
                    v.key(),
                    f.instances.len() / 13,
                    took
                );
            }
        }
        app.act("lake.fog_on", "on").unwrap();
    }
}
