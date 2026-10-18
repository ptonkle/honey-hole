//! The map tool: scans seeds into the pregenerated list, and prints maps as
//! JSON for the preview renderer (`tools/maps/preview.py`). Every rule it
//! uses is the core's; this file only reads arguments and writes output.
//!
//!     cargo run --release --example maps -- scan [from] [count]
//!         writes data/maps/michigan-lake.json and prints why seeds failed
//!     cargo run --release --example maps -- report [from] [count]
//!         prints each seed's measurements, for tuning the validator
//!     cargo run --release --example maps -- dump seed...
//!         prints each map, its verdict, fish and sample buoy points as JSON

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use serde_json::json;
use tool_core::map::{buoy, fish, generate, list, validate, Setting, P};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let num = |k: usize, d: u32| args.get(k).and_then(|a| a.parse().ok()).unwrap_or(d);
    let setting = Setting::MichiganLake;
    match args.first().map(String::as_str) {
        Some("scan") => {
            let (from, count) = (num(1, 0), num(2, 256));
            let (maps, rejected) = list::scan(setting, from, count);
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/maps")
                .join(format!("{}.json", setting.id()));
            let text = serde_json::to_string_pretty(&maps).expect("json") + "\n";
            fs::write(&path, text).expect("write the map list");
            let mut why: BTreeMap<String, u32> = BTreeMap::new();
            for (_, problems) in &rejected {
                for p in problems {
                    let name = format!("{p:?}");
                    let name = name.split(['(', ' ', '{']).next().unwrap_or("").to_string();
                    *why.entry(name).or_default() += 1;
                }
            }
            println!(
                "{} of {} seeds accepted; written to {}",
                maps.maps.len(),
                count,
                path.display()
            );
            for (k, v) in why {
                println!("  {v:4} rejected for {k}");
            }
        }
        Some("report") => {
            let (from, count) = (num(1, 0), num(2, 40));
            let (mut making, mut judging) = (0, 0);
            for seed in from..from + count {
                let start = Instant::now();
                let map = generate(setting, seed);
                making += start.elapsed().as_micros();
                let start = Instant::now();
                let v = validate::check(&map);
                judging += start.elapsed().as_micros();
                let problems: Vec<String> = v.problems.iter().map(|p| p.describe()).collect();
                println!("{seed:4} {:?} {}", v.report, problems.join("; "));
            }
            let each = |us: u128| us / 1000 / count.max(1) as u128;
            println!(
                "{} ms to make and {} ms to judge each map",
                each(making),
                each(judging)
            );
        }
        Some("dump") => {
            let out: Vec<serde_json::Value> = args[1..]
                .iter()
                .filter_map(|a| a.parse::<u32>().ok())
                .map(|seed| dump(setting, seed))
                .collect();
            println!("{}", serde_json::to_string(&out).expect("json"));
        }
        _ => eprintln!("usage: maps scan [from] [count] | report [from] [count] | dump seed..."),
    }
}

fn dump(setting: Setting, seed: u32) -> serde_json::Value {
    let map = generate(setting, seed);
    let verdict = validate::check(&map);
    let fish = fish::place(&map, &fish::EXAMPLE_COUNTS, seed as u64);
    // sample buoys: everyone at the spawn, and four players spread out
    let s = map.spawn.at;
    let spread: Vec<P> = match &fish {
        Ok(f) => f
            .iter()
            .step_by(f.len().max(4) / 4)
            .take(4)
            .map(|f| f.at)
            .collect(),
        Err(_) => vec![s],
    };
    let buoys = vec![
        json!({"players": [s], "buoy": buoy::fair_point(&map, &[s], seed as u64)}),
        json!({"players": spread, "buoy": buoy::fair_point(&map, &spread, seed as u64)}),
    ];
    json!({
        "setting": setting.id(),
        "seed": seed,
        "n": map.n,
        "cell_mm": map.cell_mm,
        "rows": map.rows(),
        "ground": map.ground,
        "islands": map.islands,
        "reeds": map.reeds,
        "trees": map.trees,
        "spawn": map.spawn,
        "report": verdict.report,
        "problems": verdict.problems.iter().map(|p| p.describe()).collect::<Vec<_>>(),
        "fish": fish.as_ref().ok(),
        "fish_error": fish.as_ref().err(),
        "buoys": buoys,
        "stamp": list::fingerprint(&map),
    })
}
