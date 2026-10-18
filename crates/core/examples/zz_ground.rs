use std::fs;
use tool_core::ai::Crew;
use tool_core::map::{generate, list, Setting};
use tool_core::roster::Roster;
use tool_core::sim::{Difficulty, Match, MatchEvent, Occupant, Setup, WreckCause};
use tool_core::tuning;
fn main() {
    let read = |p: &str| fs::read_to_string(p).unwrap();
    let t = tuning::parse(&read("data/tuning.json")).unwrap();
    let roster = Roster::parse(&read("data/fish.json")).unwrap();
    let maps = list::parse(&read("data/maps/michigan-lake.json")).unwrap();
    for seed in 1..7u64 {
        let ms = list::pick(&maps, seed);
        let mut s = Setup::new(&t); s.seed = seed; s.fish = roster.0.iter().map(|f| (f.id.clone(), f.count)).collect();
        for k in 0..4 { s = s.with(k, Occupant::Ai(Difficulty::Normal), "x"); }
        let mut m = Match::new(generate(Setting::MichiganLake, ms), &t, s).unwrap();
        m.stock(&roster).unwrap();
        let mut crew = Crew::new(&m);
        let mut hist: Vec<Vec<String>> = vec![Vec::new(); 4];
        while !m.ended {
            let inp = crew.inputs(&m);
            for b in &crew.brains { let bt = m.boat(b.seat).unwrap(); let i = inp[b.seat].held; hist[b.seat].push(format!("t{} {:?} at {:?} h {} v {:?} fwd{} W{}S{}A{}D{} {:?}", m.tick, b.goal, bt.at(), bt.heading*360/65536, bt.vel(), bt.forward_speed(), i.forward as u8, i.back as u8, i.left as u8, i.right as u8, b.activity)); if hist[b.seat].len() > 12 { hist[b.seat].remove(0); } }
            let ev = m.step(&inp).to_vec();
            for e in ev { if let MatchEvent::Wrecked { seat, cause: WreckCause::Grounded } = e { println!("== seed {seed} map {ms} seat {seat} grounded"); for h in &hist[seat] { println!("  {h}"); } } }
        }
    }
}
