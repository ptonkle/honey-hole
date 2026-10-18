//! The validator: rejects a map that would play badly. Its numbers are the
//! setting's [`Params`]; [`check`] measures the map and lists every
//! [`Problem`] it finds.
//!
//! - All water is one lake: no pocket a boat cannot reach.
//! - Channels are wide enough for a boat: the roomy water (a channel's
//!   middle at [`Params::min_channel_mm`]) is one region, apart from nooks
//!   too small to matter.
//! - The spawn is on roomy water with water all round it, and the lane ahead
//!   of it is open.
//! - Islands: a sensible number and size, enough big enough to hide behind,
//!   and none touching the shore or another island.
//! - Enough water, and a crossing from the spawn of about the right length.
//! - Not too open to hide, nor too cramped for the fog to matter: of the
//!   pairs of open-water points within fog sight, the share whose view is
//!   cut by land lies in [`Params::cover`]; and enough of the water is open.

use serde::Serialize;

use super::grid::{self, FAR};
use super::trig;
use super::{Map, P};

/// What the validator measured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct Report {
    pub water_m2: i32,
    pub pockets: u32,
    /// Cells of roomy water cut off from the spawn's by a narrow channel.
    pub cut_off_cells: usize,
    pub islands: usize,
    pub hiding_islands: usize,
    /// The narrowest water between an island and other land, in mm.
    pub island_gap_mm: i32,
    /// The longest boat trip from the spawn, in mm.
    pub span_mm: i32,
    /// Thousandths of in-sight pairs whose view land cuts.
    pub cover: i32,
    /// Thousandths of the water that is open water.
    pub open_share: i32,
}

/// One reason a map is rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Problem {
    /// Water no boat can reach from the spawn: how many pockets.
    Pockets(u32),
    /// Roomy water reached only through a channel narrower than the minimum.
    NarrowChannel {
        area_m2: i32,
    },
    /// Water reaching the land kept round the map's edge, which cuts the
    /// shore straight.
    EdgeCut,
    SpawnNotRoomy,
    SpawnCrowded,
    LaneBlocked {
        at_mm: i32,
    },
    IslandCount(usize),
    IslandSize {
        island: u32,
        area_m2: i32,
    },
    TooFewHidingIslands(usize),
    IslandTouches {
        island: u32,
        gap_mm: i32,
    },
    WaterArea(i32),
    Span(i32),
    TooOpen(i32),
    TooCramped(i32),
    LittleOpenWater(i32),
}

impl Problem {
    /// The reason in a few words, for the previews and the scan log.
    pub fn describe(&self) -> String {
        match self {
            Problem::Pockets(k) => format!("{k} pocket(s) of water cut off from the lake"),
            Problem::NarrowChannel { area_m2 } => {
                format!("{area_m2} m² of water behind a channel too narrow for a boat")
            }
            Problem::EdgeCut => "the lake runs into the map's edge".into(),
            Problem::SpawnNotRoomy => "the spawn is not on open water".into(),
            Problem::SpawnCrowded => "land too close round the spawn".into(),
            Problem::LaneBlocked { at_mm } => format!("land {} m ahead of the spawn", at_mm / 1000),
            Problem::IslandCount(k) => format!("{k} islands"),
            Problem::IslandSize { island, area_m2 } => format!("island {island} is {area_m2} m²"),
            Problem::TooFewHidingIslands(k) => format!("only {k} island(s) big enough to hide behind"),
            Problem::IslandTouches { island, gap_mm } => {
                format!("island {island} is {} m from other land", gap_mm / 1000)
            }
            Problem::WaterArea(m2) => format!("{} thousand m² of water", m2 / 1000),
            Problem::Span(mm) => format!("the crossing from the spawn is {} m", mm / 1000),
            Problem::TooOpen(c) => format!("too open to hide: land cuts {}% of views", c / 10),
            Problem::TooCramped(c) => format!("too cramped: land cuts {}% of views", c / 10),
            Problem::LittleOpenWater(s) => format!("only {}% of the water is open", s / 10),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Verdict {
    pub report: Report,
    pub problems: Vec<Problem>,
}

impl Verdict {
    pub fn ok(&self) -> bool {
        self.problems.is_empty()
    }
}

/// Measures a map against its setting's rules.
pub fn check(map: &Map) -> Verdict {
    let p = map.setting.params();
    let n = map.n;
    let cell_m2 = (map.cell_mm / 1000) * (map.cell_mm / 1000);
    let water: Vec<bool> = map.cells.iter().map(|c| c.water()).collect();
    let mut report = Report::default();
    let mut problems = Vec::new();

    // one lake
    let water_cells = water.iter().filter(|&&w| w).count();
    report.water_m2 = water_cells as i32 * cell_m2;
    let (_, regions) = grid::regions(n, &water, false);
    report.pockets = regions.saturating_sub(1);
    if report.pockets > 0 {
        problems.push(Problem::Pockets(report.pockets));
    }

    // wide channels
    let roomy: Vec<bool> = (0..n * n)
        .map(|i| water[i] && map.shore[i] >= p.roomy_mm())
        .collect();
    let (labels, count) = grid::regions(n, &roomy, false);
    let spawn_cell = map.index(map.spawn.at);
    let main = spawn_cell.map(|i| labels[i]).unwrap_or(0);
    let mut sizes = vec![0usize; count as usize + 1];
    for &l in &labels {
        sizes[l as usize] += 1;
    }
    // nooks of a few cells are shallows, not places a boat is missing
    report.cut_off_cells = (1..=count as usize)
        .filter(|&l| l as u32 != main && sizes[l] >= 12)
        .map(|l| sizes[l])
        .sum();
    if report.cut_off_cells > 0 {
        problems.push(Problem::NarrowChannel {
            area_m2: report.cut_off_cells as i32 * cell_m2,
        });
    }
    let size = map.size_mm();
    let edge_cut = (0..n * n).any(|i| {
        let c = map.centre(i);
        let edge = c.x.min(c.y).min(size - c.x).min(size - c.y);
        water[i] && edge < p.border_mm + map.cell_mm
    });
    if edge_cut {
        problems.push(Problem::EdgeCut);
    }

    // the spawn
    if main == 0 {
        problems.push(Problem::SpawnNotRoomy);
    }
    let s = map.spawn.at;
    let crowded = (0..n * n).any(|i| !water[i] && map.centre(i).dist(s) < p.spawn_clear_mm as i64);
    if crowded {
        problems.push(Problem::SpawnCrowded);
    }
    let (hx, hy) = (
        trig::cos(map.spawn.heading) as i64,
        trig::sin(map.spawn.heading) as i64,
    );
    let mut t = 0;
    while t <= p.spawn_lane_mm.0 {
        let q = P::new(
            s.x + (hx * t as i64 / trig::ONE as i64) as i32,
            s.y + (hy * t as i64 / trig::ONE as i64) as i32,
        );
        let open = map
            .index(q)
            .is_some_and(|i| water[i] && map.shore[i] >= p.spawn_lane_mm.1);
        if !open {
            problems.push(Problem::LaneBlocked { at_mm: t });
            break;
        }
        t += map.cell_mm;
    }

    // islands
    report.islands = map.islands.len();
    if report.islands < p.islands_ok.0 || report.islands > p.islands_ok.1 {
        problems.push(Problem::IslandCount(report.islands));
    }
    for isl in &map.islands {
        if isl.area_m2 < p.island_area_m2.0 || isl.area_m2 > p.island_area_m2.1 {
            problems.push(Problem::IslandSize {
                island: isl.id,
                area_m2: isl.area_m2,
            });
        }
    }
    report.hiding_islands = map
        .islands
        .iter()
        .filter(|i| i.area_m2 >= p.hiding_area_m2)
        .count();
    if report.hiding_islands < p.hiding_islands {
        problems.push(Problem::TooFewHidingIslands(report.hiding_islands));
    }
    // searched only a little beyond the rule, so a far gap reads as that much
    let limit = p.island_gap_mm + 2 * map.cell_mm;
    report.island_gap_mm = limit;
    let all = vec![true; n * n];
    for isl in &map.islands {
        let mine: Vec<usize> = (0..n * n).filter(|&i| map.island[i] == isl.id).collect();
        let d = grid::distances_within(n, map.cell_mm, &mine, &all, limit);
        // the water between: centre to centre, less a cell
        let gap = (0..n * n)
            .filter(|&i| !water[i] && map.island[i] != isl.id && d[i] != FAR)
            .map(|i| d[i] - map.cell_mm)
            .min()
            .unwrap_or(limit);
        report.island_gap_mm = report.island_gap_mm.min(gap);
        if gap < p.island_gap_mm {
            problems.push(Problem::IslandTouches {
                island: isl.id,
                gap_mm: gap,
            });
        }
    }

    // size of the lake
    if report.water_m2 < p.water_m2.0 || report.water_m2 > p.water_m2.1 {
        problems.push(Problem::WaterArea(report.water_m2));
    }
    if let Some(sc) = spawn_cell.filter(|&i| water[i]) {
        let d = grid::distances(n, map.cell_mm, &[sc], &water);
        report.span_mm = d.iter().filter(|&&v| v != FAR).max().copied().unwrap_or(0);
    }
    if report.span_mm < p.span_mm.0 || report.span_mm > p.span_mm.1 {
        problems.push(Problem::Span(report.span_mm));
    }

    // cover in the fog
    report.cover = cover(map, &roomy);
    if report.cover < p.cover.0 {
        problems.push(Problem::TooOpen(report.cover));
    }
    if report.cover > p.cover.1 {
        problems.push(Problem::TooCramped(report.cover));
    }
    let open = (0..n * n)
        .filter(|&i| water[i] && map.shore[i] >= p.open_mm)
        .count();
    report.open_share = (open * 1000 / water_cells.max(1)) as i32;
    if report.open_share < p.open_share {
        problems.push(Problem::LittleOpenWater(report.open_share));
    }

    Verdict { report, problems }
}

/// Of pairs of points on a coarse lattice of roomy water within fog sight
/// of each other, the thousandths whose straight view crosses land.
pub fn cover(map: &Map, roomy: &[bool]) -> i32 {
    let p = map.setting.params();
    let stride = 6;
    let n = map.n;
    let mut pts = Vec::new();
    for y in (stride / 2..n).step_by(stride) {
        for x in (stride / 2..n).step_by(stride) {
            if roomy[y * n + x] {
                pts.push(map.centre(y * n + x));
            }
        }
    }
    let sight2 = p.sight_mm as i64 * p.sight_mm as i64;
    let (mut pairs, mut cut) = (0i64, 0i64);
    for (k, &a) in pts.iter().enumerate() {
        for &b in &pts[k + 1..] {
            if a.dist2(b) > sight2 {
                continue;
            }
            pairs += 1;
            if !clear_view(map, a, b) {
                cut += 1;
            }
        }
    }
    if pairs == 0 {
        return 1000;
    }
    (cut * 1000 / pairs) as i32
}

/// Whether the straight line from `a` to `b` stays over water.
pub fn clear_view(map: &Map, a: P, b: P) -> bool {
    let step = map.cell_mm as i64 / 2;
    let len = a.dist(b);
    let k = (len / step).max(1);
    (0..=k).all(|j| {
        let q = P::new(
            a.x + ((b.x - a.x) as i64 * j / k) as i32,
            a.y + ((b.y - a.y) as i64 * j / k) as i32,
        );
        map.wet(q)
    })
}
