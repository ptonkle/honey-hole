//! The lake's look: `"lake"` in `data/look.json`. Every colour, share and
//! distance the world is drawn with lives here, so the look changes without
//! touching a rule. Colours are 0–255; distances are millimetres; shares
//! are percent unless named otherwise.

use serde::Deserialize;

use crate::gfx::render::Light;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LakeLook {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// the sun and fill; `fog` is the horizon's colour
    pub light: Light,
    /// the sky overhead
    pub zenith: [i32; 3],
    /// the view direction's height (hundredths) where the sky starts to
    /// turn from the horizon's colour, and where it is all the zenith's
    pub sky_band: [i32; 2],
    /// the fog starts at this share of the map's sight distance and is
    /// whole at that share
    pub fog_start_pct: i32,
    pub fog_end_pct: i32,
    pub ground: GroundLook,
    pub water: WaterLook,
    pub under: UnderLook,
    pub props: PropsLook,
    pub camera: CameraLook,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundLook {
    /// open grass, and a second green the noise blends it toward
    pub grass: [i32; 3],
    pub meadow: [i32; 3],
    /// the floor under the trees
    pub forest: [i32; 3],
    pub sand: [i32; 3],
    pub rock: [i32; 3],
    /// the lake bed by the shore and in deep water, blended over `bed_deep_mm`
    pub bed_shallow: [i32; 3],
    pub bed_deep: [i32; 3],
    pub bed_deep_mm: i32,
    /// a face lower than this above the water is beach
    pub sand_top_mm: i32,
    /// a face whose normal's height is under this (hundredths) is rock
    pub rock_slope_pct: i32,
    /// each face's colour is brightened or darkened by up to this
    pub jitter_pct: i32,
    /// a cell with this many trees within two cells of it is all forest
    /// floor; fewer blend it toward grass
    pub forest_trees: i32,
    /// the bumps between grass and meadow are about this far apart
    pub meadow_mm: i32,
    /// the map's edge drops this far, in this colour
    pub skirt_mm: i32,
    pub skirt: [i32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaterLook {
    /// by the shore and over deep water, blended over `deep_mm` of depth
    pub shallow: [i32; 3],
    pub deep: [i32; 3],
    pub deep_mm: i32,
    /// the surface's opacity looking straight down and at a grazing angle;
    /// the power (hundredths) that blends them
    pub alpha_down_pct: i32,
    pub alpha_grazing_pct: i32,
    pub fresnel_pow_pct: i32,
    /// the sun's glint and its sharpness (a power)
    pub glint: [i32; 3],
    pub shine: i32,
    /// the surface reaches under land up to this high, so waves meet the
    /// shore with no gap
    pub shore_overlap_mm: i32,
    /// exactly two waves
    pub waves: Vec<WaveLook>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaveLook {
    pub length_mm: i32,
    /// the way it travels, degrees from east toward north
    pub toward_deg: i32,
    /// from its middle to its crest
    pub height_mm: i32,
    pub period_ms: i32,
}

/// What lies under the surface takes on the water's colour with depth.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnderLook {
    pub colour: [i32; 3],
    /// at this depth the tint is whole
    pub clear_mm: i32,
    /// the tint at its strongest
    pub strength_pct: i32,
}

/// Which model stands for each thing the map places, and how many extras
/// the scene scatters. Model names are ids in `data/models/`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropsLook {
    /// the baked frames of a sway or bob, blended between
    pub frames: u32,
    /// the models for each kind of map tree; one is picked per tree
    pub pine: Vec<String>,
    pub birch: Vec<String>,
    pub maple: Vec<String>,
    pub reeds: String,
    pub lily_pads: String,
    /// lily pads per clump of reeds, in percent
    pub lily_pct: i32,
    /// lily pads float in water this deep
    pub lily_depth_mm: [i32; 2],
    pub shore_rocks: String,
    /// a share of shore cells (permille) holds rocks
    pub shore_rocks_permille: i32,
    pub boulder: String,
    /// a share of shallow cells (permille) holds a boulder
    pub boulder_permille: i32,
    pub log: String,
    /// a share of forest cells (permille) holds a fallen log
    pub log_permille: i32,
    pub dock: String,
    pub cabin: String,
    pub cabins: u32,
    /// a cabin stands this far back from the water
    pub cabin_back_mm: i32,
    /// no tree stands this near a cabin, or this near the dock
    pub cabin_clear_mm: i32,
    pub dock_clear_mm: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraLook {
    pub fov_deg: i32,
    pub near_mm: i32,
    /// the far plane with the fog off
    pub far_mm: i32,
    /// the boat camera: behind and above the boat, aimed this high over it
    pub back_mm: i32,
    pub up_mm: i32,
    pub aim_mm: i32,
    /// its pitch down, degrees, before the mouse moves it
    pub pitch_deg: i32,
    /// the eye stays this far over the ground and the water
    pub clear_mm: i32,
    /// the tour's speed, as a boat's
    pub tour_mm_per_s: i32,
    /// the overview stands this high over the lake's middle
    pub above_mm: i32,
    pub above_pitch_deg: i32,
}

impl LakeLook {
    pub fn check(&self) -> Result<(), String> {
        self.light.check()?;
        let g = &self.ground;
        let w = &self.water;
        let colours = [
            self.zenith,
            g.grass,
            g.meadow,
            g.forest,
            g.sand,
            g.rock,
            g.bed_shallow,
            g.bed_deep,
            g.skirt,
            w.shallow,
            w.deep,
            w.glint,
            self.under.colour,
        ];
        if let Some(c) = colours.iter().find(|c| c.iter().any(|v| !(0..=255).contains(v))) {
            return Err(format!("A lake colour {c:?} is outside 0 to 255."));
        }
        let shares = [
            ("fog_start_pct", self.fog_start_pct),
            ("fog_end_pct", self.fog_end_pct),
            ("alpha_down_pct", w.alpha_down_pct),
            ("alpha_grazing_pct", w.alpha_grazing_pct),
            ("strength_pct", self.under.strength_pct),
            ("jitter_pct", g.jitter_pct),
            ("rock_slope_pct", g.rock_slope_pct),
        ];
        if let Some((name, _)) = shares.iter().find(|(_, v)| !(0..=100).contains(v)) {
            return Err(format!("The lake's {name} is outside 0 to 100."));
        }
        if self.fog_start_pct >= self.fog_end_pct {
            return Err("The lake's fog must start before it ends.".into());
        }
        let lengths = [
            ("bed_deep_mm", g.bed_deep_mm),
            ("meadow_mm", g.meadow_mm),
            ("deep_mm", w.deep_mm),
            ("clear_mm", self.under.clear_mm),
            ("fresnel_pow_pct", w.fresnel_pow_pct),
            ("shine", w.shine),
            ("near_mm", self.camera.near_mm),
            ("tour_mm_per_s", self.camera.tour_mm_per_s),
        ];
        if let Some((name, _)) = lengths.iter().find(|(_, v)| *v <= 0) {
            return Err(format!("The lake's {name} must be more than 0."));
        }
        if w.waves.len() != 2 {
            return Err("The lake's water needs exactly two waves.".into());
        }
        if w.waves.iter().any(|v| v.length_mm <= 0 || v.period_ms <= 0) {
            return Err("A wave needs a length and a period.".into());
        }
        if self.props.frames == 0 {
            return Err("The props need at least one frame.".into());
        }
        if self.props.pine.is_empty() || self.props.birch.is_empty() || self.props.maple.is_empty() {
            return Err("Each kind of tree needs a model.".into());
        }
        Ok(())
    }

    /// Every model the lake places, by id.
    pub fn model_ids(&self) -> Vec<&str> {
        let p = &self.props;
        let mut out: Vec<&str> = p.pine.iter().chain(&p.birch).chain(&p.maple).map(|s| s.as_str()).collect();
        out.extend([
            p.reeds.as_str(),
            p.lily_pads.as_str(),
            p.shore_rocks.as_str(),
            p.boulder.as_str(),
            p.log.as_str(),
            p.dock.as_str(),
            p.cabin.as_str(),
        ]);
        out.sort();
        out.dedup();
        out
    }
}
