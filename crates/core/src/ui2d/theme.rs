//! The 2D layer's theme, from `data/ui2d.json`: the fonts, the colours of
//! the Fish-Boy's screens and the tiers, the Fish-Boy's raised view, and
//! the stand-in lake behind it. Layout (where things sit on a screen) is
//! the core's; the look is data.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::{Font, Rgba};
use crate::gfx::render::Light;
use crate::map::fish::Tier;

pub type C = [i32; 3];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// font roles by name; "mono" is the one every screen uses
    pub fonts: BTreeMap<String, Font>,
    pub hud: Hud,
    pub fishboy: FishBoyLook,
    pub screen: ScreenLook,
    pub radar: RadarLook,
    pub tiers: Tiers,
    pub fishpedia: FishpediaLook,
    pub standin: Standin,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hud {
    /// the HUD's virtual canvas, letterboxed into the window
    pub canvas: [i32; 2],
    pub ink: C,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishBoyLook {
    /// the screen's virtual canvas (the screen part is 16:9)
    pub canvas: [i32; 2],
    /// the first-person camera's vertical field of view, degrees
    pub fov_deg: i32,
    /// how much of the view the screen spans, percent of the width or
    /// height, whichever binds (the brief: 80)
    pub screen_pct: i32,
    /// raising or lowering the device takes this long
    pub raise_ms: i32,
    /// the light on the device and in the Fishpedia's boxes
    pub light: Light,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScreenLook {
    pub background: C,
    /// the bright phosphor green of text and lines
    pub phosphor: C,
    /// a dimmer green for rules and unpicked tabs
    pub dim: C,
    /// the dark ink on a picked tab
    pub ink: C,
    /// the scanlines: every this many units, this dark (percent opacity)
    pub scanline_every: i32,
    pub scanline_pct: i32,
    pub warning: C,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadarLook {
    pub panel: C,
    pub land: C,
    pub shallow: C,
    pub deep: C,
    pub shore: C,
    /// water deeper than this (mm) is drawn as deep
    pub deep_mm: i32,
    /// one block of the map picture, in mm (2.5 px of the first preview,
    /// at 4 px per 4 m cell: the person's 80% fidelity)
    pub block_mm: i32,
    /// the darker rows of the map picture, every other row, percent
    pub scan_pct: i32,
    pub grid: C,
    pub grid_lines: i32,
    pub ring: C,
    /// range rings round your boat, in metres apart
    pub ring_m: i32,
    pub sweep: C,
    /// the sweep's soft band, in metres
    pub sweep_band_m: i32,
    pub sweep_pct: i32,
    /// the players' colours on the radar, brighter than the boats', by
    /// `PLAYER_COLOURS` name
    pub players: BTreeMap<String, C>,
    pub label_back: C,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TierLook {
    pub colour: C,
    /// the frame's shape, so a tier never shows by colour alone:
    /// "double", "notched", "dashed" or "plain"
    pub frame: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tiers {
    pub legendary: TierLook,
    pub rare: TierLook,
    pub uncommon: TierLook,
    pub common: TierLook,
}

impl Tiers {
    pub fn get(&self, t: Tier) -> &TierLook {
        match t {
            Tier::Legendary => &self.legendary,
            Tier::Rare => &self.rare,
            Tier::Uncommon => &self.uncommon,
            Tier::Common => &self.common,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishpediaLook {
    /// a fish's box (the brief's wireframe draws it blue)
    pub box_colour: C,
    pub box_line: C,
    /// a fish with none left is drawn this dim, percent
    pub gone_pct: i32,
    /// the outline drawn for a fish whose model is not made yet
    pub silhouette: C,
}

/// The plain lake drawn behind the Fish-Boy until the world renderer
/// draws the real one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Standin {
    pub water: C,
    pub land: C,
    pub light: Light,
    /// the fog's start and end, metres
    pub fog_m: [i32; 2],
}

pub const FRAMES: [&str; 4] = ["double", "notched", "dashed", "plain"];

fn colour_ok(name: &str, c: C) -> Result<(), String> {
    if c.iter().any(|v| !(0..=255).contains(v)) {
        return Err(format!("The ui2d colour \"{name}\" is outside 0 to 255."));
    }
    Ok(())
}

impl Theme {
    pub fn parse(json: &str) -> Result<Theme, String> {
        let t: Theme =
            serde_json::from_str(json).map_err(|e| format!("The ui2d file could not be read: {e}"))?;
        t.check()?;
        Ok(t)
    }

    fn check(&self) -> Result<(), String> {
        if !self.fonts.contains_key("mono") {
            return Err("The ui2d file has no font \"mono\".".into());
        }
        for (name, f) in &self.fonts {
            if !(20..=120).contains(&f.advance) {
                return Err(format!("The font \"{name}\" has an advance outside 20 to 120."));
            }
        }
        let s = &self.screen;
        let r = &self.radar;
        let named = [
            ("hud.ink", self.hud.ink),
            ("screen.background", s.background),
            ("screen.phosphor", s.phosphor),
            ("screen.dim", s.dim),
            ("screen.ink", s.ink),
            ("screen.warning", s.warning),
            ("radar.panel", r.panel),
            ("radar.land", r.land),
            ("radar.shallow", r.shallow),
            ("radar.deep", r.deep),
            ("radar.shore", r.shore),
            ("radar.grid", r.grid),
            ("radar.ring", r.ring),
            ("radar.sweep", r.sweep),
            ("radar.label_back", r.label_back),
            ("fishpedia.box_colour", self.fishpedia.box_colour),
            ("fishpedia.box_line", self.fishpedia.box_line),
            ("fishpedia.silhouette", self.fishpedia.silhouette),
            ("standin.water", self.standin.water),
            ("standin.land", self.standin.land),
        ];
        for (name, c) in named {
            colour_ok(name, c)?;
        }
        for name in crate::app::gallery::PLAYER_COLOURS {
            match r.players.get(name) {
                Some(c) => colour_ok(name, *c)?,
                None => return Err(format!("The ui2d file has no radar colour \"{name}\".")),
            }
        }
        for (name, t) in [
            ("legendary", &self.tiers.legendary),
            ("rare", &self.tiers.rare),
            ("uncommon", &self.tiers.uncommon),
            ("common", &self.tiers.common),
        ] {
            colour_ok(name, t.colour)?;
            if !FRAMES.contains(&t.frame.as_str()) {
                return Err(format!("The tier \"{name}\" has no frame shape \"{}\".", t.frame));
            }
        }
        let f = &self.fishboy;
        if f.canvas.iter().any(|v| *v <= 0) || self.hud.canvas.iter().any(|v| *v <= 0) {
            return Err("A ui2d canvas must have a size.".into());
        }
        if !(10..=120).contains(&f.fov_deg) || !(10..=100).contains(&f.screen_pct) || f.raise_ms <= 0 {
            return Err("The Fish-Boy's view needs a field of view, a screen share and a raise.".into());
        }
        if r.block_mm <= 0 || r.ring_m <= 0 || r.grid_lines <= 0 || s.scanline_every <= 0 {
            return Err("The radar's blocks, rings, grid and scanlines need a size.".into());
        }
        f.light.check()?;
        self.standin.light.check()?;
        Ok(())
    }

    pub fn mono(&self) -> &Font {
        &self.fonts["mono"]
    }
}

pub fn c(rgb: C) -> Rgba {
    Rgba::solid(rgb)
}
