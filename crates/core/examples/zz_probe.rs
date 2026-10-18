use std::fs;
use tool_core::ai::{fish, Brain};
use tool_core::map::fish::Tier;
use tool_core::map::rng::Rng;
use tool_core::map::{generate, Setting};
use tool_core::sim::fishing::reel::{Outcome, Reel, BAR, FULL};
use tool_core::sim::fishing::view::ReelView;
use tool_core::sim::{Difficulty, Match, Occupant, Setup};
use tool_core::tuning;
fn main() {
    let mut v: serde_json::Value = serde_json::from_str(&fs::read_to_string("data/tuning.json").unwrap()).unwrap();
    // overrides: KEY=tier:value,... for fish_speed_per_s, fish_turn_ms, fish_dart_pct, drain_ms
    for key in ["fish_speed_per_s", "fish_turn_ms", "fish_dart_pct", "drain_ms"] {
        if let Ok(o) = std::env::var(key) { for kv in o.split(',') { let (t, n) = kv.split_once(':').unwrap(); v["fishing"]["reel"][key][t] = n.parse::<i64>().unwrap().into(); } }
    }
    let t = tuning::parse(&v.to_string()).unwrap();
    let mut s = Setup::new(&t);
    for k in 0..3 { s = s.with(k, Occupant::Ai(Difficulty::ALL[k]), "x"); }
    let m = Match::new(generate(Setting::MichiganLake, 0), &t, s).unwrap();
    let rt = &t.fishing.reel;
    for tier in [Tier::Common, Tier::Uncommon, Tier::Rare, Tier::Legendary] {
        let mut line = format!("{tier:?}:");
        for d in 0..3 {
            let mut rng = Rng::new(5);
            let (mut ok, mut ms, n) = (0, 0u64, 200);
            for k in 0..n {
                let mut b = Brain::new(d, Difficulty::ALL[d], &m);
                let mut r = Reel::new(tier, rt, false);
                let mut tick = 1000u32 + k;
                loop {
                    let v = ReelView { zone: r.zone / (BAR / 1000), zone_h: r.zone_h / (BAR/1000), fish: r.marker / (BAR/1000), meter: r.meter/(FULL/1000), in_zone: r.in_zone(), tension: 0, tier };
                    let hold = fish::reel_hold(&mut b, &v, tick);
                    tick += 1;
                    match r.step(rt, &mut rng, tick, hold, 100) { Outcome::Fighting => {}, Outcome::Landed => { ok += 1; ms += r.ticks as u64 * 20; break }, Outcome::Broke => break }
                }
            }
            line += &format!("  {:?} {}% {:.1}s", Difficulty::ALL[d], ok * 100 / n, ms as f64 / ok.max(1) as f64 / 1000.0);
        }
        println!("{line}");
    }
}
