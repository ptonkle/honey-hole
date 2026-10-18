//! Maps: a setting's lake, made from a seed.
//!
//! The setting (the northern Michigan lake) is chosen; the layout comes from
//! a seed. [`generate`] makes a map from a setting and a seed, [`validate`]
//! rejects seeds that would play badly, and [`list`] holds the seeds that
//! passed, pregenerated into `data/maps/<setting>.json`, and picks one for a
//! match. [`fish`] places the fish at match start and [`buoy`] finds a fair
//! point for an item buoy.
//!
//! Units: positions are whole millimetres on the ground plane, `x` east and
//! `y` north, from the map's south-west corner. Heights are millimetres
//! above the lake's surface (water cells are below 0). Angles are binary
//! units, a full turn being [`trig::TURN`]. The map is a square grid of
//! [`Params::cells`] cells of [`Params::cell_mm`] each side; the grid is the
//! description that collision, the radar and the 3D world read.
//!
//! Every number that shapes a setting is in its [`Params`].

pub mod buoy;
pub mod fish;
mod generate;
pub mod grid;
pub mod list;
mod noise;
pub mod rng;
pub mod trig;
pub mod validate;

#[cfg(test)]
mod tests;

pub use generate::generate;

use serde::Serialize;

/// A place, in millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct P {
    pub x: i32,
    pub y: i32,
}

impl P {
    pub fn new(x: i32, y: i32) -> P {
        P { x, y }
    }

    /// Squared distance in mm², which never overflows on a map.
    pub fn dist2(self, o: P) -> i64 {
        let (dx, dy) = ((self.x - o.x) as i64, (self.y - o.y) as i64);
        dx * dx + dy * dy
    }

    pub fn dist(self, o: P) -> i64 {
        trig::isqrt(self.dist2(o))
    }
}

/// A map setting: where the lake is and what grows on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Setting {
    MichiganLake,
}

impl Setting {
    pub const ALL: [Setting; 1] = [Setting::MichiganLake];

    /// The setting's name in data files (`data/maps/<id>.json`).
    pub fn id(self) -> &'static str {
        match self {
            Setting::MichiganLake => "michigan-lake",
        }
    }

    pub fn from_id(id: &str) -> Option<Setting> {
        Setting::ALL.into_iter().find(|s| s.id() == id)
    }

    pub fn params(self) -> &'static Params {
        match self {
            Setting::MichiganLake => &MICHIGAN_LAKE,
        }
    }
}

/// What a cell is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Cell {
    /// Water deeper than the setting's shallows.
    Deep,
    /// Water near the shore, shallow enough for reeds.
    Shallow,
    /// The land around the lake (it reaches the map's edge).
    Land,
    /// The land of an island.
    Island,
}

impl Cell {
    pub fn water(self) -> bool {
        matches!(self, Cell::Deep | Cell::Shallow)
    }

    /// One letter per cell, for the rows of the exported grid.
    pub fn letter(self) -> char {
        match self {
            Cell::Deep => 'D',
            Cell::Shallow => 'S',
            Cell::Land => 'L',
            Cell::Island => 'I',
        }
    }
}

/// An island: one piece of land with water all round.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Island {
    /// 1, 2, ... as in [`Map::island`].
    pub id: u32,
    /// The middle of its cells.
    pub at: P,
    pub area_m2: i32,
}

/// A patch of reeds in the shallows: a habitat fish may use later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReedPatch {
    pub at: P,
    pub radius_mm: i32,
    /// Where each clump of reeds stands.
    pub clumps: Vec<P>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TreeKind {
    Pine,
    Birch,
    Maple,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Tree {
    pub at: P,
    pub kind: TreeKind,
    /// Size against the model's own, in thousandths.
    pub scale: i32,
}

/// Where every boat starts and respawns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Spawn {
    pub at: P,
    /// The way the boats face: across the lake.
    pub heading: i32,
    /// The nearest shore behind the spawn, for a dock or boat launch.
    pub dock: P,
}

/// [`Map::shore`] is measured this far and no farther.
pub const SHORE_REACH_MM: i32 = 120_000;

/// One map: the grid and what stands on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Map {
    pub setting: Setting,
    pub seed: u32,
    /// Cells per side.
    pub n: usize,
    pub cell_mm: i32,
    /// What each cell is, row by row from the south.
    pub cells: Vec<Cell>,
    /// Height of each cell's middle in mm: below 0 is the lake bed.
    pub ground: Vec<i32>,
    /// For a water cell, mm to the nearest land; for land, mm to the
    /// nearest water (between cell middles); at most [`SHORE_REACH_MM`].
    pub shore: Vec<i32>,
    /// Which island a cell belongs to (0: not an island).
    pub island: Vec<u32>,
    pub islands: Vec<Island>,
    pub reeds: Vec<ReedPatch>,
    pub trees: Vec<Tree>,
    pub spawn: Spawn,
}

impl Map {
    /// A map from which cells are water, with the depths and land heights
    /// the shore distances give. The generator then roughens the ground;
    /// tests build small maps this way.
    pub fn from_water(
        setting: Setting,
        seed: u32,
        n: usize,
        cell_mm: i32,
        water: &[bool],
        spawn: Spawn,
    ) -> Map {
        let all = vec![true; n * n];
        let land_cells: Vec<usize> = (0..n * n).filter(|&i| !water[i]).collect();
        let water_cells: Vec<usize> = (0..n * n).filter(|&i| water[i]).collect();
        let reach = SHORE_REACH_MM;
        let to_land = grid::distances_within(n, cell_mm, &land_cells, &all, reach);
        let to_water = grid::distances_within(n, cell_mm, &water_cells, &all, reach);
        let shore: Vec<i32> = (0..n * n)
            .map(|i| if water[i] { to_land[i] } else { to_water[i] }.min(reach))
            .collect();

        // land regions joined across corners; one touching the edge is not an island
        let land: Vec<bool> = water.iter().map(|w| !w).collect();
        let (labels, count) = grid::regions(n, &land, true);
        let mut edge = vec![false; count as usize + 1];
        for i in 0..n * n {
            let (x, y) = (i % n, i / n);
            if x == 0 || y == 0 || x == n - 1 || y == n - 1 {
                edge[labels[i] as usize] = true;
            }
        }
        let mut renumber = vec![0u32; count as usize + 1];
        let mut sums: Vec<(i64, i64, i64)> = Vec::new();
        for (i, &l) in labels.iter().enumerate() {
            let l = l as usize;
            if l == 0 || edge[l] {
                continue;
            }
            if renumber[l] == 0 {
                sums.push((0, 0, 0));
                renumber[l] = sums.len() as u32;
            }
            let s = &mut sums[renumber[l] as usize - 1];
            let c = centre(n, cell_mm, i);
            s.0 += c.x as i64;
            s.1 += c.y as i64;
            s.2 += 1;
        }
        let island: Vec<u32> = labels.iter().map(|&l| renumber[l as usize]).collect();
        let cell_m2 = (cell_mm / 1000) * (cell_mm / 1000);
        let islands = sums
            .iter()
            .enumerate()
            .map(|(k, &(sx, sy, c))| Island {
                id: k as u32 + 1,
                at: P::new((sx / c) as i32, (sy / c) as i32),
                area_m2: c as i32 * cell_m2,
            })
            .collect();
        let params = setting.params();
        let ground = (0..n * n)
            .map(|i| {
                if water[i] {
                    -params.depth_at(shore[i])
                } else {
                    params.land_rise(shore[i])
                }
            })
            .collect();
        let mut map = Map {
            setting,
            seed,
            n,
            cell_mm,
            cells: vec![Cell::Land; n * n],
            ground,
            shore,
            island,
            islands,
            reeds: Vec::new(),
            trees: Vec::new(),
            spawn,
        };
        map.classify();
        map
    }

    /// Sets each cell's kind from its height and island.
    pub fn classify(&mut self) {
        let shallow = self.setting.params().shallow_mm;
        for i in 0..self.n * self.n {
            let g = self.ground[i];
            self.cells[i] = if g < 0 {
                if g >= -shallow {
                    Cell::Shallow
                } else {
                    Cell::Deep
                }
            } else if self.island[i] != 0 {
                Cell::Island
            } else {
                Cell::Land
            };
        }
    }

    /// The map's side in mm.
    pub fn size_mm(&self) -> i32 {
        self.n as i32 * self.cell_mm
    }

    /// The cell holding a point, if it is on the map.
    pub fn index(&self, p: P) -> Option<usize> {
        if p.x < 0 || p.y < 0 || p.x >= self.size_mm() || p.y >= self.size_mm() {
            return None;
        }
        Some((p.y / self.cell_mm) as usize * self.n + (p.x / self.cell_mm) as usize)
    }

    /// The middle of a cell.
    pub fn centre(&self, i: usize) -> P {
        centre(self.n, self.cell_mm, i)
    }

    /// Whether a point is on water (off the map counts as land).
    pub fn wet(&self, p: P) -> bool {
        self.index(p).is_some_and(|i| self.cells[i].water())
    }

    /// The water cells a boat can reach without squeezing through a channel
    /// narrower than the setting's [`Params::min_channel_mm`]: the region of
    /// roomy water holding the spawn, and the water within reach of its edge.
    pub fn boatable(&self) -> Vec<bool> {
        let p = self.setting.params();
        let n = self.n;
        let roomy: Vec<bool> = (0..n * n)
            .map(|i| self.cells[i].water() && self.shore[i] >= p.roomy_mm())
            .collect();
        let (labels, _) = grid::regions(n, &roomy, false);
        let main = self.index(self.spawn.at).map(|i| labels[i]).unwrap_or(0);
        if main == 0 {
            return vec![false; n * n];
        }
        let sources: Vec<usize> = (0..n * n).filter(|&i| labels[i] == main).collect();
        let water: Vec<bool> = self.cells.iter().map(|c| c.water()).collect();
        let reach = grid::distances(n, self.cell_mm, &sources, &water);
        (0..n * n).map(|i| reach[i] <= p.roomy_mm()).collect()
    }

    /// The grid as one string per row, north row first, one letter per cell
    /// ([`Cell::letter`]).
    pub fn rows(&self) -> Vec<String> {
        (0..self.n)
            .rev()
            .map(|y| (0..self.n).map(|x| self.cells[y * self.n + x].letter()).collect())
            .collect()
    }
}

fn centre(n: usize, cell_mm: i32, i: usize) -> P {
    P::new(
        (i % n) as i32 * cell_mm + cell_mm / 2,
        (i / n) as i32 * cell_mm + cell_mm / 2,
    )
}

/// The numbers that shape one setting's maps and judge them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Params {
    // the grid
    pub cells: usize,
    pub cell_mm: i32,
    /// Land kept round the map's edge.
    pub border_mm: i32,

    // the lake
    /// The lake's half-length along its long axis, before the wobble.
    pub half_length_mm: (i32, i32),
    /// Long axis over short, in thousandths.
    pub aspect: (i32, i32),
    /// The largest wobble of the outline for each of 2 to 6 lobes, in
    /// thousandths of the radius.
    pub wobble: [i32; 5],
    /// Arms joined to the main body, and their half-lengths.
    pub lobes: (i32, i32),
    pub lobe_half_mm: (i32, i32),
    /// How far small coves and points push the shore in or out.
    pub ragged_mm: i32,
    pub ragged_wave_mm: i32,
    /// The spawn sits this far from the shore at one end of the lake.
    pub spawn_offshore_mm: i32,
    /// Water pockets cut off from the lake and this small become land (marsh).
    pub pocket_fill_cells: usize,
    /// Land specks this small become water.
    pub speck_cells: usize,

    // islands
    pub island_tries: (i32, i32),
    pub island_radius_mm: (i32, i32),
    /// Water kept between a new island and the shore or another island.
    pub island_room_mm: i32,

    // depth and height
    pub shallow_mm: i32,
    pub max_depth_mm: i32,
    pub hill_mm: i32,

    // reeds and trees
    pub reed_patches: (i32, i32),
    pub reed_radius_mm: (i32, i32),
    pub tree_spacing_mm: i32,
    /// Trees grow only this near the water: the fog hides land beyond.
    pub tree_reach_mm: i32,

    // the validator
    pub min_channel_mm: i32,
    /// Water all round the spawn, for four boats.
    pub spawn_clear_mm: i32,
    /// Open water ahead of the spawn: this long and twice `.1` wide.
    pub spawn_lane_mm: (i32, i32),
    pub islands_ok: (usize, usize),
    pub island_area_m2: (i32, i32),
    /// Islands this big are cover a boat can hide behind; a map needs
    /// `hiding_islands` of them.
    pub hiding_area_m2: i32,
    pub hiding_islands: usize,
    /// Water between an island and other land.
    pub island_gap_mm: i32,
    pub water_m2: (i32, i32),
    /// The longest boat trip from the spawn over water.
    pub span_mm: (i32, i32),
    /// How far a boat sees through the fog.
    pub sight_mm: i32,
    /// Of the pairs of open-water points within sight of each other, the
    /// share (thousandths) whose view is cut by land. Below: too open to
    /// hide. Above: too cramped for the fog to matter.
    pub cover: (i32, i32),
    /// Water at least this far from land is open water; a map needs at
    /// least `open_share` thousandths of its water open.
    pub open_mm: i32,
    pub open_share: i32,
}

impl Params {
    /// Water this far from land is roomy enough for a boat: the middle of a
    /// channel exactly [`Params::min_channel_mm`] wide.
    pub fn roomy_mm(&self) -> i32 {
        (self.min_channel_mm + self.cell_mm) / 2
    }

    /// The lake's depth this far from the shore, before the bed is roughened.
    pub fn depth_at(&self, shore_mm: i32) -> i32 {
        (300 + (shore_mm as i64 * 18 / 100) as i32).min(self.max_depth_mm)
    }

    /// Land's height this far from the water, before the hills.
    pub fn land_rise(&self, shore_mm: i32) -> i32 {
        300 + (shore_mm.min(200_000) as i64 * 6 / 100) as i32
    }
}

/// The first setting: a northern Michigan inland lake.
pub const MICHIGAN_LAKE: Params = Params {
    cells: 192,
    cell_mm: 4000,
    border_mm: 40_000,
    half_length_mm: (225_000, 265_000),
    aspect: (1250, 1700),
    wobble: [110, 100, 70, 50, 35],
    lobes: (2, 4),
    lobe_half_mm: (70_000, 140_000),
    ragged_mm: 22_000,
    ragged_wave_mm: 72_000,
    spawn_offshore_mm: 30_000,
    pocket_fill_cells: 60,
    speck_cells: 6,
    island_tries: (3, 7),
    island_radius_mm: (10_000, 32_000),
    island_room_mm: 38_000,
    shallow_mm: 1800,
    max_depth_mm: 15_000,
    hill_mm: 14_000,
    reed_patches: (4, 7),
    reed_radius_mm: (7_000, 14_000),
    tree_spacing_mm: 7000,
    tree_reach_mm: 110_000,
    min_channel_mm: 12_000,
    spawn_clear_mm: 12_000,
    spawn_lane_mm: (120_000, 16_000),
    islands_ok: (2, 6),
    island_area_m2: (100, 7000),
    hiding_area_m2: 600,
    hiding_islands: 2,
    island_gap_mm: 20_000,
    water_m2: (120_000, 300_000),
    span_mm: (440_000, 640_000),
    sight_mm: 120_000,
    cover: (70, 400),
    open_mm: 24_000,
    open_share: 450,
};
